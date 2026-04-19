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
          <img v-if="item.product_image_url" :src="item.product_image_url" :alt="item.product_name" />
          <div v-else class="no-img-placeholder">?</div>
        </div>
        <!-- 商品信息 -->
        <div class="item-details">
          <span class="item-name">{{ item.product_name }}</span>
          <span class="item-price">¥{{ item.product_price.toFixed(2) }}</span>
        </div>
        <!-- 数量 -->
        <span class="item-quantity">x {{ item.quantity }}</span>
      </div>
    </div>

    <div class="order-footer">
      <span class="total-amount">总计: ¥{{ order.total_amount.toFixed(2) }}</span>
      <!-- 【修改】只有在待处理状态下才显示按钮 -->
      <div v-if="!isCompleted" class="button-group">
        <n-button tertiary type="error" size="small" @click="$emit('cancel', order.id)">取消</n-button>
        <n-button type="primary" @click="$emit('complete', order.id)">完成配货</n-button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue';
import { NButton } from 'naive-ui';
import { formatTimestamp } from '@/utils/dateFormatter';

const props = defineProps({
  order: { type: Object, required: true },
  isCompleted: { type: Boolean, default: false }
});
defineEmits(['complete', 'cancel']);

// 【新增】定义后端 URL 以便正确加载图片
const backendUrl = 'http://127.0.0.1:5140';

const formattedTime = computed(() => {
  return formatTimestamp(props.order.timestamp);
});
</script>

<style scoped>
.order-card.is-completed {
  border-left-color: var(--order-completed); /* 已完成的订单用灰色边框 */
  opacity: 0.8;
}
.button-group { display: flex; gap: 8px; }
.btn-cancel { /* ... 危险操作的样式 ... */ }
/* --- 整体卡片样式 --- */
.order-card {
  background-color: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-left: 4px solid var(--accent-color);
  padding: 10px 14px;
  margin-bottom: 10px;
  border-radius: var(--radius-md);
  display: flex;
  flex-direction: column;
}

/* --- 订单头 --- */
.order-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border-color);
}
.order-header h4 { margin: 0; font-size: var(--font-base); color: var(--primary-text-color); }
.order-header .order-time { font-size: var(--font-sm); color: var(--text-muted); }

/* --- 商品列表 --- */
.item-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 6px;
}

.order-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.item-thumbnail {
  flex-shrink: 0;
}

.item-thumbnail img, .no-img-placeholder {
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
  gap: 6px;
  min-width: 0;
}

.item-name {
  font-weight: 600;
  font-size: var(--font-base);
  color: var(--primary-text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-price {
  font-size: var(--font-sm);
  color: var(--text-muted);
  flex-shrink: 0;
}

.item-quantity {
  font-size: var(--font-md);
  font-weight: 800;
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
  padding-top: 8px;
  border-top: 1px solid var(--border-color);
}
.total-amount strong {
  font-size: var(--font-lg);
  color: var(--accent-color);
}
/* 【新增】按钮组容器样式 */
.actions {
  display: flex;
  gap: 0.75rem; /* 按钮之间的间距 */
}

/* 【修改】通用按钮样式，确保 .btn 基础样式存在 */
.btn {
  padding: 8px 16px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-weight: bold;
  border: 1px solid;
  transition: background-color 0.2s, color 0.2s;
}

/* “完成”按钮样式 */
.btn-complete {
  background-color: var(--accent-color);
  color: var(--bg-color);
  border-color: var(--accent-color);
}

/* “取消”按钮样式 */
.btn-cancel {
  background-color: transparent;
  color: var(--error-color);
  border-color: var(--error-color);
}

.btn-cancel:hover {
  background-color: var(--error-color);
  color: var(--text-white);
}
</style>