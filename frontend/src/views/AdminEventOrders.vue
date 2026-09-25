<template>
  <PageShell embedded width="content">
    <div class="page-hint-row">
      <p class="page-hint">查看并管理当前展会的所有订单记录。</p>
      <HelpBubble page="event-orders" />
    </div>

    <SectionCard title="订单列表" class="list-section">
      <!-- 筛选保持原有能力（状态 / 金额范围 / 商品名），收成表格上方的一行。 -->
      <div class="filter-bar">
        <n-select
          v-model:value="statusFilter"
          :options="statusOptions"
          placeholder="订单状态"
          class="filter-status"
        />
        <div class="amount-range">
          <n-input-number
            v-model:value="minAmount"
            :min="0"
            :precision="2"
            placeholder="最低金额"
            clearable
            class="filter-amount"
          >
            <template #prefix>¥</template>
          </n-input-number>
          <span class="range-separator">-</span>
          <n-input-number
            v-model:value="maxAmount"
            :min="0"
            :precision="2"
            placeholder="最高金额"
            clearable
            class="filter-amount"
          >
            <template #prefix>¥</template>
          </n-input-number>
        </div>
        <n-input
          v-model:value="productNameFilter"
          placeholder="按商品名称搜索"
          clearable
          class="filter-product"
        />
        <n-button v-if="hasFilters" secondary class="clear-btn" @click="clearFilters">
          清空筛选
        </n-button>
      </div>

      <AsyncState
        :loading="store.isLoading"
        :error="store.error"
        :empty="!filteredOrders.length"
        loading-text="正在加载订单..."
        @retry="reload"
      >
        <n-data-table
          :columns="columns"
          :data="filteredOrders"
          :row-key="rowKey"
          :scroll-x="1120"
          size="small"
        />

        <template #empty>
          <EmptyState
            icon="📝"
            title="暂无订单"
            desc="当顾客通过点单页面下单后，订单会自动出现在这里。你可以在这里查看、完成或取消订单。"
            hint="将展会设为「进行中」，然后分享点单链接给顾客"
          />
        </template>
      </AsyncState>
    </SectionCard>

    <!-- 设为「已完成」前先确认收款：显示原价/应收/已套用的套装（可逐个拆），实收可改（spec 4.3） -->
    <ReceiptModal
      :show="showReceiptModal"
      :gross-amount="pendingOrder?.gross_amount"
      :solved-amount="pendingOrder?.solved_amount"
      :lots="pendingOrder?.lots ?? []"
      @confirm="onReceiptConfirm"
      @cancel="closeReceipt"
    />
  </PageShell>
</template>

<script setup lang="ts">
import { ref, computed, h, onMounted, onUnmounted, type VNodeChild } from 'vue'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import {
  NButton,
  NDataTable,
  NDropdown,
  NInput,
  NInputNumber,
  NSelect,
  NTag,
  type DataTableColumns,
  type DropdownOption,
} from 'naive-ui'
import type { Schemas } from '@/api/client'
import { PageShell, SectionCard, AsyncState, EmptyState, Money } from '@/components/ui'
import HelpBubble from '@/components/shared/HelpBubble.vue'
import { useFeedback } from '@/composables/useFeedback'
import ReceiptModal from '@/components/vendor/ReceiptModal.vue'
import { formatTimestamp } from '@/utils/dateFormatter'
import { formatYuan, toCents, type Cents } from '@/utils/money'

const props = defineProps<{ id: number }>()

const store = useEventDetailStore()
const fb = useFeedback()

const statusFilter = ref('all')
const minAmount = ref<number | null>(null)
const maxAmount = ref<number | null>(null)
const productNameFilter = ref('')

const statusOptions = [
  { label: '所有订单', value: 'all' },
  { label: '待处理', value: 'pending' },
  { label: '已完成', value: 'completed' },
  { label: '已取消', value: 'cancelled' },
]

const hasFilters = computed(
  () =>
    statusFilter.value !== 'all' ||
    minAmount.value !== null ||
    maxAmount.value !== null ||
    Boolean(productNameFilter.value)
)

