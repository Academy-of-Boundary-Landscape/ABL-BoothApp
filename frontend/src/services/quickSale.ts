import { api, ApiRequestError, unwrap, type Schemas } from '@/api/client'
import type { Cents } from '@/utils/money'

export interface PendingQuickSale {
  eventId: number
  items: Schemas['CartItemRequest'][]
  channel: string
  expectedAmount: Cents
  finalAmount: Cents
  requestId: string
}

const STORAGE_KEY = 'vendor_quick_sale_pending_v1'

export function getPendingQuickSale(): PendingQuickSale | null {
  const raw = localStorage.getItem(STORAGE_KEY)
  if (!raw) return null
  try {
    const value: unknown = JSON.parse(raw)
    if (
      value !== null &&
      typeof value === 'object' &&
      typeof (value as PendingQuickSale).eventId === 'number' &&
      Array.isArray((value as PendingQuickSale).items) &&
      (value as PendingQuickSale).items.length > 0 &&
      (value as PendingQuickSale).items.every(
        (item) => typeof item.product_id === 'number' && typeof item.quantity === 'number'
      ) &&
      typeof (value as PendingQuickSale).channel === 'string' &&
      typeof (value as PendingQuickSale).expectedAmount === 'number' &&
      typeof (value as PendingQuickSale).finalAmount === 'number' &&
      typeof (value as PendingQuickSale).requestId === 'string'
    ) {
      return value as PendingQuickSale
    }
  } catch {
    // 旧版或损坏的暂存数据不能构造可靠重试请求。
  }
  localStorage.removeItem(STORAGE_KEY)
  return null
}

export async function recordQuickSale(
  eventId: number,
  items: Schemas['CartItemRequest'][],
  channel: string,
  expectedAmount: Cents,
  finalAmount: Cents = expectedAmount
): Promise<Schemas['OrderResponse']> {
  if (!items.length) throw new Error('请先选择商品')
  const pending = getPendingQuickSale()
  if (pending) {
    if (
      pending.eventId !== eventId ||
      JSON.stringify(pending.items) !== JSON.stringify(items) ||
      pending.channel !== channel ||
      pending.expectedAmount !== expectedAmount ||
      pending.finalAmount !== finalAmount
    ) {
      throw new Error('上一笔快售结果未确认，请先重试上一笔')
    }
    return submit(pending)
  }
  const request: PendingQuickSale = {
    eventId,
    items: items.map((item) => ({ ...item })),
    channel,
    expectedAmount,
    finalAmount,
    requestId: crypto.randomUUID(),
  }
  // 必须先保存，再发请求。超时或页面刷新后才能用同一个编号安全重试。
  localStorage.setItem(STORAGE_KEY, JSON.stringify(request))
  return submit(request)
}

export async function retryPendingQuickSale(): Promise<Schemas['OrderResponse']> {
  const request = getPendingQuickSale()
  if (!request) throw new Error('没有待确认的快售')
  return submit(request)
}

async function submit(request: PendingQuickSale): Promise<Schemas['OrderResponse']> {
  try {
    const order = await unwrap(
      api.POST('/events/{event_id}/quick-sales', {
        params: { path: { event_id: request.eventId } },
        body: {
          items: request.items,
          channel: request.channel,
          expected_amount: request.expectedAmount,
          final_amount: request.finalAmount,
          request_id: request.requestId,
        },
      })
    )
    if (getPendingQuickSale()?.requestId === request.requestId) {
      localStorage.removeItem(STORAGE_KEY)
    }
    if (order.status !== 'completed') {
      throw new Error('这笔快售已撤销或未完成，请到订单页核对')
    }
    return order
  } catch (error) {
    // 明确的客户端拒绝不会写入账本；未知网络结果保留编号，供摊主重试确认。
    if (error instanceof ApiRequestError && error.status >= 400 && error.status < 500) {
      if (getPendingQuickSale()?.requestId === request.requestId) {
        localStorage.removeItem(STORAGE_KEY)
      }
    }
    throw error
  }
}
