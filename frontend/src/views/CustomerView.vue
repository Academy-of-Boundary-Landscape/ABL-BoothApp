<template>
  <div class="customer-view">
    <!-- ================================================================
         传统模式：Sidebar + ProductGrid + Cart
    ================================================================= -->
    <template v-if="!isVisionMode">
      <!-- Sidebar：分类 -->
      <div class="sidebar" v-if="!isMobile">
        <div class="sidebar-header">
          <span class="header-title">商品分类</span>
        </div>

        <n-scrollbar class="sidebar-scroll" content-class="sidebar-content">
          <div
            class="menu-item"
            :class="{ active: selectedCategory === '' }"
            @click="selectedCategory = ''"
          >
            <span class="menu-text">全部</span>
            <div class="active-indicator" v-if="selectedCategory === ''"></div>
          </div>

          <div
            v-for="cat in categoryOptions"
            :key="cat"
            class="menu-item"
            :class="{ active: selectedCategory === cat }"
            @click="selectedCategory = cat"
          >
            <span class="menu-text">{{ cat }}</span>
            <div class="active-indicator" v-if="selectedCategory === cat"></div>
          </div>
        </n-scrollbar>
      </div>

      <!-- 中间：商品展示 -->
      <div class="product-panel">
        <!-- 工具栏：分类(mobile) + 视图控制 + 模式切换 -->
        <div class="toolbar">
          <!-- Mobile 分类横滚 -->
          <div class="toolbar-categories" v-if="isMobile">
            <div
              class="cat-chip"
              :class="{ active: selectedCategory === '' }"
              @click="selectedCategory = ''"
            >全部</div>
            <div
              v-for="cat in categoryOptions"
              :key="cat"
              class="cat-chip"
              :class="{ active: selectedCategory === cat }"
              @click="selectedCategory = cat"
            >{{ cat }}</div>
          </div>

          <div class="toolbar-row">
            <!-- 管理控件：默认折叠，点齿轮展开 -->
            <div class="toolbar-left" v-if="showAdminControls">
              <span class="toolbar-label">视图</span>
              <div class="slider-wrap">
                <n-slider
                  v-model:value="cardSizeIndex"
                  :min="0"
                  :max="2"
                  :step="1"
                  :tooltip="false"
                  @update:value="onCardSizeUserChange"
                />
              </div>
            </div>

            <div class="toolbar-center">
              <div class="mode-toggle">
                <button
                  class="mode-btn"
                  :class="{ active: !isVisionMode }"
                  @click="isVisionMode = false"
                >商品列表</button>
                <button
                  class="mode-btn"
                  :class="{ active: isVisionMode }"
                  @click="isVisionMode = true"
                >拍照识别</button>
              </div>
            </div>

            <div class="toolbar-right">
              <n-button
                v-if="showAdminControls"
                size="small"
                round
                :type="isEditMode ? 'primary' : 'default'"
                :secondary="!isEditMode"
                @click="toggleEditMode"
              >
                {{ isEditMode ? '保存顺序' : '调整顺序' }}
              </n-button>
              <button
                class="admin-toggle-btn"
                :class="{ active: showAdminControls }"
                @click="toggleAdminControls"
                title="展开/折叠管理控件"
              >⚙</button>
            </div>
          </div>

          <!-- 管理控件展开时：导航快捷入口 -->
          <div class="toolbar-nav" v-if="showAdminControls">
            <router-link to="/admin" class="nav-chip">管理后台</router-link>
            <router-link to="/vendor" class="nav-chip">摊主页面</router-link>
            <router-link to="/" class="nav-chip">展会选择</router-link>
          </div>
        </div>

        <!-- 标签筛选 -->
        <div v-if="allTags.length > 0" class="tag-filter-bar">
          <span
            v-for="tag in allTags"
            :key="tag"
            class="tag-chip"
            :class="{ active: selectedTag === tag }"
            @click="selectedTag = selectedTag === tag ? null : tag"
          >
            {{ tag }}
          </span>
        </div>

        <!-- 商品列表 -->
        <div class="product-scroll">
          <n-spin :show="store.isLoading" content-class="spin-content">
            <ProductGrid
              v-if="mutableProducts.length > 0"
              v-model:products="mutableProducts"
              :card-size="cardSize"
              :editable="isEditMode"
              @add-to-cart="store.addToCart"
              @order-changed="saveOrderToLocal"
            />
            <div v-else class="empty-state">
              <span class="empty-emoji">🌵</span>
              <p>暂无商品</p>
            </div>
          </n-spin>
        </div>
      </div>
    </template>

    <!-- ================================================================
         Vision 模式：取景 + 结果
    ================================================================= -->
    <div v-if="isVisionMode" class="vision-panel">
      <!-- Vision 模式也有工具栏，保持切换入口 -->
      <div class="toolbar">
        <div class="toolbar-row">
          <div class="toolbar-left"></div>
          <div class="toolbar-center">
            <div class="mode-toggle">
              <button
                class="mode-btn"
                :class="{ active: !isVisionMode }"
                @click="isVisionMode = false"
              >商品列表</button>
              <button
                class="mode-btn"
                :class="{ active: isVisionMode }"
                @click="isVisionMode = true"
              >拍照识别</button>
            </div>
          </div>
          <div class="toolbar-right"></div>
        </div>
      </div>

      <div class="vision-panel__body">
        <VisionSearch
          camera-mode
          facing-mode="user"
          mode="order"
          :event-id="numericEventId"
          :top-k="5"
          @select="onVisionSelect"
        />
      </div>
    </div>

    <!-- ======== 购物车（两种模式共用） ======== -->
    <div class="cart-panel-desktop" v-if="!isMobile">
      <ShoppingCart
        :cart="store.cart"
        :total="store.cartTotal"
        :is-checking-out="isCheckingOut"
        @add-to-cart="store.addToCart"
        @remove-from-cart="store.removeFromCart"
        @checkout="handleCheckout"
      />
    </div>

    <ShoppingCart
      v-if="isMobile"
      :cart="store.cart"
      :total="store.cartTotal"
      :is-checking-out="isCheckingOut"
      @add-to-cart="store.addToCart"
      @remove-from-cart="store.removeFromCart"
      @checkout="handleCheckout"
    />

    <PaymentModal
      :show="showPaymentModal"
      :total="orderTotal"
      :qr-code-urls="store.qrCodeUrls"
      @close="closePaymentModal"
    />

    <!-- ======== 闲置吸引屏 ======== -->
    <Transition name="attract-fade">
      <div v-if="showAttractScreen" class="attract-screen">
        <div class="attract-content">
          <p class="attract-welcome">欢迎光临</p>
          <h1 class="attract-event" v-if="store.activeEvent?.name">{{ store.activeEvent.name }}</h1>

          <div class="attract-modes">
            <button class="attract-mode-btn" @click="enterWithMode(false)">
              <span class="attract-mode-icon">&#9783;</span>
              <span class="attract-mode-label">浏览点单</span>
              <span class="attract-mode-desc">翻看商品列表，点击加入购物车</span>
            </button>
            <button class="attract-mode-btn" @click="enterWithMode(true)">
              <span class="attract-mode-icon">&#9862;</span>
              <span class="attract-mode-label">拍照识别</span>
              <span class="attract-mode-desc">对准商品拍一拍，自动识别下单</span>
            </button>
          </div>

          <p class="attract-sub" @click="dismissAttractScreen">— 或点击此处直接开始 —</p>
        </div>
      </div>
    </Transition>

    <!-- ======== 连接状态横幅 ======== -->
    <Transition name="guide-slide">
      <div v-if="!isConnected && !showAttractScreen" class="disconnect-bar">
        连接已断开，请检查网络 · 恢复后将自动重连
      </div>
    </Transition>

    <!-- ======== 首次操作引导条 ======== -->
    <Transition name="guide-toast">
      <div v-if="showGuideBar && isConnected" class="guide-toast" @click="showGuideBar = false">
        <div class="guide-mode-label">
          {{ isVisionMode ? '拍照识别模式' : '浏览点单模式' }}
        </div>
        <div class="guide-steps">
          <template v-if="isVisionMode">
            <span class="guide-step"><span class="guide-num">1</span>对准商品拍照</span>
            <span class="guide-arrow">›</span>
            <span class="guide-step"><span class="guide-num">2</span>选择匹配结果</span>
            <span class="guide-arrow">›</span>
            <span class="guide-step"><span class="guide-num">3</span>结算付款</span>
          </template>
          <template v-else>
            <span class="guide-step"><span class="guide-num">1</span>点击商品加入购物车</span>
            <span class="guide-arrow">›</span>
            <span class="guide-step"><span class="guide-num">2</span>右侧查看已选</span>
            <span class="guide-arrow">›</span>
            <span class="guide-step"><span class="guide-num">3</span>结算付款</span>
          </template>
        </div>
        <div class="guide-switch-hint">
          顶部可切换到「{{ isVisionMode ? '浏览点单' : '拍照识别' }}」模式
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup>
import { useAlert } from '@/services/useAlert'
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useCustomerStore } from '@/stores/customerStore'
import { useConnectionCheck } from '@/composables/useConnectionCheck'
import ProductGrid from '@/components/customer/ProductGrid.vue'
import ShoppingCart from '@/components/customer/ShoppingCart.vue'
import PaymentModal from '@/components/customer/PaymentModal.vue'
import VisionSearch from '@/components/shared/VisionSearch.vue'
import { NScrollbar, NSpin, NSlider, NButton, useDialog } from 'naive-ui'

