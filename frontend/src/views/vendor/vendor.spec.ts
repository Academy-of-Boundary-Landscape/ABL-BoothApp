import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent } from 'vue'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import router from '@/router'
import { useOrderStore } from '@/stores/orderStore'
import { useEventStore } from '@/stores/eventStore'
import VendorClosing from '@/views/vendor/VendorClosing.vue'
import CustomerView from '@/views/CustomerView.vue'
import RefundModal from '@/components/vendor/RefundModal.vue'
import { cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

interface ApiGetOptions {
  params?: { path?: Record<string, number>; query?: { status?: string } }
  signal?: AbortSignal
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
  canAccessVendorPage: vi.fn(),
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

vi.mock('@/stores/authStore', () => ({
  useAuthStore: () => ({
    isAdmin: false,
    user: null,
    canAccessVendorPage: mocks.canAccessVendorPage,
    login: vi.fn(),
    logout: vi.fn(),
  }),
}))

function makeEvent(overrides: Partial<Schemas['EventResponse']> = {}): Schemas['EventResponse'] {
  return {
    id: 3,
    name: '测试展会',
    date: '2026-10-01',
    location: '上海某会展中心',
    status: '进行中',
    qrcode_url: null,
    qrcode_urls: [],
    ...overrides,
  }
}

function makeOrder(
  id: number,
  overrides: Partial<Schemas['OrderResponse']> = {}
): Schemas['OrderResponse'] {
  return {
    id,
    event_id: 3,
    gross_amount: cents(1000),
    solved_amount: cents(1000),
    final_amount: cents(1000),
    refunded_amount: cents(0),
    status: 'pending',
    timestamp: '2026-10-01T10:00:00Z',
    channel: null,
    completed_at: null,
    items: [],
    lots: [],
    ...overrides,
  }
}

function makeClosingState(): Schemas['ClosingState'] {
  return {
    status: '进行中',
    pending_orders: [],
    onsite_remaining: [],
    blockers: [],
    stocktaken_at: null,
  }
}

/** 把测试里会碰到的端点都兜住；未知路径回落空数组。 */
function setupApi(pending: () => Schemas['OrderResponse'][]) {
  mocks.apiGet.mockImplementation((path, opts) => {
    switch (path) {
      case '/events':
        return Promise.resolve([makeEvent()])
      case '/events/{event_id}/orders': {
        const status = opts?.params?.query?.status
        return Promise.resolve(status === 'completed' ? completedOrders : pending())
      }
      case '/events/{event_id}/products':
        return Promise.resolve([])
      case '/events/{event_id}/closing':
        return Promise.resolve(makeClosingState())
      case '/server-info':
        return Promise.resolve({})
      default:
        return Promise.resolve([])
    }
  })
}

let pendingOrders: Schemas['OrderResponse'][] = []
let completedOrders: Schemas['OrderResponse'][] = []

beforeEach(() => {
  sessionStorage.clear()
  localStorage.clear()
  pendingOrders = []
  completedOrders = []
  setActivePinia(createPinia())
  mocks.canAccessVendorPage.mockReturnValue(true)
  setupApi(() => pendingOrders)
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('摊主端路由', () => {
  it('/vendor/3 重定向到 vendor-orders 且保留 :id', async () => {
    await router.push('/vendor/3')
    expect(router.currentRoute.value.name).toBe('vendor-orders')
    expect(router.currentRoute.value.params.id).toBe('3')
  })

  const CHILD_ROUTES = [
    ['vendor-orders', '/vendor/3/orders'],
    ['vendor-inventory', '/vendor/3/inventory'],
    ['vendor-closing', '/vendor/3/closing'],
  ] as const

  it.each(CHILD_ROUTES)('%s 继承父路由 meta.role=vendor', (name, path) => {
    const resolved = router.resolve(path)
    expect(resolved.name).toBe(name)
    expect(resolved.meta.role).toBe('vendor')
    expect(resolved.meta.requiresAuth).toBe(true)
  })

  it('未授权访问 /vendor/3/closing 被守卫送去 login，且 redirect 是完整子路径', async () => {
    mocks.canAccessVendorPage.mockReturnValue(false)
    await router.push('/vendor/3/closing')
    expect(router.currentRoute.value.name).toBe('login')
    expect(router.currentRoute.value.query.redirect).toBe('/vendor/3/closing')
  })
})

describe('VendorShell 轮询与提示音', () => {
  const Host = defineComponent({ template: '<router-view />' })

  async function mountShell(pinia: Pinia): Promise<VueWrapper> {
    await router.push('/vendor/3/closing')
    const wrapper = mount(Host, { global: { plugins: [pinia, router] } })
    await flushPromises()
    return wrapper
  }

  it('只启动一次轮询；收摊 tab 下来新单响一次并加角标；切回订单 tab 不重复响', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    pendingOrders = [makeOrder(1)]
    const play = vi
      .spyOn(HTMLMediaElement.prototype, 'play')
      .mockImplementation(() => Promise.resolve())

    const orderStore = useOrderStore()
    const setActiveEvent = vi.spyOn(orderStore, 'setActiveEvent')

    const wrapper = await mountShell(pinia)

    // 外壳挂载只调一次 setActiveEvent（= 只启动一次轮询）；首屏已有 1 单不响铃。
    expect(setActiveEvent).toHaveBeenCalledTimes(1)
    expect(play).not.toHaveBeenCalled()
    expect(wrapper.find('.vendor-tab__badge').text()).toBe('1')

    // 模拟下一次轮询多回来一条待处理单：响一次、角标 +1。
    pendingOrders = [makeOrder(1), makeOrder(2)]
    await orderStore.pollPendingOrders()
    await flushPromises()

    expect(play).toHaveBeenCalledTimes(1)
    expect(wrapper.find('.vendor-tab__badge').text()).toBe('2')

    // 切到订单 tab：外壳没重建，不重新启动轮询、也不再响。
    await router.push('/vendor/3/orders')
    await flushPromises()

    expect(play).toHaveBeenCalledTimes(1)
    expect(setActiveEvent).toHaveBeenCalledTimes(1)

    wrapper.unmount()
  })

  it('首次轮询 0 条 → 下一次 1 条：第一张新单提示音响恰好一次', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    pendingOrders = []
    const play = vi
      .spyOn(HTMLMediaElement.prototype, 'play')
      .mockImplementation(() => Promise.resolve())

    const orderStore = useOrderStore()
    const wrapper = await mountShell(pinia)

    expect(play).not.toHaveBeenCalled()

    // 下一次轮询回来一张真正的新单：0 → 1，必须响。
    pendingOrders = [makeOrder(1)]
    await orderStore.pollPendingOrders()
    await flushPromises()

    expect(play).toHaveBeenCalledTimes(1)
    wrapper.unmount()
  })

  it('首次轮询已有 2 条：不响（初始化不算新单）', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    pendingOrders = [makeOrder(1), makeOrder(2)]
    const play = vi
      .spyOn(HTMLMediaElement.prototype, 'play')
      .mockImplementation(() => Promise.resolve())

    const wrapper = await mountShell(pinia)
    await flushPromises()

    expect(play).not.toHaveBeenCalled()
    wrapper.unmount()
  })
})

describe('VendorShell 营业额', () => {
  const Host = defineComponent({ template: '<router-view />' })

  it('外壳挂载后首次渲染营业额即为非零（不用等下一次轮询）', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    // 外壳 onMounted 之前 activeEventId 必须先钉上；否则 LiveStats 首次
    // fetchCompletedOrders 会因 activeEventId 为空直接 return，营业额显示 ¥0。
    completedOrders = [
      makeOrder(1, { status: 'completed', final_amount: cents(1234), refunded_amount: cents(0) }),
    ]

    await router.push('/vendor/3/orders')
    const wrapper = mount(Host, { global: { plugins: [pinia, router] } })
    await flushPromises()

    expect(wrapper.find('.live-stats__tiles').text()).toContain('¥12.34')
    wrapper.unmount()
  })

  it('营业额 = Σ(实收 − 已退)', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const orderStore = useOrderStore()
    orderStore.completedOrders = [
      makeOrder(1, { status: 'completed', final_amount: cents(1000), refunded_amount: cents(300) }),
      makeOrder(2, { status: 'completed', final_amount: cents(500), refunded_amount: cents(0) }),
    ]

    expect(orderStore.totalRevenue).toBe(1200)
  })
})

