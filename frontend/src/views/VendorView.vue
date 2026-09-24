<template>
  <PageShell title="待处理订单" width="wide">
    <template #subtitle>
      <template v-if="eventName">
        当前展会: <strong>{{ eventName }}</strong>
      </template>
      <template v-else>正在加载展会信息...</template>
    </template>
    <template #actions>
      <router-link to="/admin" class="back-link">← 管理后台</router-link>
      <div class="header-actions">
        <n-button @click="showInventoryModal = true">登记赠送/报废</n-button>
        <n-button @click="openClosing">
          {{ isEventSettled ? '查看收摊状态' : '收摊' }}
        </n-button>
        <n-button type="primary" :loading="isRefreshing" @click="manualRefresh">
          {{ isRefreshing ? '刷新中' : '手动刷新' }}
        </n-button>
      </div>
    </template>

    <main class="vendor-body">
      <!-- 左栏：订单 -->
      <div class="order-column">
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

      <!-- 右栏：库存统计（宽屏时显示为侧栏） -->
      <div class="stats-column">
        <LiveStats class="live-stats-module" :event-id="props.id" />
      </div>
    </main>

    <!-- 隐藏的音频播放器，引用 public 目录下的 notify.mp3 -->
    <audio ref="audioRef" src="/notify.mp3" preload="auto"></audio>

    <!-- 完成配货前先确认收款：显示原价/应收/已套用的套装（可逐个拆），实收可改（spec 4.3） -->
    <ReceiptModal
      :show="showReceiptModal"
      :gross-amount="pendingOrder?.gross_amount"
      :solved-amount="pendingOrder?.solved_amount"
      :lots="pendingOrder?.lots ?? []"
      @confirm="onReceiptConfirm"
      @cancel="closeReceipt"
    />

    <!-- 赠送/报废登记：商品候选复用收摊接口的现场仓余额，登记成功刷新库存统计 -->
    <InventoryLogModal
      :show="showInventoryModal"
      :event-id="props.id"
      @close="showInventoryModal = false"
      @logged="onInventoryLogged"
    />

    <!-- 退货：逐行退（同商品可能拆多行），金额与去向在弹窗里定 -->
    <RefundModal
      :show="showRefundModal"
      :event-id="props.id"
      :order="refundOrder"
      @close="closeRefund"
    />

    <!-- 收摊向导：第几步由后端状态推出来，详见组件注释 -->
    <ClosingWizard
      :show="showClosingWizard"
      :event-id="props.id"
      @close="showClosingWizard = false"
      @settled="onClosingSettled"
    />
  </PageShell>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { NButton, NTabs, NTabPane, NAlert } from 'naive-ui'
