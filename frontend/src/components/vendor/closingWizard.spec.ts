import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import ClosingWizard from '@/components/vendor/ClosingWizard.vue'
import type { Schemas } from '@/api/client'

interface ApiGetOptions {
  params?: { path?: Record<string, number>; query?: { status?: string } }
}

const mocks = vi.hoisted(() => ({
  apiGet: vi.fn<(path: string, opts?: ApiGetOptions) => Promise<unknown>>(),
  apiPost: vi.fn(),
  apiPut: vi.fn(),
  apiDelete: vi.fn(),
  fbSuccess: vi.fn(),
  fbInfo: vi.fn(),
  fbWarning: vi.fn(),
  fbError: vi.fn(),
  confirm: vi.fn(),
}))

vi.mock('@/api/client', () => {
  class MockApiRequestError extends Error {
    readonly status: number
    readonly serverMessage?: string
    constructor(status: number, body: unknown, message?: string) {
      const server =
        body !== null &&
        typeof body === 'object' &&
        typeof (body as { error?: unknown }).error === 'string'
          ? (body as { error: string }).error
          : undefined
      super(server ?? message ?? `请求失败（${status}）`)
      this.name = 'ApiRequestError'
      this.status = status
      this.serverMessage = server
    }
  }
  return {
    api: { GET: mocks.apiGet, POST: mocks.apiPost, PUT: mocks.apiPut, DELETE: mocks.apiDelete },
    unwrap: (p: unknown) => p,
    errorMessage: (e: unknown, fallback: string) =>
      (e instanceof MockApiRequestError && e.serverMessage) ||
      (e instanceof Error && e.message) ||
      fallback,
    ApiRequestError: MockApiRequestError,
    isTauri: false,
  }
})

vi.mock('@/composables/useFeedback', () => ({
  useFeedback: () => ({
    success: mocks.fbSuccess,
    info: mocks.fbInfo,
    warning: mocks.fbWarning,
    error: mocks.fbError,
    loading: vi.fn(),
    confirm: mocks.confirm,
    alert: vi.fn(),
  }),
}))

function onsiteRows(n: number): Schemas['ClosingOnSiteRow'][] {
  return Array.from({ length: n }, (_, i) => ({
    event_product_id: i + 1,
    name: `商品${i + 1}`,
    owner_name: '测试社团',
    owner_society_id: 1,
    product_code: `P${i + 1}`,
    qty: 10,
  }))
}

function makeState(overrides: Partial<Schemas['ClosingState']> = {}): Schemas['ClosingState'] {
  return {
    status: '进行中',
    pending_orders: [],
    onsite_remaining: [],
    blockers: [],
    stocktaken_at: null,
    ...overrides,
  }
}

let closingState: Schemas['ClosingState']

function setupApi() {
  mocks.apiGet.mockImplementation((path) => {
    switch (path) {
      case '/events/{event_id}/closing':
        return Promise.resolve(closingState)
      case '/events/{event_id}/orders':
        return Promise.resolve([])
      case '/events/{event_id}/products':
        return Promise.resolve([])
      default:
        return Promise.resolve([])
    }
  })
}

let pinia: Pinia

function mountWizard(): VueWrapper {
  return mount(ClosingWizard, {
    props: { eventId: 3 },
    global: { plugins: [pinia], stubs: { ReceiptModal: true } },
  })
}

/** 填一行盘点：setValue 后必须 blur，NInputNumber 带 precision 只在 blur 时落 model。 */
async function fillRow(w: VueWrapper, index: number, value: string) {
  const inputs = w.findAll('[data-test="stocktake-row"] input')
  await inputs[index].setValue(value)
  await inputs[index].trigger('blur')
}

