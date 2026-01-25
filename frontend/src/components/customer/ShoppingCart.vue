<template>
  <!-- 
    根容器：
    桌面端：普通 div，撑满高度
    移动端：仅作为逻辑容器，内容通过 fixed 定位跳出
  -->
  <div class="shopping-cart-root">
    
    <!-- ✅ 移动端遮罩层 (点击关闭) -->
    <transition name="fade">
      <div 
        v-if="isMobile && expanded" 
        class="cart-backdrop"
        @click="toggleCart"
      ></div>
    </transition>

    <!-- 购物车主体 -->
    <div
      class="cart-container"
      :class="{ 
        'is-mobile': isMobile, 
        'is-expanded': expanded,
        'is-desktop': !isMobile
      }"
    >
      <!-- 1. 顶部/手机底部 触发栏 -->
      <div class="cart-header" @click="isMobile ? toggleCart() : null">
        <div class="header-left">
          <span class="icon">🛒</span>
          <span class="title">购物车</span>
          <span class="count-badge" v-if="cartCount > 0">{{ cartCount }}</span>
        </div>
        
        <div class="header-right">
          <span class="total-price">¥{{ total.toFixed(2) }}</span>
          <!-- 手机端箭头 -->
          <span v-if="isMobile" class="toggle-icon">
            {{ expanded ? '▼' : '▲' }}
          </span>
        </div>
      </div>

      <!-- 2. 内容区域 (列表 + 结算) -->
      <div class="cart-body">
        <div class="list-scroll-area">
          <ul v-if="cart.length" class="cart-list">
            <li v-for="item in cart" :key="item.id" class="cart-item">
              <div class="item-info">
                <div class="item-name">{{ item.name }}</div>
                <div class="item-price-row">
                  <span class="unit-price">¥{{ item.price }}</span>
                </div>
              </div>

              <div class="item-controls">
                <button 
                  class="ctrl-btn minus"
                  @click.stop="$emit('removeFromCart', item.id)"
                >-</button>
                <span class="qty">{{ item.quantity }}</span>
                <button 
                  class="ctrl-btn plus"
                  @click.stop="$emit('addToCart', item)"
                >+</button>
              </div>
            </li>
          </ul>

          <!-- 空购物车提示 -->
          <div v-else class="empty-cart">
            <span class="empty-icon">🍂</span>
            <p>还没选购商品</p>
          </div>
        </div>

        <!-- 底部结算区 -->
        <div class="cart-footer">
          <div class="footer-row">
            <span>合计</span>
            <span class="big-total">¥{{ total.toFixed(2) }}</span>
          </div>
          <n-button
            type="primary"
            block
            round
            size="large"
            :disabled="!cart.length || isCheckingOut"
            :loading="isCheckingOut"
            @click="$emit('checkout')"
            class="checkout-btn"
          >
            {{ isCheckingOut ? '提交中...' : '去结算' }}
          </n-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { NButton } from 'naive-ui'

const props = defineProps({
  cart: { type: Array, required: true },
  total: { type: Number, required: true },
  isCheckingOut: { type: Boolean, default: false }
})

defineEmits(['addToCart', 'removeFromCart', 'checkout'])

const isMobile = ref(false)
const expanded = ref(false)

const cartCount = computed(() => props.cart.reduce((sum, item) => sum + item.quantity, 0))

function checkMobile() {
  // ✅ 统一断点为 768px
  isMobile.value = window.innerWidth <= 768
  if (!isMobile.value) {
    // 桌面端默认永远展开，expanded 状态仅用于移动端
    expanded.value = true 
  } else {
    expanded.value = false
  }
}

function toggleCart() {
  if (isMobile.value) {
    expanded.value = !expanded.value
  }
}

onMounted(() => {
  checkMobile()
  window.addEventListener('resize', checkMobile)
})
onUnmounted(() => {
  window.removeEventListener('resize', checkMobile)
})
</script>

<style scoped>
/* ============================================================================
   通用样式 (Desktop First)
============================================================================ */
.shopping-cart-root {
  height: 100%;
  width: 100%;
}

