// 两套**相反**的符号约定，来自两个不同的接口：
//   GET /adjustments 返回 DB 存的值 = 对「社团往来」余额的影响，to_them 存负数
//   SettlementReport 里的 amount    = 对「我应转给」的影响，  to_them 是正数
// 谁也别想把这两个函数 DRY 成一个——它们看着像，符号正好相反。

import { formatYuan, fromCents, toCents, type Cents } from './money'

/**
 * 取金额的绝对值用于「我多给 ¥x」这类句式。
 *
 * `Cents` 是 branded number，`-cents` 会退化成裸 number；这里经 fromCents/toCents
 * 往返转回 `Cents`（整数分无损），保持「金额只经 money.ts 换算」的约定。
 */
function formatMagnitude(cents: Cents): string {
  return formatYuan(cents < 0 ? toCents(-fromCents(cents)) : cents)
}

/**
 * 结算调整列表（`GET /events/:id/adjustments`）的方向。
 *
 * 返回的就是 `settlement_adjustments.amount`：**负 = 我要多给他们**（`to_them`）、
 * 正 = 他们要多给我（`to_me`）。不要把原始符号摆出来，按方向说人话。
 */
export function describeEntryAdjustment(cents: Cents): string {
  if (cents < 0) return `我多给 ${formatMagnitude(cents)}`
  if (cents > 0) return `他们多给 ${formatYuan(cents)}`
  return '—'
}

/**
 * 结算单（`SettlementReport`）里调整的方向。
 *
 * 后端已经把 `settlement_adjustments.amount` 取负换算成「对我应转给的影响」，
 * 所以这里**正 = 我多给**（`to_them`）、负 = 他们多给（`to_me`）。和上面
 * 列表接口的符号正好相反，两边各按各自的数据源显示，不要合并成一个函数。
 *
 * 数字也按这个「对我应转给的影响」带符号——结算单那几行是要照面值直接相加的，
 * 只写方向不带号，摊主得先知道「他们多给」意味着减才加得出来。
 */
export function describeReportAdjustment(cents: Cents): string {
  if (cents > 0) return `我多给 +${formatYuan(cents)}`
  if (cents < 0) return `他们多给 −${formatMagnitude(cents)}`
  return '—'
}

/**
 * 手工折让那一项的标签。
 *
 * `manual_discount_net` 的约定是**折让为正、加价为负**（见
 * `src-tauri/src/domain/settlement.rs` 那个字段的文档注释）。加价时数字虽然带
 * `+`（数学没错），标签若还写「手工折让」，「手工折让 +¥60.00」就会被读成
 * 折让被撤销或加倍。数字的符号由调用方的 `formatSigned` 给，这里只管措辞，
 * 所以不要顺手把两者并成一个函数。
 */
export function manualDiscountLabel(cents: Cents): string {
  return cents < 0 ? '手工加价' : '手工折让'
}
