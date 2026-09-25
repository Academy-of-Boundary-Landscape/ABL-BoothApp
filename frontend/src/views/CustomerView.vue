<template>
  <div class="customer-view" :class="{ 'customer-view--cartbar': !cartAsSidebar }">
    <!-- ================================================================
         点单模式：分类侧栏（平板/桌面）+ 商品网格
    ================================================================= -->
    <template v-if="!isVisionMode">
      <!-- 分类侧栏：手机不渲染（改用工具栏横滚分类） -->
      <aside v-if="showSidebar" class="category-sidebar">
        <div class="category-sidebar__title">商品分类</div>

        <n-scrollbar class="category-sidebar__scroll" content-class="category-sidebar__content">
          <button
            type="button"
            class="category-item"
            :class="{ 'is-active': selectedCategory === '' }"
            @click="selectedCategory = ''"
          >
            <span class="category-item__text">全部</span>
            <span v-if="selectedCategory === ''" class="category-item__dot" />
          </button>

          <button
            v-for="cat in categoryOptions"
            :key="cat"
            type="button"
            class="category-item"
            :class="{ 'is-active': selectedCategory === cat }"
            @click="selectedCategory = cat"
          >
            <span class="category-item__text">{{ cat }}</span>
            <span v-if="selectedCategory === cat" class="category-item__dot" />
          </button>
        </n-scrollbar>
      </aside>

      <!-- 中间：商品展示 -->
      <section class="product-panel">
        <!-- 工具栏：管理控件 + 模式切换 + 快捷入口 -->
        <div class="toolbar">
          <!-- 手机：分类横滚 -->
          <div v-if="isPhone" class="toolbar__categories">
            <button
              type="button"
              class="cat-chip"
              :class="{ 'is-active': selectedCategory === '' }"
              @click="selectedCategory = ''"
            >
              全部
            </button>
            <button
              v-for="cat in categoryOptions"
              :key="cat"
              type="button"
              class="cat-chip"
              :class="{ 'is-active': selectedCategory === cat }"
              @click="selectedCategory = cat"
            >
              {{ cat }}
            </button>
          </div>

          <div class="toolbar__row">
            <!-- 管理控件：默认折叠，点齿轮展开 -->
            <div v-if="showAdminControls" class="toolbar__left">
              <span class="toolbar__label">视图</span>
              <div class="toolbar__slider">
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

            <div class="toolbar__center">
              <div class="mode-toggle">
                <button
                  type="button"
                  class="mode-btn"
                  :class="{ 'is-active': !isVisionMode }"
                  @click="isVisionMode = false"
                >
                  商品列表
                </button>
                <button
                  type="button"
                  class="mode-btn"
                  :class="{ 'is-active': isVisionMode }"
                  @click="isVisionMode = true"
                >
                  拍照识别
                </button>
              </div>
            </div>

            <div class="toolbar__right">
              <router-link
                v-if="canReturnToVendor"
                class="nav-chip vendor-return"
                :to="{ name: 'vendor-orders', params: { id: props.id } }"
              >
                回摊主端
              </router-link>
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
                type="button"
                class="admin-toggle-btn"
                :class="{ 'is-active': showAdminControls }"
                title="展开/折叠管理控件"
                @click="toggleAdminControls"
              >
                ⚙
              </button>
            </div>
          </div>

          <!-- 管理控件展开时：导航快捷入口 -->
          <div v-if="showAdminControls" class="toolbar__nav">
            <router-link to="/admin" class="nav-chip">管理后台</router-link>
            <router-link to="/vendor" class="nav-chip">摊主页面</router-link>
            <router-link to="/" class="nav-chip">展会选择</router-link>
          </div>
        </div>

        <!-- 标签筛选 -->
        <div v-if="allTags.length > 0" class="tag-filter">
          <button
            v-for="tag in allTags"
            :key="tag"
            type="button"
            class="tag-chip"
            :class="{ 'is-active': selectedTag === tag }"
            @click="selectedTag = selectedTag === tag ? null : tag"
          >
            {{ tag }}
          </button>
        </div>

        <!-- 商品列表：加载 / 错误 / 空态统一走 AsyncState -->
        <div class="product-scroll">
          <AsyncState
            :loading="store.isLoading"
            :error="store.error"
            :empty="mutableProducts.length === 0"
            overlay
            loading-text="正在加载商品…"
            @retry="store.fetchProductsForEvent()"
          >
            <ProductGrid
              v-model:products="mutableProducts"
              :card-size="cardSize"
              :editable="isEditMode"
              @add-to-cart="store.addToCart"
              @order-changed="saveOrderToLocal"
            />
            <template #empty>
              <EmptyState icon="🌵" title="暂无商品" desc="摊主还没有上架商品，稍后再来看看吧。" />
            </template>
          </AsyncState>
        </div>
      </section>
    </template>

    <!-- ================================================================
         拍照识别模式：取景 + 识别结果
    ================================================================= -->
    <section v-else class="vision-panel">
      <div class="toolbar">
        <div class="toolbar__row">
          <div class="toolbar__left toolbar__left--empty" />
          <div class="toolbar__center">
            <div class="mode-toggle">
              <button
                type="button"
                class="mode-btn"
                :class="{ 'is-active': !isVisionMode }"
                @click="isVisionMode = false"
              >
                商品列表
              </button>
              <button
                type="button"
                class="mode-btn"
                :class="{ 'is-active': isVisionMode }"
                @click="isVisionMode = true"
              >
                拍照识别
              </button>
            </div>
          </div>
          <div class="toolbar__right">
            <router-link
              v-if="canReturnToVendor"
              class="nav-chip vendor-return"
              :to="{ name: 'vendor-orders', params: { id: props.id } }"
            >
              回摊主端
            </router-link>
          </div>
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
    </section>

    <!-- ======== 购物车：宽屏侧栏；平板竖屏 / 手机为底部可展开条 ======== -->
    <div v-if="cartAsSidebar" class="cart-sidebar">
      <ShoppingCart
        variant="sidebar"
        :cart="store.cart"
        :total="store.cartTotal"
        :payable="store.cartSummary.payable"
        :discounts="store.cartSummary.discounts"
        :quote-notice="store.quoteError"
        :quote-pending="store.quotePending"
        :is-checking-out="isCheckingOut"
        @add-to-cart="store.addToCart"
        @remove-from-cart="store.removeFromCart"
        @checkout="handleCheckout"
      />
    </div>

    <ShoppingCart
      v-else
      variant="bar"
      :cart="store.cart"
      :total="store.cartTotal"
      :payable="store.cartSummary.payable"
      :discounts="store.cartSummary.discounts"
      :quote-notice="store.quoteError"
      :quote-pending="store.quotePending"
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
    <Transition name="fade">
      <div v-if="showAttractScreen" class="attract-screen">
        <div class="attract-content">
          <p class="attract-welcome">欢迎光临</p>
          <h1 v-if="store.activeEvent?.name" class="attract-event">{{ store.activeEvent.name }}</h1>

          <div class="attract-modes">
            <button type="button" class="attract-mode-btn" @click="enterWithMode(false)">
              <span class="attract-mode-icon">&#9783;</span>
              <span class="attract-mode-label">浏览点单</span>
              <span class="attract-mode-desc">翻看商品列表，点击加入购物车</span>
            </button>
            <button type="button" class="attract-mode-btn" @click="enterWithMode(true)">
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
            <span class="guide-step"><span class="guide-num">2</span>查看已选</span>
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

