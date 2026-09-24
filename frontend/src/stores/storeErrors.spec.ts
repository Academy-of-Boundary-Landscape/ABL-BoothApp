// @vitest-environment node
/**
 * store 的写操作失败时抛出的 Error.message 必须与 ③b 之前逐字一致。
 *
 * 旧 store 一律 `throw new Error(err.response?.data?.error || '<中文兜底>')`，组件读 `err.message`。
 * 所以网络错误、纯文本错误体时，用户看到的是这些中文兜底——不能变成「网络错误」「请求失败（422）」。
 * 这张表来自 ③b 之前的 JS store 里的每一处 throw。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

let respond: () => Promise<Response> = () => Promise.reject(new TypeError('Failed to fetch'))

vi.mock('@/api/client', async () => {
  const core = await import('@/api/core')
  const api = core.createApiClient({
    baseUrl: 'http://x/api',
    fetch: () => respond(),
    timeoutMs: 30_000,
    getToken: () => null,
    currentPath: () => '/admin',
    onUnauthorized: () => {},
    onUploadError: () => {},
  })
  return { ...core, api }
})
vi.mock('@/router', () => ({ default: { push: vi.fn(), currentRoute: { value: { path: '/' } } } }))
vi.mock('naive-ui', () => ({
  createDiscreteApi: () => ({ dialog: { warning: vi.fn() }, message: { error: vi.fn() } }),
}))
vi.mock('@tauri-apps/plugin-http', () => ({ fetch: vi.fn() }))

const network = () => Promise.reject(new TypeError('Failed to fetch'))
const plain422 = () =>
  Promise.resolve(new Response('Failed to deserialize the JSON body', { status: 422 }))
const json409 = () =>
  Promise.resolve(
    new Response(JSON.stringify({ error: '展会已结算，不能再改账' }), {
      status: 409,
      headers: { 'content-type': 'application/json' },
    })
  )

type Case = [
  store: string,
  action: string,
  call: (s: Record<string, never>) => Promise<unknown>,
  fallback: string,
]
const fd = () => new FormData()
/* eslint-disable @typescript-eslint/no-unsafe-function-type */
const cases: Case[] = [
  ['closingStore', 'fetchState', (s) => (s.fetchState as Function)(1), '无法加载收摊状态。'],
  ['closingStore', 'stocktake', (s) => (s.stocktake as Function)(1, []), '提交盘点失败。'],
  ['closingStore', 'takeback', (s) => (s.takeback as Function)(1), '确认带回失败。'],
  ['closingStore', 'settle', (s) => (s.settle as Function)(1), '结束展会失败。'],
  [
    'eventDetailStore',
    'addProductToEvent',
    (s) => (s.addProductToEvent as Function)(1, {}),
    '上架商品失败。',
  ],
  [
    'eventDetailStore',
    'updateEventProduct',
    (s) => (s.updateEventProduct as Function)(1, {}),
    '更新商品失败。',
  ],
  [
    'eventDetailStore',
    'restockEventProduct',
    (s) => (s.restockEventProduct as Function)(1, 1, 1),
    '补货失败。',
  ],
  [
    'eventDetailStore',
    'deleteEventProduct',
    (s) => (s.deleteEventProduct as Function)(1),
    '下架商品失败。',
  ],
  [
    'eventDetailStore',
    'adminUpdateOrderStatus',
    (s) => (s.adminUpdateOrderStatus as Function)(1, 1, 'completed', '现金'),
    '更新订单状态失败。',
  ],
  ['eventStore', 'createEvent', (s) => (s.createEvent as Function)(fd()), '创建展会失败，请重试。'],
  [
    'eventStore',
    'updateEventStatus',
    (s) => (s.updateEventStatus as Function)(1, '进行中'),
    '更新状态失败，请重试。',
  ],
  ['eventStore', 'updateEvent', (s) => (s.updateEvent as Function)(1, fd()), '更新展会信息失败。'],
  ['eventStore', 'deleteEvent', (s) => (s.deleteEvent as Function)(1), '删除展会失败，请重试。'],
  [
    'inventoryLogStore',
    'logGift',
    (s) => (s.logGift as Function)(1, { event_product_id: 1, qty: 1 }),
    '登记赠送失败。',
  ],
  [
    'inventoryLogStore',
    'logScrap',
    (s) => (s.logScrap as Function)(1, { event_product_id: 1, qty: 1 }),
    '登记报废失败。',
  ],
  ['inventoryLogStore', 'reverse', (s) => (s.reverse as Function)(1, 1), '撤销失败。'],
  ['lotStore', 'createLot', (s) => (s.createLot as Function)(1, {}), '新建套装失败。'],
  ['lotStore', 'updateLot', (s) => (s.updateLot as Function)(1, 1, {}), '更新套装失败。'],
  ['lotStore', 'deleteLot', (s) => (s.deleteLot as Function)(1, 1), '删除套装失败。'],
  ['lotStore', 'previewLot', (s) => (s.previewLot as Function)(1, {}), '试算失败。'],
  [
    'productStore',
    'createMasterProduct',
    (s) => (s.createMasterProduct as Function)(fd()),
    '创建商品失败，请检查输入。',
  ],
  [
    'productStore',
    'updateMasterProduct',
    (s) => (s.updateMasterProduct as Function)(1, fd()),
    '更新商品失败，请重试。',
  ],
  [
    'productStore',
    'toggleProductStatus',
    (s) => (s.toggleProductStatus as Function)({ id: 1, is_active: true }),
    '更新商品状态失败。',
  ],
  [
    'settlementStore',
    'createAdvance',
    (s) => (s.createAdvance as Function)(1, {}),
    '新增垫付失败。',
  ],
  [
    'settlementStore',
    'deleteAdvance',
    (s) => (s.deleteAdvance as Function)(1, 1),
    '删除垫付失败。',
  ],
  [
    'settlementStore',
    'createAdjustment',
    (s) => (s.createAdjustment as Function)(1, {}),
    '新增结算调整失败。',
  ],
  [
    'settlementStore',
    'deleteAdjustment',
    (s) => (s.deleteAdjustment as Function)(1, 1),
    '删除结算调整失败。',
  ],
  ['settlementStore', 'reconcile', (s) => (s.reconcile as Function)(1, []), '提交清点失败。'],
  ['societyStore', 'createSociety', (s) => (s.createSociety as Function)('x'), '新建社团失败。'],
  ['societyStore', 'updateSociety', (s) => (s.updateSociety as Function)(1, {}), '更新社团失败。'],
  ['societyStore', 'deleteSociety', (s) => (s.deleteSociety as Function)(1), '删除社团失败。'],
]
/* eslint-enable @typescript-eslint/no-unsafe-function-type */

