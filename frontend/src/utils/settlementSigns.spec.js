import { describe, it, expect } from 'vitest'
import { describeEntryAdjustment, describeReportAdjustment } from './settlementSigns.js'

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
    expect(describeReportAdjustment(2000)).toBe('我多给 ¥20.00')
    expect(describeReportAdjustment(-500)).toBe('他们多给 ¥5.00')
  })
})