import { useOrderStore } from '@/stores/orderStore'
import { useEventStore } from '@/stores/eventStore'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import { PageShell, EmptyState } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import LiveStats from '@/components/vendor/LiveStats.vue'
import OrderCard from '@/components/order/OrderCard.vue'
import ReceiptModal from '@/components/vendor/ReceiptModal.vue'
import InventoryLogModal from '@/components/vendor/InventoryLogModal.vue'
import RefundModal from '@/components/vendor/RefundModal.vue'
import ClosingWizard from '@/components/vendor/ClosingWizard.vue'
import { formatYuan, type Cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

const props = defineProps<{ id: string | number }>()

const audioRef = ref<HTMLAudioElement | null>(null)
const store = useOrderStore()
const eventStore = useEventStore()
const eventDetailStore = useEventDetailStore()
const fb = useFeedback()

const isRefreshing = ref(false)
const currentTab = ref('pending')
const isInitialized = ref(false) // 用于标记第一次加载，避免页面一打开就响

const eventName = computed(() => {
  const event = eventStore.events.find((e) => e.id === parseInt(String(props.id), 10))
  return event ? event.name : `展会 #${props.id}`
})

const isEventSettled = computed(() => {
  const event = eventStore.events.find((e) => e.id === parseInt(String(props.id), 10))
  return event?.status === '已结算'
})

// 播放声音的函数
const playNoticeSound = () => {
  if (audioRef.value) {
    audioRef.value.currentTime = 0
    // 现代浏览器要求用户必须点击过页面后才能自动播放声音
    audioRef.value.play().catch((err) => {
      console.warn('音频播放尝试失败（用户尚未与页面交互或文件路径不正确）:', err)
    })
  }
}

async function manualRefresh() {
  isRefreshing.value = true
  // 顺便在这里尝试播放一下声音，让浏览器”解锁”音频播放权限
  playNoticeSound()

  try {
    await Promise.all([
      store.pollPendingOrders(),
      eventDetailStore.fetchProductsForEvent(Number(props.id)),
      store.fetchCompletedOrders(),
    ])
  } catch (err) {
    console.error('手动刷新失败:', err)
  } finally {
    isRefreshing.value = false
  }
}

// 核心逻辑：监听订单数量变化
watch(
  () => store.pendingOrders.length,
  (newCount, oldCount) => {
    // 只有当数量增加，且不是第一次初始化加载时才响铃
    if (isInitialized.value && newCount > oldCount) {
      playNoticeSound()
      fb.info('收到新订单！', { keepAliveOnHover: true })
    }

    // 首次加载后标记为已初始化
    if (!isInitialized.value && newCount !== undefined) {
      isInitialized.value = true
    }
  }
)

// 点「完成配货」先确认收款，确认了才真正调接口。
const showReceiptModal = ref(false)
const pendingOrder = ref<Schemas['OrderResponse'] | null>(null)

// 赠送/报废登记弹窗。登记会动现场仓余额，成功后刷一次库存统计。
const showInventoryModal = ref(false)

async function onInventoryLogged() {
  await eventDetailStore.fetchProductsForEvent(Number(props.id))
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

// ===== 收摊向导 =====
const showClosingWizard = ref(false)

function openClosing() {
  // 已结算的展会也进同一个向导：向导的第 4 屏（step 由后端状态推出）会停在
  // 「账本已冻结」，并写清之后还能补什么、结算单去哪儿看。**不要跳管理端**——
  // /admin/** 要求 admin 角色，而能站在本页的会话是 vendor，两者互斥，跳过去
  // 只会被重定向到 /login/admin。
  showClosingWizard.value = true
}

/** 向导里结算成功后，头部按钮当场变成「查看收摊状态」，不用等重新拉展会列表。 */
function onClosingSettled() {
  const event = eventStore.events.find((e) => e.id === parseInt(String(props.id), 10))
  if (event) event.status = '已结算'
  store.fetchCompletedOrders()
}

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

onMounted(() => {
  if (eventStore.events.length === 0) {
    eventStore.fetchEvents()
  }
  store.setActiveEvent(props.id)
  eventDetailStore.fetchProductsForEvent(Number(props.id))
})

onUnmounted(() => {
  store.stopPolling()
})
</script>

<style scoped>
/* ===== Body: 自适应双栏 ===== */
.vendor-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}

.order-column {
  flex: 1;
  min-width: 0;
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

.back-link {
  font-size: var(--font-sm);
  color: var(--text-muted);
  text-decoration: none;
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-pill);
  border: 1px solid var(--border-color);
  transition: all 0.15s;
  white-space: nowrap;
}
.back-link:hover {
  background: var(--accent-color);
  color: var(--text-white);
  border-color: var(--accent-color);
}

.header-actions {
  display: flex;
  gap: var(--space-sm);
  flex-wrap: wrap;
  justify-content: flex-end;
}

/* ===== 宽屏双栏 (平板/电脑) ===== */
@media (--not-phone) {
  .vendor-body {
    flex-direction: row;
    align-items: flex-start;
  }

  .order-column {
    flex: 1;
  }

  .stats-column {
    flex: 0 0 300px;
    position: sticky;
    top: 80px;
    max-height: calc(100vh - 100px);
    overflow-y: auto;
  }
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
