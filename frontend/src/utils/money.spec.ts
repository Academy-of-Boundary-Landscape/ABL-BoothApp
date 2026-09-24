import { describe, it, expect } from 'vitest'
import { cents, fromCents, toCents, formatCents, formatYuan, type Cents } from './money'

// 测试里构造 Cents 输入允许用 as——生产代码里唯一的 as Cents 在 money.ts 的 toCents()。
const c = (n: number) => n as Cents

describe('money', () => {
  it('formats cents with two decimals', () => {
    expect(formatCents(c(1990))).toBe('19.90')
    expect(formatCents(c(5))).toBe('0.05')
    expect(formatCents(c(0))).toBe('0.00')
    expect(formatCents(c(-1990))).toBe('-19.90')
  })

  it('never puts NaN on screen', () => {
    // 后端字段改名时漏掉某处，宁可显示 -- 也不要 NaN
    expect(formatCents(undefined)).toBe('--')
    expect(formatCents(null)).toBe('--')
    // 运行时防御：类型之外的脏值（比如还没迁 TS 的调用方传进来的字符串）
    expect(formatCents('abc' as unknown as Cents)).toBe('--')
  })

  it('rounds yuan input to whole cents', () => {
    // 19.99 * 100 === 1998.9999... 在 JS 里是真的
    expect(toCents(19.99)).toBe(1999)
    expect(toCents('30')).toBe(3000)
    expect(toCents('')).toBe(0)
  })

  it('round-trips through cents', () => {
    expect(toCents(fromCents(c(1999)))).toBe(1999)
  })

  it('prefixes yuan sign', () => {
    expect(formatYuan(c(1990))).toBe('¥19.90')
    expect(formatYuan(undefined)).toBe('--')
  })

  it('a bare number is not Cents (checked by vue-tsc, not at runtime)', () => {
    // @ts-expect-error 裸 number 不能当金额——必须来自后端响应或 toCents()
    expect(formatYuan(1990)).toBe('¥19.90')
    // @ts-expect-error 元 → 分只能走 toCents，不能直接断言
    const yuan: Cents = 19.9
    expect(yuan).toBe(19.9)
  })

  it('cents() re-brands integer-cent arithmetic and rounds stray fractions', () => {
    // Cents 做 + - * / 之后会退化成 number；分摊、合计之后用 cents() 标回来
    const a = c(3333)
    const b = c(1667)
    expect(cents(a + b)).toBe(5000)
    expect(cents((a * 2) / 3)).toBe(2222)
    expect(cents(Number.NaN)).toBe(0)
  })
})
