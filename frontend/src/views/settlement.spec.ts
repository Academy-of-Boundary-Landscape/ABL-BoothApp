import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import AdminEventSettlement from '@/views/AdminEventSettlement.vue'
import { useSettlementStore } from '@/stores/settlementStore'
import { cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

const mocks = vi.hoisted(() => ({
  apiGet: vi.fn(),
  apiPost: vi.fn(),
  apiPut: vi.fn(),
  apiDelete: vi.fn(),
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
    success: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    error: vi.fn(),
    loading: vi.fn(),
    confirm: vi.fn(),
    alert: vi.fn(),
  }),
}))

function makeReport(channels: Schemas['ChannelLine'][]): Schemas['SettlementReport'] {
  return {
    actual_total: cents(0),
    channels,
    event_date: '2026-10-01',
    event_name: '测试展会',
    generated_at: '2026-10-01T12:00:00Z',
    last_changed_at: null,
    societies: [],
    stocktaken: false,
    stocktaken_at: null,
    transfer_total: cents(0),
    vendor_retained: cents(0),
    warnings: [],
  }
}

function channelLine(overrides: Partial<Schemas['ChannelLine']> = {}): Schemas['ChannelLine'] {
  return {
    channel: '现金',
    book: cents(10000),
    actual: cents(10000),
    counted: false,
    diff: cents(0),
    ...overrides,
  }
}

function mountSettlement() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const store = useSettlementStore()
  return { pinia, store }
}

beforeEach(() => {
  mocks.apiGet.mockReset()
  mocks.apiGet.mockResolvedValue([])
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('AdminEventSettlement 收摊清点预填', () => {
  it('已清点渠道初值等于上次 actual，未清点渠道初值为空', async () => {
    const { pinia, store } = mountSettlement()
    store.report = makeReport([
      channelLine({ channel: '现金', book: cents(10000), actual: cents(9850), counted: true }),
      channelLine({ channel: '微信', book: cents(5000), actual: cents(5000), counted: false }),
    ])

    const wrapper = mount(AdminEventSettlement, {
      props: { id: 3 },
      global: {
        plugins: [pinia],
        stubs: { SettlementReportView: true, SettlementWarnings: true },
      },
    })
    await flushPromises()

    const inputs = wrapper.findAll('.n-data-table .n-input__input-el')
    expect(inputs).toHaveLength(2)
    // 已清点渠道预填上次人工实收（9850 分 = 98.50 元）。
    expect((inputs[0].element as HTMLInputElement).value).toBe('98.50')
    // 未清点渠道留空，不预填账面值。
    expect((inputs[1].element as HTMLInputElement).value).toBe('')
    wrapper.unmount()
  })
})
