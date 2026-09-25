import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { NCheckbox, NDataTable } from 'naive-ui'
import ImportFromEventDrawer from './ImportFromEventDrawer.vue'
import { ApiRequestError, type Schemas } from '@/api/client'

const mocks = vi.hoisted(() => ({
  apiGet: vi.fn(),
  apiPost: vi.fn(),
  fbSuccess: vi.fn(),
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
    api: { GET: mocks.apiGet, POST: mocks.apiPost },
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
    info: vi.fn(),
    warning: vi.fn(),
    error: vi.fn(),
    loading: vi.fn(),
    confirm: vi.fn(),
    alert: vi.fn(),
  }),
}))

const CURRENT_EVENT_ID = 1

function makeEvent(overrides: Partial<Schemas['EventResponse']>): Schemas['EventResponse'] {
  return {
    id: 1,
    name: '本场',
    date: '2026-10-01',
    location: null,
    status: '筹备',
    qrcode_url: null,
    qrcode_urls: [],
    ...overrides,
  }
}

function makeSourceProduct(): Schemas['ProductEventProduct'] {
  return {
    id: 11,
    event_id: 2,
    master_product_id: 101,
    name: '徽章',
    product_code: 'A1',
    category: null,
    tags: '',
    image_url: null,
    owner_society_id: 9,
    owner_society_name: '社团',
    onsite_qty: 4,
    stocked_qty: 6,
    unit_price: 1000 as Schemas['ProductEventProduct']['unit_price'],
  }
}

function makeSourceLot(): Schemas['LotResponse'] {
  return {
    id: 7,
    event_id: 2,
    name: '任选三件',
    pick_count: 3,
    allow_repeat: false,
    total_price: 3000 as Schemas['LotResponse']['total_price'],
    owner_society_id: 9,
    owner_society_name: '社团',
    candidate_ids: [11],
  }
}

function mountDrawer(): VueWrapper {
  return mount(ImportFromEventDrawer, {
    props: {
      show: true,
      eventId: CURRENT_EVENT_ID,
      targetMasterIds: new Set<number>(),
      libraryPrices: new Map([[101, null]]),
    },
    global: { plugins: [createPinia()] },
    attachTo: document.body,
  })
}

function clickButton(text: string) {
  const btn = [...document.querySelectorAll('button')].find(
    (b) => (b.textContent ?? '').trim() === text
  )
  expect(btn, `按钮「${text}」应存在`).toBeTruthy()
  btn!.click()
}

beforeEach(() => {
  mocks.apiGet.mockReset()
  mocks.apiPost.mockReset()
  mocks.fbSuccess.mockReset()
  mocks.apiGet.mockImplementation((path: string) => {
    if (path === '/events') {
      return Promise.resolve([
        makeEvent({ id: CURRENT_EVENT_ID, name: '本场', date: '2026-10-01' }),
        makeEvent({ id: 2, name: '上一场', date: '2026-09-01' }),
      ])
    }
    if (path === '/events/{event_id}/products') return Promise.resolve([makeSourceProduct()])
    if (path === '/events/{event_id}/lots') return Promise.resolve([makeSourceLot()])
    return Promise.resolve([])
  })
})

afterEach(() => {
  document.body.innerHTML = ''
})

describe('ImportFromEventDrawer', () => {
  it('默认选最近一场非本场，并加载它的商品与套装', async () => {
    const wrapper = mountDrawer()
    await flushPromises()

    // 源展会下拉默认是日期最近的「上一场」（id=2）。
    const select = wrapper.findComponent({ name: 'Select' })
    expect(select.props('value')).toBe(2)
    expect(document.body.textContent).toContain('徽章')
    wrapper.unmount()
  })

  it('import 失败时抽屉不关、错误原文可见、勾选保留（Review Focus 5）', async () => {
    mocks.apiPost.mockRejectedValue(
      new ApiRequestError(400, { error: '套装「任选三件」的候选映射不上' })
    )
    const wrapper = mountDrawer()
    await flushPromises()

    // 勾选第一张商品表的第一个数据行（第一个 checkbox 是表头全选）。
    const productTable = wrapper.findAllComponents(NDataTable)[0]
    const boxes = productTable.findAllComponents(NCheckbox)
    expect(boxes.length).toBeGreaterThanOrEqual(2)
    await boxes[1].trigger('click')
    await flushPromises()
    expect(document.body.textContent).toContain('导入 1 件商品')

    clickButton('导入')
    await flushPromises()

    // 后端错误原文显示在抽屉里，抽屉没发关闭事件，勾选仍在。
    expect(document.body.textContent).toContain('套装「任选三件」的候选映射不上')
    expect(wrapper.emitted('update:show')).toBeUndefined()
    expect(document.body.textContent).toContain('导入 1 件商品')
    const checked = document.querySelectorAll('.n-data-table-tbody .n-checkbox--checked')
    expect(checked.length).toBeGreaterThan(0)
    expect(mocks.fbSuccess).not.toHaveBeenCalled()

    wrapper.unmount()
  })

  it('导入成功 → fb.success + emit imported 并关闭抽屉', async () => {
    mocks.apiPost.mockResolvedValue({ products: [makeSourceProduct()], lots: [] })
    const wrapper = mountDrawer()
    await flushPromises()

    const productTable = wrapper.findAllComponents(NDataTable)[0]
    await productTable.findAllComponents(NCheckbox)[1].trigger('click')
    await flushPromises()

    clickButton('导入')
    await flushPromises()

    expect(mocks.apiPost).toHaveBeenCalledTimes(1)
    expect(mocks.fbSuccess).toHaveBeenCalledTimes(1)
    expect(wrapper.emitted('imported')).toHaveLength(1)
    expect(wrapper.emitted('update:show')?.[0]).toEqual([false])

    wrapper.unmount()
  })
})
