import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import type { Ref } from 'vue'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { NDrawer } from 'naive-ui'
import CustomerView from '@/views/CustomerView.vue'
import VisionSearch from '@/components/shared/VisionSearch.vue'
import { AppModal } from '@/components/ui'
import { useCustomerStore } from '@/stores/customerStore'
import { cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

// 断点可切：测试里手动设置，模拟手机 / 平板竖屏 / 桌面三种视口。
// 必须返回真正的 ref（模板里的 ref 才会被自动解包），所以在 mock 工厂里创建。
const viewportRef = vi.hoisted(() => ({
  current: null as null | { isPhone: Ref<boolean>; isTablet: Ref<boolean> },
}))

vi.mock('@/composables/useViewport', async () => {
  const { ref } = await import('vue')
  const current = { isPhone: ref(false), isTablet: ref(true) }
  viewportRef.current = current
  return { useViewport: () => current }
})

function viewport(): { isPhone: Ref<boolean>; isTablet: Ref<boolean> } {
  if (!viewportRef.current) throw new Error('useViewport mock 未初始化')
  return viewportRef.current
}

const mocks = vi.hoisted(() => ({
  apiGet: vi.fn<(path: string, opts?: { signal?: AbortSignal }) => Promise<unknown>>(),
  apiPost: vi.fn(),
  apiPut: vi.fn(),
  apiDelete: vi.fn(),
  fbAlert: vi.fn(),
  searchByImage: vi.fn(),
  getVisionStatus: vi.fn(),
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
    alert: mocks.fbAlert,
  }),
}))

vi.mock('@/services/vision', () => ({
  searchByImage: mocks.searchByImage,
  getVisionStatus: mocks.getVisionStatus,
}))

function makeProduct(
  overrides: Partial<Schemas['ProductEventProduct']> = {}
): Schemas['ProductEventProduct'] {
  return {
    id: 1,
    event_id: 3,
    master_product_id: 11,
    name: '测试商品',
    product_code: 'P001',
    category: '徽章',
    tags: '',
    owner_society_id: 1,
    owner_society_name: '测试社团',
    stocked_qty: 100,
    onsite_qty: 10,
    unit_price: cents(1000),
    image_url: null,
    ...overrides,
  }
}

const RESULT: Schemas['VisionSearchResult'] = {
  master_product_id: 11,
  name: '测试商品',
  product_code: 'P001',
  score: 0.9,
  thumb_url: null,
}

/** jsdom 没有 ResizeObserver；VisionSearch 挂载时会用到。 */
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}

beforeEach(() => {
  localStorage.clear()
  sessionStorage.clear()
  viewport().isPhone.value = false
  viewport().isTablet.value = true
  vi.stubGlobal('ResizeObserver', ResizeObserverStub)
  mocks.apiGet.mockImplementation((path) => {
    switch (path) {
      case '/events/{event_id}/products':
        return Promise.resolve([])
      case '/events/{id}':
        return Promise.resolve({ id: 3, name: '测试展会', status: '进行中' })
      case '/server-info':
        return Promise.resolve({})
      default:
        return Promise.resolve([])
    }
  })
})

afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

describe('VisionSearch 识别结果外壳', () => {
  interface VisionSearchInternals {
    isCameraActive: boolean
    results: Schemas['VisionSearchResult'][]
  }

  async function mountWithResults(): Promise<VueWrapper> {
    const wrapper = mount(VisionSearch, {
      props: { cameraMode: true, mode: 'order', eventId: 3 },
    })
    await flushPromises()
    const vm = wrapper.vm as unknown as VisionSearchInternals
    vm.isCameraActive = true
    vm.results = [RESULT]
    await nextTick()
    return wrapper
  }

  it('isPhone 为真时识别结果渲染为底部 drawer', async () => {
    viewport().isPhone.value = true
    const wrapper = await mountWithResults()

    const drawer = wrapper.findComponent(NDrawer)
    expect(drawer.exists()).toBe(true)
    expect(drawer.props('placement')).toBe('bottom')
    expect(drawer.props('show')).toBe(true)
    expect(wrapper.findComponent(AppModal).exists()).toBe(false)

    wrapper.unmount()
  })

  it('非手机时识别结果仍用 AppModal', async () => {
    viewport().isPhone.value = false
    const wrapper = await mountWithResults()

    expect(wrapper.findComponent(AppModal).exists()).toBe(true)
    expect(wrapper.findComponent(NDrawer).exists()).toBe(false)

    wrapper.unmount()
  })
})

