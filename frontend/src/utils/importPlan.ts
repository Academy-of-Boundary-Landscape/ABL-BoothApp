/**
 * 展前导入的纯计算层（spec §4.2）。
 *
 * 两个抽屉（从商品库选 / 从上一场导入）都不在组件里算「哪些能选、价格用哪个、套装能不能导入」，
 * 一律调这里的函数，UI 只渲染返回值——这样这套最容易被真实数据咬到的规则（价格空 / 套装候选没勾 /
 * 候选已在目标展会）能被单测逐条钉住，而不是靠手点。
 *
 * 金额一律是分（Cents）；只有商品库 `default_price` 是元，在页面里经 `toCents` 换算后传进来。
 */
import type { Schemas } from '@/api/client'
import { cents, toCents, type Cents } from '@/utils/money'

/** 源展会里的一个上架商品，外加商品库现价（可能为空）。 */
export interface SourceProduct {
  eventProductId: number
  masterProductId: number
  name: string
  /** 源展会的售价（分）。 */
  lastPrice: Cents
  /** 源展会累计进货量，用于「上一场进货 N」旁注。 */
  stockedQty: number
  /** 商品库 default_price（元）换算成分；default_price 为空时为 null。 */
  libraryPrice: Cents | null
}

/** 源展会里的一个套装；候选是源展会的 event_product id。 */
export interface SourceLot {
  lotId: number
  name: string
  candidateEventProductIds: number[]
}

/** 勾选态：选中的源商品、选中的源套装、每个商品的本次售价来源。 */
export interface PlanState {
  pickedProducts: Set<number>
  pickedLots: Set<number>
  priceChoice: Map<number, 'last' | 'library'>
}

export type ImportRequest = Schemas['ProductImportRequest']

/** 从商品库选：商品库里的一个商品在待上架表里的形状。 */
export interface LibraryProduct {
  masterProductId: number
  name: string
  productCode: string
}

/** 待上架表里的一行：商品 + 用户填的售价（元）/ 库存（未填为 null）。 */
export interface LibraryPickItem extends LibraryProduct {
  imageUrl: string | null
  price: number | null
  stock: number | null
  libraryPrice: Cents | null
}

export function emptyPlan(): PlanState {
  return { pickedProducts: new Set(), pickedLots: new Set(), priceChoice: new Map() }
}

/**
 * 「本次售价」该不该给二选一。
 *
 * 商品库现价为空（default_price 为 null），或与上一场售价相同 → 没什么好选的，直接用上一场售价。
 * 这条同时挡掉了「default_price 是 null 时显示 NaN / ¥0.00」（Review Focus 4）。
 */
export function priceOptions(p: SourceProduct): { showChoice: boolean; defaultPrice: Cents } {
  if (p.libraryPrice === null || p.libraryPrice === p.lastPrice) {
    return { showChoice: false, defaultPrice: p.lastPrice }
  }
  return { showChoice: true, defaultPrice: p.lastPrice }
}

/** 某个源商品当前生效的本次售价。 */
function effectivePrice(p: SourceProduct, st: PlanState): Cents {
  const choice = st.priceChoice.get(p.eventProductId) ?? 'last'
  if (choice === 'library' && p.libraryPrice !== null) return p.libraryPrice
  return p.lastPrice
}

/**
 * 套装当前能不能导入。
 *
 * 候选的 master 既不在本场（`targetMasterIds`）、也没被本次勾选 → 记进 `missing`，
 * 让 UI 能写明「需要先勾选 XX」。
 */
export function lotAvailability(
  lot: SourceLot,
  st: PlanState,
  byEp: Map<number, SourceProduct>,
  targetMasterIds: Set<number>
): { ok: true } | { ok: false; missing: string[] } {
  const missing: string[] = []
  for (const epId of lot.candidateEventProductIds) {
    const p = byEp.get(epId)
    if (!p) {
      // 源套装的候选不在源商品列表里（数据不一致）：没有名字可展示，用 id 兜底。
      missing.push(`#${epId}`)
      continue
    }
    if (targetMasterIds.has(p.masterProductId)) continue
    if (st.pickedProducts.has(epId)) continue
    missing.push(p.name)
  }
  return missing.length ? { ok: false, missing } : { ok: true }
}

function cloneState(st: PlanState, overrides: Partial<PlanState> = {}): PlanState {
  return {
    pickedProducts: overrides.pickedProducts ?? new Set(st.pickedProducts),
    pickedLots: overrides.pickedLots ?? new Set(st.pickedLots),
    priceChoice: overrides.priceChoice ?? new Map(st.priceChoice),
  }
}

