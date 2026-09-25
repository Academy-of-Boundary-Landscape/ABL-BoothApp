<template>
  <!--
    根容器：
    宽屏（sidebar）：普通 div，撑满侧栏高度。
    平板竖屏 / 手机（bar）：零尺寸逻辑容器，内容通过 fixed 定位跳出，避免抢占内容区高度。
  -->
  <div class="shopping-cart" :class="[`shopping-cart--${variant}`]">
    <!-- ✅ 展开态遮罩层（仅底部条） -->
    <Transition name="fade">
      <div v-if="isBar && expanded" class="cart-backdrop" @click="toggleCart"></div>
    </Transition>

    <!-- 购物车主体 -->
    <div class="cart-container" :class="{ 'cart-container--bar': isBar, 'is-expanded': expanded }">
      <!-- 触发栏：底部条可点开合；侧栏为纯标题 -->
      <div
        class="cart-header"
        :class="{ 'cart-header--clickable': isBar }"
        @click="isBar && toggleCart()"
      >
        <div class="header-left">
          <span class="header-icon">🛒</span>
          <span class="header-title">购物车</span>
          <span v-if="cartCount > 0" class="count-badge">{{ cartCount }}</span>
          <span v-if="isBar" class="count-unit">件</span>
        </div>

        <div class="header-right">
          <span v-if="isBar" class="total-label">合计</span>
          <span class="total-price"><Money :value="payable" size="lg" /></span>
          <!-- 底部条：展开 / 收起指示 -->
          <span v-if="isBar" class="toggle-icon">{{ expanded ? '▼' : '▲' }}</span>
        </div>
      </div>

      <!-- 内容区域（列表 + 结算） -->
      <div class="cart-body">
        <div class="list-scroll-area">
          <ul v-if="cart.length" class="cart-list">
            <li v-for="item in cart" :key="item.id" class="cart-item">
              <div class="item-thumb">
                <img
                  v-if="item.image_url"
                  :src="item.image_url"
                  :alt="item.name"
                  class="thumb-img"
                />
                <span v-else class="thumb-fallback">{{ item.name?.charAt(0) || '?' }}</span>
              </div>

              <div class="item-info">
                <div class="item-name">{{ item.name }}</div>
                <div class="item-price-row">
                  <span class="unit-price"><Money :value="item.unit_price" size="md" /></span>
                </div>
              </div>

              <div class="item-controls">
                <button
                  type="button"
                  class="ctrl-btn minus"
                  aria-label="减少一件"
                  @click.stop="$emit('removeFromCart', item.id)"
                >
                  −
                </button>
                <span class="qty">{{ item.quantity }}</span>
                <button
                  type="button"
                  class="ctrl-btn plus"
                  aria-label="增加一件"
                  @click.stop="$emit('addToCart', item)"
                >
                  +
                </button>
              </div>
            </li>
          </ul>

          <!-- 空购物车提示 -->
          <div v-else class="empty-cart">
            <EmptyState icon="🛒" title="购物车是空的">
              <template #hint
                >点击商品卡片上的 <span class="hint-plus">+</span> 加入购物车</template
              >
            </EmptyState>
          </div>
        </div>

        <!-- 底部结算区 -->
        <div class="cart-footer">
          <!-- D3：顾客端价格必须可解释。被优化过的总价必须说清楚是怎么来的，
               否则自助点单的顾客不会信任它。 -->
          <div v-if="payable !== total" class="footer-row subtle">
            <span>原价</span>
            <span class="struck"><Money :value="total" size="sm" strike /></span>
          </div>
          <div v-for="d in discounts" :key="d.name" class="footer-row discount">
            <span
              >已应用：{{ d.name }}<template v-if="d.count > 1"> ×{{ d.count }}</template></span
            >
            <span>−{{ formatYuan(d.saved) }}</span>
          </div>
          <p v-if="quoteNotice" class="quote-notice">{{ quoteNotice }}</p>
          <!-- 报价在途：此时显示的 payable 是原价，明说一句，别让顾客以为优惠没了。 -->
          <p v-if="quotePending" class="quote-notice">优惠计算中…</p>
          <div class="footer-row">
            <span>应付</span>
            <span class="big-total"><Money :value="payable" size="lg" /></span>
          </div>
          <n-button
            type="primary"
            block
            round
            size="large"
            :disabled="!cart.length || isCheckingOut || quotePending"
            :loading="isCheckingOut"
            class="checkout-btn"
            @click="$emit('checkout')"
          >
            {{ isCheckingOut ? '提交中...' : '去结算' }}
          </n-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onUnmounted, watch } from 'vue'
