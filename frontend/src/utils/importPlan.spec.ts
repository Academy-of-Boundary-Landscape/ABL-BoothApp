import { describe, expect, it } from 'vitest'
import { cents } from '@/utils/money'
import {
  buildImportRequest,
  buildLibraryImportRequest,
  emptyPlan,
  lotAvailability,
  priceOptions,
  toggleLot,
  toggleProduct,
  type SourceLot,
  type SourceProduct,
} from './importPlan'

function prod(overrides: Partial<SourceProduct> = {}): SourceProduct {
  return {
    eventProductId: 1,
    masterProductId: 101,
    name: '徽章',
    lastPrice: cents(1000),
    stockedQty: 5,
    libraryPrice: cents(1000),
    ...overrides,
  }
}

function lot(overrides: Partial<SourceLot> = {}): SourceLot {
  return { lotId: 7, name: '任选三件', candidateEventProductIds: [1, 2], ...overrides }
}

describe('priceOptions', () => {
  it('商品库现价为空 → 不显示二选一，用上一场售价（Review Focus 4）', () => {
    const opts = priceOptions(prod({ lastPrice: cents(1990), libraryPrice: null }))
    expect(opts.showChoice).toBe(false)
    expect(opts.defaultPrice).toBe(1990)
    expect(opts.defaultPrice).not.toBe(0)
    expect(Number.isNaN(Number(opts.defaultPrice))).toBe(false)
  })

  it('商品库现价与上一场相同 → 不显示二选一', () => {
    const opts = priceOptions(prod({ lastPrice: cents(2500), libraryPrice: cents(2500) }))
    expect(opts.showChoice).toBe(false)
    expect(opts.defaultPrice).toBe(2500)
  })

  it('商品库现价不同 → 显示二选一，默认仍是上一场售价', () => {
    const opts = priceOptions(prod({ lastPrice: cents(1000), libraryPrice: cents(1200) }))
    expect(opts.showChoice).toBe(true)
    expect(opts.defaultPrice).toBe(1000)
  })
})

describe('lotAvailability / toggleLot', () => {
  const byEp = new Map<number, SourceProduct>([
    [1, prod({ eventProductId: 1, masterProductId: 101, name: '徽章' })],
    [2, prod({ eventProductId: 2, masterProductId: 102, name: '贴纸' })],
  ])

  it('候选既不在本场也没勾选 → missing 列出商品名', () => {
    const res = lotAvailability(lot(), emptyPlan(), byEp, new Set())
    expect(res.ok).toBe(false)
    if (!res.ok) expect(res.missing).toEqual(['徽章', '贴纸'])
  })

  it('勾选套装时自动勾上它不在本场的候选商品', () => {
    const next = toggleLot(lot(), emptyPlan(), new Set(), byEp)
    expect([...next.pickedLots]).toEqual([7])
    expect([...next.pickedProducts].sort()).toEqual([1, 2])
    expect(lotAvailability(lot(), next, byEp, new Set()).ok).toBe(true)
  })

  it('候选的 master 已在本场 → 不需勾选也可用，且不会被自动勾上', () => {
    const target = new Set([101, 102])
    const res = lotAvailability(lot(), emptyPlan(), byEp, target)
    expect(res.ok).toBe(true)
    const next = toggleLot(lot(), emptyPlan(), target, byEp)
    expect(next.pickedLots.has(7)).toBe(true)
    expect(next.pickedProducts.size).toBe(0)
  })

  it('取消套装只取消套装，不连坐它带进来的商品', () => {
    const picked = toggleLot(lot(), emptyPlan(), new Set(), byEp)
    const off = toggleLot(lot(), picked, new Set(), byEp)
    expect(off.pickedLots.size).toBe(0)
    expect([...off.pickedProducts].sort()).toEqual([1, 2])
  })
})

