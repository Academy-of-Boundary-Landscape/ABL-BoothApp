import type { Schemas } from '@/api/client'

/**
 * 订单是否每行都退满（`refunded_qty >= quantity`）。
 *
 * 判据用订单里带回的逐行退货数，避免为每张已完成单各发一个请求。
 * `items` 为空返回 false——没有商品的订单谈不上「全部退完」。
 */
export function isFullyRefunded(order: Schemas['OrderResponse']): boolean {
  return order.items.length > 0 && order.items.every((item) => item.refunded_qty >= item.quantity)
}
