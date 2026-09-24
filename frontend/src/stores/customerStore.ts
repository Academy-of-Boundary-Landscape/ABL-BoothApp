import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api, errorMessage, unwrap, type Schemas } from '@/api/client'
import { useFeedback } from '@/composables/useFeedback'
import { getImageUrl } from '@/services/url'
import { summarizeQuote, type QuoteSummary } from '@/utils/quote'
import { cents, type Cents } from '@/utils/money'

// 购物车条目 = 场次商品 + 数量
type CartItem = Schemas['ProductEventProduct'] & { quantity: number }

// 获取弹窗函数

export const useCustomerStore = defineStore('customer', () => {
  // --- State ---
  const products = ref<Schemas['ProductEventProduct'][]>([])
  const cart = ref<CartItem[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const activeEventId = ref<number | null>(null)
  const activeEvent = ref<Schemas['EventResponse'] | null>(null) // 新增：当前展会信息

  // --- Actions ---
  async function setupStoreForEvent(eventId: string | number) {
    if (!eventId) {
      error.value = '未提供展会ID。'
      return
    }
    activeEventId.value = parseInt(String(eventId), 10)
    // 并行加载商品和展会信息，都完成后再允许下单
    await Promise.all([fetchProductsForEvent(), fetchEventInfo()])
  }

  async function initializeEventFromUrl() {
    const urlParams = new URLSearchParams(window.location.search)
    const eventIdFromUrl = urlParams.get('event')
    if (eventIdFromUrl) {
      activeEventId.value = parseInt(eventIdFromUrl, 10)
      await Promise.all([fetchProductsForEvent(), fetchEventInfo()])
    } else {
      error.value = '未指定展会ID，无法加载商品。请使用包含 ?event=ID 的链接访问。'
    }
  }

  // 获取商品列表 (HTTP)
  async function fetchProductsForEvent() {
    const eventId = activeEventId.value
    if (!eventId) return
    isLoading.value = true
    error.value = null
    try {
      const list = await unwrap(
        api.GET('/events/{event_id}/products', { params: { path: { event_id: eventId } } })
      )
      products.value = list.map((product) => ({
        ...product,
        image_url: getImageUrl(product.image_url ?? ''),
      }))
    } catch (e) {
      error.value = '加载商品失败，请联系摊主。'
      console.error(e)
    } finally {
      isLoading.value = false
    }
  }

  // 新增：获取展会信息
  async function fetchEventInfo() {
    const eventId = activeEventId.value
    if (!eventId) return
    try {
      const data = await unwrap(api.GET('/events/{id}', { params: { path: { id: eventId } } }))
      activeEvent.value = {
        ...data,
        qrcode_url: getImageUrl(data.qrcode_url ?? ''),
        qrcode_urls: (data.qrcode_urls || []).map((u) => getImageUrl(u)).filter(Boolean),
      }
    } catch (e) {
      console.error('加载展会信息失败:', e)
      // 不设置error，因为这不是关键功能
    }
  }

  // --- 购物车操作 ---

  function addToCart(product: Schemas['ProductEventProduct']) {
    const existingItem = cart.value.find((item) => item.id === product.id)
    if (existingItem) {
      if (existingItem.quantity < product.onsite_qty) {
        existingItem.quantity++
      } else {
        useFeedback().alert({
          title: '错误',
          content: `抱歉，"${product.name}" 库存不足！`,
          type: 'error',
        })
      }
    } else {
      if (product.onsite_qty > 0) {
        cart.value.push({ ...product, quantity: 1 })
      }
    }
    scheduleQuote()
  }

  function removeFromCart(productId: number) {
    const itemIndex = cart.value.findIndex((item) => item.id === productId)
    if (itemIndex !== -1) {
      if (cart.value[itemIndex].quantity > 1) {
        cart.value[itemIndex].quantity--
      } else {
        cart.value.splice(itemIndex, 1)
      }
    }
    scheduleQuote()
  }

  function clearCart() {
    cart.value = []
    scheduleQuote()
  }

  async function submitOrder() {
    const eventId = activeEventId.value
    if (!eventId || cart.value.length === 0) return

    const orderData: Schemas['CreateOrderRequest'] = {
      items: cart.value.map((item) => ({
        product_id: item.id,
        quantity: item.quantity,
      })),
    }

    try {
      // 成功后返回订单数据，让视图可以触发后续操作（如弹窗）
      return await unwrap(
        api.POST('/events/{event_id}/orders', {
          params: { path: { event_id: eventId } },
          body: orderData,
        })
      )
    } catch (e) {
      console.error('Order submission failed:', e)

      throw new Error(errorMessage(e, '下单失败，请重试。'))
    }
  }

  // --- Getters ---
  // 单位：分（整数运算，展示端由 formatYuan 除以 100）。
  const cartTotal = computed<Cents>(() => {
    return cents(cart.value.reduce((total, item) => total + item.unit_price * item.quantity, 0))
  })

  // --- 报价 ---
  // 折扣由服务端算（`domain/solver.rs`）。前端不重写一份求解器：同一套集合覆盖
  // 两份实现，取整规则一漂移就是「显示 145 实收 150」。
  const quote = ref<Schemas['LotQuoteResponse'] | null>(null)
  const quoteError = ref<string | null>(null)
  // 报价在途（debounce 窗口 + 请求往返）。为真时 cartSummary 必须按原价显示：
  // 否则上一车的 quote 配上这一车的 cartTotal，会渲染出「原价 ~~¥50~~ / 应付 ¥30」
  // 却一条优惠说明都没有，正好打中 D3 要保护的那个决策点。
  const quotePending = ref(false)
  let quoteSeq = 0
  let quoteTimer: ReturnType<typeof setTimeout> | null = null

  /**
   * 购物车一变就重新报价。300ms debounce + 请求序号两道都需要：
   * 顾客连点加号会发好几次，而乱序返回会让价格来回跳。
   */
  function scheduleQuote() {
    if (quoteTimer) clearTimeout(quoteTimer)
    if (!activeEventId.value || cart.value.length === 0) {
      quoteSeq += 1 // 作废在途的请求，免得它回来给空车填上价
      quote.value = null
      quoteError.value = null
      quotePending.value = false
      return
    }
    // 同样要作废在途的请求。**少了这一句 quotePending 就形同虚设**：
    // 上一车的请求落回来时 seq 仍等于 quoteSeq，于是它把过期报价写进 quote、
    // 顺手把 quotePending 清掉，而新的 debounce 还没到点——购物车就会显示
    // 「原价 ¥50 / 应付 ¥30」却一条优惠说明都没有，结算按钮还是亮的。
    // 反复点加号就能撞上，窗口约 350ms。
    quoteSeq += 1
    quotePending.value = true
    quoteTimer = setTimeout(fetchQuote, 300)
  }

  async function fetchQuote() {
    const seq = (quoteSeq += 1)
    const eventId = activeEventId.value
    if (!eventId) return
    const items = cart.value.map((item) => ({ product_id: item.id, quantity: item.quantity }))
    try {
      const data = await unwrap(
        api.POST('/events/{event_id}/quote', {
          params: { path: { event_id: eventId } },
          body: { items },
        })
      )
      if (seq !== quoteSeq) return // 旧请求，结果丢掉
      quote.value = data
      quoteError.value = null
    } catch (e) {
      if (seq !== quoteSeq) return
      // **不静默**：退回原价，同时明说没套用优惠。后端在超限时给的就是人话。
      quote.value = null
      quoteError.value = errorMessage(e, '优惠暂时算不出来，按原价显示')
    } finally {
      if (seq === quoteSeq) quotePending.value = false
    }
  }

  const cartSummary = computed<QuoteSummary>(() => {
    // 在途期间先按原价显示，宁可少报优惠也不显示无法解释的价格。
    if (quotePending.value) {
      return { gross: cartTotal.value, payable: cartTotal.value, discounts: [] }
    }
    return summarizeQuote(quote.value, cartTotal.value)
  })

  const cartItemCount = computed(() => {
    return cart.value.reduce((total, item) => total + item.quantity, 0)
  })

  // 收款码 URL（向后兼容单码）
  const qrCodeUrl = computed(() => {
    return activeEvent.value?.qrcode_url || null
  })

  // 收款码 URL 数组（多码）
  const qrCodeUrls = computed(() => {
    return activeEvent.value?.qrcode_urls || []
  })

  return {
    products,
    cart,
    isLoading,
    error,
    activeEventId,
    activeEvent, // 新增：导出展会信息
    qrCodeUrl,
    qrCodeUrls,
    fetchProductsForEvent,
    fetchEventInfo, // 新增：导出获取展会信息方法
    addToCart,
    removeFromCart,
    clearCart,
    submitOrder,
    cartTotal,
    cartItemCount,
    quote,
    quoteError,
    quotePending,
    cartSummary,
    initializeEventFromUrl,
    setupStoreForEvent,
  }
})
