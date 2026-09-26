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

  function buttonsWithText(wrapper: VueWrapper, text: string) {
    return wrapper.findAll('button').filter((b) => b.text().includes(text))
  }

  function visionButtons(wrapper: VueWrapper) {
    return buttonsWithText(wrapper, '拍照识别')
  }

  function scanButtons(wrapper: VueWrapper) {
    return buttonsWithText(wrapper, '扫码')
  }

  it('识别未就绪时，吸引屏链接和工具栏的拍照识别入口都置灰', async () => {
    mocks.getVisionStatus.mockResolvedValue({ is_ready: false })
    const wrapper = mountCustomer()
    await flushPromises()
    const btns = visionButtons(wrapper)
    expect(btns.length).toBe(2)
    for (const b of btns) expect(b.attributes('disabled')).toBeDefined()
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

  it('非安全上下文 / 无 getUserMedia 时，扫码入口（工具栏 + 吸引屏）都置灰', async () => {
    mocks.getVisionStatus.mockResolvedValue({ is_ready: true })
    const wrapper = mountCustomer()
    await flushPromises()

    const btns = scanButtons(wrapper)
    expect(btns.length).toBe(2)
    for (const b of btns) expect(b.attributes('disabled')).toBeDefined()
    wrapper.unmount()
  })

  it('安全上下文且有 getUserMedia 时，扫码入口可用', async () => {
    mocks.getVisionStatus.mockResolvedValue({ is_ready: true })
    vi.stubGlobal('isSecureContext', true)
    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      value: { getUserMedia: vi.fn() },
    })
    const wrapper = mountCustomer()
    await flushPromises()

    const btns = scanButtons(wrapper)
    expect(btns.length).toBe(2)
    for (const b of btns) expect(b.attributes('disabled')).toBeUndefined()
    wrapper.unmount()
    // 清掉测试注入的 own property，避免影响后续用例。
    Reflect.deleteProperty(navigator, 'mediaDevices')
  })
})

describe('CustomerView 扫码枪', () => {
  function press(code: string) {
    for (const ch of code) {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: ch, bubbles: true, cancelable: true }))
    }
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }))
  }

  function mountCustomer() {
    const pinia = createPinia()
    setActivePinia(pinia)
    const store = useCustomerStore()
    const wrapper = mount(CustomerView, {
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
    return { wrapper, store }
  }

  it('吸引屏显示时扫码枪命中 → 先撤掉吸引屏再加购', async () => {
    const { wrapper, store } = mountCustomer()
    await flushPromises()

    // setupStoreForEvent 的拉取完成后，注入本场商品。
    store.products = [makeProduct({ barcode: '4901234567894', onsite_qty: 5 })]
    expect(wrapper.find('.attract-screen').exists()).toBe(true)

    press('4901234567894')
    await nextTick()

    expect(store.cart).toHaveLength(1)
    expect(store.cart[0].product_code).toBe('P001')
    expect(wrapper.find('.attract-screen').exists()).toBe(false)
    wrapper.unmount()
  })

  it('确认面板打开时扫码枪命中 → 购物车不变', async () => {
    const { wrapper, store } = mountCustomer()
    await flushPromises()

    const product = makeProduct({ barcode: '4901234567894', onsite_qty: 5 })
    store.products = [product]
    store.cart = [{ ...product, quantity: 2 }]
    await nextTick()

    // 打开结算确认面板。
    await wrapper.find('.bar-checkout-btn').trigger('click')
    const panel = wrapper.findComponent({ name: 'OrderConfirmPanel' })
    expect(panel.props('show')).toBe(true)

    press('4901234567894')
    await nextTick()

    // 扫码枪被 enabled=false 挡住：没有加购、数量不变。
    expect(store.cart).toHaveLength(1)
    expect(store.cart[0].quantity).toBe(2)
    wrapper.unmount()
  })

  it('购物车已占满库存 → 扫码枪不加购、不弹「库存不足」', async () => {
    const { wrapper, store } = mountCustomer()
    await flushPromises()

    const product = makeProduct({ barcode: '4901234567894', onsite_qty: 5 })
    store.products = [product]
    store.cart = [{ ...product, quantity: 5 }]
    await nextTick()

    mocks.fbAlert.mockClear()
    press('4901234567894')
    await nextTick()

    expect(store.cart).toHaveLength(1)
    expect(store.cart[0].quantity).toBe(5)
    expect(mocks.fbAlert).not.toHaveBeenCalled()
    wrapper.unmount()
  })
})

