<template>
  <div class="stats-container">
    <div class="stats-header" @click="collapsed = !collapsed">
      <h3>实时销售统计</h3>
      <n-button tertiary size="small" class="collapse-btn">
        {{ collapsed ? '展开' : '收起' }}
      </n-button>
    </div>

    <div v-show="!collapsed">
      <!-- 营业额 + 待处理 -->
      <div class="stat-row">
        <div class="stat-card">
          <span class="label">当前营业额</span>
          <span class="value revenue">¥{{ orderStore.totalRevenue.toFixed(2) }}</span>
        </div>
        <div class="stat-card">
          <span class="label">待处理订单</span>
          <span class="value">{{ orderStore.pendingOrders.length }}</span>
        </div>
      </div>

      <!-- 库存速览 -->
      <div class="stock-section">
        <div class="stock-section-header">
          <h4>库存速览</h4>
          <n-button
            v-if="eventDetailStore.products.length > 0"
            text
            size="tiny"
            @click="stockExpanded = !stockExpanded"
          >
            {{ stockExpanded ? '收起详情' : '展开详情' }}
          </n-button>
        </div>

        <div v-if="eventDetailStore.isLoading" class="loading">
          <n-spin size="small" />
        </div>

        <!-- 紧凑模式：色块网格 -->
        <div v-else-if="!stockExpanded" class="stock-grid">
          <div
            v-for="product in eventDetailStore.products"
            :key="product.id"
            class="stock-chip"
            :class="stockLevel(product)"
          >
            <span class="chip-name">{{ product.name }}</span>
            <span class="chip-count">{{ product.current_stock }}</span>
          </div>
        </div>

        <!-- 详情模式：进度条列表 -->
        <div v-else class="stock-list">
          <div
            v-for="product in eventDetailStore.products"
            :key="product.id"
            class="stock-item"
          >
            <span class="product-name">{{ product.name }}</span>
            <n-progress
              type="line"
              :percentage="stockPercentage(product)"
              :show-indicator="false"
              :color="stockColor(product)"
              rail-color="var(--bg-secondary)"
            />
            <span class="stock-value" :class="stockLevel(product)">
              {{ product.current_stock }} / {{ product.initial_stock }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { useOrderStore } from '@/stores/orderStore'
import { onMounted, onUnmounted, ref } from 'vue'
import { NButton, NSpin, NProgress } from 'naive-ui'
import { useEventDetailStore } from '@/stores/eventDetailStore'

const props = defineProps({
  eventId: { type: String, required: true },
})

const collapsed = ref(false)
const stockExpanded = ref(false)
const orderStore = useOrderStore()
const eventDetailStore = useEventDetailStore()

function stockPercentage(product) {
  if (product.initial_stock === 0) return 0
  return (product.current_stock / product.initial_stock) * 100
}

function stockLevel(product) {
  if (product.current_stock === 0) return 'level-out'
  if (product.current_stock <= 5) return 'level-critical'
  const pct = stockPercentage(product)
  if (pct <= 20) return 'level-low'
  return 'level-ok'
}

function stockColor(product) {
  const level = stockLevel(product)
  if (level === 'level-out') return 'var(--text-disabled)'
  if (level === 'level-critical') return 'var(--error-color)'
  if (level === 'level-low') return 'var(--warning-color)'
  return 'var(--accent-color)'
}

let timer = null
async function refreshStats() {
  await Promise.all([
    orderStore.fetchCompletedOrders?.(),
    eventDetailStore.fetchProductsForEvent?.(props.eventId),
  ])
}
onMounted(() => {
  refreshStats()
  timer = setInterval(refreshStats, 5000)
})
onUnmounted(() => {
  if (timer) clearInterval(timer)
})
</script>

<style scoped>
.stats-container {
  background-color: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: 1rem;
  margin-bottom: 1.5rem;
}

.stats-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
  user-select: none;
}
.stats-header h3 {
  margin: 0;
  font-size: var(--font-md);
}
.collapse-btn {
  min-width: 48px;
}

/* 营业额卡片 */
.stat-row {
  display: flex;
  gap: 10px;
  margin-top: 12px;
}
.stat-card {
  flex: 1;
  background: var(--bg-color);
  border-radius: var(--radius-md);
  padding: 10px 12px;
  text-align: center;
}
.stat-card .label {
  display: block;
  font-size: var(--font-sm);
  color: var(--text-muted);
  margin-bottom: 4px;
}
.stat-card .value {
  display: block;
  font-size: var(--font-xl);
  font-weight: 700;
}
.stat-card .revenue {
  color: var(--accent-color);
}

/* 库存区域 */
.stock-section {
  margin-top: 12px;
}
.stock-section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}
.stock-section-header h4 {
  margin: 0;
  font-size: var(--font-base);
  font-weight: 600;
  color: var(--primary-text-color);
}

.loading {
  display: flex;
  justify-content: center;
  padding: 16px 0;
}

/* ===== 紧凑色块网格 ===== */
.stock-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.stock-chip {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 4px 10px;
  border-radius: var(--radius-pill);
  font-size: var(--font-sm);
  line-height: 1.3;
  background: var(--bg-secondary);
  color: var(--primary-text-color);
  border: 1px solid var(--border-color);
  transition: border-color 0.15s;
}

.chip-name {
  max-width: 8em;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chip-count {
  font-weight: 700;
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
  font-weight: 800;
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
  gap: 8px;
}

.stock-item {
  display: grid;
  grid-template-columns: 2.5fr 2fr 1fr;
  align-items: center;
  gap: 8px;
}

.product-name {
  font-size: var(--font-sm);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.stock-value {
  text-align: right;
  font-family: monospace;
  font-size: var(--font-sm);
}

.stock-value.level-ok { color: var(--primary-text-color); }
.stock-value.level-low { color: var(--warning-color); }
.stock-value.level-critical { color: var(--error-color); font-weight: 700; }
.stock-value.level-out { color: var(--text-disabled); }
</style>