<script setup lang="ts">
import { useFeedback } from '@/composables/useFeedback'
import { useViewport } from '@/composables/useViewport'
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useCustomerStore } from '@/stores/customerStore'
import { useAuthStore } from '@/stores/authStore'
import { useConnectionCheck } from '@/composables/useConnectionCheck'
import ProductGrid from '@/components/customer/ProductGrid.vue'
import ShoppingCart from '@/components/customer/ShoppingCart.vue'
import PaymentModal from '@/components/customer/PaymentModal.vue'
import VisionSearch from '@/components/shared/VisionSearch.vue'
import { AsyncState, EmptyState } from '@/components/ui'
import { cents, formatYuan, type Cents } from '@/utils/money'
import type { Schemas } from '@/api/client'
import { NScrollbar, NSlider, NButton } from 'naive-ui'

const props = defineProps<{ id: string }>()
const store = useCustomerStore()
const authStore = useAuthStore()
const { isConnected } = useConnectionCheck()

// 只有能进这个展会摊主端的会话才显示「回摊主端」（§3.7）；不登录自助点单的平板不显示。
const canReturnToVendor = computed(() => authStore.canAccessVendorPage(props.id))

// ===================== 布局断点 =====================
// 目标（spec §5.7 / §7）：平板横屏（≥1025）三栏；平板竖屏（641–1024）购物车改底部条；
// 手机（≤640）沿用竖屏形态（无侧栏 + 底部条）。
const { isPhone, isTablet } = useViewport()
const showSidebar = computed(() => !isPhone.value)
const cartAsSidebar = computed(() => !isTablet.value)

