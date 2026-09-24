/**
 * 金额工具。**后端传的一律是分（整数）**，这里是唯一的换算与显示入口。
 *
 * 为什么后端不直接传元：分摊规则的核心断言是「各行金额之和必须精确等于总价」，
 * 浮点下写不出来（0.1 + 0.2 !== 0.3，而 19.9 这种价格摊位上遍地都是）。
 * 权威值在后端是整数分，前端只在显示的最后一刻除以 100。
 *
 * `Cents` 是 branded type：后端响应里的金额（schema.d.ts 里 `format: cents` 的字段）
 * 天然是 Cents；用户输入只能经 toCents() 变成 Cents。裸 number 传给 formatYuan 会报类型错误——
 * 这就是「把分当成元显示」在编译期被拦下的方式。
 */

/** 金额，单位：分。只能来自后端响应或 toCents()。 */
export type Cents = number & { readonly __brand: 'Cents' }

/** 分 → 元（数字）。只用于需要参与计算的场合，显示一律走 formatCents。 */
export function fromCents(cents: Cents | null | undefined): number {
  return Number(cents || 0) / 100
}

/** 元（用户输入）→ 分。四舍五入到整数分，避免 19.99 * 100 === 1998.9999 这类浮点残渣。 */
export function toCents(yuan: number | string): Cents {
  const n = Number(yuan)
  // 全仓只有这里和 cents() 两处 as Cents：「元 → 分」与「分的运算结果 → 分」两道门。
  return (Number.isFinite(n) ? Math.round(n * 100) : 0) as Cents
}

/**
 * 整数分运算的结果 → Cents。`Cents` 做 + - * / 之后会退化成 number，
 * 合计、分摊、差额算完之后用它标回来（四舍五入到整数分，非有限值记为 0）。
 * 与 toCents 的区别：toCents 的输入是**元**，这里的输入已经是**分**。
 */
export function cents(n: number): Cents {
  return (Number.isFinite(n) ? Math.round(n) : 0) as Cents
}

/** 分 → 显示字符串，带两位小数。非法值回落到 '--'，不要让 NaN 上屏。 */
export function formatCents(cents: Cents | null | undefined): string {
  // null 会被 Number() 变成 0，必须先单独挡掉，否则显示成 '0.00' 而不是 '--'。
  if (cents === null || cents === undefined) return '--'
  const n = Number(cents)
  if (!Number.isFinite(n)) return '--'
  const sign = n < 0 ? '-' : ''
  const a = Math.abs(n)
  return `${sign}${Math.floor(a / 100)}.${String(a % 100).padStart(2, '0')}`
}

/** 分 → 带 ¥ 前缀的显示字符串。 */
export function formatYuan(cents: Cents | null | undefined): string {
  const s = formatCents(cents)
  return s === '--' ? s : `¥${s}`
}