const props = defineProps({ id: { type: String, required: true } })
const store = useCustomerStore()
const dialog = useDialog()
const { isConnected } = useConnectionCheck()

// ===================== 模式切换 =====================
const isVisionMode = ref(false)
const numericEventId = computed(() => parseInt(props.id, 10) || null)

// 切换模式时重新展示引导
watch(isVisionMode, () => {
  if (!showAttractScreen.value) triggerGuide()
})

function onVisionSelect(hit) {
  const { showError } = useAlert()
  const product = (store.products || []).find(
    (p) => p.master_product_id === hit.master_product_id
  )
  if (!product) {
    showError(`未找到商品「${hit.name}」，可能不在本场展会中`)
    return
  }
  if (product.current_stock <= 0) {
    showError(`「${product.name}」已售罄`)
    return
  }
  store.addToCart(product)
}

// ===================== 传统模式 =====================
const showPaymentModal = ref(false)
const orderTotal = ref(0)
const isCheckingOut = ref(false)
const selectedCategory = ref('')
const isEditMode = ref(false)
const showAdminControls = ref(localStorage.getItem('customer_admin_controls') === 'true')
function toggleAdminControls() {
  showAdminControls.value = !showAdminControls.value
  localStorage.setItem('customer_admin_controls', showAdminControls.value)
  if (!showAdminControls.value && isEditMode.value) {
    isEditMode.value = false
    saveOrderToLocal()
  }
}
const cardSizeIndex = ref(1)
const userTouchedCardSize = ref(false)
const cardSize = computed(() => ['small', 'medium', 'large'][cardSizeIndex.value] || 'medium')
function onCardSizeUserChange() { userTouchedCardSize.value = true }