describe('CustomerView 平板竖屏购物车条', () => {
  function mountCustomer(id = '3'): VueWrapper {
    const pinia = createPinia()
    setActivePinia(pinia)
    const store = useCustomerStore()
    store.cart = [{ ...makeProduct(), quantity: 2 }]

    return mount(CustomerView, {
      props: { id },
      global: {
        plugins: [pinia],
        stubs: {
          ProductGrid: true,
          VisionSearch: true,
          PaymentModal: true,
          RouterLink: { template: '<a class="router-link-stub"><slot /></a>' },
        },
      },
    })
  }

  it('平板竖屏（isTablet 且非 phone）购物车收起为底部条，显示件数与合计', async () => {
    viewport().isPhone.value = false
    viewport().isTablet.value = true
    const wrapper = mountCustomer()
    await flushPromises()

    expect(wrapper.find('.shopping-cart--bar').exists()).toBe(true)
    expect(wrapper.find('.cart-sidebar').exists()).toBe(false)

    // 收起态（未展开）也要能看到件数与合计
    expect(wrapper.find('.cart-container--bar').classes()).not.toContain('is-expanded')
    expect(wrapper.find('.count-badge').text()).toBe('2')
    expect(wrapper.find('.count-unit').text()).toBe('件')
    expect(wrapper.find('.total-label').text()).toBe('合计')
    expect(wrapper.find('.total-price').text()).toBe('¥20.00')

    wrapper.unmount()
  })

  it('宽屏（非 tablet）购物车为常驻侧栏', async () => {
    viewport().isPhone.value = false
    viewport().isTablet.value = false
    const wrapper = mountCustomer()
    await flushPromises()

    expect(wrapper.find('.cart-sidebar').exists()).toBe(true)
    expect(wrapper.find('.shopping-cart--sidebar').exists()).toBe(true)
    expect(wrapper.find('.shopping-cart--bar').exists()).toBe(false)

    wrapper.unmount()
  })
})

describe('CustomerView 下单流程', () => {
  function mountCustomer(): VueWrapper {
    const pinia = createPinia()
    setActivePinia(pinia)
    const store = useCustomerStore()
    store.cart = [{ ...makeProduct(), quantity: 2 }]
    return mount(CustomerView, {
      props: { id: '3' },
      global: {
        plugins: [pinia],
        stubs: {
          ProductGrid: true,
          VisionSearch: true,
          PaymentModal: true,
          OrderConfirmPanel: true,
          RouterLink: { template: '<a class="router-link-stub"><slot /></a>' },
        },
      },
    })
  }

  it('底部条收起态直接有「去结算」，点它打开确认面板', async () => {
    viewport().isTablet.value = true
    const wrapper = mountCustomer()
    await flushPromises()
    const panel = wrapper.findComponent({ name: 'OrderConfirmPanel' })
    expect(panel.props('show')).toBe(false)
    await wrapper.find('.bar-checkout-btn').trigger('click')
    expect(panel.props('show')).toBe(true)
    wrapper.unmount()
  })

  it('确认下单后收款页拿到订单号，关闭后成功屏显示单号', async () => {
    viewport().isTablet.value = false
    mocks.apiPost.mockResolvedValue({ id: 42, final_amount: cents(2000) })
    const wrapper = mountCustomer()
    await flushPromises()

    const panel = wrapper.findComponent({ name: 'OrderConfirmPanel' })
    panel.vm.$emit('confirm')
    await flushPromises()

    const pay = wrapper.findComponent({ name: 'PaymentModal' })
    expect(pay.props('show')).toBe(true)
    expect(pay.props('orderId')).toBe(42)
    expect(panel.props('show')).toBe(false)

    pay.vm.$emit('close')
    await nextTick()
    expect(wrapper.find('.success-screen').text()).toContain('#42')
    wrapper.unmount()
  })
})

describe('CustomerView 拍照识别入口', () => {
  function mountCustomer(): VueWrapper {
    const pinia = createPinia()
    setActivePinia(pinia)
    return mount(CustomerView, {
      props: { id: '3' },
      global: {
        plugins: [pinia],
        stubs: {
          ProductGrid: true,
          VisionSearch: true,
          PaymentModal: true,
          RouterLink: { template: '<a class="router-link-stub"><slot /></a>' },
        },
      },
    })
  }

  function visionButtons(wrapper: VueWrapper) {
    return wrapper.findAll('button').filter((b) => b.text().includes('拍照识别'))
  }

  it('识别未就绪时，吸引屏和工具栏的入口都置灰', async () => {
    mocks.getVisionStatus.mockResolvedValue({ is_ready: false })
    const wrapper = mountCustomer()
    await flushPromises()
    const btns = visionButtons(wrapper)
    expect(btns.length).toBe(2)
    for (const b of btns) expect(b.attributes('disabled')).toBeDefined()
    expect(wrapper.find('.attract-screen').text()).toContain('拍照识别暂不可用')
    wrapper.unmount()
  })

  it('这个版本没编进识别（接口 404）也置灰', async () => {
    mocks.getVisionStatus.mockRejectedValue(new Error('404'))
    const wrapper = mountCustomer()
    await flushPromises()
    for (const b of visionButtons(wrapper)) expect(b.attributes('disabled')).toBeDefined()
    wrapper.unmount()
  })

  it('就绪时入口可用', async () => {
    mocks.getVisionStatus.mockResolvedValue({ is_ready: true })
    const wrapper = mountCustomer()
    await flushPromises()
    const btns = visionButtons(wrapper)
    expect(btns.length).toBe(2)
    for (const b of btns) expect(b.attributes('disabled')).toBeUndefined()
    wrapper.unmount()
  })
})
