<template>
  <div class="customer-view">
    <!-- Sidebar：分类 -->
    <div class="sidebar">
      <div class="sidebar-header" v-if="!isMobile">
        <span class="header-title">商品分类</span>
      </div>

      <n-scrollbar class="sidebar-scroll" content-class="sidebar-content">
        <div
          class="menu-item"
          :class="{ active: selectedCategory === '' }"
          @click="selectedCategory = ''"
        >
          <span class="menu-text">全部</span>
          <div class="active-indicator" v-if="selectedCategory === '' && !isMobile"></div>
        </div>

        <div
          v-for="cat in categoryOptions"
          :key="cat"
          class="menu-item"
          :class="{ active: selectedCategory === cat }"
          @click="selectedCategory = cat"
        >
          <span class="menu-text">{{ cat }}</span>
          <div class="active-indicator" v-if="selectedCategory === cat && !isMobile"></div>
        </div>
      </n-scrollbar>
    </div>

    <!-- 中间：商品展示 -->
    <div class="product-panel">
      <!-- 顶部工具栏 -->
      <div class="card-size-toolbar">
        <div class="toolbar-content">
          <div class="toolbar-left">
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

          <div class="toolbar-right">
            <n-button
              size="small"
              round
              :type="isEditMode ? 'primary' : 'default'"
              :secondary="!isEditMode"
              @click="toggleEditMode"
            >
              <template #icon>
                <span v-if="isEditMode">💾</span>
                <span v-else>🖐</span>
              </template>
              {{ isEditMode ? '保存顺序' : '调整顺序' }}
            </n-button>
          </div>
        </div>
      </div>

      <!-- 商品列表 -->
      <n-spin :show="store.isLoading" content-class="spin-content">
        <div class="product-scroll">
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
        </div>
      </n-spin>
    </div>

    <!-- Desktop: 右侧购物车 -->
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

    <!-- Mobile: 悬浮购物车 -->
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
      :qr-code-url="store.qrCodeUrl"
      @close="closePaymentModal"
    />
  </div>
</template>

<script setup>
import { useAlert } from '@/services/useAlert'
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useCustomerStore } from '@/stores/customerStore'
import ProductGrid from '@/components/customer/ProductGrid.vue'
import ShoppingCart from '@/components/customer/ShoppingCart.vue'
import PaymentModal from '@/components/customer/PaymentModal.vue'
import { NScrollbar, NSpin, NSlider, NButton } from 'naive-ui'

const props = defineProps({ id: { type: String, required: true } })
const store = useCustomerStore()

const showPaymentModal = ref(false)
const orderTotal = ref(0)
const isCheckingOut = ref(false)
const selectedCategory = ref('')
const isEditMode = ref(false)
/** 卡片大小 */
const cardSizeIndex = ref(1)
const userTouchedCardSize = ref(false)
const cardSize = computed(() => ['small', 'medium', 'large'][cardSizeIndex.value] || 'medium')
function onCardSizeUserChange() {
  userTouchedCardSize.value = true
}

/** 响应式断点 */
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
})
onUnmounted(() => {
  window.removeEventListener('resize', syncLayout)
})

/** 分类 */
const categoryOptions = computed(() => {
  const cats = (store.products || []).map(p => p.category).filter(c => c && c.trim())
  return [...new Set(cats)]
})

/** ===========================
 *  排序（统一在父组件处理）
 *  - 子组件只 emit 更新后的 products
 *  - 父组件负责：应用本地顺序、合并过滤子集排序、存 localStorage
 =========================== */
const STORAGE_KEY = computed(() => `my_shop_custom_order::event::${props.id}`)

function readSavedIds() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY.value)
    const ids = JSON.parse(raw || '[]')
    return Array.isArray(ids) ? ids : []
  } catch {
    return []
  }
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

/** 当前“全量商品”的基准顺序（用于过滤视图合并保存） */
const baseOrderedProducts = computed(() => {
  const all = store.products || []
  return applySavedOrder(all, readSavedIds())
})

/** 给 ProductGrid 绑定的可变数组：= baseOrderedProducts 的过滤子集 */
const mutableProducts = ref([])

watch(
  [baseOrderedProducts, selectedCategory],
  ([base, cat]) => {
    // ✅ 编辑模式下不要强行覆盖用户正在拖的顺序
    if (isEditMode.value) return

    const subset = cat ? base.filter(p => p.category === cat) : base
    mutableProducts.value = [...subset]
  },
  { immediate: true }
)


function toggleEditMode() {
  isEditMode.value = !isEditMode.value
  if (!isEditMode.value) {
    // 退出编辑模式时，做一次保存（双保险）
    saveOrderToLocal()
  }
}

/**
 * ✅ 保存逻辑（关键修复点）
 * - 你拖拽的是“过滤后的子集”
 * - 保存时需要把它合并回“全量顺序”，否则会丢掉其他分类商品的相对顺序
 */