const isMobile = ref(false)
function syncLayout() {
  const mobile = window.innerWidth <= 768
  isMobile.value = mobile
  if (!userTouchedCardSize.value) cardSizeIndex.value = mobile ? 0 : 1
}

onMounted(() => {
  store.setupStoreForEvent(props.id)
  syncLayout()
  window.addEventListener('resize', syncLayout)
  ACTIVITY_EVENTS.forEach(e => window.addEventListener(e, onUserActivity, { passive: true }))
})
onUnmounted(() => {
  window.removeEventListener('resize', syncLayout)
  ACTIVITY_EVENTS.forEach(e => window.removeEventListener(e, onUserActivity))
  clearTimeout(idleTimer)
  clearTimeout(guideTimer)
})

const categoryOptions = computed(() => {
  const cats = (store.products || []).map(p => p.category).filter(c => c && c.trim())
  return [...new Set(cats)]
})

const allTags = computed(() => {
  const counts = new Map()
  ;(store.products || []).forEach(p => {
    ;(p.tags || '').split(',').filter(t => t.trim()).forEach(tag => {
      const t = tag.trim()
      counts.set(t, (counts.get(t) || 0) + 1)
    })
  })
  return [...counts.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([tag]) => tag)
})

const selectedTag = ref(null)

// ===== 排序 =====
const STORAGE_KEY = computed(() => `my_shop_custom_order::event::${props.id}`)

