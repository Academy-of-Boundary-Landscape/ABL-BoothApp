import { describe, it, expect } from 'vitest'
import {
  describeEntryAdjustment,
  describeReportAdjustment,
  manualDiscountLabel,
} from './settlementSigns'
import type { Cents } from './money'

// 测试里构造 Cents 输入允许用 as——生产代码里唯一的 as Cents 在 money.ts 的 toCents()。
const c = (n: number) => n as Cents

// 两套**相反**的符号约定，来自两个不同的接口：
//   GET /adjustments 返回 DB 存的值 = 对「社团往来」余额的影响，to_them 存负数
//   SettlementReport 里的 amount    = 对「我应转给」的影响，  to_them 是正数
// 谁也别想把这两个函数 DRY 成一个——它们看着像，符号正好相反。
describe('两套相反的调整符号', () => {
  it('列表接口：负数 = 我要多给他们', () => {
    expect(describeEntryAdjustment(c(-2000))).toBe('我多给 ¥20.00')
    expect(describeEntryAdjustment(c(500))).toBe('他们多给 ¥5.00')
  })

  it('结算单：正数 = 我要多给他们', () => {
    expect(describeReportAdjustment(c(2000))).toBe('我多给 +¥20.00')
    expect(describeReportAdjustment(c(-500))).toBe('他们多给 −¥5.00')
  })
})

describe('手工折让的标签', () => {
  it('折让为正叫「手工折让」，加价为负叫「手工加价」', () => {
    expect(manualDiscountLabel(c(6000))).toBe('手工折让')
    expect(manualDiscountLabel(c(-6000))).toBe('手工加价')
    expect(manualDiscountLabel(c(0))).toBe('手工折让')
  })
})
