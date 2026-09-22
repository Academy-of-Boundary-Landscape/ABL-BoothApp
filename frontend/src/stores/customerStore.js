import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '@/services/api'
import { useAlert } from '@/services/useAlert'
import { getImageUrl } from '@/services/url'
import { summarizeQuote } from '@/utils/quote'

// 获取弹窗函数

export const useCustomerStore = defineStore('customer', () => {
  // --- State ---
  const products = ref([])
  const cart = ref([])
  const isLoading = ref(false)
  const error = ref(null)
  const activeEventId = ref(null)
  const activeEvent = ref(null) // 新增：当前展会信息

  // --- Actions ---
  async function setupStoreForEvent(eventId) {
    if (!eventId) {
      error.value = '未提供展会ID。'
      return
    }
    activeEventId.value = parseInt(eventId, 10)
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
    if (!activeEventId.value) return
    isLoading.value = true
    error.value = null
    try {
      const response = await api.get(`/events/${activeEventId.value}/products`)
      products.value = response.data.map((product) => ({
        ...product,
        image_url: getImageUrl(product.image_url),
      }))
    } catch (err) {
      error.value = '加载商品失败，请联系摊主。'
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }

  // 新增：获取展会信息
  async function fetchEventInfo() {
    if (!activeEventId.value) return
    try {
      const response = await api.get(`/events/${activeEventId.value}`)
      activeEvent.value = {
        ...response.data,
        qrcode_url: getImageUrl(response.data.qrcode_url),
        qrcode_urls: (response.data.qrcode_urls || []).map((u) => getImageUrl(u)).filter(Boolean),
      }
    } catch (err) {
      console.error('加载展会信息失败:', err)
      // 不设置error，因为这不是关键功能
    }
  }

  // --- 购物车操作 ---

  function addToCart(product) {
    const existingItem = cart.value.find((item) => item.id === product.id)
    if (existingItem) {
      if (existingItem.quantity < product.onsite_qty) {
        existingItem.quantity++
      } else {
        const { showError } = useAlert()
        showError(`抱歉，"${product.name}" 库存不足！`)
      }
    } else {
      if (product.onsite_qty > 0) {
        cart.value.push({ ...product, quantity: 1 })
      }
    }
    scheduleQuote()
  }

  function removeFromCart(productId) {
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
    if (!activeEventId.value || cart.value.length === 0) return

    const orderData = {
      items: cart.value.map((item) => ({
        product_id: item.id,
        quantity: item.quantity,
      })),
    }

    try {
      // 使用 axios.post 发送 HTTP 请求
      const response = await api.post(`/events/${activeEventId.value}/orders`, orderData)
      // 成功后返回订单数据，让视图可以触发后续操作（如弹窗）
      return response.data
    } catch (err) {
      console.error('Order submission failed:', err)

      throw new Error(err.response?.data?.error || '下单失败，请重试。')
    }
  }

  // --- Getters ---
  // 单位：分（整数运算，展示端由 formatYuan 除以 100）。
  const cartTotal = computed(() => {
    return cart.value.reduce((total, item) => total + item.unit_price * item.quantity, 0)
  })

  // --- 报价 ---
  // 折扣由服务端算（`domain/solver.rs`）。前端不重写一份求解器：同一套集合覆盖
  // 两份实现，取整规则一漂移就是「显示 145 实收 150」。
  const quote = ref(null)
  const quoteError = ref(null)
  // 报价在途（debounce 窗口 + 请求往返）。为真时 cartSummary 必须按原价显示：
  // 否则上一车的 quote 配上这一车的 cartTotal，会渲染出「原价 ~~¥50~~ / 应付 ¥30」
  // 却一条优惠说明都没有，正好打中 D3 要保护的那个决策点。
  const quotePending = ref(false)
  let quoteSeq = 0
  let quoteTimer = null

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
    quotePending.value = true
    quoteTimer = setTimeout(fetchQuote, 300)
  }

  async function fetchQuote() {
    const seq = (quoteSeq += 1)
    const items = cart.value.map((item) => ({ product_id: item.id, quantity: item.quantity }))
    try {
      const response = await api.post(`/events/${activeEventId.value}/quote`, { items })
      if (seq !== quoteSeq) return // 旧请求，结果丢掉
      quote.value = response.data
      quoteError.value = null
    } catch (err) {
      if (seq !== quoteSeq) return
      // **不静默**：退回原价，同时明说没套用优惠。后端在超限时给的就是人话。
      quote.value = null
      quoteError.value = err.response?.data?.error || '优惠暂时算不出来，按原价显示'
    } finally {
      if (seq === quoteSeq) quotePending.value = false
    }
  }

  const cartSummary = computed(() => {
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