function readSavedIds() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY.value)
    const ids = JSON.parse(raw || '[]')
    return Array.isArray(ids) ? ids : []
  } catch { return [] }
}

function applySavedOrder(list, savedIds) {
  if (!savedIds?.length) return [...list]
  const pos = new Map(savedIds.map((id, i) => [id, i]))
  return [...list].sort((a, b) => {
    const ia = pos.has(a.id) ? pos.get(a.id) : Number.POSITIVE_INFINITY
    const ib = pos.has(b.id) ? pos.get(b.id) : Number.POSITIVE_INFINITY
    return ia - ib
  })
}

const baseOrderedProducts = computed(() => {
  const all = store.products || []
  return applySavedOrder(all, readSavedIds())
})

const mutableProducts = ref([])

watch(
  [baseOrderedProducts, selectedCategory, selectedTag],
  ([base, cat, tag]) => {
    if (isEditMode.value) return
    let subset = cat ? base.filter(p => p.category === cat) : base
    if (tag) {
      subset = subset.filter(p =>
        (p.tags || '').split(',').some(t => t.trim() === tag)
      )
    }
    // 售罄商品自动置底，有货的保持原有排序
    const inStock = subset.filter(p => p.current_stock > 0)
    const soldOut = subset.filter(p => p.current_stock <= 0)
    mutableProducts.value = [...inStock, ...soldOut]
  },
  { immediate: true }
)

function toggleEditMode() {
  isEditMode.value = !isEditMode.value
  if (!isEditMode.value) saveOrderToLocal()
}

function saveOrderToLocal() {
  const cat = selectedCategory.value
  const fullBase = baseOrderedProducts.value
  const draggedSubsetIds = mutableProducts.value.map(p => p.id)

  let mergedIds
  if (!cat) {
    mergedIds = draggedSubsetIds
  } else {
    const subsetIdSet = new Set(draggedSubsetIds)
    const baseIds = fullBase.map(p => p.id)
    const queue = [...draggedSubsetIds]
    mergedIds = baseIds.map(id => (subsetIdSet.has(id) ? queue.shift() : id))
  }

  try { localStorage.setItem(STORAGE_KEY.value, JSON.stringify(mergedIds)) }
  catch (e) { console.error('保存顺序失败', e) }
}

// ===================== 闲置吸引屏 =====================
const IDLE_TIMEOUT_MS = 60_000 // 60 秒无操作显示吸引屏
const showAttractScreen = ref(true) // 初始就显示吸引屏
const showGuideBar = ref(false)
let idleTimer = null

function resetIdleTimer() {
  clearTimeout(idleTimer)
  idleTimer = setTimeout(() => {
    showAttractScreen.value = true
    store.clearCart() // 顾客之间自动清空购物车
    isVisionMode.value = false // 回到默认的商品列表模式
    selectedTag.value = null
  }, IDLE_TIMEOUT_MS)
}

function dismissAttractScreen() {
  showAttractScreen.value = false
  selectedTag.value = null
  resetIdleTimer()
  store.fetchProductsForEvent()
  triggerGuide()
}

function enterWithMode(vision) {
  isVisionMode.value = vision
  dismissAttractScreen()
}

let guideTimer = null
function triggerGuide() {
  showGuideBar.value = true
  clearTimeout(guideTimer)
  guideTimer = setTimeout(() => { showGuideBar.value = false }, 8000)
}