// ===================== 模式切换 =====================
const isVisionMode = ref(false)
const numericEventId = computed(() => parseInt(props.id, 10) || undefined)

// 切换模式时重新展示引导
watch(isVisionMode, () => {
  if (!showAttractScreen.value) triggerGuide()
})

function onVisionSelect(hit: Schemas['VisionSearchResult']) {
  const fb = useFeedback()
  const product = (store.products || []).find((p) => p.master_product_id === hit.master_product_id)
  if (!product) {
    fb.alert({
      title: '错误',
      content: `未找到商品「${hit.name}」，可能不在本场展会中`,
      type: 'error',
    })
    return
  }
  if (product.onsite_qty <= 0) {
    fb.alert({ title: '错误', content: `「${product.name}」已售罄`, type: 'error' })
    return
  }
  store.addToCart(product)
}

// ===================== 点单模式 =====================
const showPaymentModal = ref(false)
const orderTotal = ref<Cents>(cents(0))
const isCheckingOut = ref(false)
const selectedCategory = ref('')
const isEditMode = ref(false)
const showAdminControls = ref(localStorage.getItem('customer_admin_controls') === 'true')
function toggleAdminControls() {
  showAdminControls.value = !showAdminControls.value
  localStorage.setItem('customer_admin_controls', String(showAdminControls.value))
  if (!showAdminControls.value && isEditMode.value) {
    isEditMode.value = false
    saveOrderToLocal()
  }
}
const cardSizeIndex = ref(1)
const userTouchedCardSize = ref(false)
const CARD_SIZES = ['small', 'medium', 'large'] as const
const cardSize = computed(() => CARD_SIZES[cardSizeIndex.value] || 'medium')
function onCardSizeUserChange() {
  userTouchedCardSize.value = true
}

function syncLayout() {
  if (!userTouchedCardSize.value) cardSizeIndex.value = isPhone.value ? 0 : 1
}
watch(isPhone, syncLayout)

onMounted(() => {
  store.setupStoreForEvent(props.id)
  syncLayout()
  ACTIVITY_EVENTS.forEach((e) => window.addEventListener(e, onUserActivity, { passive: true }))
})
onUnmounted(() => {
  ACTIVITY_EVENTS.forEach((e) => window.removeEventListener(e, onUserActivity))
  clearTimeout(idleTimer ?? undefined)
  clearTimeout(guideTimer ?? undefined)
})

const categoryOptions = computed(() => {
  const cats = (store.products || [])
    .map((p) => p.category)
    .filter((c): c is string => !!c && !!c.trim())
  return [...new Set(cats)]
})

