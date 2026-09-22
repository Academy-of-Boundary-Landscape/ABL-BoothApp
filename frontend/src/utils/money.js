/**
 * 金额工具。**后端传的一律是分（整数）**，这里是唯一的换算与显示入口。
 *
 * 为什么后端不直接传元：分摊规则的核心断言是「各行金额之和必须精确等于总价」，
 * 浮点下写不出来（0.1 + 0.2 !== 0.3，而 19.9 这种价格摊位上遍地都是）。
 * 权威值在后端是整数分，前端只在显示的最后一刻除以 100。
 */

/** 分 → 元（数字）。只用于需要参与计算的场合，显示一律走 formatCents。 */
export function fromCents(cents) {
  return Number(cents || 0) / 100
}

/** 元（用户输入）→ 分。四舍五入到整数分，避免 19.99 * 100 === 1998.9999 这类浮点残渣。 */
export function toCents(yuan) {
  const n = Number(yuan)
  return Number.isFinite(n) ? Math.round(n * 100) : 0
}

/** 分 → 显示字符串，带两位小数。非法值回落到 '--'，不要让 NaN 上屏。 */
export function formatCents(cents) {
  // null 会被 Number() 变成 0，必须先单独挡掉，否则显示成 '0.00' 而不是 '--'。
  if (cents === null || cents === undefined) return '--'
  const n = Number(cents)
  if (!Number.isFinite(n)) return '--'
  const sign = n < 0 ? '-' : ''
  const a = Math.abs(n)
  return `${sign}${Math.floor(a / 100)}.${String(a % 100).padStart(2, '0')}`
}

/** 分 → 带 ¥ 前缀的显示字符串。 */
export function formatYuan(cents) {
  const s = formatCents(cents)
  return s === '--' ? s : `¥${s}`
}