beforeEach(() => {
  sessionStorage.clear()
  localStorage.clear()
  pinia = createPinia()
  setActivePinia(pinia)
  closingState = makeState()
  setupApi()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('ClosingWizard 盘点（手机优先页面）', () => {
  it('盘点输入框初始全空，不预填账面数', async () => {
    closingState = makeState({ onsite_remaining: onsiteRows(5) })
    const w = mountWizard()
    await flushPromises()

    const inputs = w.findAll('[data-test="stocktake-row"] input')
    expect(inputs).toHaveLength(5)
    for (const input of inputs) {
      expect((input.element as HTMLInputElement).value).toBe('')
    }
  })

  it('填 2 / 5 行后粘性条显示「已盘 2 / 5」', async () => {
    closingState = makeState({ onsite_remaining: onsiteRows(5) })
    const w = mountWizard()
    await flushPromises()

    await fillRow(w, 0, '3')
    await fillRow(w, 1, '4')
    await flushPromises()

    expect(w.find('.stocktake-bar').text()).toContain('已盘 2 / 5')
  })

  it('开「只看未填」后只剩没填的 3 行', async () => {
    closingState = makeState({ onsite_remaining: onsiteRows(5) })
    const w = mountWizard()
    await flushPromises()

    await fillRow(w, 0, '3')
    await fillRow(w, 1, '4')
    await flushPromises()

    await w.find('.stocktake-filter .n-switch').trigger('click')
    await flushPromises()

    expect(w.findAll('[data-test="stocktake-row"]')).toHaveLength(3)
  })

  it('重新挂载（退出重进）时步骤取自 store 状态，而非组件内变量', async () => {
    closingState = makeState({ onsite_remaining: onsiteRows(5) })
    const first = mountWizard()
    await flushPromises()

    // 后端 stocktaken_at 为空 → 停在第 2 步盘点。
    expect(first.find('.steps').exists()).toBe(true)
    expect(first.text()).toContain('盘点现场仓')

    // 「跳过盘点」只是组件内的界面状态：把界面推到第 3 步带回。
    await first.find('.screen-actions button').trigger('click')
    await flushPromises()
    expect(first.text()).toContain('以下商品将带回')
    first.unmount()

    // 退出重进：后端仍然没盘点，必须回到第 2 步，而不是记住上次跳过。
    const second = mountWizard()
    await flushPromises()
    expect(second.text()).toContain('盘点现场仓')
    expect(second.findAll('[data-test="stocktake-row"]')).toHaveLength(5)
  })
})

describe('ClosingWizard 已结算', () => {
  it('后端状态已是「已结算」（外壳里的展会列表还是旧的）→ 发一次 settled 让页面换结算单', async () => {
    closingState = makeState({ status: '已结算' })
    const w = mountWizard()
    await flushPromises()

    expect(w.emitted('settled')).toHaveLength(1)
  })

  it('未结算时不发 settled', async () => {
    closingState = makeState({ onsite_remaining: onsiteRows(2) })
    const w = mountWizard()
    await flushPromises()

    expect(w.emitted('settled')).toBeUndefined()
  })
})

// 2026-09-26 收摊走查（场景 4：停在收摊 tab 时来新单）。
// 外壳的轮询会响铃、弹「收到新订单！」、订单 tab 出角标，但向导的 state 只在挂载时拉一次：
// 摊主照旧填完盘点点提交，后端 409「还有 1 单待处理，清完才能盘点」。
// 曾经 `closingStore.stocktake` 在 fetchState 之前就抛了，向导原地停在盘点屏，
// 要切走 tab 再切回来才看得到第 ① 步——而切 tab 会把刚填的实数全部清空（见下一组）。
describe('ClosingWizard 盘点时来了新单', () => {
  it('提交盘点被 409（有待处理订单）挡回后，向导应刷新状态并回到第 ① 步', async () => {
    const { ApiRequestError } = await import('@/api/client')
    closingState = makeState({ onsite_remaining: onsiteRows(2) })
    const w = mountWizard()
    await flushPromises()
    await fillRow(w, 0, '10')
    await fillRow(w, 1, '10')

    // 顾客端此刻下了一单：后端状态变了，提交被挡。
    closingState = makeState({
      onsite_remaining: onsiteRows(2),
      pending_orders: [
        { id: 8, created_at: '2026-09-25 16:32:00', final_amount: 4000, item_count: 1 },
      ] as Schemas['ClosingState']['pending_orders'],
      blockers: ['还有 1 单待处理，逐单完成或取消之后才能盘点'],
    })
    mocks.apiPost.mockRejectedValueOnce(
      new ApiRequestError(409, { error: '还有 1 单待处理，清完才能盘点' })
    )

    const submit = w.findAll('.screen-actions button').find((b) => b.text().includes('提交盘点'))!
    await submit.trigger('click')
    await flushPromises()

    expect(mocks.fbError).toHaveBeenCalled()
    expect(w.text()).toContain('还有 1 单待处理')
    expect(w.findAll('[data-test="stocktake-row"]')).toHaveLength(0)
  })
})

// 同一次走查：中途退出再进（或切 tab，组件会卸载），步骤不丢但已填的实数全部清空。
describe('ClosingWizard 盘点草稿', () => {
  it('卸载后重新挂载，已填的实数恢复，没填的仍为空', async () => {
    closingState = makeState({ onsite_remaining: onsiteRows(3) })
    const first = mountWizard()
    await flushPromises()
    await fillRow(first, 0, '7')
    await fillRow(first, 2, '0')
    await flushPromises()
    first.unmount()

    const second = mountWizard()
    await flushPromises()
    const values = second
      .findAll('[data-test="stocktake-row"] input')
      .map((i) => (i.element as HTMLInputElement).value)
    expect(values).toEqual(['7', '', '0'])
    expect(second.find('.stocktake-bar').text()).toContain('已盘 2 / 3')
  })

  it('草稿按展会区分，别的展会不会串进来', async () => {
    localStorage.setItem('closing-stocktake-draft:99', JSON.stringify({ 1: 5 }))
    closingState = makeState({ onsite_remaining: onsiteRows(1) })
    const w = mountWizard()
    await flushPromises()
    expect((w.find('[data-test="stocktake-row"] input').element as HTMLInputElement).value).toBe('')
  })

  it('盘点提交成功后清掉草稿', async () => {
    closingState = makeState({ onsite_remaining: onsiteRows(1) })
    const w = mountWizard()
    await flushPromises()
    await fillRow(w, 0, '10')
    await flushPromises()
    expect(localStorage.getItem('closing-stocktake-draft:3')).not.toBeNull()

    mocks.apiPost.mockResolvedValueOnce({})
    const submit = w.findAll('.screen-actions button').find((b) => b.text().includes('提交盘点'))!
    await submit.trigger('click')
    await flushPromises()

    expect(localStorage.getItem('closing-stocktake-draft:3')).toBeNull()
  })
})