import { NButton } from 'naive-ui'
import { EmptyState, Money } from '@/components/ui'
import { formatYuan, type Cents } from '@/utils/money'
import type { Schemas } from '@/api/client'
import type { QuoteDiscount } from '@/utils/quote'

// 与 customerStore 里的同名别名保持一致：场次商品 + 数量。
type CartItem = Schemas['ProductEventProduct'] & { quantity: number }

const props = withDefaults(
  defineProps<{
    cart: CartItem[]
    /** 原价合计（分） */
    total: Cents
    /** 折后应付（分）。报价失败时等于 total。 */
    payable: Cents
    /** [{ name, saved, count }] */
    discounts?: QuoteDiscount[]
    /** 报价失败时的提示。非空就必须显示——不能让顾客以为原价就是应付价。 */
    quoteNotice?: string | null
    /** 报价在途（debounce/请求中）。为真时 payable 只是原价，不能结算。 */
    quotePending?: boolean
    isCheckingOut?: boolean
    /** sidebar：宽屏侧栏，常展开；bar：平板竖屏 / 手机底部可展开条。 */
    variant?: 'sidebar' | 'bar'
  }>(),
  {
    discounts: () => [],
    quoteNotice: null,
    quotePending: false,
    isCheckingOut: false,
    variant: 'sidebar',
  }
)

defineEmits<{
  (e: 'addToCart', item: CartItem): void
  (e: 'removeFromCart', id: number): void
  (e: 'checkout'): void
}>()

const isBar = computed(() => props.variant === 'bar')
const expanded = ref(false)

const cartCount = computed(() => props.cart.reduce((sum, item) => sum + item.quantity, 0))

function toggleCart() {
  if (isBar.value) expanded.value = !expanded.value
}

function syncBodyScrollLock(locked: boolean) {
  if (typeof document === 'undefined') return
  document.body.style.overflow = locked ? 'hidden' : ''
}

onUnmounted(() => {
  syncBodyScrollLock(false)
})

watch(
  [isBar, expanded],
  ([bar, isExpanded]) => {
    syncBodyScrollLock(bar && isExpanded)
  },
  { immediate: true }
)
</script>

<style scoped>
/* ============================================================================
   通用（侧栏优先）
============================================================================ */
.shopping-cart {
  --cart-bar-h: 60px;

  height: 100%;
  width: 100%;
  min-height: 0;
}

/* 底部条：零尺寸根，避免在 flex 父容器里抢占空间 */
.shopping-cart--bar {
  position: relative;
  width: 0;
  height: 0;
  min-height: 0;
}

.cart-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  background: var(--card-bg-color);
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

/* 头部 */
.cart-header {
  flex: 0 0 var(--cart-bar-h);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-sm);
  padding: 0 var(--space-lg);
  border-bottom: 1px solid var(--border-color);
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
}
.cart-header--clickable {
  cursor: pointer;
}

.header-left {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  min-width: 0;
}
.header-icon {
  font-size: var(--font-xl);
}
.header-title {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
}
.count-badge {
  min-width: 24px;
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-lg);
  background: var(--error-color);
  color: var(--text-white);
  font-size: var(--font-base);
  font-weight: var(--weight-bold);
  line-height: 1.3;
  text-align: center;
}
.count-unit {
  color: var(--text-muted);
  font-size: var(--font-sm);
  font-weight: var(--weight-regular);
}

