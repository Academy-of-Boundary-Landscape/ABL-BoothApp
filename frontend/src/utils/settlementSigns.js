// 两套**相反**的符号约定，来自两个不同的接口：
//   GET /adjustments 返回 DB 存的值 = 对「社团往来」余额的影响，to_them 存负数
//   SettlementReport 里的 amount    = 对「我应转给」的影响，  to_them 是正数
// 谁也别想把这两个函数 DRY 成一个——它们看着像，符号正好相反。

import { formatYuan } from './money'

/**
 * 结算调整列表（`GET /events/:id/adjustments`）的方向。
 *
 * 返回的就是 `settlement_adjustments.amount`：**负 = 我要多给他们**（`to_them`）、
 * 正 = 他们要多给我（`to_me`）。不要把原始符号摆出来，按方向说人话。
 */
export function describeEntryAdjustment(cents) {
  if (cents < 0) return `我多给 ${formatYuan(-cents)}`
  if (cents > 0) return `他们多给 ${formatYuan(cents)}`
  return '—'
}

/**
 * 结算单（`SettlementReport`）里调整的方向。
 *
 * 后端已经把 `settlement_adjustments.amount` 取负换算成「对我应转给的影响」，
 * 所以这里**正 = 我多给**（`to_them`）、负 = 他们多给（`to_me`）。和上面
 * 列表接口的符号正好相反，两边各按各自的数据源显示，不要合并成一个函数。
 */
export function describeReportAdjustment(cents) {
  if (cents > 0) return `我多给 ${formatYuan(cents)}`
  if (cents < 0) return `他们多给 ${formatYuan(-cents)}`
  return '—'
}