/**
 * 勾选 / 取消一个套装。
 *
 * 勾选时自动勾上它「不在本场」的候选商品（已在市场上的候选不用重复勾）；
 * 取消套装只取消套装本身，不连坐它带进来的商品——商品可能被别的套装或用户自己要着。
 */
export function toggleLot(
  lot: SourceLot,
  st: PlanState,
  targetMasterIds: Set<number>,
  byEp: Map<number, SourceProduct>
): PlanState {
  const pickedLots = new Set(st.pickedLots)
  if (pickedLots.has(lot.lotId)) {
    pickedLots.delete(lot.lotId)
    return cloneState(st, { pickedLots })
  }

  pickedLots.add(lot.lotId)
  const pickedProducts = new Set(st.pickedProducts)
  for (const epId of lot.candidateEventProductIds) {
    const p = byEp.get(epId)
    if (p && !targetMasterIds.has(p.masterProductId)) pickedProducts.add(epId)
  }
  return cloneState(st, { pickedLots, pickedProducts })
}

/**
 * 勾选 / 取消一个源商品。
 *
 * 取消商品后，依赖它、且因此不再可用的已选套装自动取消（spec §4.2 的联动）。
 */
export function toggleProduct(
  epId: number,
  st: PlanState,
  lots: SourceLot[],
  targetMasterIds: Set<number>,
  byEp: Map<number, SourceProduct>
): PlanState {
  const pickedProducts = new Set(st.pickedProducts)
  if (pickedProducts.has(epId)) {
    pickedProducts.delete(epId)
  } else {
    pickedProducts.add(epId)
  }

  const pickedLots = new Set(st.pickedLots)
  if (!pickedProducts.has(epId)) {
    const next = cloneState(st, { pickedProducts, pickedLots })
    for (const lot of lots) {
      if (!pickedLots.has(lot.lotId)) continue
      if (!lotAvailability(lot, next, byEp, targetMasterIds).ok) pickedLots.delete(lot.lotId)
    }
  }
  return cloneState(st, { pickedProducts, pickedLots })
}

/** 商品库现价缺省时，建请求用的兜底价（分）。 */
function libraryFallback(p: LibraryProduct & { libraryPrice?: Cents | null }): Cents {
  return p.libraryPrice ?? cents(0)
}

/**
 * 从上一场导入：把勾选态编译成 `POST /events/{id}/products/import` 的请求体。
 *
 * - 库存没填（null / undefined）→ 0（「收全量」：不预填就不替用户猜）。
 * - 售价按 priceChoice：选「用现价」且现价存在才用现价，否则回落到上一场售价。
 */
export function buildImportRequest(
  st: PlanState,
  stock: Map<number, number | null>,
  byEp: Map<number, SourceProduct>
): ImportRequest {
  const products: NonNullable<ImportRequest['products']> = []
  for (const epId of [...st.pickedProducts].sort((a, b) => a - b)) {
    const p = byEp.get(epId)
    if (!p) continue
    const qty = stock.get(epId)
    products.push({
      master_product_id: p.masterProductId,
      unit_price: effectivePrice(p, st),
      initial_stock: qty === null || qty === undefined ? 0 : Math.max(0, Math.trunc(qty)),
    })
  }

  const lots: NonNullable<ImportRequest['lots']> = [...st.pickedLots]
    .sort((a, b) => a - b)
    .map((lotId) => ({ source_lot_id: lotId }))

  return { products, lots }
}

/**
 * 从商品库选：勾选的商品 + 每行售价（元，null 用商品库现价）/ 库存（null → 0）→ 请求体。
 * 与 `buildImportRequest` 共用同一端点，`lots` 为空。
 */
export function buildLibraryImportRequest(
  picked: Set<number>,
  priceYuan: Map<number, number | null>,
  stock: Map<number, number | null>,
  byMaster: Map<number, LibraryProduct & { libraryPrice?: Cents | null }>
): ImportRequest {
  const products: NonNullable<ImportRequest['products']> = []
  for (const masterId of [...picked].sort((a, b) => a - b)) {
    const p = byMaster.get(masterId)
    if (!p) continue
    const yuan = priceYuan.get(masterId)
    const qty = stock.get(masterId)
    products.push({
      master_product_id: masterId,
      unit_price: yuan === null || yuan === undefined ? libraryFallback(p) : toCents(yuan),
      initial_stock: qty === null || qty === undefined ? 0 : Math.max(0, Math.trunc(qty)),
    })
  }
  return { products, lots: [] }
}
