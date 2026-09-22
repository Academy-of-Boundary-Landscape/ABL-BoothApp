/**
 * 把 `/quote` 的响应整理成购物车要显示的三样东西：原价、应付、逐条优惠。
 *
 * 纯函数——不碰网络也不碰 store，所以能直接单测。
 *
 * `quote` 为 null（还没报价，或报价失败）时退回原价合计。**这不是静默降级**：
 * 调用方必须另外把 `quoteError` 显示出来，否则顾客会以为原价就是应付价。
 */
export function summarizeQuote(quote, grossFallback) {
  if (!quote) {
    return { gross: grossFallback, payable: grossFallback, discounts: [] }
  }
  // 同一个套装可以套用多次（「任选 3 本 100」买 6 本 = 两个实例）。
  // 按名字合并再带一个次数——列两行一模一样的文字会让人以为系统算重了。
  const byName = new Map()
  for (const lot of quote.lots || []) {
    const saved = (lot.original_amount || 0) - (lot.price || 0)
    const entry = byName.get(lot.name) || { name: lot.name, saved: 0, count: 0 }
    entry.saved += saved
    entry.count += 1
    byName.set(lot.name, entry)
  }
  return {
    gross: quote.gross_amount,
    payable: quote.solved_amount,
    discounts: [...byName.values()].filter((d) => d.saved > 0),
  }
}