// 计算属性，根据筛选器动态过滤订单
const filteredOrders = computed(() => {
  let orders = store.allOrders

  if (statusFilter.value !== 'all') {
    orders = orders.filter((order) => order.status === statusFilter.value)
  }

  // 金额范围筛选（输入是元，订单里是分）
  if (minAmount.value !== null) {
    const min = toCents(minAmount.value)
    orders = orders.filter((order) => order.final_amount >= min)
  }
  if (maxAmount.value !== null) {
    const max = toCents(maxAmount.value)
    orders = orders.filter((order) => order.final_amount <= max)
  }

  if (productNameFilter.value.trim()) {
    const keyword = productNameFilter.value.trim().toLowerCase()
    orders = orders.filter((order) =>
      order.items.some((item) => item.product_name.toLowerCase().includes(keyword))
    )
  }

  return orders
})

// --- 表格列 ---
function rowKey(order: Schemas['OrderResponse']) {
  return order.id
}

function statusText(status: Schemas['OrderStatus']) {
  const map: Record<Schemas['OrderStatus'], string> = {
    pending: '待处理',
    completed: '已完成',
    cancelled: '已取消',
  }
  return map[status] || status
}

function tagType(status: Schemas['OrderStatus']) {
  if (status === 'pending') return 'warning'
  if (status === 'completed') return 'success'
  if (status === 'cancelled') return 'default'
  return 'default'
}

function renderAmount(order: Schemas['OrderResponse']): VNodeChild {
  const nodes: VNodeChild[] = []
  // 只在真打折（实收 < 原价）时显示原价：加价显示删除线会被读成「便宜了」。
  if (order.gross_amount !== order.final_amount) {
    nodes.push(h(Money, { value: order.gross_amount, strike: true, size: 'sm' }))
    nodes.push(' ')
  }
  nodes.push(h(Money, { value: order.final_amount }))
  return h('span', { class: 'amount-cell' }, nodes)
}

const columns: DataTableColumns<Schemas['OrderResponse']> = [
  {
    type: 'expand',
    width: 44,
    renderExpand: (order) => renderExpand(order),
  },
  {
    title: '单号',
    key: 'id',
    width: 84,
    render: (order) => h('strong', `#${order.id}`),
  },
  {
    title: '时间',
    key: 'timestamp',
    width: 168,
    render: (order) => formatTimestamp(order.timestamp),
  },
  {
    title: '状态',
    key: 'status',
    width: 96,
    render: (order) =>
      h(
        NTag,
        { type: tagType(order.status), size: 'small', round: true },
        {
          default: () => statusText(order.status),
        }
      ),
  },
  {
    title: '原价 → 实收',
    key: 'amount',
    width: 168,
    render: (order) => renderAmount(order),
  },
  {
    title: '已退',
    key: 'refunded_amount',
    width: 110,
    // 没有退货时显示「—」而不是「¥0.00」，避免被当成真的退过钱。
    render: (order) =>
      order.refunded_amount === 0
        ? h('span', { class: 'refunded-cell muted-dash' }, '—')
        : h('span', { class: 'refunded-cell' }, [h(Money, { value: order.refunded_amount })]),
  },
  {
    title: '渠道',
    key: 'channel',
    width: 110,
    render: (order) => order.channel || '—',
  },
  {
    title: '操作',
    key: 'actions',
    width: 96,
    fixed: 'right',
    render: (order) =>
      h(
        NDropdown,
        {
          options: actionOptions(order.status),
          trigger: 'click',
          onSelect: (key: string | number) => changeStatus(order.id, key as Schemas['OrderStatus']),
        },
        { default: () => h(NButton, { size: 'small' }, { default: () => '操作' }) }
      ),
  },
]

function renderExpand(order: Schemas['OrderResponse']): VNodeChild {
  const items = order.items.map((item) =>
    h('div', { class: 'expand-item', key: item.id }, [
      h('span', { class: 'expand-item__name' }, item.product_name),
      h('span', { class: 'expand-item__qty' }, `×${item.quantity}`),
      item.lot_name ? h('span', { class: 'expand-item__lot' }, item.lot_name) : null,
      h('span', { class: 'expand-item__paid' }, [
        '实付 ',
        h(Money, { value: item.paid_amount, size: 'sm' }),
      ]),
      h('span', { class: 'expand-item__refund' }, `已退 ${item.refunded_qty}`),
    ])
  )

  const lots = order.lots.map((lot) =>
    h('span', { class: 'expand-lot', key: lot.id }, `${lot.name} · ${formatYuan(lot.price)}`)
  )

  return h('div', { class: 'expand-panel' }, [
    h('div', { class: 'expand-panel__title' }, '商品明细'),
    items.length
      ? h('div', { class: 'expand-items' }, items)
      : h('p', { class: 'expand-empty' }, '无商品行'),
    h('div', { class: 'expand-panel__title' }, '套装'),
    lots.length
      ? h('div', { class: 'expand-lots' }, lots)
      : h('p', { class: 'expand-empty' }, '未使用套装'),
  ])
}

