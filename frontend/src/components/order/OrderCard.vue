<template>
  <div class="order-card">
    <div class="order-header">
      <h4>订单 #{{ order.id }}</h4>
      <span class="order-time">{{ formattedTime }} (UTC+8)</span>
    </div>

    <!-- 【核心改动】将 <ul> 改为 <div>，并修改内部结构 -->
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
          <span class="item-price">{{ formatYuan(item.product_price) }}</span>
        </div>
        <!-- 数量 -->
        <span class="item-quantity">x {{ item.quantity }}</span>
      </div>
    </div>

    <div class="order-footer">
      <span class="total-amount">
        <span v-if="order.final_amount < order.gross_amount" class="struck">
          {{ formatYuan(order.gross_amount) }}
        </span>
        总计: {{ formatYuan(order.final_amount) }}
      </span>
      <!-- 【修改】只有在待处理状态下才显示按钮 -->
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
import { NButton } from 'naive-ui'
import { formatTimestamp } from '@/utils/dateFormatter'
import { formatYuan } from '@/utils/money'
import type { Schemas } from '@/api/client'

const props = withDefaults(
  defineProps<{ order: Schemas['OrderResponse']; isCompleted?: boolean }>(),
  { isCompleted: false }
)
defineEmits<{ (e: 'complete', id: number): void; (e: 'cancel', id: number): void }>()

const formattedTime = computed(() => {
  return formatTimestamp(props.order.timestamp)
})
</script>

<style scoped>
.order-card.is-completed {
  border-left-color: var(--bg-secondary); /* 已完成的订单用灰色边框 */
  opacity: 0.8;
}
.button-group {
  display: flex;
  gap: var(--space-sm);
}
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

/* --- 订单头 --- */
.order-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-sm);
  padding-bottom: var(--space-sm);
  border-bottom: 1px solid var(--border-color);
}
.order-header h4 {
  margin: 0;
  font-size: var(--font-base);
  color: var(--primary-text-color);
}
.order-header .order-time {
  font-size: var(--font-sm);
  color: var(--text-muted);
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
  gap: var(--space-sm);
  min-width: 0;
}

.item-name {
  font-weight: var(--weight-bold);
  font-size: var(--font-base);
  color: var(--primary-text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-lot {
  align-self: flex-start;
  padding: 0 var(--space-sm);
  border-radius: var(--radius-sm);
  background-color: var(--accent-color-light);
  color: var(--accent-color);
  font-size: var(--font-xs);
  line-height: 1.6;
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
  padding-top: var(--space-sm);
  border-top: 1px solid var(--border-color);
}
.total-amount strong {
  font-size: var(--font-lg);
  color: var(--accent-color);
}
.total-amount .struck {
  margin-right: var(--space-sm);
  color: var(--text-disabled);
  text-decoration: line-through;
  font-weight: var(--weight-regular);
}
</style>
