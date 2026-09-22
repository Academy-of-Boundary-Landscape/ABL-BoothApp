import { describe, it, expect } from 'vitest'
import { fromCents, toCents, formatCents, formatYuan } from './money.js'

describe('money', () => {
  it('formats cents with two decimals', () => {
    expect(formatCents(1990)).toBe('19.90')
    expect(formatCents(5)).toBe('0.05')
    expect(formatCents(0)).toBe('0.00')
    expect(formatCents(-1990)).toBe('-19.90')
  })

  it('never puts NaN on screen', () => {
    // 后端字段改名时漏掉某处，宁可显示 -- 也不要 NaN
    expect(formatCents(undefined)).toBe('--')
    expect(formatCents(null)).toBe('--')
    expect(formatCents('abc')).toBe('--')
  })

  it('rounds yuan input to whole cents', () => {
    // 19.99 * 100 === 1998.9999... 在 JS 里是真的
    expect(toCents(19.99)).toBe(1999)
    expect(toCents('30')).toBe(3000)
    expect(toCents('')).toBe(0)
  })

  it('round-trips through cents', () => {
    expect(toCents(fromCents(1999))).toBe(1999)
  })

  it('prefixes yuan sign', () => {
    expect(formatYuan(1990)).toBe('¥19.90')
    expect(formatYuan(undefined)).toBe('--')
  })
})
