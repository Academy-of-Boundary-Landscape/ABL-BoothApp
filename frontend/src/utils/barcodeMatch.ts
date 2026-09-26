/**
 * 点单页扫码的纯匹配函数（spec §4.1）。
 *
 * 商品只用到 `product_code` 与可选的 `barcode` 两个字段，因此这里用泛型接住，
 * 不依赖后端生成的 `Schemas` 类型——`barcode` 字段的落地与调用方可以并行开发。
 * 函数无副作用、无依赖，断网也能本地匹配。
 */

export type BarcodeMatch<T> =
  | { kind: 'hit'; products: T[] } // products.length >= 1
  | { kind: 'ignored' } // 日本书籍分类价格码，界面无反应
  | { kind: 'not_found'; code: string }

/**
 * 商业条码规范化键：12 位 UPC-A 前补 `0` 对齐 13 位 EAN-13（不同识别引擎对同一张码
 * 的输出不一致）；13 位及「其它」保持原样。
 */
function barcodeKey(value: string): string {
  return /^\d{12}$/.test(value) ? `0${value}` : value
}

/**
 * 商品侧条码取键：`null` / 空串（含纯空白）返回 `null` 表示跳过；其余去首尾空格后
 * 按扫描侧同一规则规范化。存储层已规范化（spec §3.3），这里再 trim 一次是防御旧数据
 * 或手工写入的空格，不会造成误配。
 */
function productBarcodeKey(barcode: string | null | undefined): string | null {
  if (barcode == null) return null
  const trimmed = barcode.trim()
  if (trimmed === '') return null
  return barcodeKey(trimmed)
}

/**
 * 把一次扫描文本匹配到本场商品（spec §4.1 的顺序）：
 * 1. 去首尾空格，空串 → `not_found`；
 * 2. `191` / `192` 开头的 13 位日本书籍价格码 → `ignored`；
 * 3. 按商业条码规范化键比较，命中 ≥1 件就用这些，**不再看 `product_code`**；
 * 4. 否则按 `product_code`（两边 trim + 不区分大小写）命中一件；
 * 5. 都没有 → `not_found`。
 */
export function matchBarcode<T extends { product_code: string; barcode?: string | null }>(
  raw: string,
  products: readonly T[]
): BarcodeMatch<T> {
  const code = raw.trim()

  // 1. 空扫码文本没有可找的东西。
  if (code === '') return { kind: 'not_found', code: '' }

  // 2. 日本书籍封底的分类价格码直接忽略，避免每扫一本书都闪一次红。
  if (/^19[12]\d{10}$/.test(code)) return { kind: 'ignored' }

  // 3. 先按商业条码找，命中一件或多件就用这些（保持商品原有顺序）。
  const key = barcodeKey(code)
  const byBarcode = products.filter((product) => productBarcodeKey(product.barcode) === key)
  if (byBarcode.length > 0) return { kind: 'hit', products: byBarcode }

  // 4. 再按 product_code 找：本场内唯一，最多一件。
  const lower = code.toLowerCase()
  const byCode = products.find((product) => product.product_code.trim().toLowerCase() === lower)
  if (byCode !== undefined) return { kind: 'hit', products: [byCode] }

  // 5. 都没有。
  return { kind: 'not_found', code }
}