.header-right {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}
.total-label {
  color: var(--text-muted);
  font-size: var(--font-sm);
  font-weight: var(--weight-regular);
}
.total-price {
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}
.toggle-icon {
  color: var(--text-muted);
  font-size: var(--font-sm);
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
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 0 var(--space-md);
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
  gap: var(--space-sm);
  padding: var(--space-sm) 0;
  border-bottom: 1px dashed var(--border-color);
}

/* 商品缩略图（圆形） */
.item-thumb {
  flex-shrink: 0;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  overflow: hidden;
  background: var(--bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
}
.thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.thumb-fallback {
  font-size: var(--font-base);
  font-weight: var(--weight-bold);
  color: var(--text-muted);
}

.item-info {
  flex: 1;
  min-width: 0;
}
.item-name {
  margin-bottom: var(--space-xs);
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.unit-price {
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}

/* 加减按钮控件 */
.item-controls {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  padding: var(--space-xs);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.ctrl-btn {
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--radius-md);
  font-size: var(--font-xl);
  font-weight: var(--weight-bold);
  line-height: 1;
  box-shadow: var(--shadow-sm);
  cursor: pointer;
  -webkit-tap-highlight-color: transparent;
  touch-action: manipulation;
  transition: transform 0.12s;
}
.ctrl-btn.minus {
  background: var(--bg-secondary);
  color: var(--primary-text-color);
}
.ctrl-btn.plus {
  background: var(--accent-color);
  color: var(--text-white);
}
.ctrl-btn:active {
  transform: scale(0.9);
}
.qty {
  min-width: 28px;
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  text-align: center;
  font-variant-numeric: tabular-nums;
}

/* 底部结算 */
.cart-footer {
  padding: var(--space-lg);
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- iPhone 底部安全区适配，env() 无法用 space token 表达 */
  padding-bottom: calc(var(--space-lg) + env(safe-area-inset-bottom, 0px));
  border-top: 1px solid var(--border-color);
  background: var(--card-bg-color);
}
.footer-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: var(--space-md);
  font-size: var(--font-base);
  color: var(--text-muted);
}
.footer-row.subtle {
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.footer-row.subtle .struck {
  text-decoration: line-through;
}
.footer-row.discount {
  color: var(--success-color);
  font-size: var(--font-sm);
}
.quote-notice {
  margin: var(--space-xs) 0;
  font-size: var(--font-sm);
  color: var(--warning-color);
  line-height: 1.4;
}
.big-total {
  color: var(--accent-color);
  font-weight: var(--weight-bold);
  font-variant-numeric: tabular-nums;
}
.checkout-btn {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  height: 48px;
}

/* 空状态 */
.empty-cart {
  height: 100%;
  min-height: 200px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-xl) var(--space-lg);
}
.hint-plus {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--accent-color);
  color: var(--text-white);
  font-size: var(--font-sm);
  font-weight: var(--weight-bold);
  line-height: 1;
}

/* ============================================================================
   底部可展开条（平板竖屏 / 手机）
============================================================================ */
.cart-container--bar {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  width: 100%;
  height: auto;
  max-height: 80vh;
  z-index: 2000;
  border-radius: var(--radius-xl) var(--radius-xl) 0 0;
  box-shadow: var(--shadow-xl);
  transform: translateY(calc(100% - var(--cart-bar-h) - env(safe-area-inset-bottom)));
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- iPhone 底部安全区适配，env() 无法用 space token 表达 */
  padding-bottom: env(safe-area-inset-bottom);
}
.cart-container--bar .cart-header {
  border-bottom: none;
}
.cart-container--bar.is-expanded {
  transform: translateY(0);
}
.cart-container--bar.is-expanded .cart-header {
  border-bottom: 1px solid var(--border-color);
}
.cart-container--bar .cart-body {
  max-height: calc(80vh - var(--cart-bar-h) - env(safe-area-inset-bottom, 0px));
}

/* 展开态遮罩层 */
.cart-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1999;
  background: var(--overlay-color);
  backdrop-filter: blur(2px);
}

/* 窄屏（手机）：允许商品名占 2 行，避免过早被截断 */
@media (--phone) {
  .item-name {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    white-space: normal;
    overflow: hidden;
    line-height: 1.3;
  }
}
</style>
