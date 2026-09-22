import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/services/api'
import { getImageUrl } from '@/services/url'

export const useEventDetailStore = defineStore('eventDetail', () => {
  // --- State ---
  const event = ref(null) // 存储当前展会的详细信息 (暂时未使用，但可扩展)
  const products = ref([]) // 存储该展会上架的商品列表
  const isLoading = ref(false)
  const error = ref(null)
  const allOrders = ref([])

  // 预处理订单数据，将商品图片 URL 转换为完整路径
  const processOrders = (orders) => {
    return orders.map((order) => ({
      ...order,
      items: order.items.map((item) => ({
        ...item,
        product_image_url: getImageUrl(item.product_image_url),
      })),
    }))
  }

  // --- Actions ---
  // 获取指定展会的商品列表
  async function fetchProductsForEvent(eventId) {
    isLoading.value = true
    error.value = null
    try {
      const response = await api.get(`/events/${eventId}/products`)
      products.value = response.data.map((product) => ({
        ...product,
        image_url: getImageUrl(product.image_url),
      }))
    } catch (err) {
      error.value = '无法加载展会商品列表。'
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }

  // 为展会添加一个商品
  async function addProductToEvent(eventId, productData) {
    try {
      const response = await api.post(`/events/${eventId}/products`, productData)
      const product = { ...response.data, image_url: getImageUrl(response.data.image_url) }
      products.value.unshift(product)
      return product
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '上架商品失败。')
    }
  }

  // 更新展会商品的价格（库存不能直接改，只能走 restock / 盘点）
  async function updateEventProduct(productId, productData) {
    try {
      const response = await api.put(`/products/${productId}`, productData)
      const product = { ...response.data, image_url: getImageUrl(response.data.image_url) }
      const index = products.value.findIndex((p) => p.id === productId)
      if (index !== -1) {
        products.value[index] = product
      }
      return product
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '更新商品失败。')
    }
  }

  // 补货：记一条「外部 → 现场仓」的进货移动，库存只能这样增加。
  async function restockEventProduct(eventId, productId, qty, note) {
    try {
      const payload = { qty }
      if (note) payload.note = note
      const response = await api.post(`/events/${eventId}/products/${productId}/restock`, payload)
      const product = { ...response.data, image_url: getImageUrl(response.data.image_url) }
      const index = products.value.findIndex((p) => p.id === productId)
      if (index !== -1) {
        products.value[index] = product
      }
      return product
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '补货失败。')
    }
  }

  // 从展会中移除一个商品
  async function deleteEventProduct(productId) {
    try {
      await api.delete(`/products/${productId}`)
      products.value = products.value.filter((p) => p.id !== productId)
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '下架商品失败。')
    }
  }
  async function fetchAllOrdersForEvent(eventId) {
    isLoading.value = true
    error.value = null
    try {
      // 不带 status 参数，获取所有订单
      const response = await api.get(`/events/${eventId}/orders`)
      allOrders.value = processOrders(response.data)
    } catch (err) {
      error.value = '无法加载订单列表。'
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }
  async function adminUpdateOrderStatus(
    eventId,
    orderId,
    newStatus,
    channel,
    finalAmount,
    unapplyLotIds
  ) {
    try {
      const payload = { status: newStatus }
      // 只有「完成」才需要渠道：账本里钱那条腿得有对手账户。
      if (channel) payload.channel = channel
      // 不传就等于 solved_amount（后端决定），所以只在真拿到数字时才带上。
      if (Number.isFinite(finalAmount)) payload.final_amount = finalAmount
      // 空数组也不传：后端对非 completed 的转换会拒绝这个字段，少传少一处可能。
      if (unapplyLotIds?.length) payload.unapply_lot_ids = unapplyLotIds
      const response = await api.put(`/events/${eventId}/orders/${orderId}/status`, payload)
      // 更新成功后，在本地 allOrders 列表中找到并更新该订单
      const processedOrders = processOrders([response.data])
      const index = allOrders.value.findIndex((o) => o.id === orderId)
      if (index !== -1) {
        allOrders.value[index] = processedOrders[0]
      }
      return processedOrders[0]
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '更新订单状态失败。')
    }
  }
  // 重置状态，以便在切换不同展会详情页时清空旧数据
  function resetStore() {
    event.value = null
    products.value = []
    error.value = null
    allOrders.value = []
  }

  return {
    event,
    products,
    isLoading,
    error,
    fetchProductsForEvent,
    addProductToEvent,
    updateEventProduct,
    restockEventProduct,
    deleteEventProduct,
    resetStore,
    adminUpdateOrderStatus,
    fetchAllOrdersForEvent,
    allOrders,
  }
})
