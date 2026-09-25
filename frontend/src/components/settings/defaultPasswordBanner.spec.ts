import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'

const mocks = vi.hoisted(() => ({
  apiGet: vi.fn(),
  push: vi.fn(),
  routeName: { value: 'admin-events' as string },
}))

vi.mock('@/api/client', () => ({
  api: { GET: (...a: unknown[]) => mocks.apiGet(...a) },
  unwrap: (p: Promise<unknown>) => p,
}))
vi.mock('vue-router', () => ({
  useRoute: () => ({
    get name() {
      return mocks.routeName.value
    },
  }),
  useRouter: () => ({ push: mocks.push }),
}))

async function mountBanner() {
  vi.resetModules()
  const { default: Banner } = await import('./DefaultPasswordBanner.vue')
  const w = mount(Banner)
  await flushPromises()
  return w
}

beforeEach(() => {
  sessionStorage.clear()
  mocks.apiGet.mockReset()
  mocks.push.mockReset()
  mocks.routeName.value = 'admin-events'
})

describe('DefaultPasswordBanner', () => {
  it('两个都是默认值时逐一点名，并给出去修改的入口', async () => {
    mocks.apiGet.mockResolvedValue({ admin: true, vendor: true })
    const w = await mountBanner()
    expect(w.text()).toContain('管理员密码（admin123）')
    expect(w.text()).toContain('摊主密码（vendor123）')
    await w.find('button.n-button').trigger('click')
    expect(mocks.push).toHaveBeenCalledWith({ name: 'admin-settings', hash: '#security' })
  })

  it('都改过了就不显示', async () => {
    mocks.apiGet.mockResolvedValue({ admin: false, vendor: false })
    const w = await mountBanner()
    expect(w.find('.default-password-banner').exists()).toBe(false)
  })

  it('接口失败（比如不是管理员）不显示，也不报错', async () => {
    mocks.apiGet.mockRejectedValue(new Error('403'))
    const w = await mountBanner()
    expect(w.find('.default-password-banner').exists()).toBe(false)
  })

  it('本次会话里关掉过就不再显示', async () => {
    sessionStorage.setItem('default-password-banner-dismissed', '1')
    mocks.apiGet.mockResolvedValue({ admin: true, vendor: false })
    const w = await mountBanner()
    expect(w.find('.default-password-banner').exists()).toBe(false)
  })
})
