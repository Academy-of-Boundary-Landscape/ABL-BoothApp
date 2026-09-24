import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, unwrap, type Schemas } from '@/api/client'
import { getImageUrl } from '@/services/url'
import type { Cents } from '@/utils/money'

export const useEventDetailStore = defineStore('eventDetail', () => {
  // --- State ---
  const event = ref<Schemas['EventResponse'] | null>(null) // 存储当前展会的详细信息 (暂时未使用，但可扩展)
  const products = ref<Schemas['ProductEventProduct'][]>([]) // 存储该展会上架的商品列表
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const allOrders = ref<Schemas['OrderResponse'][]>([])

  // 预处理订单数据，将商品图片 URL 转换为完整路径
  const processOrders = (orders: Schemas['OrderResponse'][]): Schemas['OrderResponse'][] => {
    return orders.map((order) => ({
      ...order,
      items: order.items.map((item) => ({
        ...item,
        product_image_url: getImageUrl(item.product_image_url ?? ''),
      })),
    }))
  }

  // --- Actions ---
  // 获取指定展会的商品列表
  async function fetchProductsForEvent(eventId: number) {
    isLoading.value = true
    error.value = null
    try {
      const response = await unwrap(
        api.GET('/events/{event_id}/products', { params: { path: { event_id: eventId } } })
      )
      products.value = response.map((product) => ({
        ...product,
        image_url: getImageUrl(product.image_url ?? ''),
      }))
    } catch (err) {
      error.value = '无法加载展会商品列表。'
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }

  // 为展会添加一个商品
  async function addProductToEvent(eventId: number, productData: Schemas['ProductAddRequest']) {
    try {
      const response = await unwrap(
        api.POST('/events/{event_id}/products', {
          params: { path: { event_id: eventId } },
          body: productData,
        })
      )
      const product = { ...response, image_url: getImageUrl(response.image_url ?? '') }
      products.value.unshift(product)
      return product
    } catch (err) {
      console.error(err)
      throw err
    }
  }

  // 更新展会商品的价格（库存不能直接改，只能走 restock / 盘点）
  async function updateEventProduct(
    productId: number,
    productData: Schemas['ProductUpdateRequest']
  ) {
    try {
      const response = await unwrap(
        api.PUT('/products/{id}', {
          params: { path: { id: productId } },
          body: productData,
        })
      )
      const product = { ...response, image_url: getImageUrl(response.image_url ?? '') }
      const index = products.value.findIndex((p) => p.id === productId)
      if (index !== -1) {
        products.value[index] = product
      }
      return product
    } catch (err) {
      console.error(err)
      throw err
    }
  }

  // 补货：记一条「外部 → 现场仓」的进货移动，库存只能这样增加。
  async function restockEventProduct(
    eventId: number,
    productId: number,
    qty: number,
    note?: string
  ) {
    try {
      const payload: Schemas['ProductRestockRequest'] = { qty }
      if (note) payload.note = note
      const response = await unwrap(
        api.POST('/events/{event_id}/products/{id}/restock', {
          params: { path: { event_id: eventId, id: productId } },
          body: payload,
        })
      )
      const product = { ...response, image_url: getImageUrl(response.image_url ?? '') }
      const index = products.value.findIndex((p) => p.id === productId)
      if (index !== -1) {
        products.value[index] = product
      }
      return product
    } catch (err) {
      console.error(err)
      throw err
    }
  }

  // 从展会中移除一个商品
  async function deleteEventProduct(productId: number) {
    try {
      await unwrap(api.DELETE('/products/{id}', { params: { path: { id: productId } } }))
      products.value = products.value.filter((p) => p.id !== productId)
    } catch (err) {
      console.error(err)
      throw err
    }
  }
  async function fetchAllOrdersForEvent(eventId: number) {
    isLoading.value = true
    error.value = null
    try {
      // 不带 status 参数，获取所有订单
      const response = await unwrap(
        api.GET('/events/{event_id}/orders', { params: { path: { event_id: eventId } } })
      )
      allOrders.value = processOrders(response)
    } catch (err) {
      error.value = '无法加载订单列表。'
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }
  async function adminUpdateOrderStatus(
    eventId: number,
    orderId: number,
    newStatus: Schemas['OrderStatus'],
    channel?: string,
    finalAmount?: Cents | null,
    unapplyLotIds?: number[]
  ) {
    try {
      const payload: Schemas['OrderUpdateStatusRequest'] = { status: newStatus }
      // 只有「完成」才需要渠道：账本里钱那条腿得有对手账户。
      if (channel) payload.channel = channel
      // 不传就等于 solved_amount（后端决定），所以只在真拿到数字时才带上。
      if (Number.isFinite(finalAmount)) payload.final_amount = finalAmount
      // 空数组也不传：后端对非 completed 的转换会拒绝这个字段，少传少一处可能。
      if (unapplyLotIds?.length) payload.unapply_lot_ids = unapplyLotIds
      const response = await unwrap(
        api.PUT('/events/{event_id}/orders/{order_id}/status', {
          params: { path: { event_id: eventId, order_id: orderId } },
          body: payload,
        })
      )
      // 更新成功后，在本地 allOrders 列表中找到并更新该订单
      const processedOrders = processOrders([response])
      const index = allOrders.value.findIndex((o) => o.id === orderId)
      if (index !== -1) {
        allOrders.value[index] = processedOrders[0]
      }
      return processedOrders[0]
    } catch (err) {
      console.error(err)
      throw err
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
