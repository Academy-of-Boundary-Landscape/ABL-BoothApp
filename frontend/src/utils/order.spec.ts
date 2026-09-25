import { describe, expect, it } from 'vitest'
import { isFullyRefunded } from '@/utils/order'
import { cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

function makeItem(
  overrides: Partial<Schemas['OrderItemResponse']> = {}
): Schemas['OrderItemResponse'] {
  return {
    id: 1,
    allocated_amount: cents(0),
    lot_name: null,
    paid_amount: cents(1000),
    product_id: 1,
    product_image_url: null,
    product_name: '测试商品',
    product_price: cents(500),
    quantity: 2,
    refunded_amount: cents(0),
    refunded_qty: 0,
    ...overrides,
  }
}

function makeOrder(items: Schemas['OrderItemResponse'][]): Schemas['OrderResponse'] {
  return {
    id: 1,
    event_id: 3,
    gross_amount: cents(1000),
    solved_amount: cents(1000),
    final_amount: cents(1000),
    refunded_amount: cents(0),
    status: 'completed',
    timestamp: '2026-10-01T10:00:00Z',
    channel: '现金',
    completed_at: null,
    items,
    lots: [],
  }
}

describe('isFullyRefunded', () => {
  it('items 为空返回 false（没商品的订单谈不上全部退完）', () => {
    expect(isFullyRefunded(makeOrder([]))).toBe(false)
  })

  it('每一行都退满时返回 true', () => {
    const order = makeOrder([
      makeItem({ id: 1, quantity: 2, refunded_qty: 2 }),
      makeItem({ id: 2, quantity: 1, refunded_qty: 3 }),
    ])
    expect(isFullyRefunded(order)).toBe(true)
  })

  it('只要有一行没退满就返回 false', () => {
    const order = makeOrder([
      makeItem({ id: 1, quantity: 2, refunded_qty: 2 }),
      makeItem({ id: 2, quantity: 2, refunded_qty: 1 }),
    ])
    expect(isFullyRefunded(order)).toBe(false)
  })

  it('完全没退过返回 false', () => {
    expect(isFullyRefunded(makeOrder([makeItem({ quantity: 2, refunded_qty: 0 })]))).toBe(false)
  })
})