// 监听任何交互事件来重置闲置计时器
const ACTIVITY_EVENTS = ['pointerdown', 'pointermove', 'keydown', 'scroll']
function onUserActivity() {
  if (!showAttractScreen.value) resetIdleTimer()
  // 任何交互都关闭引导条
  if (showGuideBar.value) showGuideBar.value = false
}

// 首次加购时关闭引导条
watch(() => store.cart.length, (newLen, oldLen) => {
  if (newLen > oldLen && showGuideBar.value) showGuideBar.value = false
})

// ===== 下单 =====
async function handleCheckout() {
  const { showError } = useAlert()
  if (isCheckingOut.value) return

  const itemCount = store.cartItemCount
  const totalAmount = store.cartTotal

  dialog.info({
    title: '确认下单',
    content: `共 ${itemCount} 件商品，合计 ¥${totalAmount.toFixed(2)}`,
    positiveText: '确认下单',
    negativeText: '再看看',
    onPositiveClick: async () => {
      isCheckingOut.value = true
      try {
        const newOrder = await store.submitOrder()
        if (newOrder) {
          orderTotal.value = store.cartTotal
          showPaymentModal.value = true
          store.clearCart()
          store.fetchProductsForEvent()
        }
      } catch (error) {
        showError(error?.message || '下单失败')
        store.clearCart()
        store.fetchProductsForEvent()
      } finally {
        isCheckingOut.value = false
      }
    },
  })
}
function closePaymentModal() {
  showPaymentModal.value = false
  // 付款完成是一个自然的"交接点"，直接回到吸引屏等待下一位顾客
  showAttractScreen.value = true
  store.clearCart()
  isVisionMode.value = false
}
</script>

<style scoped>
/* ===================== 根容器 ===================== */
.customer-view {
  --sidebar-w: 180px;
  --cart-w: 280px;

  display: flex;
  height: 100%;
  overflow: hidden;
  background-color: var(--bg-color);
  color: var(--primary-text-color);
}

/* ===================== Sidebar（仅桌面） ===================== */
.sidebar {
  flex: 0 0 var(--sidebar-w);
  width: var(--sidebar-w);
  min-height: 0;
  background: var(--card-bg-color);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
}
.sidebar-header {
  height: 48px;
  display: flex;
  align-items: center;
  padding: 0 16px;
  font-weight: 800;
  font-size: var(--font-md);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}
.sidebar-scroll { flex: 1; min-height: 0; }

:deep(.sidebar-content) {
  display: flex;
  flex-direction: column;
  padding: 8px;
  gap: 2px;
}
.menu-item {
  padding: 8px 12px;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: var(--font-base);
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: space-between;
  transition: all 0.15s;
}
.menu-item:hover {
  background-color: var(--bg-secondary);
  color: var(--primary-text-color);
}
.menu-item.active {
  background-color: color-mix(in srgb, var(--accent-color) 18%, transparent);
  color: var(--accent-color);
  font-weight: 600;
}
.active-indicator {
  width: 4px;
  height: 14px;
  border-radius: 2px;
  background-color: var(--accent-color);
}

/* ===================== 中间面板（传统 & Vision 共用结构） ===================== */
.product-panel,
.vision-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

/* ===== 工具栏 ===== */
.toolbar {
  flex-shrink: 0;
  border-bottom: 1px solid color-mix(in srgb, var(--border-color) 60%, transparent);
  background: var(--bg-color);
}

.toolbar-categories {
  display: flex;
  gap: 6px;
  padding: 8px 12px 0;
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
}
.cat-chip {
  flex-shrink: 0;
  padding: 4px 14px;
  border-radius: var(--radius-pill);
  font-size: var(--font-sm);
  cursor: pointer;
  background: var(--bg-secondary);
  color: var(--text-muted);
  border: 1px solid transparent;
  transition: all 0.15s;
}
.cat-chip.active {
  background: var(--accent-color);
  color: white;
}