describe('CustomerView 扫码面板与结算 / 闲置', () => {
  /** 扫码面板替身：只声明 emits，便于在父组件测试里手动触发 activity / choosing。 */
  const ScanPanelStub = {
    name: 'BarcodeScanPanel',
    props: {
      products: { type: Array, default: () => [] },
      cart: { type: Array, default: () => [] },
      single: { type: Boolean, default: false },
    },
    emits: ['add', 'code', 'close', 'activity', 'choosing'],
    template: `<div class="barcode-scan-stub" />`,
  }

  function press(code: string) {
    for (const ch of code) {
      window.dispatchEvent(
        new KeyboardEvent('keydown', { key: ch, bubbles: true, cancelable: true })
      )
    }
    window.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })
    )
  }

  function mountWithScan() {
    // 扫码入口要求安全上下文 + getUserMedia。
    vi.stubGlobal('isSecureContext', true)
    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      value: { getUserMedia: vi.fn() },
    })
    const pinia = createPinia()
    setActivePinia(pinia)
    const store = useCustomerStore()
    const wrapper = mount(CustomerView, {
      props: { id: '3' },
      global: {
        plugins: [pinia],
        stubs: {
          BarcodeScanPanel: ScanPanelStub,
          ProductGrid: true,
          VisionSearch: true,
          PaymentModal: true,
          OrderConfirmPanel: true,
          RouterLink: { template: '<a class="router-link-stub"><slot /></a>' },
        },
      },
    })
    return { wrapper, store }
  }

  /** 从吸引屏进入扫码模式，等面板挂载。 */
  async function enterScanMode(wrapper: VueWrapper) {
    await flushPromises()
    const scanButtons = wrapper.findAll('button').filter((b) => b.text().includes('扫码'))
    await scanButtons[scanButtons.length - 1].trigger('click')
    await flushPromises()
  }

  afterEach(() => {
    Reflect.deleteProperty(navigator, 'mediaDevices')
  })

  it('scan 模式下打开结算确认面板 → 扫码面板被卸载，取消后重新渲染', async () => {
    const { wrapper, store } = mountWithScan()
    await enterScanMode(wrapper)
    expect(wrapper.findComponent({ name: 'BarcodeScanPanel' }).exists()).toBe(true)

    store.cart = [{ ...makeProduct(), quantity: 1 }]
    await nextTick()
    await wrapper.find('.bar-checkout-btn').trigger('click')
    await nextTick()

    expect(wrapper.findComponent({ name: 'OrderConfirmPanel' }).props('show')).toBe(true)
    expect(wrapper.findComponent({ name: 'BarcodeScanPanel' }).exists()).toBe(false)

    // 取消结算后仍在 scan 模式 → 面板重新挂载。
    wrapper.findComponent({ name: 'OrderConfirmPanel' }).vm.$emit('cancel')
    await nextTick()
    expect(wrapper.findComponent({ name: 'BarcodeScanPanel' }).exists()).toBe(true)
    wrapper.unmount()
  })

  it('扫码面板 emit activity → 闲置计时器被重置，60 秒内不跳吸引屏', async () => {
    vi.useFakeTimers()
    try {
      const { wrapper } = mountWithScan()
      await vi.advanceTimersByTimeAsync(0)
      await nextTick()

      const scanButtons = wrapper.findAll('button').filter((b) => b.text().includes('扫码'))
      await scanButtons[scanButtons.length - 1].trigger('click')
      await vi.advanceTimersByTimeAsync(0)
      await nextTick()

      const panel = wrapper.findComponent({ name: 'BarcodeScanPanel' })
      expect(panel.exists()).toBe(true)

      // 进入扫码时 resetIdleTimer 起算，60s 会跳吸引屏；40s 处扫码续期一次。
      await vi.advanceTimersByTimeAsync(40000)
      panel.vm.$emit('activity')
      await vi.advanceTimersByTimeAsync(40000)
      expect(wrapper.find('.attract-screen').exists()).toBe(false)

      // 再超过 60s 不续期 → 吸引屏如期出现，证明计时器确实在走。
      await vi.advanceTimersByTimeAsync(21000)
      expect(wrapper.find('.attract-screen').exists()).toBe(true)
      wrapper.unmount()
    } finally {
      vi.useRealTimers()
    }
  })

  it('扫码面板多件选择中 → 扫码枪停用；结束后恢复加购', async () => {
    const { wrapper, store } = mountWithScan()
    await enterScanMode(wrapper)
    // 进入 scan 模式会重新拉商品，等拉完再注入本场商品。
    store.products = [makeProduct({ barcode: '4901234567894', onsite_qty: 5 })]
    await nextTick()

    const panel = wrapper.findComponent({ name: 'BarcodeScanPanel' })
    panel.vm.$emit('choosing', true)
    await nextTick()

    press('4901234567894')
    await nextTick()
    expect(store.cart).toHaveLength(0)

    panel.vm.$emit('choosing', false)
    await nextTick()
    press('4901234567894')
    await nextTick()
    expect(store.cart).toHaveLength(1)
    wrapper.unmount()
  })
})
