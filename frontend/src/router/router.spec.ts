import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import router from '@/router'

// beforeEach 守卫要求 admin 权限；测试里直接把 sessionStorage 的用户写成 admin。
function loginAsAdmin() {
  sessionStorage.setItem('user', JSON.stringify({ role: 'admin', access: 'all' }))
}

describe('管理端路由', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    loginAsAdmin()
  })

  it('/admin 重定向到 admin-events', async () => {
    await router.push('/admin')
    expect(router.currentRoute.value.name).toBe('admin-events')
  })

  it('/admin/about 重定向到 admin-settings 且 hash 为 #about', async () => {
    await router.push('/admin/about')
    expect(router.currentRoute.value.name).toBe('admin-settings')
    expect(router.currentRoute.value.hash).toBe('#about')
  })

  it('/admin/events/5/stats 解析为 admin-event-stats 且 params.id === "5"', () => {
    const resolved = router.resolve('/admin/events/5/stats')
    expect(resolved.name).toBe('admin-event-stats')
    expect(resolved.params.id).toBe('5')
  })

  const CHILD_ROUTES = [
    ['admin-event-products', '/admin/events/5/products'],
    ['admin-event-lots', '/admin/events/5/lots'],
    ['admin-event-orders', '/admin/events/5/orders'],
    ['admin-event-stats', '/admin/events/5/stats'],
    ['admin-event-settlement', '/admin/events/5/settlement'],
  ] as const

  it.each(CHILD_ROUTES)('%s 继承父路由 meta.role=admin', (name, path) => {
    const resolved = router.resolve(path)
    expect(resolved.name).toBe(name)
    expect(resolved.meta.role).toBe('admin')
    expect(resolved.meta.requiresAuth).toBe(true)
  })
})