describe('VendorOrders 退货后刷新', () => {
  const Host = defineComponent({ template: '<router-view />' })

  it('关闭退货弹窗后重拉已完成单（「已全部退货」置灰与扣退款营业额才会更新）', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    completedOrders = [makeOrder(1, { status: 'completed' })]

    await router.push('/vendor/3/orders')
    const wrapper = mount(Host, { global: { plugins: [pinia, router] } })
    await flushPromises()

    const completedCalls = () =>
      mocks.apiGet.mock.calls.filter(
        ([path, opts]) =>
          path === '/events/{event_id}/orders' && opts?.params?.query?.status === 'completed'
      ).length
    const before = completedCalls()

    wrapper.findComponent(RefundModal).vm.$emit('close')
    await flushPromises()

    expect(completedCalls()).toBe(before + 1)
    wrapper.unmount()
  })
})

describe('CustomerView 回摊主端', () => {
  function mountCustomer(): VueWrapper {
    return mount(CustomerView, {
      props: { id: '3' },
      shallow: true,
      global: {
        stubs: { RouterLink: { template: '<a class="router-link-stub"><slot /></a>' } },
      },
    })
  }

  it('canAccessVendorPage 为真时渲染「回摊主端」', () => {
    mocks.canAccessVendorPage.mockReturnValue(true)
    const wrapper = mountCustomer()
    expect(wrapper.find('.vendor-return').exists()).toBe(true)
  })

  it('canAccessVendorPage 为假时不渲染「回摊主端」', () => {
    mocks.canAccessVendorPage.mockReturnValue(false)
    const wrapper = mountCustomer()
    expect(wrapper.find('.vendor-return').exists()).toBe(false)
  })
})

describe('VendorClosing', () => {
  const stubs = {
    ClosingWizard: { template: '<div class="closing-wizard-stub" />' },
    SettlementReportView: { template: '<div class="settlement-report-stub" />' },
  }

  it('展会已结算时渲染 SettlementReportView 而非 ClosingWizard', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const eventStore = useEventStore()
    eventStore.events = [makeEvent({ id: 3, status: '已结算' })]

    const wrapper = mount(VendorClosing, {
      props: { id: '3' },
      global: { plugins: [pinia], stubs },
    })

    expect(wrapper.find('.settlement-report-stub').exists()).toBe(true)
    expect(wrapper.find('.closing-wizard-stub').exists()).toBe(false)
  })

  it('展会未结算时渲染收摊向导', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const eventStore = useEventStore()
    eventStore.events = [makeEvent({ id: 3, status: '进行中' })]

    const wrapper = mount(VendorClosing, {
      props: { id: '3' },
      global: { plugins: [pinia], stubs },
    })

    expect(wrapper.find('.closing-wizard-stub').exists()).toBe(true)
    expect(wrapper.find('.settlement-report-stub').exists()).toBe(false)
  })
})
