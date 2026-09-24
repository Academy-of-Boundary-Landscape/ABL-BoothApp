<template>
  <PageShell
    title="订单管理"
    subtitle="查看并管理当前展会的所有订单记录。"
    help="event-orders"
    width="content"
  >
    <!-- 筛选器区块 -->
    <SectionCard
      title="订单筛选"
      collapsible
      v-model:collapsed="isFilterCollapsed"
      class="filter-section"
    >
      <div class="filter-content">
        <div class="filter-row">
          <label for="status-filter">状态:</label>
          <n-select
            id="status-filter"
            v-model:value="statusFilter"
            :options="statusOptions"
            placeholder="选择筛选状态"
            class="status-select"
          />
        </div>

        <div class="filter-row">
          <label>金额范围:</label>
          <div class="amount-range">
            <n-input-number
              v-model:value="minAmount"
              :min="0"
              :precision="2"
              placeholder="最小金额"
              clearable
              class="amount-input"
            >
              <template #prefix>¥</template>
            </n-input-number>
            <span class="range-separator">-</span>
            <n-input-number
              v-model:value="maxAmount"
              :min="0"
              :precision="2"
              placeholder="最大金额"
              clearable
              class="amount-input"
            >
              <template #prefix>¥</template>
            </n-input-number>
          </div>
        </div>

        <div class="filter-row">
          <label for="product-filter">商品名称:</label>
          <n-input
            id="product-filter"
            v-model:value="productNameFilter"
            placeholder="输入商品名称搜索"
            clearable
            class="product-input"
          />
        </div>

        <n-button
          v-if="
            statusFilter !== 'all' || minAmount !== null || maxAmount !== null || productNameFilter
          "
          @click="clearFilters"
          class="clear-btn"
          secondary
        >
          清空筛选
        </n-button>
      </div>
    </SectionCard>

    <!-- 订单列表区块 -->
    <SectionCard
      title="订单列表"
      collapsible
      v-model:collapsed="isListCollapsed"
      class="list-section"
    >
      <AsyncState
        :loading="store.isLoading"
        :error="store.error"
        :empty="!filteredOrders.length"
        loading-text="正在加载订单..."
      >
        <div class="table-scroll">
          <n-table class="order-table" size="small">
            <thead>
              <tr>
                <th>订单ID</th>
                <th>下单时间</th>
                <th>商品详情</th>
                <th>总金额</th>
                <th class="column-status">状态</th>
                <th class="column-actions">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="order in filteredOrders" :key="order.id">
                <td>
                  <strong>#{{ order.id }}</strong>
                </td>
                <td>{{ formatTimestamp(order.timestamp) }}</td>
                <td>
                  <ul class="item-list">
                    <li v-for="item in order.items" :key="item.id">
                      {{ item.product_name }} x {{ item.quantity }}
                      <!-- 同一个商品可能在一张订单里出现两行（进套装 / 散着，spec 4.5）。
                           不标出来，摊主配货时会以为系统重复计数了。 -->
                      <span v-if="item.lot_name" class="item-lot">{{ item.lot_name }}</span>
                    </li>
                  </ul>
                </td>
                <td>
                  <!-- 只在真打折时显示删除线：加价（实收 > 原价）显示删除线会被读成「便宜了」。 -->
                  <span v-if="order.final_amount < order.gross_amount" class="struck">
                    {{ formatYuan(order.gross_amount) }}
                  </span>
                  <strong>{{ formatYuan(order.final_amount) }}</strong>
                </td>
                <td>
                  <n-tag :type="tagType(order.status)" size="large" round>{{
                    statusText(order.status)
                  }}</n-tag>
                </td>
                <td>
                  <n-dropdown
                    :options="actionOptions(order.status)"
                    @select="(key) => changeStatus(order.id, key)"
                  >
                    <n-button size="large">操作</n-button>
                  </n-dropdown>
                </td>
              </tr>
            </tbody>
          </n-table>
        </div>

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
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import {
  NSelect,
  NTable,
  NTag,
  NDropdown,
  NButton,
  NInput,
  NInputNumber,
  type DropdownOption,
} from 'naive-ui'
import type { Schemas } from '@/api/client'
import { PageShell, SectionCard, AsyncState, EmptyState } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import ReceiptModal from '@/components/vendor/ReceiptModal.vue'
import { formatTimestamp } from '@/utils/dateFormatter'
import { formatYuan, toCents, type Cents } from '@/utils/money'

const props = defineProps<{ id: number }>()

