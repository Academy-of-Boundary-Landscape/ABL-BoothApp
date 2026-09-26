/**
 * 扫码命中后的统一处理（spec §4.2）：匹配 → 售罄判断 → 加购 → 提示。
 *
 * 扫码面板（BarcodeScanPanel）与点单页扫码枪（CustomerView + useScanGun）共用这一份逻辑，
 * 区别只在「加购」和「提示」怎么落地：面板 emit `add` 并写自己的提示行，点单页直接
 * `store.addToCart` 并用 `useFeedback` 的轻提示（非模态）。
 */
import { matchBarcode } from '@/utils/barcodeMatch'
import type { Schemas } from '@/api/client'

export type ScanProduct = Schemas['ProductEventProduct']

export type ScanOutcome =
  | { kind: 'added'; product: ScanProduct }
  | { kind: 'sold_out'; product: ScanProduct }
  | { kind: 'out_of_stock'; product: ScanProduct }
  | { kind: 'not_found'; code: string }
  | { kind: 'multiple'; products: ScanProduct[] }
  | { kind: 'ignored' }

/** 购物车里的一行：只关心商品 id 与已有数量。 */
export interface CartLine {
  id: number
  quantity: number
}

export interface ScanResultHandlerOptions {
  /** 本场商品快照（每次扫描时取，避免拿到过期的售罄状态）。 */
  products: () => readonly ScanProduct[]
  /** 购物车当前内容：用于判断同商品是否已被加满库存。缺省视为空车。 */
  cart?: () => readonly CartLine[]
  /** 命中且有货：加入购物车（或由面板 emit 给父组件）。 */
  addToCart: (product: ScanProduct) => void
  /** 提示：kind 为 error 时是失败文案。 */
  notify: (message: string, kind: 'success' | 'error') => void
}

export function useScanResultHandler(opts: ScanResultHandlerOptions): {
  handleCode(code: string): ScanOutcome
  resolveProduct(product: ScanProduct): ScanOutcome
} {
  /** 购物车里该商品已有数量（按 id 找 quantity；没有就是 0）。 */
  function quantityInCart(productId: number): number {
    return (opts.cart?.() ?? []).find((line) => line.id === productId)?.quantity ?? 0
  }

  function resolveProduct(product: ScanProduct): ScanOutcome {
    if (product.onsite_qty <= 0) {
      opts.notify(`${product.name} 已售罄`, 'error')
      return { kind: 'sold_out', product }
    }
    // 购物车已占满本场库存：按失败处理，不调用 addToCart——它会自己弹「库存不足」
    // 模态框并打断连续扫描。
    if (quantityInCart(product.id) >= product.onsite_qty) {
      opts.notify(`${product.name} 库存不足`, 'error')
      return { kind: 'out_of_stock', product }
    }
    opts.addToCart(product)
    opts.notify(`+1 ${product.name}`, 'success')
    return { kind: 'added', product }
  }

  function handleCode(raw: string): ScanOutcome {
    const result = matchBarcode(raw, opts.products())
    if (result.kind === 'ignored') return { kind: 'ignored' }
    if (result.kind === 'not_found') {
      opts.notify(`未找到条码 ${result.code}`, 'error')
      return { kind: 'not_found', code: result.code }
    }
    // 命中多件：交给调用方弹选择列表，选完再走 resolveProduct。
    if (result.products.length > 1) {
      return { kind: 'multiple', products: result.products }
    }
    return resolveProduct(result.products[0])
  }

  return { handleCode, resolveProduct }
}