const allTags = computed(() => {
  const counts = new Map<string, number>()
  ;(store.products || []).forEach((p) => {
    ;(p.tags || '')
      .split(',')
      .filter((t) => t.trim())
      .forEach((tag) => {
        const t = tag.trim()
        counts.set(t, (counts.get(t) || 0) + 1)
      })
  })
  return [...counts.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([tag]) => tag)
})

const selectedTag = ref<string | null>(null)

// ===== 排序 =====
const STORAGE_KEY = computed(() => `my_shop_custom_order::event::${props.id}`)

function readSavedIds(): number[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY.value)
    const ids: unknown = JSON.parse(raw || '[]')
    if (!Array.isArray(ids)) return []
    return ids.filter((id): id is number => typeof id === 'number')
  } catch {
    return []
  }
}

function applySavedOrder(list: Schemas['ProductEventProduct'][], savedIds: number[]) {
  if (!savedIds?.length) return [...list]
  const pos = new Map(savedIds.map((id, i) => [id, i]))
  return [...list].sort((a, b) => {
    const ia = pos.get(a.id) ?? Number.POSITIVE_INFINITY
    const ib = pos.get(b.id) ?? Number.POSITIVE_INFINITY
    return ia - ib
  })
}

const baseOrderedProducts = computed(() => {
  const all = store.products || []
  return applySavedOrder(all, readSavedIds())
})

const mutableProducts = ref<Schemas['ProductEventProduct'][]>([])