const store = useEventDetailStore()
const statusFilter = ref('all') // 筛选器的状态
const minAmount = ref<number | null>(null) // 最小金额
const maxAmount = ref<number | null>(null) // 最大金额
const productNameFilter = ref('') // 商品名称筛选
const fb = useFeedback()
const isFilterCollapsed = ref(false)
const isListCollapsed = ref(false)
const statusOptions = [
  { label: '所有订单', value: 'all' },
  { label: '待处理', value: 'pending' },
  { label: '已完成', value: 'completed' },
  { label: '已取消', value: 'cancelled' },
]

// 计算属性，根据筛选器动态过滤订单
const filteredOrders = computed(() => {
  let orders = store.allOrders

  // 状态筛选
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

  // 商品名称筛选
  if (productNameFilter.value.trim()) {
    const keyword = productNameFilter.value.trim().toLowerCase()
    orders = orders.filter((order) =>
      order.items.some((item) => item.product_name.toLowerCase().includes(keyword))
    )
  }

  return orders
})

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

// --- 辅助函数 ---
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

// --- 生命周期 ---
onMounted(() => {
  store.fetchAllOrdersForEvent(props.id)
})
onUnmounted(() => {
  store.resetStore() // 离开时重置store
})
</script>

<style scoped>
.filter-section,
.list-section {
  margin-bottom: var(--space-2xl);
}

.filter-content {
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}

.filter-row {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
  flex-wrap: wrap;
}

.filter-content label {
  font-size: var(--font-base);
  font-weight: var(--weight-medium);
  color: var(--primary-text-color);
  white-space: nowrap;
  min-width: 90px;
}

.status-select {
  min-width: 200px;
  flex: 1;
  max-width: 300px;
}

.amount-range {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  flex: 1;
}

.amount-input {
  flex: 1;
  min-width: 120px;
  max-width: 200px;
}

.range-separator {
  color: var(--text-muted);
  font-weight: var(--weight-medium);
}

.product-input {
  flex: 1;
  max-width: 25rem;
}

.clear-btn {
  align-self: flex-start;
  margin-left: var(--space-2xl);
}

/* --- 表格样式 --- */
.order-table {
  width: 100%;
  margin-top: 0;
  border-collapse: collapse;
  border-spacing: 0;
  text-align: left;
  font-size: var(--font-base);
}
.order-table th {
  padding: var(--space-md) var(--space-lg);
  background-color: var(--card-bg-color);
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
  border-bottom: 2px solid var(--accent-color);
  white-space: nowrap;
}
.order-table tbody tr:hover {
  background-color: var(--accent-color-light);
}
.order-table th:first-child,
.order-table td:first-child {
  padding-left: 0;
}
.order-table th:last-child,
.order-table td:last-child {
  text-align: right;
  padding-right: 0;
}

/* 套装归属标签：与 OrderCard.vue 保持一致，避免摊主以为系统重复计数 */
.item-lot {
  margin-left: var(--space-xs);
  padding: 0 var(--space-sm);
  border-radius: var(--radius-sm);
  background-color: var(--accent-color-light);
  color: var(--accent-color);
  font-size: var(--font-xs);
  line-height: 1.6;
  white-space: nowrap;
}

/* 打折前的原价，只在实收低于原价时出现 */
.struck {
  margin-right: var(--space-sm);
  color: var(--text-disabled);
  text-decoration: line-through;
  font-weight: var(--weight-regular);
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* 响应式布局 */
@media (--phone) {
  .filter-content {
    gap: var(--space-lg);
  }

  .filter-row {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-sm);
  }

  .filter-content label {
    font-size: var(--font-sm);
    min-width: auto;
  }

  .status-select,
  .product-input {
    max-width: none;
    width: 100%;
  }

  .amount-range {
    width: 100%;
    flex-wrap: wrap;
  }

  .amount-input {
    max-width: none;
    min-width: 0;
  }

  .clear-btn {
    margin-left: 0;
    width: 100%;
  }

  .order-table {
    font-size: var(--font-sm);
    min-width: 600px;
  }

  .order-table th,
  .order-table td {
    padding: var(--space-sm);
  }

  .item-list {
    font-size: var(--font-sm);
  }
}

@media (--phone) {
  .filter-content {
    gap: var(--space-md);
  }

  .filter-row {
    gap: var(--space-sm);
  }

  .filter-content label {
    font-size: var(--font-sm);
  }

  .amount-input {
    flex: 1 1 calc(50% - 1rem);
  }

  .order-table {
    font-size: var(--font-xs);
    min-width: 550px;
  }

  .order-table th,
  .order-table td {
    padding: var(--space-sm) var(--space-xs);
  }

  .order-table th {
    font-size: var(--font-xs);
  }

  .item-list {
    padding-left: var(--space-lg);
    margin: 0;
    font-size: var(--font-xs);
  }

  .item-list li {
    margin-bottom: var(--space-xs);
  }
}
</style>