async function load(store: string) {
  const mod = (await import(`./${store}.ts`)) as Record<string, () => Record<string, never>>
  const use = Object.values(mod).find((v) => typeof v === 'function' && v.name.startsWith('use'))!
  return use()
}

beforeEach(() => {
  setActivePinia(createPinia())
})

describe('store 失败时的文案与 ③b 之前一致', () => {
  for (const [store, action, call, fallback] of cases) {
    it(`${store}.${action}：网络错误 → 旧兜底`, async () => {
      respond = network
      await expect(call(await load(store))).rejects.toThrow(new Error(fallback))
    })
    it(`${store}.${action}：纯文本错误体 → 旧兜底`, async () => {
      respond = plain422
      await expect(call(await load(store))).rejects.toThrow(new Error(fallback))
    })
    it(`${store}.${action}：后端给了话 → 后端原文`, async () => {
      respond = json409
      await expect(call(await load(store))).rejects.toThrow(new Error('展会已结算，不能再改账'))
    })
  }

  it('orderStore.cancelOrder：固定文案，不透出后端原文', async () => {
    respond = json409
    const s = (await load('orderStore')) as Record<string, unknown>
    ;(s as { activeEventId: number }).activeEventId = 1
    await expect((s.cancelOrder as (id: number) => Promise<void>)(1)).rejects.toThrow(
      new Error('取消订单失败。')
    )
  })

  it('orderStore.markOrderAsCompleted：网络错误 → 旧兜底', async () => {
    respond = network
    const s = (await load('orderStore')) as Record<string, unknown>
    ;(s as { activeEventId: number }).activeEventId = 1
    await expect(
      (s.markOrderAsCompleted as (id: number, ch: string) => Promise<void>)(1, '现金')
    ).rejects.toThrow(new Error('更新订单状态失败。'))
  })

  it('customerStore.submitOrder：网络错误 → 旧兜底', async () => {
    respond = network
    const s = (await load('customerStore')) as Record<string, unknown>
    ;(s as { activeEventId: number }).activeEventId = 1
    ;(s as { cart: unknown[] }).cart = [{ id: 1, quantity: 1, price: 100 }]
    await expect((s.submitOrder as () => Promise<void>)()).rejects.toThrow(
      new Error('下单失败，请重试。')
    )
  })

  // syncStore 的旧规则不同：后端原文 → err.message → 兜底。API 错误以外的异常保留它自己的 message。
  it('syncStore.importProducts：网络错误 → 旧兜底', async () => {
    respond = network
    const s = (await load('syncStore')) as Record<string, unknown>
    const file = new File([new Uint8Array([1, 2, 3])], 'a.boothpack')
    await expect((s.importProducts as (f: File) => Promise<void>)(file)).rejects.toThrow(
      new Error('导入失败，请检查文件格式或稍后重试')
    )
  })

  it('syncStore.importProducts：后端给了话 → 后端原文', async () => {
    respond = json409
    const s = (await load('syncStore')) as Record<string, unknown>
    const file = new File([new Uint8Array([1, 2, 3])], 'a.boothpack')
    await expect((s.importProducts as (f: File) => Promise<void>)(file)).rejects.toThrow(
      new Error('展会已结算，不能再改账')
    )
  })

  it('syncStore.exportProducts：非 API 异常保留自己的 message', async () => {
    respond = () =>
      Promise.resolve(
        new Response(new Uint8Array([1]), {
          status: 200,
          headers: { 'content-type': 'application/zip' },
        })
      )
    const s = (await load('syncStore')) as Record<string, unknown>
    // node 环境没有 window，走到「当前环境不支持下载/保存」
    await expect((s.exportProducts as () => Promise<void>)()).rejects.toThrow(
      new Error('当前环境不支持下载/保存')
    )
  })
})
