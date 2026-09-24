import { describe, it, expect } from 'vitest'
import {
  describeEntryAdjustment,
  describeReportAdjustment,
  manualDiscountLabel,
} from './settlementSigns.js'

// 两套**相反**的符号约定，来自两个不同的接口：
//   GET /adjustments 返回 DB 存的值 = 对「社团往来」余额的影响，to_them 存负数
//   SettlementReport 里的 amount    = 对「我应转给」的影响，  to_them 是正数
// 谁也别想把这两个函数 DRY 成一个——它们看着像，符号正好相反。
describe('两套相反的调整符号', () => {
  it('列表接口：负数 = 我要多给他们', () => {
    expect(describeEntryAdjustment(-2000)).toBe('我多给 ¥20.00')
    expect(describeEntryAdjustment(500)).toBe('他们多给 ¥5.00')
  })

  it('结算单：正数 = 我要多给他们', () => {
    expect(describeReportAdjustment(2000)).toBe('我多给 +¥20.00')
    expect(describeReportAdjustment(-500)).toBe('他们多给 −¥5.00')
  })
})

describe('手工折让的标签', () => {
  it('折让为正叫「手工折让」，加价为负叫「手工加价」', () => {
    expect(manualDiscountLabel(6000)).toBe('手工折让')
    expect(manualDiscountLabel(-6000)).toBe('手工加价')
    expect(manualDiscountLabel(0)).toBe('手工折让')
  })
})
