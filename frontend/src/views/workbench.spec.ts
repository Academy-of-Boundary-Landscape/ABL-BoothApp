import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'
import { NButton } from 'naive-ui'
import { ref } from 'vue'
import WorkbenchIndex from '@/views/WorkbenchIndex.vue'
import AdminEventWorkbench from '@/views/AdminEventWorkbench.vue'
import { WORKBENCH_EVENT, type WorkbenchEventContext } from '@/composables/useWorkbenchEvent'
import { resolveActiveKey } from '@/views/adminNav'
import { ApiRequestError, type Schemas } from '@/api/client'

const mocks = vi.hoisted(() => ({
  fbError: vi.fn(),
  apiGet: vi.fn(),
  apiPut: vi.fn(),
}))

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
    api: { GET: mocks.apiGet, PUT: mocks.apiPut },
    unwrap: (p: unknown) => p,
    errorMessage: (e: unknown, fallback: string) =>
      (e instanceof MockApiRequestError && e.serverMessage) ||
      (e instanceof Error && e.message) ||
      fallback,
    ApiRequestError: MockApiRequestError,
  }
})

function makeEvent(overrides: Partial<Schemas['EventResponse']> = {}): Schemas['EventResponse'] {
  return {
    id: 5,
    name: '测试展会',
    date: '2026-10-01',
    location: '上海某会展中心',
    status: '筹备',
    qrcode_url: null,
    qrcode_urls: [],
    ...overrides,
  }
}

function makeCtx(
  overrides: Partial<{
    event: Schemas['EventResponse'] | null
    loading: boolean
    error: string | null
  }> = {}
): WorkbenchEventContext {
  return {
    event: ref(overrides.event ?? null),
    loading: ref(overrides.loading ?? false),
    error: ref(overrides.error ?? null),
    reload: vi.fn().mockResolvedValue(undefined),
  }
}

function makeRouter(): Router {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      {
        path: '/admin/events',
        name: 'admin-events',
        component: { template: '<div />' },
      },
      {
        path: '/admin/events/:id',
        name: 'admin-event-workbench',
        component: { template: '<div><router-view /></div>' },
        children: [
          {
            path: '',
            name: 'admin-event-workbench-index',
            component: { template: '<div />' },
          },
          { path: 'products', name: 'admin-event-products', component: { template: '<div />' } },
          { path: 'lots', name: 'admin-event-lots', component: { template: '<div />' } },
          { path: 'orders', name: 'admin-event-orders', component: { template: '<div />' } },
          { path: 'stats', name: 'admin-event-stats', component: { template: '<div />' } },
          {
            path: 'settlement',
            name: 'admin-event-settlement',
            component: { template: '<div />' },
          },
        ],
      },
    ],
  })
}

function mountIndex(ctx: WorkbenchEventContext, router: Router) {
  return mount(WorkbenchIndex, {
    global: {
      plugins: [createPinia(), router],
      provide: { [WORKBENCH_EVENT as symbol]: ctx },
    },
  })
}

describe('WorkbenchIndex', () => {
  it.each([
    ['筹备', 'admin-event-products'],
    ['进行中', 'admin-event-orders'],
    ['已结算', 'admin-event-settlement'],
  ] as const)('展会状态 %s → router.replace 到 %s', async (status, target) => {
    const router = makeRouter()
    const replace = vi.spyOn(router, 'replace').mockResolvedValue(undefined)
    const ctx = makeCtx({ event: makeEvent({ status }) })

    mountIndex(ctx, router)
    await flushPromises()

    expect(replace).toHaveBeenCalledTimes(1)
    expect(replace).toHaveBeenCalledWith({ name: target, params: { id: 5 } })
  })

  it('展会 404 渲染错误态与返回按钮，且不 router.replace', async () => {
    const router = makeRouter()
    const replace = vi.spyOn(router, 'replace').mockResolvedValue(undefined)
    const ctx = makeCtx({ event: null, loading: false, error: '展会不存在或已被删除。' })

    const wrapper = mountIndex(ctx, router)
    await flushPromises()

    expect(wrapper.text()).toContain('展会不存在或已被删除。')
    expect(wrapper.text()).toContain('返回展会列表')
    expect(replace).not.toHaveBeenCalled()
  })
})

describe('AdminEventWorkbench 开始展会', () => {
  beforeEach(() => {
    mocks.fbError.mockClear()
    mocks.apiGet.mockReset()
    mocks.apiPut.mockReset()
  })

  it('失败时调用 fb.error 显示后端原文，按钮恢复可点', async () => {
    mocks.apiGet.mockResolvedValue(makeEvent({ status: '筹备' }))
    mocks.apiPut.mockRejectedValue(new ApiRequestError(400, { error: '后端说不行' }))
    const router = makeRouter()
    await router.push('/admin/events/5')

    const wrapper = mount(AdminEventWorkbench, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    const startBtn = wrapper.findAllComponents(NButton).find((b) => b.text().includes('开始展会'))
    expect(startBtn).toBeTruthy()

    await startBtn!.trigger('click')
    await flushPromises()

    expect(mocks.fbError).toHaveBeenCalledTimes(1)
    expect((mocks.fbError.mock.calls[0][0] as Error).message).toContain('后端说不行')
    expect(startBtn!.props('loading')).toBe(false)
  })

  it('直接访问不存在的展会子页：页头报错并给出返回入口，不永远显示加载中', async () => {
    mocks.apiGet.mockRejectedValue(new ApiRequestError(404, { error: '展会不存在或已被删除。' }))
    const router = makeRouter()
    await router.push('/admin/events/999/products')

    const wrapper = mount(AdminEventWorkbench, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    expect(wrapper.text()).not.toContain('正在加载展会')
    expect(wrapper.text()).toContain('展会不存在或无法加载')
    expect(wrapper.text()).toContain('展会不存在或已被删除。')
    expect(wrapper.text()).toContain('返回展会列表')
    // 状态条与 tab 行不该渲染，也不该再渲染子页。
    expect(wrapper.find('.status-bar').exists()).toBe(false)
    expect(wrapper.find('.workbench-tabs').exists()).toBe(false)
  })
})

describe('AdminLayout activeKey', () => {
  it('/admin/events/5/orders 高亮到 /admin/events', () => {
    expect(resolveActiveKey('/admin/events/5/orders')).toBe('/admin/events')
  })

  it('精确匹配设置页', () => {
    expect(resolveActiveKey('/admin/settings')).toBe('/admin/settings')
  })
})
