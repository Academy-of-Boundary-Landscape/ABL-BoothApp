import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { nextTick, ref } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { NButton } from 'naive-ui'
import AdminEventOrders from '@/views/AdminEventOrders.vue'
import VendorOrders from '@/views/vendor/VendorOrders.vue'
import OrderCard from '@/components/order/OrderCard.vue'
import { useOrderStore } from '@/stores/orderStore'
import { VENDOR_POLLING, type VendorPolling } from '@/composables/useVendorPolling'
import { cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

const mocks = vi.hoisted(() => ({
  apiGet: vi.fn(),
  fbError: vi.fn(),
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
    api: { GET: mocks.apiGet, POST: vi.fn(), PUT: vi.fn(), DELETE: vi.fn() },
    unwrap: (p: unknown) => p,
    errorMessage: (e: unknown, fallback: string) =>
      (e instanceof MockApiRequestError && e.serverMessage) ||
      (e instanceof Error && e.message) ||
      fallback,
    ApiRequestError: MockApiRequestError,
  }
})

vi.mock('@/composables/useFeedback', () => ({
  useFeedback: () => ({
    success: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    error: mocks.fbError,
    loading: vi.fn(),
    confirm: vi.fn(),
    alert: vi.fn(),
  }),
}))

function makeItem(
  overrides: Partial<Schemas['OrderItemResponse']> = {}
): Schemas['OrderItemResponse'] {
  return {
    id: 1,
    allocated_amount: cents(0),
    lot_name: null,
    paid_amount: cents(1000),
    product_id: 1,
    product_image_url: null,
    product_name: '测试商品',
    product_price: cents(500),
    quantity: 2,
    refunded_amount: cents(0),
    refunded_qty: 0,
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
    status: 'completed',
    timestamp: '2026-10-01T10:00:00Z',
    channel: '现金',
    completed_at: null,
    items: [],
    lots: [],
    ...overrides,
  }
}

const RouterLinkStub = { template: '<a class="router-link-stub"><slot /></a>' }

function mountAdminOrders(orders: Schemas['OrderResponse'][]) {
  mocks.apiGet.mockResolvedValue(orders)
  return mount(AdminEventOrders, {
    props: { id: 5 },
    global: {
      plugins: [createPinia()],
      stubs: { RouterLink: RouterLinkStub },
    },
  })
}

describe('AdminEventOrders 已退列', () => {
  beforeEach(() => {
    mocks.apiGet.mockReset()
    mocks.fbError.mockReset()
  })

  it('refunded_amount 为 0 时显示「—」而不是 ¥0.00', async () => {
    const wrapper = mountAdminOrders([
      makeOrder(1, { refunded_amount: cents(0), items: [makeItem()] }),
    ])
    await flushPromises()
    await nextTick()

    const cell = wrapper.find('.refunded-cell')
    expect(cell.exists()).toBe(true)
    expect(cell.text()).toBe('—')
  })

  it('有退货金额时显示实际退款数', async () => {
    const wrapper = mountAdminOrders([
      makeOrder(2, { refunded_amount: cents(500), items: [makeItem({ refunded_qty: 1 })] }),
    ])
    await flushPromises()
    await nextTick()

    expect(wrapper.find('.refunded-cell').text()).toBe('¥5.00')
  })
})

describe('VendorOrders 已退完置灰', () => {
  const LiveStatsStub = { template: '<div class="livestats-stub" />' }

  function mountVendorOrders(orders: Schemas['OrderResponse'][]) {
    const pinia = createPinia()
    setActivePinia(pinia)
    useOrderStore().completedOrders = orders
    const polling: VendorPolling = { pendingCount: ref(0), refresh: vi.fn() }
    return mount(VendorOrders, {
      props: { id: '3' },
      global: {
        plugins: [pinia],
        provide: { [VENDOR_POLLING as symbol]: polling },
        stubs: { LiveStats: LiveStatsStub, RouterLink: RouterLinkStub },
      },
    })
  }

  it('全部退完的单「退货」禁用并提示，部分退的单可点', async () => {
    const fullyRefunded = makeOrder(1, {
      refunded_amount: cents(1000),
      items: [makeItem({ quantity: 2, refunded_qty: 2, refunded_amount: cents(1000) })],
    })
    const partiallyRefunded = makeOrder(2, {
      refunded_amount: cents(500),
      items: [makeItem({ quantity: 2, refunded_qty: 1, refunded_amount: cents(500) })],
    })

    const wrapper = mountVendorOrders([fullyRefunded, partiallyRefunded])
    await flushPromises()

    const refundButtons = wrapper
      .findAllComponents(NButton)
      .filter((button) => button.text() === '退货')
    expect(refundButtons).toHaveLength(2)
    expect(refundButtons[0].props('disabled')).toBe(true)
    expect(refundButtons[1].props('disabled')).toBe(false)
    expect(wrapper.text()).toContain('已全部退货')
  })
})

describe('OrderCard 全退置灰', () => {
  it('每一行都退满时卡片打上置灰类', () => {
    const order = makeOrder(1, {
      refunded_amount: cents(1000),
      items: [makeItem({ quantity: 2, refunded_qty: 2 })],
    })
    const wrapper = mount(OrderCard, { props: { order, isCompleted: true } })
    expect(wrapper.classes()).toContain('order-card--refunded')
  })

  it('只退了一部分时不打置灰类', () => {
    const order = makeOrder(2, {
      refunded_amount: cents(500),
      items: [makeItem({ quantity: 2, refunded_qty: 1 })],
    })
    const wrapper = mount(OrderCard, { props: { order, isCompleted: true } })
    expect(wrapper.classes()).not.toContain('order-card--refunded')
  })
})
