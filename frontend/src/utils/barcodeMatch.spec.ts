import { describe, expect, it } from 'vitest'
import { matchBarcode } from '@/utils/barcodeMatch'

interface Product {
  product_code: string
  barcode?: string | null
}

/** 商品夹具；`barcode` 不传即 `undefined`，显式传 `null` / `''` 用于覆盖空值分支。 */
function product(product_code: string, barcode?: string | null): Product {
  return barcode === undefined ? { product_code } : { product_code, barcode }
}

describe('matchBarcode', () => {
  describe('规则 1：去首尾空格，空串未找到', () => {
    it('空串返回 not_found，code 为空串', () => {
      expect(matchBarcode('', [product('P001')])).toEqual({ kind: 'not_found', code: '' })
    })

    it('纯空白同样按空串处理', () => {
      expect(matchBarcode('   ', [product('P001')])).toEqual({ kind: 'not_found', code: '' })
    })
  })

  describe('规则 2：191/192 开头的 13 位价格码被忽略', () => {
    it('1920123456789 被忽略', () => {
      expect(matchBarcode('1920123456789', [])).toEqual({ kind: 'ignored' })
    })

    it('1910123456789 被忽略', () => {
      expect(matchBarcode('1910123456789', [])).toEqual({ kind: 'ignored' })
    })

    it('即使某商品 barcode 恰好是价格码，也优先忽略', () => {
      expect(matchBarcode('1920123456789', [product('P001', '1920123456789')])).toEqual({
        kind: 'ignored',
      })
    })

    it('9784061234567 不是价格码', () => {
      // 没有商品时是「未找到」，而不是「忽略」。
      expect(matchBarcode('9784061234567', [])).toEqual({
        kind: 'not_found',
        code: '9784061234567',
      })
      // 有对应商品时正常命中。
      const book = product('ISBN-1', '9784061234567')
      expect(matchBarcode('9784061234567', [book])).toEqual({ kind: 'hit', products: [book] })
    })
  })

  describe('规则 3：按商业条码规范化键匹配', () => {
    it('13 位条码精确命中', () => {
      const item = product('P001', '4901234567894')
      expect(matchBarcode('4901234567894', [item])).toEqual({ kind: 'hit', products: [item] })
    })

    it('扫描 12 位 UPC-A 命中存成 13 位 0 前缀的商品', () => {
      const item = product('P001', '0490123456789')
      expect(matchBarcode('490123456789', [item])).toEqual({ kind: 'hit', products: [item] })
    })

    it('扫描 13 位 0 前缀 EAN-13 命中存成 12 位 UPC-A 的商品', () => {
      const item = product('P001', '490123456789')
      expect(matchBarcode('0490123456789', [item])).toEqual({ kind: 'hit', products: [item] })
    })

    it('两件共用同一 barcode → hit 两件，保持原有顺序', () => {
      const first = product('A', '4901234567894')
      const second = product('B', '4901234567894')
      expect(matchBarcode('4901234567894', [first, second])).toEqual({
        kind: 'hit',
        products: [first, second],
      })
    })

    it('商业条码命中后不再看 product_code', () => {
      // 第一件 product_code 恰好等于扫描值；第二件 barcode 等于扫描值。只应返回第二件。
      const byCode = product('4901234567894')
      const byBarcode = product('OTHER', '4901234567894')
      expect(matchBarcode('4901234567894', [byCode, byBarcode])).toEqual({
        kind: 'hit',
        products: [byBarcode],
      })
    })

    it('商品 barcode 为 null / 空串时跳过，不参与商业条码匹配', () => {
      const empty = product('P-EMPTY', '')
      const blank = product('P-BLANK', '   ')
      const nil = product('P-NULL', null)
      expect(matchBarcode('4901234567894', [empty, blank, nil])).toEqual({
        kind: 'not_found',
        code: '4901234567894',
      })
    })

    it('非数字条码原样比较（Code128 等）', () => {
      const item = product('P001', 'ABC-123')
      expect(matchBarcode('  ABC-123  ', [item])).toEqual({ kind: 'hit', products: [item] })
    })

    it('长度非 12/13 的纯数字条码原样比较', () => {
      const item = product('P001', '12345678')
      expect(matchBarcode('12345678', [item])).toEqual({ kind: 'hit', products: [item] })
    })
  })

  describe('规则 4：按 product_code 匹配', () => {
    it('" p001 " 命中 product_code "P001"', () => {
      const target = product('P001')
      expect(matchBarcode(' p001 ', [product('OTHER'), target])).toEqual({
        kind: 'hit',
        products: [target],
      })
    })

    it('大小写与首尾空格都不敏感', () => {
      const target = product('  MixedCase  ')
      expect(matchBarcode('mixedcase', [target])).toEqual({ kind: 'hit', products: [target] })
    })

    it('商业条码都没有时回落到 product_code', () => {
      const target = product('P001', null)
      expect(matchBarcode('P001', [product('OTHER'), target])).toEqual({
        kind: 'hit',
        products: [target],
      })
    })

    it('product_code 命中只返回一件', () => {
      const target = product('K001')
      expect(matchBarcode('k001', [target])).toEqual({ kind: 'hit', products: [target] })
    })
  })

  describe('规则 5：都没有 → 未找到', () => {
    it('返回 not_found 并带上 trim 后的扫描值', () => {
      expect(matchBarcode(' nope ', [product('P001', '4901234567894')])).toEqual({
        kind: 'not_found',
        code: 'nope',
      })
    })

    it('商品列表为空也是未找到', () => {
      expect(matchBarcode('4901234567894', [])).toEqual({
        kind: 'not_found',
        code: '4901234567894',
      })
    })
  })
})
