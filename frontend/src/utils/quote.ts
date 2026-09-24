import { fromCents, toCents, type Cents } from './money'

/** `/quote` 响应里的一个套装实例。 */
export interface QuoteLot {
  lot_id?: number
  name: string
  price?: Cents | null
  original_amount?: Cents | null
}

/** `/quote` 响应。 */
export interface QuoteResponse {
  gross_amount: Cents
  solved_amount: Cents
  lots?: QuoteLot[] | null
}

/** 合并后的单条优惠。 */
export interface QuoteDiscount {
  name: string
  saved: Cents
  count: number
}

/** 购物车要显示的三样东西。 */
export interface QuoteSummary {
  gross: Cents
  payable: Cents
  discounts: QuoteDiscount[]
}

/**
 * 把 `/quote` 的响应整理成购物车要显示的三样东西：原价、应付、逐条优惠。
 *
 * 纯函数——不碰网络也不碰 store，所以能直接单测。
 *
 * `quote` 为 null（还没报价，或报价失败）时退回原价合计。**这不是静默降级**：
 * 调用方必须另外把 `quoteError` 显示出来，否则顾客会以为原价就是应付价。
 */
export function summarizeQuote(
  quote: QuoteResponse | null | undefined,
  grossFallback: Cents
): QuoteSummary {
  if (!quote) {
    return { gross: grossFallback, payable: grossFallback, discounts: [] }
  }
  // 同一个套装可以套用多次（「任选 3 本 100」买 6 本 = 两个实例）。
  // 按名字合并再带一个次数——列两行一模一样的文字会让人以为系统算重了。
  // 优惠额在元上累加（Cents 是 branded 类型，直接相减会退化成裸 number）。
  const byName = new Map<string, { name: string; savedYuan: number; count: number }>()
  for (const lot of quote.lots || []) {
    const savedYuan = fromCents(lot.original_amount) - fromCents(lot.price)
    const entry = byName.get(lot.name) || { name: lot.name, savedYuan: 0, count: 0 }
    entry.savedYuan += savedYuan
    entry.count += 1
    byName.set(lot.name, entry)
  }
  return {
    gross: quote.gross_amount,
    payable: quote.solved_amount,
    discounts: [...byName.values()]
      .filter((d) => d.savedYuan > 0)
      .map((d) => ({ name: d.name, saved: toCents(d.savedYuan), count: d.count })),
  }
}
