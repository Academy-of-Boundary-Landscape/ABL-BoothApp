import { fromCents, toCents, type Cents } from './money'

/** 参与退款摊分的订单行。 */
export interface RefundLine {
  order_line_id: number
  remaining_qty: number
  remaining_paid: Cents
}

/** 每行选择的退款件数，键是 `order_line_id`。 */
export type QtyByLine = Record<number, number>

/**
 * 摊分结果是整数分（number），brand 回 `Cents` 只经 `toCents`。
 * 分 → 元 → 分对整数分无损，浮点残渣会被 toCents 的四舍五入吃掉。
 */
function brandCents(values: number[]): Cents[] {
  return values.map((v) => toCents(v / 100))
}

/**
 * 把 `total` 按 `weights` 分成同长的一组数，和精确等于 `total`。
 *
 * `capped` 为 true 时以权重本身为上限（退款摊回各行时用），
 * 为 false 时不设上限（把一行的金额按件数切成「退的」和「留的」时用——
 * 那里的权重是件数，拿件数当分的上限会把结果截成件数那么小）。
 *
 * **这一份只用于界面实时显示，权威计算在后端**
 * （`src-tauri/src/domain/allocation.rs` 的 `apportion`）。
 * 规则必须完全一致，否则摊主会看到「界面上写退 33，提交完变成 34」。
 * 规则：各自向下取整，余数按「权重降序、下标升序」逐分派发，跳过已顶到上限的那一份。
 */
export function splitRefund(total: Cents, weights: number[], capped: boolean = true): Cents[] {
  const n = weights.length
  const out: number[] = new Array<number>(n).fill(0)
  if (n === 0 || total <= 0) return brandCents(out)
  const sum = weights.reduce((a, b) => a + b, 0)
  if (sum <= 0) return brandCents(out) // 0 元行：除以零会让界面显示一排 NaN

  let assigned = 0
  for (let i = 0; i < n; i++) {
    out[i] = Math.floor((total * weights[i]) / sum)
    assigned += out[i]
  }

  const order = weights
    .map((w, i) => [w, i])
    .sort((a, b) => b[0] - a[0] || a[1] - b[1])
    .map(([, i]) => i)

  let rest = total - assigned
  while (rest > 0) {
    let moved = false
    for (const i of order) {
      if (rest === 0) break
      if (capped && out[i] >= weights[i]) continue
      out[i] += 1
      rest -= 1
      moved = true
    }
    // capped 时全顶到上限会到此为止（理论上不可达，total ≤ Σweights）；
    // 不设上限时这个分支永远走不到。
    if (!moved) break
  }
  return brandCents(out)
}

/**
 * 所选各行按件数等比切出的实付之和——「实际退款」输入框的默认值。
 *
 * 每一行的权重是件数而不是分，所以切一行的「退的/留的」必须用**不带 cap** 的
 * `splitRefund`——拿件数当分的上限会把结果截成件数那么小。取整规则和后端
 * `apportion(remaining_paid, &[qty, remaining_qty - qty], None)[0]` 逐分一致。
 *
 * 和 `splitRefund` 一样在元上累加，最后经 `toCents` 落回整数分。
 */
export function defaultRefundTotal(lines: RefundLine[], qtyByLine: QtyByLine): Cents {
  const totalYuan = lines.reduce((sumYuan, l) => {
    const q = Number(qtyByLine[l.order_line_id] || 0)
    if (q <= 0 || l.remaining_qty <= 0) return sumYuan
    return sumYuan + fromCents(splitRefund(l.remaining_paid, [q, l.remaining_qty - q], false)[0])
  }, 0)
  return toCents(totalYuan)
}
