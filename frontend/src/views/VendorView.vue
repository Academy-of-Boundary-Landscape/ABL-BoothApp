<template>
  <div class="vendor-view">
    <header class="page-header">
      <div class="header-content">
        <div class="header-title-row">
          <h1>待处理订单</h1>
          <router-link to="/admin" class="back-link">← 管理后台</router-link>
        </div>
        <p v-if="eventName">
          当前展会: <strong>{{ eventName }}</strong>
        </p>
        <p v-else>正在加载展会信息...</p>
      </div>
      <div class="header-actions">
        <n-button @click="showInventoryModal = true">登记赠送/报废</n-button>
        <n-button @click="openClosing">
          {{ isEventSettled ? '查看结算' : '收摊' }}
        </n-button>
        <n-button type="primary" :loading="isRefreshing" @click="manualRefresh">
          {{ isRefreshing ? '刷新中' : '手动刷新' }}
        </n-button>
      </div>
    </header>

    <main class="vendor-body">
      <!-- 左栏：订单 -->
      <div class="order-column">
        <n-alert
          v-if="store.pendingOrders.length"
          type="warning"
          :bordered="false"
          style="margin-bottom: 0.75rem"
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
          <div v-if="!store.pendingOrders.length" class="no-orders-message">
            <span style="font-size: 2rem; display: block; margin-bottom: 0.5rem">📭</span>
            <p>暂无待处理订单</p>
            <p style="font-size: var(--font-sm); color: var(--text-disabled)">
              新订单将自动出现，并伴有声音提醒
            </p>
          </div>
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
          <div v-if="!store.completedOrders.length" class="no-orders-message">暂无已完成订单</div>
          <!-- OrderCard 本身不动（④ 要整体重做），只在外面补一个「退货」入口。
               已全额退完的单按钮置灰；标志来自每单退货接口的 lines（没有批量端点）。 -->
          <div v-for="order in store.completedOrders" :key="order.id" class="completed-entry">
            <OrderCard :order="order" :is-completed="true" />
            <div class="completed-entry-actions">
              <n-button
                size="small"
                secondary
                :disabled="fullyRefundedOrders.has(order.id)"
                @click="openRefund(order)"
              >
                {{ fullyRefundedOrders.has(order.id) ? '已退完' : '退货' }}
              </n-button>
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
      :gross-amount="pendingOrder?.gross_amount ?? 0"
      :solved-amount="pendingOrder?.solved_amount ?? 0"
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
      @loaded="onRefundFlag"
      @refunded="onRefundFlag"
    />

    <!-- 收摊向导：第几步由后端状态推出来，详见组件注释 -->
    <ClosingWizard
      :show="showClosingWizard"
      :event-id="props.id"
      @close="showClosingWizard = false"
      @settled="onClosingSettled"
    />
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { NButton, NTabs, NTabPane, NAlert, useDialog, useMessage } from 'naive-ui'
import { useRouter } from 'vue-router'
import { useOrderStore } from '@/stores/orderStore'
import { useEventStore } from '@/stores/eventStore'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import LiveStats from '@/components/vendor/LiveStats.vue'
import OrderCard from '@/components/order/OrderCard.vue'
import ReceiptModal from '@/components/vendor/ReceiptModal.vue'
import InventoryLogModal from '@/components/vendor/InventoryLogModal.vue'
import RefundModal from '@/components/vendor/RefundModal.vue'
import ClosingWizard from '@/components/vendor/ClosingWizard.vue'
import { formatYuan } from '@/utils/money'
import api from '@/services/api'

const props = defineProps({
  id: { type: String, required: true },
})

const audioRef = ref(null)
const store = useOrderStore()
const eventStore = useEventStore()
const eventDetailStore = useEventDetailStore()
const router = useRouter()
const message = useMessage()
const dialog = useDialog()

const isRefreshing = ref(false)
const currentTab = ref('pending')
const isInitialized = ref(false) // 用于标记第一次加载，避免页面一打开就响

const eventName = computed(() => {
  const event = eventStore.events.find((e) => e.id === parseInt(props.id, 10))
  return event ? event.name : `展会 #${props.id}`
})

const isEventSettled = computed(() => {
  const event = eventStore.events.find((e) => e.id === parseInt(props.id, 10))
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
      eventDetailStore.fetchProductsForEvent(props.id),
      store.fetchCompletedOrders(),
    ])
    await refreshRefundFlags()
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
      message.info('收到新订单！', { keepAliveOnHover: true })
    }

    // 首次加载后标记为已初始化
    if (!isInitialized.value && newCount !== undefined) {
      isInitialized.value = true
    }
  }
)

// 点「完成配货」先确认收款，确认了才真正调接口。
const showReceiptModal = ref(false)
const pendingOrder = ref(null)

// 赠送/报废登记弹窗。登记会动现场仓余额，成功后刷一次库存统计。
const showInventoryModal = ref(false)

async function onInventoryLogged() {
  await eventDetailStore.fetchProductsForEvent(props.id)
}

// ===== 退货 =====
const showRefundModal = ref(false)
const refundOrder = ref(null)
// orderId -> 是否已全额退完。退货接口没有批量版，只能逐单拉 lines；
// 拉过的结果缓存在这里，切 tab 回来不重复请求。
const refundFlagCache = new Map()
const fullyRefundedOrders = ref(new Set())

