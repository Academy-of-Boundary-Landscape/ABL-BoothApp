<template>
  <div
    class="order-card"
    :class="{ 'order-card--completed': isCompleted, 'order-card--refunded': fullyRefunded }"
  >
    <div class="order-header">
      <div class="order-title">
        <h4>订单 #{{ order.id }}</h4>
        <n-tag v-if="fullyRefunded" size="small" round :bordered="false">已全部退货</n-tag>
      </div>
      <span class="order-time">{{ formattedTime }} (UTC+8)</span>
    </div>

    <div class="item-list">
      <div v-for="item in order.items" :key="item.id" class="order-item">
        <!-- 缩略图容器 -->
        <div class="item-thumbnail">
          <img
            v-if="item.product_image_url"
            :src="item.product_image_url"
            :alt="item.product_name"
          />
          <div v-else class="no-img-placeholder">?</div>
        </div>
        <!-- 商品信息 -->
        <div class="item-details">
          <span class="item-name">{{ item.product_name }}</span>
          <!-- 同一个商品可能在一张订单里出现两行（2 件进套装、1 件散着，spec 4.5）。
               不标出来，摊主配货时会以为系统重复计数了。 -->
          <span v-if="item.lot_name" class="item-lot">{{ item.lot_name }}</span>
          <span v-if="isCompleted && item.refunded_qty > 0" class="item-refund">
            已退 {{ item.refunded_qty }}
          </span>
          <span class="item-price">{{ formatYuan(item.product_price) }}</span>
        </div>
        <!-- 数量 -->
        <span class="item-quantity">x {{ item.quantity }}</span>
      </div>
    </div>

    <div class="order-footer">
      <div class="order-summary">
        <span class="total-amount">
          <span v-if="order.final_amount < order.gross_amount" class="struck">
            {{ formatYuan(order.gross_amount) }}
          </span>
          总计: {{ formatYuan(order.final_amount) }}
        </span>
        <span v-if="order.refunded_amount > 0" class="refunded-total">
          已退 {{ formatYuan(order.refunded_amount) }}
        </span>
      </div>

      <!-- 只有在待处理状态下才显示按钮 -->
      <div v-if="!isCompleted" class="button-group">
        <n-button tertiary type="error" size="small" @click="$emit('cancel', order.id)"
          >取消</n-button
        >
        <n-button type="primary" @click="$emit('complete', order.id)">完成配货</n-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton, NTag } from 'naive-ui'
import { formatTimestamp } from '@/utils/dateFormatter'
import { formatYuan } from '@/utils/money'
import { isFullyRefunded } from '@/utils/order'
import type { Schemas } from '@/api/client'

const props = withDefaults(
  defineProps<{ order: Schemas['OrderResponse']; isCompleted?: boolean }>(),
  { isCompleted: false }
)
defineEmits<{ (e: 'complete', id: number): void; (e: 'cancel', id: number): void }>()

const formattedTime = computed(() => {
  return formatTimestamp(props.order.timestamp)
})

// 已完成单每行都退满（`refunded_qty >= quantity`）时整张卡片置灰。
const fullyRefunded = computed(() => props.isCompleted && isFullyRefunded(props.order))
</script>

<style scoped>
/* --- 整体卡片样式 --- */
.order-card {
  background-color: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-left: 4px solid var(--accent-color);
  padding: var(--space-sm) var(--space-md);
  margin-bottom: var(--space-sm);
  border-radius: var(--radius-md);
  display: flex;
  flex-direction: column;
}

.order-card--completed {
  border-left-color: var(--bg-secondary);
}

.order-card--refunded {
  opacity: 0.6;
  border-left-color: var(--text-disabled);
}

/* --- 订单头 --- */
.order-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-sm);
  margin-bottom: var(--space-sm);
  padding-bottom: var(--space-sm);
  border-bottom: 1px solid var(--border-color);
}

.order-title {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  min-width: 0;
}

.order-header h4 {
  margin: 0;
  font-size: var(--font-base);
  color: var(--primary-text-color);
}

.order-header .order-time {
  font-size: var(--font-sm);
  color: var(--text-muted);
  flex-shrink: 0;
}

/* --- 商品列表 --- */
.item-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  margin-bottom: var(--space-sm);
}

.order-item {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.item-thumbnail {
  flex-shrink: 0;
}

.item-thumbnail img,
.no-img-placeholder {
  width: 36px;
  height: 36px;
  object-fit: cover;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-color);
}

.no-img-placeholder {
  display: flex;
  justify-content: center;
  align-items: center;
  color: var(--text-disabled);
  background-color: var(--bg-color);
  font-size: var(--font-sm);
}

.item-details {
  flex-grow: 1;
  display: flex;
  flex-direction: row;
  align-items: baseline;
  flex-wrap: wrap;
  gap: var(--space-sm);
  min-width: 0;
}

.item-name {
  font-weight: var(--weight-bold);
  font-size: var(--font-base);
  color: var(--primary-text-color);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-lot {
  padding: 0 var(--space-sm);
  border-radius: var(--radius-sm);
  background-color: var(--accent-color-light);
  color: var(--accent-color);
  font-size: var(--font-xs);
  line-height: 1.6;
}

.item-refund {
  color: var(--error-color);
  font-size: var(--font-xs);
}

.item-price {
  font-size: var(--font-sm);
  color: var(--text-muted);
  flex-shrink: 0;
}

.item-quantity {
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
  flex-shrink: 0;
  min-width: 32px;
  text-align: right;
}

/* --- 订单尾 --- */
.order-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-sm);
  padding-top: var(--space-sm);
  border-top: 1px solid var(--border-color);
}

.order-summary {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: var(--space-sm);
}

.total-amount {
  color: var(--primary-text-color);
}

.total-amount .struck {
  margin-right: var(--space-sm);
  color: var(--text-disabled);
  text-decoration: line-through;
  font-weight: var(--weight-regular);
}

.refunded-total {
  color: var(--error-color);
  font-size: var(--font-sm);
}

.button-group {
  display: flex;
  gap: var(--space-sm);
}
</style>