.toolbar-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  gap: 8px;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--card-bg-color);
  padding: 4px 12px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--border-color);
}
.toolbar-label { font-size: var(--font-sm); color: var(--text-muted); }
.slider-wrap { width: 72px; }

.toolbar-center { flex-shrink: 0; }
.toolbar-right { display: flex; align-items: center; gap: 8px; }

.admin-toggle-btn {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: 1px solid var(--border-color);
  background: var(--bg-secondary);
  color: var(--text-muted);
  font-size: 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  flex-shrink: 0;
}
.admin-toggle-btn.active {
  background: var(--accent-color);
  color: white;
  border-color: var(--accent-color);
}

.toolbar-nav {
  display: flex;
  gap: 6px;
  padding: 0 12px 8px;
}
.nav-chip {
  padding: 4px 12px;
  border-radius: var(--radius-pill);
  font-size: var(--font-sm);
  background: var(--bg-secondary);
  color: var(--text-muted);
  text-decoration: none;
  border: 1px solid var(--border-color);
  transition: all 0.15s;
}
.nav-chip:hover {
  background: var(--accent-color);
  color: white;
  border-color: var(--accent-color);
}

/* 模式切换 */
.mode-toggle {
  display: flex;
  gap: 2px;
  background: var(--bg-secondary);
  padding: 3px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--border-color);
}
.mode-btn {
  border: none;
  background: transparent;
  padding: 4px 14px;
  border-radius: var(--radius-pill);
  font-size: var(--font-sm);
  cursor: pointer;
  color: var(--text-muted);
  transition: all 0.15s;
  white-space: nowrap;
}
.mode-btn.active {
  background: var(--accent-color);
  color: white;
  font-weight: 600;
}

/* ===== 标签筛选栏 ===== */
.tag-filter-bar {
  display: flex;
  gap: 8px;
  padding: 8px 12px;
  overflow-x: auto;
  flex-shrink: 0;
  scrollbar-width: none;
}
.tag-filter-bar::-webkit-scrollbar { display: none; }

.tag-chip {
  flex-shrink: 0;
  padding: 4px 14px;
  border-radius: 999px;
  border: 1.5px solid var(--border-color);
  background: var(--card-bg-color);
  color: var(--text-color);
  font-size: var(--font-sm, 13px);
  cursor: pointer;
  user-select: none;
  transition: all 0.15s;
  white-space: nowrap;
}
.tag-chip:hover {
  border-color: var(--accent-color);
}
.tag-chip.active {
  background: var(--accent-color);
  border-color: var(--accent-color);
  color: #fff;
  font-weight: 600;
}

/* ===== 商品滚动区 ===== */
.product-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 8px 12px;
}

.spin-content {
  min-height: 100%;
}

.empty-state {
  height: 100%;
  min-height: 200px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
}
.empty-emoji { font-size: 3rem; margin-bottom: 10px; }

/* ===== Vision 面板 body ===== */
.vision-panel__body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  padding: 12px;
  display: flex;
  flex-direction: column;
}

/* ===================== 购物车（桌面） ===================== */
.cart-panel-desktop {
  flex: 0 0 var(--cart-w);
  width: var(--cart-w);
  min-height: 0;
  border-left: 1px solid var(--border-color);
  background: var(--card-bg-color);
}

/* ===================== 闲置吸引屏 ===================== */
.attract-screen {
  position: fixed;
  inset: 0;
  z-index: 9000;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  user-select: none;
  overflow: hidden;
  background: var(--bg-color);
  /* iPhone X+ 刘海/底部横条安全区 */
  padding:
    env(safe-area-inset-top)
    env(safe-area-inset-right)
    env(safe-area-inset-bottom)
    env(safe-area-inset-left);
  box-sizing: border-box;
}

