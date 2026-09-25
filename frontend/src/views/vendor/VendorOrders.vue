<!--
  摊主 · 订单（spec §5.5）：待处理 / 已完成两个分段、收款弹窗、退货弹窗。
  从旧 `VendorView.vue` 原样搬来；「手动刷新」改成页内的刷新图标按钮，调外壳的
  `refresh`（轮询与提示音都在外壳里）。「已退完」置灰留给 Task 4 P3 用
  `refunded_qty` 做，这里不预取。
-->
<template>
  <PageShell embedded width="wide">
    <div class="order-column">
      <div class="order-toolbar">
        <n-button
          text
          size="small"
          class="refresh-btn"
          :loading="isRefreshing"
          title="刷新订单"
          @click="manualRefresh"
        >
          <n-icon><RefreshOutline /></n-icon>
          刷新
        </n-button>
      </div>

      <n-alert
        v-if="store.pendingOrders.length"
        type="warning"
        :bordered="false"
        style="margin-bottom: var(--space-md)"
      >
        有 {{ store.pendingOrders.length }} 条待处理订单，请及时处理。
      </n-alert>

      <div class="order-tabs">
        <n-tabs v-model:value="currentTab" type="line" animated>
          <n-tab-pane :name="'pending'" :tab="'待处理 (' + store.pendingOrders.length + ')'" />
          <n-tab-pane name="completed" tab="已完成" />
        </n-tabs>
      </div>

      <div v-show="currentTab === 'pending'" class="order-feed">
        <EmptyState
          v-if="!store.pendingOrders.length"
          icon="📭"
          title="暂无待处理订单"
          desc="新订单将自动出现，并伴有声音提醒"
        />
        <TransitionGroup name="list" tag="div">
          <OrderCard
            v-for="order in store.pendingOrders"
            :key="order.id"
            :order="order"
            @complete="completeOrder"
            @cancel="cancelOrder"
          />
        </TransitionGroup>
      </div>

      <div v-show="currentTab === 'completed'" class="order-feed">
        <p class="revenue-summary">
          今日已完成订单总额: <strong>{{ formatYuan(store.totalRevenue) }}</strong>
        </p>
        <EmptyState v-if="!store.completedOrders.length" icon="" title="暂无已完成订单" />
        <!-- OrderCard 本身不动（④ 要整体重做），只在外面补一个「退货」入口。
             不做「已退完」置灰预取：那要为每张已完成单各发一个请求，400 单的场次
             会把 3 秒一次的待处理轮询挤在浏览器连接队列后面。退货弹窗里每行
             本来就标了「已退完」；④ 重做列表时会从批量查询带回这个标记。 -->
        <div v-for="order in store.completedOrders" :key="order.id" class="completed-entry">
          <OrderCard :order="order" :is-completed="true" />
          <div class="completed-entry-actions">
            <n-button size="small" secondary @click="openRefund(order)">退货</n-button>
          </div>
        </div>
      </div>
    </div>

    <!-- 完成配货前先确认收款：显示原价/应收/已套用的套装（可逐个拆），实收可改（spec 4.3） -->
    <ReceiptModal
      :show="showReceiptModal"
      :gross-amount="pendingOrder?.gross_amount"
      :solved-amount="pendingOrder?.solved_amount"
      :lots="pendingOrder?.lots ?? []"
      @confirm="onReceiptConfirm"
      @cancel="closeReceipt"
    />

    <!-- 退货：逐行退（同商品可能拆多行），金额与去向在弹窗里定 -->
    <RefundModal
      :show="showRefundModal"
      :event-id="props.id"
      :order="refundOrder"
      @close="closeRefund"
    />
  </PageShell>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NAlert, NButton, NIcon, NTabs, NTabPane } from 'naive-ui'
import { RefreshOutline } from '@vicons/ionicons5'
import { useOrderStore } from '@/stores/orderStore'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import { PageShell, EmptyState } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import { useVendorPollingContext } from '@/composables/useVendorPolling'
import OrderCard from '@/components/order/OrderCard.vue'
import ReceiptModal from '@/components/vendor/ReceiptModal.vue'
import RefundModal from '@/components/vendor/RefundModal.vue'
import { formatYuan, type Cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

const props = defineProps<{ id: string | number }>()

const store = useOrderStore()
const eventDetailStore = useEventDetailStore()
const fb = useFeedback()
const { refresh } = useVendorPollingContext()

const isRefreshing = ref(false)
const currentTab = ref('pending')

async function manualRefresh() {
  isRefreshing.value = true
  try {
    await refresh()
  } finally {
    isRefreshing.value = false
  }
}

// 点「完成配货」先确认收款，确认了才真正调接口。
const showReceiptModal = ref(false)
const pendingOrder = ref<Schemas['OrderResponse'] | null>(null)

function completeOrder(orderId: number) {
  pendingOrder.value = store.pendingOrders.find((o) => o.id === orderId) || null
  showReceiptModal.value = true
}

function closeReceipt() {
  showReceiptModal.value = false
  pendingOrder.value = null
}

async function onReceiptConfirm(payload: {
  channel: string
  finalAmount: Cents
  unapplyLotIds: number[]
}) {
  const order = pendingOrder.value
  showReceiptModal.value = false
  if (!order) return
  try {
    await store.markOrderAsCompleted(
      order.id,
      payload.channel,
      payload.finalAmount,
      payload.unapplyLotIds
    )
    await eventDetailStore.fetchProductsForEvent(Number(props.id))
    await store.fetchCompletedOrders()
    fb.success('已记录收款')
  } catch (error) {
    fb.error(error, '操作失败')
  } finally {
    pendingOrder.value = null
  }
}

async function cancelOrder(orderId: number) {
  await fb.confirm({
    title: '确认取消',
    content: '确定要取消这个订单吗？此操作无法撤销。',
    positiveText: '确认',
    negativeText: '返回',
    danger: true,
    onConfirm: async () => {
      try {
        await store.cancelOrder(orderId)
        fb.success('订单已取消')
      } catch (error) {
        fb.error(error, '取消失败')
      }
    },
  })
}

// ===== 退货 =====
const showRefundModal = ref(false)
const refundOrder = ref<Schemas['OrderResponse'] | null>(null)

function openRefund(order: Schemas['OrderResponse']) {
  refundOrder.value = order
  showRefundModal.value = true
}

function closeRefund() {
  showRefundModal.value = false
  refundOrder.value = null
}
</script>

<style scoped>
.order-column {
  min-width: 0;
}

.order-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: var(--space-xs);
}

.order-tabs {
  margin-bottom: var(--space-md);
}

/* 已完成单的「退货」入口。OrderCard 本身不动（④ 要整体重做），
   只在卡片外面加一行按钮。 */
.completed-entry-actions {
  display: flex;
  justify-content: flex-end;
  margin: calc(-1 * var(--space-xs)) 0 var(--space-sm);
}

.revenue-summary {
  text-align: right;
  font-size: var(--font-md);
  margin-bottom: var(--space-md);
  color: var(--primary-text-color);
}
.revenue-summary strong {
  color: var(--accent-color);
}

/* ===== 订单进出动画 ===== */
.list-enter-active,
.list-leave-active {
  transition: all 0.25s ease;
}
.list-enter-from {
  opacity: 0;
  transform: translateY(-12px);
}
.list-leave-to {
  opacity: 0;
  transform: translateX(30px);
}
</style>
