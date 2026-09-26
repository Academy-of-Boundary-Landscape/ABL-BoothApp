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
  | { kind: 'not_found'; code: string }
  | { kind: 'multiple'; products: ScanProduct[] }
  | { kind: 'ignored' }

export interface ScanResultHandlerOptions {
  /** 本场商品快照（每次扫描时取，避免拿到过期的售罄状态）。 */
  products: () => readonly ScanProduct[]
  /** 命中且有货：加入购物车（或由面板 emit 给父组件）。 */
  addToCart: (product: ScanProduct) => void
  /** 提示：kind 为 error 时是失败文案。 */
  notify: (message: string, kind: 'success' | 'error') => void
}

export function useScanResultHandler(opts: ScanResultHandlerOptions): {
  handleCode(code: string): ScanOutcome
  resolveProduct(product: ScanProduct): ScanOutcome
} {
  function resolveProduct(product: ScanProduct): ScanOutcome {
    if (product.onsite_qty <= 0) {
      opts.notify(`${product.name} 已售罄`, 'error')
      return { kind: 'sold_out', product }
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