.attract-content {
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.attract-welcome {
  font-size: var(--font-xl);
  font-weight: 600;
  color: var(--text-muted);
  margin: 0;
  letter-spacing: 0.2em;
}

.attract-event {
  font-size: clamp(2rem, 5vw, 3.5rem);
  font-weight: 900;
  color: var(--accent-color);
  margin: 0 0 2rem;
  line-height: 1.2;
}

.attract-modes {
  display: flex;
  gap: 20px;
  margin: 2rem 0 0;
  flex-wrap: wrap;
  justify-content: center;
}

.attract-mode-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 24px 32px;
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--card-bg-color);
  cursor: pointer;
  transition: all 0.2s;
  min-width: 180px;
}
.attract-mode-btn:hover,
.attract-mode-btn:active {
  border-color: var(--accent-color);
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
}

.attract-mode-icon {
  font-size: 2.5rem;
  line-height: 1;
}

.attract-mode-label {
  font-size: var(--font-lg);
  font-weight: 700;
  color: var(--primary-text-color);
}

.attract-mode-desc {
  font-size: var(--font-sm, 13px);
  color: var(--text-muted);
  max-width: 140px;
  line-height: 1.4;
}

.attract-sub {
  margin: 2rem 0 0;
  font-size: var(--font-base);
  color: var(--text-muted);
  opacity: 0.5;
  letter-spacing: 0.05em;
  cursor: pointer;
}
.attract-sub:hover { opacity: 0.8; }

.attract-fade-enter-active { transition: opacity 0.3s; }
.attract-fade-leave-active { transition: opacity 0.5s; }
.attract-fade-enter-from,
.attract-fade-leave-to { opacity: 0; }

/* ===================== 引导条 ===================== */
.guide-toast {
  position: fixed;
  /* 避开 iPhone X+ 底部手势横条 */
  bottom: calc(24px + env(safe-area-inset-bottom));
  left: 50%;
  transform: translateX(-50%);
  z-index: 8000;
  padding: 14px 24px;
  background: var(--card-bg-color);
  border: 1.5px solid var(--border-color);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-lg, 0 8px 24px rgba(0,0,0,0.15));
  cursor: pointer;
  user-select: none;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.guide-mode-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--accent-color);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.guide-switch-hint {
  font-size: 11px;
  color: var(--text-muted);
  opacity: 0.7;
}

.guide-steps {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--font-sm, 13px);
  color: var(--text-color);
  flex-wrap: wrap;
  justify-content: center;
}

.guide-step {
  display: flex;
  align-items: center;
  gap: 6px;
}

.guide-num {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--accent-color);
  color: #fff;
  font-size: 11px;
  font-weight: 700;
  flex-shrink: 0;
}

.guide-arrow {
  color: var(--text-muted);
  font-size: 16px;
}

.guide-toast-enter-active { transition: transform 0.3s ease, opacity 0.3s ease; }
.guide-toast-leave-active { transition: transform 0.25s ease, opacity 0.25s ease; }
.guide-toast-enter-from { transform: translateX(-50%) translateY(24px); opacity: 0; }
.guide-toast-leave-to { transform: translateX(-50%) translateY(12px); opacity: 0; }

/* ===================== 断连横幅 ===================== */
.disconnect-bar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 8500;
  padding: 10px 16px;
  text-align: center;
  font-size: var(--font-base);
  font-weight: 600;
  color: white;
  background: var(--error-color, #d03050);
  box-shadow: var(--shadow-md);
  animation: disconnect-pulse 2s ease-in-out infinite;
}
@keyframes disconnect-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.8; }
}

/* ===================== Mobile ===================== */
@media (max-width: 768px) {
  .customer-view {
    flex-direction: column;
  }

  .product-panel,
  .vision-panel {
    flex: 1;
    min-height: 0;
  }

  .product-scroll {
    /* 给底部悬浮购物车留空间 */
    padding-bottom: calc(72px + env(safe-area-inset-bottom, 0px));
  }

  .vision-panel__body {
    padding-bottom: calc(72px + env(safe-area-inset-bottom, 0px));
  }

  .toolbar-left .slider-wrap { width: 60px; }

  .toolbar-right :deep(.n-button) {
    padding: 0 8px;
    font-size: 0.75rem;
  }
}
</style>