async function refreshRefundFlags() {
  const orders = store.completedOrders
  if (!orders.length) {
    fullyRefundedOrders.value = new Set()
    return
  }
  const results = await Promise.allSettled(
    orders.map((o) => {
      if (refundFlagCache.has(o.id)) return Promise.resolve(refundFlagCache.get(o.id))
      return api.get(`/events/${props.id}/orders/${o.id}/refunds`).then((r) => {
        const lines = r.data?.lines || []
        const flag = lines.length > 0 && lines.every((l) => l.remaining_qty === 0)
        refundFlagCache.set(o.id, flag)
        return flag
      })
    })
  )
  const next = new Set()
  results.forEach((r, i) => {
    if (r.status === 'fulfilled' && r.value) next.add(orders[i].id)
  })
  fullyRefundedOrders.value = next
}

function openRefund(order) {
  refundOrder.value = order
  showRefundModal.value = true
}

function closeRefund() {
  showRefundModal.value = false
  refundOrder.value = null
}

/** RefundModal 每次加载 / 退完货都会带上这张单的最新可退状态。 */
function onRefundFlag({ orderId, fullyRefunded }) {
  refundFlagCache.set(orderId, fullyRefunded)
  if (!fullyRefunded) return
  const next = new Set(fullyRefundedOrders.value)
  next.add(orderId)
  fullyRefundedOrders.value = next
}

// 切到已完成 tab 时补拉一次各单的可退状态（首次进入时 completedOrders 可能刚回来）。
watch(currentTab, (tab) => {
  if (tab === 'completed') refreshRefundFlags()
})

// ===== 收摊向导 =====
const showClosingWizard = ref(false)

function openClosing() {
  if (isEventSettled.value) {
    // 管理端结算页由后续批次建在同一路由（plan 钉死的 admin-event-settlement）。
    router.push(`/admin/events/${props.id}/settlement`)
    return
  }
  showClosingWizard.value = true
}

/** 向导里结算成功后，头部按钮当场变成「查看结算」，不用等重新拉展会列表。 */
function onClosingSettled() {
  const event = eventStore.events.find((e) => e.id === parseInt(props.id, 10))
  if (event) event.status = '已结算'
  store.fetchCompletedOrders()
}

function completeOrder(orderId) {
  pendingOrder.value = store.pendingOrders.find((o) => o.id === orderId) || null
  showReceiptModal.value = true
}

function closeReceipt() {
  showReceiptModal.value = false
  pendingOrder.value = null
}

async function onReceiptConfirm({ channel, finalAmount, unapplyLotIds }) {
  const order = pendingOrder.value
  showReceiptModal.value = false
  if (!order) return
  try {
    await store.markOrderAsCompleted(order.id, channel, finalAmount, unapplyLotIds)
    await eventDetailStore.fetchProductsForEvent(props.id)
    await store.fetchCompletedOrders()
    message.success('已记录收款')
  } catch (error) {
    message.error(error?.message || '操作失败')
  } finally {
    pendingOrder.value = null
  }
}

async function cancelOrder(orderId) {
  dialog.warning({
    title: '确认取消',
    content: '确定要取消这个订单吗？此操作无法撤销。',
    positiveText: '确认',
    negativeText: '返回',
    async onPositiveClick() {
      try {
        await store.cancelOrder(orderId)
        message.success('订单已取消')
      } catch (error) {
        message.error(error?.message || '取消失败')
      }
    },
  })
}

onMounted(() => {
  if (eventStore.events.length === 0) {
    eventStore.fetchEvents()
  }
  store.setActiveEvent(props.id)
  eventDetailStore.fetchProductsForEvent(props.id)
})

onUnmounted(() => {
  store.stopPolling()
})
</script>

<style scoped>
.vendor-view {
  max-width: 1100px;
  margin: 0 auto;
  padding: 1rem;
}

/* ===== Header ===== */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 1rem;
  border-bottom: 1px solid var(--border-color);
  padding-bottom: 0.75rem;
  position: sticky;
  top: 0;
  background: var(--bg-color);
  z-index: 10;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
}
.header-title-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.page-header h1 {
  margin: 0;
  color: var(--accent-color);
  font-size: var(--font-xl);
}
.back-link {
  font-size: var(--font-sm);
  color: var(--text-muted);
  text-decoration: none;
  padding: 2px 8px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--border-color);
  transition: all 0.15s;
  white-space: nowrap;
}
.back-link:hover {
  background: var(--accent-color);
  color: white;
  border-color: var(--accent-color);
}
.page-header p {
  margin: 4px 0 0;
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.header-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

/* ===== Body: 自适应双栏 ===== */
.vendor-body {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.order-column {
  flex: 1;
  min-width: 0;
}

.order-tabs {
  margin-bottom: 0.75rem;
}

.no-orders-message {
  text-align: center;
  padding: 2rem;
  color: var(--text-muted);
  font-size: var(--font-base);
}

/* 已完成单的「退货」入口。OrderCard 本身不动（④ 要整体重做），
   只在卡片外面加一行按钮。 */
.completed-entry-actions {
  display: flex;
  justify-content: flex-end;
  margin: -4px 0 10px;
}

.revenue-summary {
  text-align: right;
  font-size: var(--font-md);
  margin-bottom: 0.75rem;
  color: var(--primary-text-color);
}
.revenue-summary strong {
  color: var(--accent-color);
}

/* ===== 宽屏双栏 (平板/电脑) ===== */
@media (min-width: 768px) {
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