describe('toggleProduct', () => {
  const byEp = new Map<number, SourceProduct>([
    [1, prod({ eventProductId: 1, masterProductId: 101, name: '徽章' })],
    [2, prod({ eventProductId: 2, masterProductId: 102, name: '贴纸' })],
  ])
  const lots = [lot()]

  it('取消被套装依赖的候选 → 套装自动取消', () => {
    const picked = toggleLot(lot(), emptyPlan(), new Set(), byEp)
    const after = toggleProduct(1, picked, lots, new Set(), byEp)
    expect(after.pickedProducts.has(1)).toBe(false)
    expect(after.pickedLots.has(7)).toBe(false)
    // 另一个候选还在，套装仍不可用
    expect(lotAvailability(lot(), after, byEp, new Set()).ok).toBe(false)
  })

  it('取消候选但套装候选已在本场 → 套装保留', () => {
    const picked = { ...toggleLot(lot(), emptyPlan(), new Set(), byEp) }
    const after = toggleProduct(1, picked, lots, new Set([101, 102]), byEp)
    expect(after.pickedLots.has(7)).toBe(true)
  })
})

describe('buildImportRequest', () => {
  const byEp = new Map<number, SourceProduct>([
    [
      1,
      prod({
        eventProductId: 1,
        masterProductId: 101,
        lastPrice: cents(1000),
        libraryPrice: cents(1500),
      }),
    ],
    [
      2,
      prod({
        eventProductId: 2,
        masterProductId: 102,
        lastPrice: cents(2000),
        libraryPrice: cents(2000),
      }),
    ],
  ])

  it('库存没填 → 0，售价按选择（默认上一场 / 选现价用现价）', () => {
    const st = emptyPlan()
    st.pickedProducts = new Set([1, 2])
    st.pickedLots = new Set([7, 8])
    st.priceChoice = new Map<number, 'last' | 'library'>([[1, 'library']])

    const req = buildImportRequest(st, new Map(), byEp)
    expect(req.products).toEqual([
      { master_product_id: 101, unit_price: 1500, initial_stock: 0 },
      { master_product_id: 102, unit_price: 2000, initial_stock: 0 },
    ])
    expect(req.lots).toEqual([{ source_lot_id: 7 }, { source_lot_id: 8 }])
  })

  it('库存填了正整数 → 原样带入；null 行按 0', () => {
    const st = emptyPlan()
    st.pickedProducts = new Set([1, 2])
    const req = buildImportRequest(
      st,
      new Map([
        [1, 12],
        [2, null],
      ]),
      byEp
    )
    expect(req.products).toEqual([
      { master_product_id: 101, unit_price: 1000, initial_stock: 12 },
      { master_product_id: 102, unit_price: 2000, initial_stock: 0 },
    ])
  })

  it('选了「用现价」但现价为空 → 回落到上一场售价', () => {
    const emptyLib = new Map<number, SourceProduct>([
      [
        1,
        prod({
          eventProductId: 1,
          masterProductId: 101,
          lastPrice: cents(1000),
          libraryPrice: null,
        }),
      ],
    ])
    const st = emptyPlan()
    st.pickedProducts = new Set([1])
    st.priceChoice = new Map<number, 'last' | 'library'>([[1, 'library']])
    const req = buildImportRequest(st, new Map(), emptyLib)
    expect(req.products).toEqual([{ master_product_id: 101, unit_price: 1000, initial_stock: 0 }])
  })
})

describe('buildLibraryImportRequest', () => {
  const byMaster = new Map([
    [11, { masterProductId: 11, name: '徽章', productCode: 'A1', libraryPrice: cents(1200) }],
    [12, { masterProductId: 12, name: '贴纸', productCode: 'A2', libraryPrice: null }],
  ])

  it('售价留空用商品库现价；填了元则换算成分；库存空 → 0', () => {
    const req = buildLibraryImportRequest(
      new Set([11, 12]),
      new Map([
        [11, null],
        [12, 3.5],
      ]),
      new Map([[11, 8]]),
      byMaster
    )
    expect(req).toEqual({
      products: [
        { master_product_id: 11, unit_price: 1200, initial_stock: 8 },
        { master_product_id: 12, unit_price: 350, initial_stock: 0 },
      ],
      lots: [],
    })
  })
})
