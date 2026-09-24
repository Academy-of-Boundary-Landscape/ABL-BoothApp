/**
 * 退款金额的摊分。**这一份只用于界面实时显示，权威计算在后端**
 * （`src-tauri/src/domain/allocation.rs` 的 `apportion`）。
 *
 * 两边必须是同一套取整规则，否则摊主会看到「界面上写退 33，提交完变成 34」。
 * 规则：各自向下取整，余数按「权重降序、下标升序」逐分派发，跳过已顶到上限的那一份。
 */
export function splitRefund(total, weights) {
  const n = weights.length
  const out = new Array(n).fill(0)
  if (n === 0 || total <= 0) return out
  const sum = weights.reduce((a, b) => a + b, 0)
  if (sum <= 0) return out // 0 元行：除以零会让界面显示一排 NaN

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
      if (out[i] < weights[i]) {
        out[i] += 1
        rest -= 1
        moved = true
      }
    }
    if (!moved) break // 全部顶到上限，理论上不可达（total ≤ Σweights）
  }
  return out
}

/** 所选各行按件数等比切出的实付之和——「实际退款」输入框的默认值。 */
export function defaultRefundTotal(lines, qtyByLine) {
  return lines.reduce((sum, l) => {
    const q = Number(qtyByLine[l.order_line_id] || 0)
    if (q <= 0 || l.remaining_qty <= 0) return sum
    return sum + Math.floor((l.remaining_paid * q) / l.remaining_qty)
  }, 0)
}
