import { describe, it, expect } from 'vitest'
import { splitRefund, defaultRefundTotal } from './refund'

describe('splitRefund', () => {
  it('和精确等于总额', () => {
    // 后端用的是「向下取整 + 余数逐分派发」（domain/allocation.rs 的 apportion）。
    // 前端这份只用于**显示**，但显示的数加起来对不上总额一样会让摊主不信任。
    const out = splitRefund(1000, [334, 333, 333])
    expect(out.reduce((a, b) => a + b, 0)).toBe(1000)
  })

  it('每一份都不超过该行实付', () => {
    const out = splitRefund(1000, [200, 900])
    expect(out[0]).toBeLessThanOrEqual(200)
    expect(out[1]).toBeLessThanOrEqual(900)
    expect(out.reduce((a, b) => a + b, 0)).toBe(1000)
  })

  it('权重全零时给全零而不是 NaN', () => {
    // 0 元 SKU（预售取货）。除以零会让界面显示一排 NaN。
    expect(splitRefund(0, [0, 0])).toEqual([0, 0])
  })

  it('总额为零时给全零', () => {
    expect(splitRefund(0, [500, 500])).toEqual([0, 0])
  })

  it('空输入不炸', () => {
    expect(splitRefund(0, [])).toEqual([])
  })
})

describe('defaultRefundTotal', () => {
  it('默认退款等于所选各行实付之和', () => {
    const lines = [
      { order_line_id: 1, remaining_qty: 2, remaining_paid: 1000 },
      { order_line_id: 2, remaining_qty: 1, remaining_paid: 600 },
    ]
    // 第一行退 1 件（一半），第二行不退
    expect(defaultRefundTotal(lines, { 1: 1, 2: 0 })).toBe(500)
  })

  it('一件不退时是 0', () => {
    const lines = [{ order_line_id: 1, remaining_qty: 2, remaining_paid: 1000 }]
    expect(defaultRefundTotal(lines, { 1: 0 })).toBe(0)
  })

  it('和后端 apportion 一致：余数归要退的那份', () => {
    // 后端：apportion(3333, [1, 1]) = [1667, 1666]，余数按「权重降序、下标升序」派给第一份。
    // 只做向下取整会得到 1666，于是界面写 ¥16.66、后端实退 ¥16.67。
    const lines = [{ order_line_id: 1, remaining_qty: 2, remaining_paid: 3333 }]
    expect(defaultRefundTotal(lines, { 1: 1 })).toBe(1667)
  })

  it('多行各差一分时差额会累积', () => {
    const lines = [
      { order_line_id: 1, remaining_qty: 2, remaining_paid: 3333 },
      { order_line_id: 2, remaining_qty: 2, remaining_paid: 3333 },
      { order_line_id: 3, remaining_qty: 2, remaining_paid: 3334 },
    ]
    expect(defaultRefundTotal(lines, { 1: 1, 2: 1, 3: 1 })).toBe(5001)
  })
})
