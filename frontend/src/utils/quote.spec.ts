import { describe, it, expect } from 'vitest'
import { summarizeQuote } from './quote'
import type { Cents } from './money'

// 测试里构造 Cents 输入允许用 as——生产代码里唯一的 as Cents 在 money.ts 的 toCents()。
const c = (n: number) => n as Cents

describe('summarizeQuote', () => {
  it('没有报价时退回原价合计，且不编造优惠', () => {
    expect(summarizeQuote(null, c(8000))).toEqual({ gross: 8000, payable: 8000, discounts: [] })
  })

  it('把一个套装折算成一条「省了多少」', () => {
    const quote = {
      gross_amount: c(9000),
      solved_amount: c(8000),
      lots: [{ lot_id: 1, name: '任选2件50', price: c(5000), original_amount: c(6000) }],
    }
    expect(summarizeQuote(quote, c(9000))).toEqual({
      gross: 9000,
      payable: 8000,
      discounts: [{ name: '任选2件50', saved: 1000, count: 1 }],
    })
  })

  it('同一个套装套用两次合并成一条，带次数', () => {
    // 「任选 3 本 100」买 6 本 = 两个实例。列两行同名的会让人以为系统算重了。
    const quote = {
      gross_amount: c(24000),
      solved_amount: c(20000),
      lots: [
        { lot_id: 1, name: '任选3本100', price: c(10000), original_amount: c(12000) },
        { lot_id: 1, name: '任选3本100', price: c(10000), original_amount: c(12000) },
      ],
    }
    const out = summarizeQuote(quote, c(24000))
    expect(out.discounts).toEqual([{ name: '任选3本100', saved: 4000, count: 2 }])
  })

  it('一分钱都没省的套装不显示', () => {
    const quote = {
      gross_amount: c(5000),
      solved_amount: c(5000),
      lots: [{ lot_id: 1, name: '不划算的套装', price: c(5000), original_amount: c(5000) }],
    }
    expect(summarizeQuote(quote, c(5000)).discounts).toEqual([])
  })
})