.cart-container {
  display: flex;
  flex-direction: column;
  background: var(--card-bg-color);
  height: 100%;
  overflow: hidden;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

/* 头部 */
.cart-header {
  flex: 0 0 60px; /* 与左侧 Sidebar 标题高度一致 */
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  border-bottom: 1px solid var(--border-color);
  font-weight: 700;
  color: var(--primary-text-color);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.count-badge {
  background: var(--error-color, #d03050);
  color: white;
  font-size: 0.75rem;
  padding: 1px 6px;
  border-radius: 10px;
  line-height: 1.2;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}
.total-price {
  font-family: 'DIN Alternate', sans-serif;
  font-size: 1.1rem;
  color: var(--accent-color);
}

/* 列表区 */
.cart-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  position: relative;
}

.list-scroll-area {
  flex: 1;
  overflow-y: auto;
  padding: 0 12px;
}

/* 列表项 */
.cart-list {
  list-style: none;
  padding: 0;
  margin: 0;
}
.cart-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 0;
  border-bottom: 1px dashed var(--border-color);
}
.item-info {
  flex: 1;
  min-width: 0;
  margin-right: 10px;
}
.item-name {
  font-size: 0.9rem;
  font-weight: 500;
  margin-bottom: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.unit-price {
  font-size: 0.85rem;
  color: var(--text-muted);
}

/* 加减按钮控件 */
.item-controls {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--bg-secondary, #f5f5f9);
  padding: 2px;
  border-radius: 6px;
}
.ctrl-btn {
  width: 24px;
  height: 24px;
  border: none;
  background: white;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: var(--primary-text-color);
  font-size: 16px;
  line-height: 1;
  box-shadow: 0 1px 2px rgba(0,0,0,0.05);
}
.ctrl-btn:active {
  transform: scale(0.9);
}
.qty {
  font-weight: 700;
  font-size: 0.9rem;
  min-width: 16px;
  text-align: center;
}

/* 底部结算 */
.cart-footer {
  padding: 16px;
  border-top: 1px solid var(--border-color);
  background: var(--card-bg-color);
}
.footer-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 12px;
  font-size: 0.9rem;
  color: var(--text-muted);
}
.big-total {
  font-size: 1.4rem;
  font-weight: 800;
  color: var(--accent-color);
}
.checkout-btn {
  font-weight: 700;
}

/* 空状态 */
.empty-cart {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  min-height: 200px;
}
.empty-icon {
  font-size: 2.5rem;
  margin-bottom: 10px;
  opacity: 0.5;
}

/* ============================================================================
   📱 Mobile Specific Styles (移动端抽屉模式)
============================================================================ */
.cart-container.is-mobile {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  width: 100%;
  height: auto; /* 自动高度，不占满全屏 */
  max-height: 80vh; /* 最大高度 */
  z-index: 2000;
  border-radius: 20px 20px 0 0;
  box-shadow: 0 -4px 20px rgba(0,0,0,0.15);
  transform: translateY(calc(100% - 60px - env(safe-area-inset-bottom))); /* 默认只露出头部 */
  padding-bottom: env(safe-area-inset-bottom); /* 适配 iPhone X 横条 */
}

/* 移动端头部特殊处理 */
.cart-container.is-mobile .cart-header {
  height: 60px;
  background: var(--card-bg-color); /* 保证不透明 */
  cursor: pointer;
  border-bottom: none; /* 收起时不需要线 */
}

/* 移动端展开状态 */
.cart-container.is-mobile.is-expanded {
  transform: translateY(0);
}
.cart-container.is-mobile.is-expanded .cart-header {
  border-bottom: 1px solid var(--border-color);
}

/* 移动端遮罩层 */
.cart-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(2px);
  z-index: 1999;
}

/* 动画 */
.fade-enter-active, .fade-leave-active {
  transition: opacity 0.3s;
}
.fade-enter-from, .fade-leave-to {
  opacity: 0;
}
</style>