watch(
  [baseOrderedProducts, selectedCategory, selectedTag],
  ([base, cat, tag]) => {
    if (isEditMode.value) return
    let subset = cat ? base.filter((p) => p.category === cat) : base
    if (tag) {
      subset = subset.filter((p) => (p.tags || '').split(',').some((t) => t.trim() === tag))
    }
    // 售罄商品自动置底，有货的保持原有排序
    const inStock = subset.filter((p) => p.onsite_qty > 0)
    const soldOut = subset.filter((p) => p.onsite_qty <= 0)
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
  const draggedSubsetIds = mutableProducts.value.map((p) => p.id)

  let mergedIds: number[]
  if (!cat) {
    mergedIds = draggedSubsetIds
  } else {
    const subsetIdSet = new Set(draggedSubsetIds)
    const baseIds = fullBase.map((p) => p.id)
    const queue = [...draggedSubsetIds]
    mergedIds = baseIds.map((id) => (subsetIdSet.has(id) ? queue.shift()! : id))
  }

  try {
    localStorage.setItem(STORAGE_KEY.value, JSON.stringify(mergedIds))
  } catch (e) {
    console.error('保存顺序失败', e)
  }
}

// ===================== 闲置吸引屏 =====================
const IDLE_TIMEOUT_MS = 60_000 // 60 秒无操作显示吸引屏
const showAttractScreen = ref(true) // 初始就显示吸引屏
const showGuideBar = ref(false)
let idleTimer: ReturnType<typeof setTimeout> | null = null

function resetIdleTimer() {
  clearTimeout(idleTimer ?? undefined)
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

function enterWithMode(vision: boolean) {
  isVisionMode.value = vision
  dismissAttractScreen()
}

let guideTimer: ReturnType<typeof setTimeout> | null = null
function triggerGuide() {
  showGuideBar.value = true
  clearTimeout(guideTimer ?? undefined)
  guideTimer = setTimeout(() => {
    showGuideBar.value = false
  }, 8000)
}

// 监听任何交互事件来重置闲置计时器
const ACTIVITY_EVENTS = ['pointerdown', 'pointermove', 'keydown', 'scroll']
function onUserActivity() {
  if (!showAttractScreen.value) resetIdleTimer()
  // 任何交互都关闭引导条
  if (showGuideBar.value) showGuideBar.value = false
}

// 首次加购时关闭引导条
watch(
  () => store.cart.length,
  (newLen, oldLen) => {
    if (newLen > oldLen && showGuideBar.value) showGuideBar.value = false
  }
)

// ===== 下单 =====
async function handleCheckout() {
  const fb = useFeedback()
  if (isCheckingOut.value) return

  const itemCount = store.cartItemCount
  const totalAmount = store.cartSummary.payable // 确认框里也该是折后价

  await fb.confirm({
    title: '确认下单',
    content: `共 ${itemCount} 件商品，合计 ${formatYuan(totalAmount)}`,
    positiveText: '确认下单',
    negativeText: '再看看',
    type: 'info',
    onConfirm: async () => {
      isCheckingOut.value = true
      try {
        const newOrder = await store.submitOrder()
        if (newOrder) {
          // **金额取自下单响应，不是购物车的报价。** 报价只是预览，两次之间
          // 摊主完全可能刚改过 Lot 配置——顾客扫码付的数必须是服务端落账的那个数。
          orderTotal.value = newOrder.final_amount
          showPaymentModal.value = true
          store.clearCart()
          store.fetchProductsForEvent()
        }
      } catch (error) {
        fb.alert({
          title: '错误',
          content: (error instanceof Error && error.message) || '下单失败',
          type: 'error',
        })
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
  --sidebar-w: 196px;
  --cart-w: 300px;
  --cart-bar-h: 60px;

  display: flex;
  height: 100%;
  overflow: hidden;
  background-color: var(--bg-color);
  color: var(--primary-text-color);
}

/* ===================== 分类侧栏（平板 / 桌面） ===================== */
.category-sidebar {
  flex: 0 0 var(--sidebar-w);
  width: var(--sidebar-w);
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--card-bg-color);
  border-right: 1px solid var(--border-color);
}
.category-sidebar__title {
  flex-shrink: 0;
  height: 56px;
  display: flex;
  align-items: center;
  padding: 0 var(--space-lg);
  border-bottom: 1px solid var(--border-color);
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
}
.category-sidebar__scroll {
  flex: 1;
  min-height: 0;
}
:deep(.category-sidebar__content) {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  padding: var(--space-sm);
}
.category-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-sm);
  width: 100%;
  min-height: 48px;
  padding: var(--space-sm) var(--space-md);
  border: none;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--secondary-text-color);
  font-size: var(--font-base);
  text-align: left;
  cursor: pointer;
  transition:
    background-color 0.15s,
    color 0.15s;
}
.category-item:hover {
  background-color: var(--bg-secondary);
  color: var(--primary-text-color);
}
.category-item.is-active {
  background-color: color-mix(in srgb, var(--accent-color) 18%, transparent);
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}
.category-item__text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.category-item__dot {
  flex-shrink: 0;
  width: 4px;
  height: 14px;
  border-radius: var(--radius-sm);
  background-color: var(--accent-color);
}

/* ===================== 中间面板（点单 & 识别共用结构） ===================== */
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
.toolbar__categories {
  display: flex;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-md) 0;
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
}
.cat-chip {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  min-height: 44px;
  padding: var(--space-xs) var(--space-md);
  border: 1px solid transparent;
  border-radius: var(--radius-pill);
  background: var(--bg-secondary);
  color: var(--text-muted);
  font-size: var(--font-sm);
  cursor: pointer;
  transition: all 0.15s;
}
.cat-chip.is-active {
  background: var(--accent-color);
  color: var(--text-white);
  font-weight: var(--weight-bold);
}

.toolbar__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-md);
}
.toolbar__left {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-xs) var(--space-md);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-pill);
  background: var(--card-bg-color);
}
.toolbar__left--empty {
  padding: 0;
  border: none;
  background: transparent;
}
.toolbar__label {
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.toolbar__slider {
  width: 88px;
}
.toolbar__center {
  flex-shrink: 0;
}
.toolbar__right {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.admin-toggle-btn {
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border: 1px solid var(--border-color);
  border-radius: 50%;
  background: var(--bg-secondary);
  color: var(--text-muted);
  font-size: var(--font-md);
  cursor: pointer;
  transition: all 0.2s;
}
.admin-toggle-btn.is-active {
  background: var(--accent-color);
  border-color: var(--accent-color);
  color: var(--text-white);
}

.toolbar__nav {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-sm);
  padding: 0 var(--space-md) var(--space-sm);
}
.nav-chip {
  display: inline-flex;
  align-items: center;
  min-height: 44px;
  padding: var(--space-xs) var(--space-md);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-pill);
  background: var(--bg-secondary);
  color: var(--text-muted);
  font-size: var(--font-sm);
  text-decoration: none;
  transition: all 0.15s;
}
.nav-chip:hover {
  background: var(--accent-color);
  border-color: var(--accent-color);
  color: var(--text-white);
}