function saveOrderToLocal() {
  const cat = selectedCategory.value
  const fullBase = baseOrderedProducts.value // 全量商品（已应用旧顺序）
  const draggedSubsetIds = mutableProducts.value.map(p => p.id)

  let mergedIds
  if (!cat) {
    // “全部”视图：直接保存当前顺序即可
    mergedIds = draggedSubsetIds
  } else {
    // 分类视图：只替换该分类子集的相对顺序，其他商品保持原有相对位置
    const subsetIdSet = new Set(draggedSubsetIds)

    // 先按 fullBase 的顺序拿到全量 id
    const baseIds = fullBase.map(p => p.id)

    // 把 baseIds 里属于该分类的元素按 draggedSubsetIds 的顺序替换
    const queue = [...draggedSubsetIds]
    mergedIds = baseIds.map(id => (subsetIdSet.has(id) ? queue.shift() : id))
  }

  try {
    localStorage.setItem(STORAGE_KEY.value, JSON.stringify(mergedIds))
    // 你如果想提示：这里可以 n-message.success("顺序已保存")
  } catch (e) {
    console.error('保存顺序失败', e)
  }
}

/** 下单 */
async function handleCheckout() {
  const { showSuccess, showError } = useAlert()
  if (isCheckingOut.value) return
  isCheckingOut.value = true
  try {
    const newOrder = await store.submitOrder()
    if (newOrder) {
      orderTotal.value = store.cartTotal
      showPaymentModal.value = true
      //showSuccess('下单成功')
      store.clearCart()
    }
  } catch (error) {
    showError(error?.message || '下单失败')
  } finally {
    isCheckingOut.value = false
  }
}
function closePaymentModal() {
  showPaymentModal.value = false
}
</script>

<style scoped>
.customer-view {
  --sidebar-w: 200px;
  --cart-w: 300px;
  --header-h: 60px;

  display: flex;
  height: 100vh;
  height: 100dvh;
  overflow: hidden;
  background-color: var(--bg-color);
  color: var(--primary-text-color);
}

/* Sidebar */
.sidebar {
  flex: 0 0 var(--sidebar-w);
  width: var(--sidebar-w);
  background: var(--card-bg-color);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  z-index: 10;
}
.sidebar-header {
  height: var(--header-h);
  display: flex;
  align-items: center;
  padding: 0 16px;
  font-weight: 800;
  font-size: 1.05rem;
  border-bottom: 1px solid var(--border-color);
}
.sidebar-scroll { flex: 1; }

:deep(.sidebar-content) {
  display: flex;
  flex-direction: column;
  padding: 10px;
  gap: 4px;
}
.menu-item {
  padding: 10px 14px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.95rem;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: space-between;
  transition: all 0.2s;
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

/* Product Panel */
.product-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  position: relative;
}

.card-size-toolbar {
  flex: 0 0 auto;
  padding: 10px 20px;
  background: var(--bg-color);
  z-index: 5;
}
.toolbar-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}
.toolbar-left {
  display: flex;
  align-items: center;
  gap: 10px;
  background: var(--card-bg-color);
  padding: 4px 12px;
  border-radius: 999px;
  border: 1px solid var(--border-color);
}
.toolbar-label {
  font-size: 0.8rem;
  color: var(--text-muted);
}
.slider-wrap { width: 80px; }
.toolbar-right { display: flex; align-items: center; }

.spin-content {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.product-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 0 20px 40px;
}
.empty-state {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
}
.empty-emoji { font-size: 3rem; margin-bottom: 10px; }

/* Cart */
.cart-panel-desktop {
  flex: 0 0 var(--cart-w);
  width: var(--cart-w);
  border-left: 1px solid var(--border-color);
  background: var(--card-bg-color);
  z-index: 10;
}

/* Mobile */
@media (max-width: 768px) {
  .customer-view { flex-direction: column; }

  .sidebar {
    flex: 0 0 auto;
    width: 100%;
    height: auto;
    border-right: none;
    border-bottom: 1px solid var(--border-color);
  }

  :deep(.sidebar-content) {
    flex-direction: row;
    padding: 8px 12px;
    gap: 8px;
    overflow-x: auto;
  }

  .menu-item {
    flex: 0 0 auto;
    padding: 6px 14px;
    border-radius: 999px;
    background: var(--bg-secondary);
    border: 1px solid transparent;
  }
  .menu-item.active {
    background: var(--accent-color);
    color: white;
  }
  .active-indicator { display: none; }

  .card-size-toolbar { padding: 8px 12px; }
  .product-scroll {
    padding-bottom: calc(80px + env(safe-area-inset-bottom));
  }

  .toolbar-right :deep(.n-button) {
    padding: 0 10px;
    font-size: 12px;
  }
}
</style>