function actionOptions(status: Schemas['OrderStatus']) {
  const opts: DropdownOption[] = []
  if (status !== 'pending') opts.push({ label: '设为待处理', key: 'pending' })
  if (status !== 'completed') opts.push({ label: '设为已完成', key: 'completed' })
  if (status !== 'cancelled') opts.push({ label: '设为已取消', key: 'cancelled' })
  return opts
}

function clearFilters() {
  statusFilter.value = 'all'
  minAmount.value = null
  maxAmount.value = null
  productNameFilter.value = ''
}

// --- 操作 ---
const showReceiptModal = ref(false)
const pendingOrder = ref<Schemas['OrderResponse'] | null>(null)

async function changeStatus(orderId: number, newStatus: Schemas['OrderStatus']) {
  if (!newStatus) return
  // 「已完成」会记一笔真实的资金移动，必须带渠道——走收款确认而不是普通确认框。
  if (newStatus === 'completed') {
    pendingOrder.value = store.allOrders.find((o) => o.id === orderId) || null
    showReceiptModal.value = true
    return
  }
  await fb.confirm({
    title: '确认操作',
    content: `确定要将订单 #${orderId} 的状态修改为 "${statusText(newStatus)}" 吗？`,
    positiveText: '确认',
    negativeText: '取消',
    onConfirm: async () => {
      try {
        await store.adminUpdateOrderStatus(props.id, orderId, newStatus)
        fb.success('状态已更新')
      } catch (error) {
        fb.error(error, '更新失败')
      }
    },
  })
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
    await store.adminUpdateOrderStatus(
      props.id,
      order.id,
      'completed',
      payload.channel,
      payload.finalAmount,
      payload.unapplyLotIds
    )
    fb.success('状态已更新')
  } catch (error) {
    fb.error(error, '更新失败')
  } finally {
    pendingOrder.value = null
  }
}

function reload() {
  return store.fetchAllOrdersForEvent(props.id)
}

// --- 生命周期 ---
onMounted(() => {
  reload()
})
onUnmounted(() => {
  store.resetStore() // 离开时重置 store
})
</script>

<style scoped>
/* 页头改 embedded 后，原副标题挪到内容区顶部；保证文字不丢。 */
.page-hint-row {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  margin-bottom: var(--space-lg);
}

.page-hint {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--font-base);
}

.list-section {
  margin-bottom: var(--space-2xl);
}

.filter-bar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-md);
  margin-bottom: var(--space-lg);
}

.filter-status {
  width: 160px;
}

.amount-range {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.filter-amount {
  width: 150px;
}

.range-separator {
  color: var(--text-muted);
}

.filter-product {
  flex: 1;
  min-width: 12rem;
  max-width: 20rem;
}

/* --- 行展开 --- */
.expand-panel {
  padding: var(--space-sm) var(--space-lg);
}

.expand-panel__title {
  margin: var(--space-xs) 0 var(--space-sm);
  color: var(--text-muted);
  font-size: var(--font-sm);
  font-weight: var(--weight-bold);
}

.expand-items {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}

.expand-item {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: var(--space-sm);
  font-size: var(--font-sm);
}

.expand-item__name {
  font-weight: var(--weight-medium);
  color: var(--primary-text-color);
}

.expand-item__qty {
  font-weight: var(--weight-bold);
  color: var(--accent-color);
}

/* 套装归属标签：同一个商品可能在一张订单里出现两行（进套装 / 散着，spec 4.5）。 */
.expand-item__lot,
.expand-lot {
  padding: 0 var(--space-sm);
  border-radius: var(--radius-sm);
  background-color: var(--accent-color-light);
  color: var(--accent-color);
  font-size: var(--font-xs);
}

.expand-item__paid,
.expand-item__refund {
  color: var(--text-muted);
}

.expand-lots {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-sm);
}

.expand-empty {
  margin: 0;
  color: var(--text-disabled);
  font-size: var(--font-sm);
}

.muted-dash {
  color: var(--text-disabled);
}

@media (--phone) {
  .page-hint-row {
    align-items: flex-start;
  }

  .filter-status,
  .filter-amount,
  .filter-product {
    width: 100%;
    max-width: none;
  }

  .amount-range {
    width: 100%;
  }

  .filter-amount {
    flex: 1;
    min-width: 0;
  }

  .clear-btn {
    width: 100%;
  }
}
</style>