/* 模式切换 */
.mode-toggle {
  display: flex;
  gap: var(--space-xs);
  padding: var(--space-xs);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-pill);
  background: var(--bg-secondary);
}
.mode-btn {
  min-height: 44px;
  padding: var(--space-xs) var(--space-lg);
  border: none;
  border-radius: var(--radius-pill);
  background: transparent;
  color: var(--text-muted);
  font-size: var(--font-sm);
  white-space: nowrap;
  cursor: pointer;
  transition: all 0.15s;
}
.mode-btn.is-active {
  background: var(--accent-color);
  color: var(--text-white);
  font-weight: var(--weight-bold);
}

/* ===== 标签筛选栏 ===== */
.tag-filter {
  display: flex;
  gap: var(--space-sm);
  flex-shrink: 0;
  padding: var(--space-sm) var(--space-md);
  overflow-x: auto;
  scrollbar-width: none;
}
.tag-filter::-webkit-scrollbar {
  display: none;
}
.tag-chip {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  min-height: 44px;
  padding: var(--space-xs) var(--space-md);
  border: 1.5px solid var(--border-color);
  border-radius: var(--radius-pill);
  background: var(--card-bg-color);
  color: var(--secondary-text-color);
  font-size: var(--font-sm);
  white-space: nowrap;
  cursor: pointer;
  user-select: none;
  transition: all 0.15s;
}
.tag-chip:hover {
  border-color: var(--accent-color);
}
.tag-chip.is-active {
  background: var(--accent-color);
  border-color: var(--accent-color);
  color: var(--text-white);
  font-weight: var(--weight-bold);
}

/* ===== 商品滚动区 ===== */
.product-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: var(--space-sm) var(--space-md);
}

/* ===== 识别面板 body ===== */
.vision-panel__body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  padding: var(--space-md);
  display: flex;
  flex-direction: column;
}

/* ===================== 购物车（宽屏侧栏） ===================== */
.cart-sidebar {
  flex: 0 0 var(--cart-w);
  width: var(--cart-w);
  min-height: 0;
  border-left: 1px solid var(--border-color);
  background: var(--card-bg-color);
}

/* 底部购物车条：给内容区留出被条盖住的空间（含 iPhone 安全区） */
.customer-view--cartbar .product-scroll,
.customer-view--cartbar .vision-panel__body {
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- 为底部购物车条（组件私有布局变量）+ iPhone 安全区留白，非间距刻度 */
  padding-bottom: calc(var(--cart-bar-h) + var(--space-xl) + env(safe-area-inset-bottom, 0px));
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
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- iPhone 安全区适配，env() 无法用 space token 表达 */
  padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom)
    env(safe-area-inset-left);
  box-sizing: border-box;
}

.attract-content {
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-sm);
}

.attract-welcome {
  font-size: var(--font-xl);
  font-weight: var(--weight-bold);
  color: var(--text-muted);
  margin: 0;
  letter-spacing: 0.2em;
}

.attract-event {
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- 引导屏 Hero 展会名，clamp 响应式字号，缩小影响观感 */
  font-size: clamp(2rem, 5vw, 3.5rem);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
  margin: 0 0 var(--space-2xl);
  line-height: 1.2;
}

