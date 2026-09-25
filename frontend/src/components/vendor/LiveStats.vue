<!--
  摊主 · 实时销售统计（spec §5.6 / §7）。

  手机单列（营业额 / 待处理两张 tile 在上，库存速览在下），平板起两列并排。
  数据自己每 5 秒拉一次；加载 / 错误 / 空态都交给 `AsyncState` + `EmptyState`，
  页面不手写错误态。
-->
<template>
  <SectionCard title="实时销售统计" collapsible v-model:collapsed="collapsed">
    <div class="live-stats__grid">
      <div class="live-stats__tiles">
        <StatTile label="当前营业额">
          <template #value>
            <Money :value="orderStore.totalRevenue" size="lg" />
          </template>
        </StatTile>
        <StatTile label="待处理订单" :value="orderStore.pendingOrders.length" />
      </div>

      <div class="live-stats__stock">
        <div class="stock-header">
          <h4>库存速览</h4>
          <n-button
            v-if="eventDetailStore.products.length > 0"
            text
            size="tiny"
            class="stock-toggle"
            @click="stockExpanded = !stockExpanded"
          >
            {{ stockExpanded ? '收起详情' : '展开详情' }}
          </n-button>
        </div>

        <AsyncState
          :loading="eventDetailStore.isLoading && !eventDetailStore.products.length"
          :error="eventDetailStore.error"
          :empty="!eventDetailStore.isLoading && !eventDetailStore.products.length"
          :keep-content="eventDetailStore.products.length > 0"
          loading-text="正在加载库存…"
          @retry="reload"
        >
          <template #empty>
            <EmptyState compact title="这场还没有上架商品。" />
          </template>

          <!-- 紧凑模式：色块网格 -->
          <div v-if="!stockExpanded" class="stock-grid">
            <div
              v-for="product in eventDetailStore.products"
              :key="product.id"
              class="stock-chip"
              :class="stockLevel(product)"
            >
              <span class="chip-name">{{ product.name }}</span>
              <span class="chip-count">{{ product.onsite_qty }}</span>
            </div>
          </div>

          <!-- 详情模式：进度条列表 -->
          <div v-else class="stock-list">
            <div v-for="product in eventDetailStore.products" :key="product.id" class="stock-item">
              <span class="product-name">{{ product.name }}</span>
              <n-progress
                type="line"
                :percentage="stockPercentage(product)"
                :show-indicator="false"
                :color="stockColor(product)"
                rail-color="var(--bg-secondary)"
              />
              <span class="stock-value" :class="stockLevel(product)">
                {{ product.onsite_qty }} / {{ product.stocked_qty }}
              </span>
            </div>
          </div>
        </AsyncState>
      </div>
    </div>
  </SectionCard>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { NButton, NProgress } from 'naive-ui'
import { AsyncState, EmptyState, Money, SectionCard, StatTile } from '@/components/ui'
import { useOrderStore } from '@/stores/orderStore'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import type { Schemas } from '@/api/client'

const props = defineProps<{ eventId: string | number }>()

const collapsed = ref(false)
const stockExpanded = ref(false)
const orderStore = useOrderStore()
const eventDetailStore = useEventDetailStore()

function stockPercentage(product: Schemas['ProductEventProduct']) {
  if (product.stocked_qty === 0) return 0
  return (product.onsite_qty / product.stocked_qty) * 100
}

function stockLevel(product: Schemas['ProductEventProduct']) {
  if (product.onsite_qty === 0) return 'level-out'
  if (product.onsite_qty <= 5) return 'level-critical'
  const pct = stockPercentage(product)
  if (pct <= 20) return 'level-low'
  return 'level-ok'
}

function stockColor(product: Schemas['ProductEventProduct']) {
  const level = stockLevel(product)
  if (level === 'level-out') return 'var(--text-disabled)'
  if (level === 'level-critical') return 'var(--error-color)'
  if (level === 'level-low') return 'var(--warning-color)'
  return 'var(--accent-color)'
}

let timer: ReturnType<typeof setInterval> | null = null
async function refreshStats() {
  await Promise.all([
    orderStore.fetchCompletedOrders?.(),
    eventDetailStore.fetchProductsForEvent?.(Number(props.eventId)),
  ])
}
/** AsyncState 的重试入口：重拉一次即可，加载 / 错误态由 store 驱动。 */
async function reload() {
  await refreshStats()
}
onMounted(() => {
  void refreshStats()
  timer = setInterval(refreshStats, 5000)
})
onUnmounted(() => {
  if (timer) clearInterval(timer)
})
</script>

<style scoped>
/* 平板起两列：左侧统计 tile，右侧库存速览；手机单列。 */
.live-stats__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-lg);
  align-items: start;
}
.live-stats__tiles {
  display: grid;
  gap: var(--space-sm);
}

.stock-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-sm);
  margin-bottom: var(--space-sm);
}
.stock-header h4 {
  margin: 0;
  font-size: var(--font-base);
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
}

/* ===== 紧凑色块网格 ===== */
.stock-grid {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-sm);
}

.stock-chip {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-pill);
  font-size: var(--font-sm);
  line-height: 1.3;
  background: var(--bg-secondary);
  color: var(--primary-text-color);
  border: 1px solid var(--border-color);
}

.chip-name {
  max-width: 8em;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chip-count {
  font-weight: var(--weight-bold);
  font-variant-numeric: tabular-nums;
}

/* 库存等级色彩 */
.stock-chip.level-ok {
  border-color: var(--border-color);
}
.stock-chip.level-ok .chip-count {
  color: var(--accent-color);
}

.stock-chip.level-low {
  border-color: var(--warning-color);
  background: color-mix(in srgb, var(--warning-color) 8%, var(--bg-secondary));
}
.stock-chip.level-low .chip-count {
  color: var(--warning-color);
}

.stock-chip.level-critical {
  border-color: var(--error-color);
  background: color-mix(in srgb, var(--error-color) 10%, var(--bg-secondary));
}
.stock-chip.level-critical .chip-count {
  color: var(--error-color);
  font-weight: var(--weight-bold);
}

.stock-chip.level-out {
  opacity: 0.5;
  text-decoration: line-through;
}
.stock-chip.level-out .chip-count {
  color: var(--text-disabled);
}

/* ===== 详情进度条列表 ===== */
.stock-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.stock-item {
  display: grid;
  grid-template-columns: 2.5fr 2fr 1fr;
  align-items: center;
  gap: var(--space-sm);
}

.product-name {
  font-size: var(--font-sm);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.stock-value {
  text-align: right;
  font-size: var(--font-sm);
  font-variant-numeric: tabular-nums;
}

.stock-value.level-ok {
  color: var(--primary-text-color);
}
.stock-value.level-low {
  color: var(--warning-color);
}
.stock-value.level-critical {
  color: var(--error-color);
  font-weight: var(--weight-bold);
}
.stock-value.level-out {
  color: var(--text-disabled);
}

@media (--phone) {
  .live-stats__grid {
    grid-template-columns: minmax(0, 1fr);
  }
  /* 手机上把「展开 / 收起详情」抬到 44px（「能用」标准）。 */
  .stock-toggle {
    min-height: 44px;
  }
}
</style>
