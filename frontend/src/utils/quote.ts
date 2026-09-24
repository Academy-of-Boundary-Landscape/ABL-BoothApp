import type { components } from '@/api/schema'
import { cents, type Cents } from './money'

type Schemas = components['schemas']

/**
 * `/quote` 响应里本函数用到的部分。直接取自生成的契约类型——后端改字段名，这里就编译不过。
 * 用 Pick 而不是整个类型：纯函数只依赖它读的字段，单测也只需要造这几个字段。
 */
export type QuoteLot = Pick<Schemas['LotQuoteLot'], 'name' | 'price' | 'original_amount'>
export type QuoteResponse = Pick<Schemas['LotQuoteResponse'], 'gross_amount' | 'solved_amount'> & {
  lots: QuoteLot[]
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
  // 优惠额按整数分累加，最后用 cents() 标回 Cents（branded 类型做减法会退化成 number）。
  const byName = new Map<string, { name: string; saved: number; count: number }>()
  for (const lot of quote.lots || []) {
    const entry = byName.get(lot.name) || { name: lot.name, saved: 0, count: 0 }
    entry.saved += lot.original_amount - lot.price
    entry.count += 1
    byName.set(lot.name, entry)
  }
  return {
    gross: quote.gross_amount,
    payable: quote.solved_amount,
    discounts: [...byName.values()]
      .filter((d) => d.saved > 0)
      .map((d) => ({ name: d.name, saved: cents(d.saved), count: d.count })),
  }
}