.attract-modes {
  display: flex;
  gap: var(--space-lg);
  margin: var(--space-2xl) 0 0;
  flex-wrap: wrap;
  justify-content: center;
}

.attract-mode-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-sm);
  min-width: 200px;
  padding: var(--space-xl) var(--space-2xl);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--card-bg-color);
  cursor: pointer;
  transition: all 0.2s;
}
.attract-mode-btn:hover,
.attract-mode-btn:active {
  border-color: var(--accent-color);
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
}

.attract-mode-icon {
  font-size: var(--font-2xl);
  line-height: 1;
}

.attract-mode-label {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
}

.attract-mode-desc {
  max-width: 160px;
  font-size: var(--font-sm);
  color: var(--text-muted);
  line-height: 1.4;
}

.attract-sub {
  margin: var(--space-2xl) 0 0;
  font-size: var(--font-base);
  color: var(--text-muted);
  opacity: 0.5;
  letter-spacing: 0.05em;
  cursor: pointer;
}
.attract-sub:hover {
  opacity: 0.8;
}

/* ===================== 引导条 ===================== */
.guide-toast {
  position: fixed;
  /* 避开 iPhone X+ 底部手势横条 */
  bottom: calc(var(--space-xl) + env(safe-area-inset-bottom));
  left: 50%;
  transform: translateX(-50%);
  z-index: 8000;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-md) var(--space-xl);
  border: 1.5px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--card-bg-color);
  box-shadow: var(--shadow-lg);
  cursor: pointer;
  user-select: none;
}

/* 底部有购物车条时，引导条上移让位 */
.customer-view--cartbar .guide-toast {
  bottom: calc(var(--cart-bar-h) + var(--space-xl) + env(safe-area-inset-bottom));
}

.guide-mode-label {
  font-size: var(--font-xs);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.guide-switch-hint {
  font-size: var(--font-xs);
  color: var(--text-muted);
  opacity: 0.7;
}

.guide-steps {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: var(--space-sm);
  font-size: var(--font-sm);
  color: var(--secondary-text-color);
}

.guide-step {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.guide-num {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--accent-color);
  color: var(--text-white);
  font-size: var(--font-xs);
  font-weight: var(--weight-bold);
}

.guide-arrow {
  color: var(--text-muted);
  font-size: var(--font-base);
}

.guide-toast-enter-active {
  transition:
    transform 0.3s ease,
    opacity 0.3s ease;
}
.guide-toast-leave-active {
  transition:
    transform 0.25s ease,
    opacity 0.25s ease;
}
.guide-toast-enter-from {
  transform: translateX(-50%) translateY(24px);
  opacity: 0;
}
.guide-toast-leave-to {
  transform: translateX(-50%) translateY(12px);
  opacity: 0;
}

/* ===================== 断连横幅 ===================== */
.disconnect-bar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 8500;
  padding: var(--space-sm) var(--space-lg);
  text-align: center;
  font-size: var(--font-base);
  font-weight: var(--weight-bold);
  color: var(--text-white);
  background: var(--error-color);
  box-shadow: var(--shadow-md);
  animation: disconnect-pulse 2s ease-in-out infinite;
}
@keyframes disconnect-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.8;
  }
}

/* ===================== 平板 ===================== */
@media (--tablet) {
  .customer-view {
    --sidebar-w: 164px;
  }
}

/* ===================== 手机 ===================== */
@media (--phone) {
  .customer-view {
    flex-direction: column;
  }

  .product-panel,
  .vision-panel {
    flex: 1;
    min-height: 0;
  }

  .toolbar__row {
    gap: var(--space-xs);
  }

  .toolbar__center {
    order: 3;
    width: 100%;
  }

  .mode-toggle {
    width: 100%;
  }

  .mode-btn {
    flex: 1;
  }

  .toolbar__right {
    margin-left: auto;
  }
}
</style>
