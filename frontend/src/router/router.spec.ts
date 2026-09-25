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

  it('/admin/events/5 进入工作台时匹配到空路径子路由（WorkbenchIndex 负责按状态跳转）', async () => {
    await router.push('/admin/events/5')
    expect(router.currentRoute.value.name).toBe('admin-event-workbench-index')
    // 外壳 + 子路由两层都要匹配上，否则外壳里的 <router-view> 是空的。
    expect(router.currentRoute.value.matched).toHaveLength(3)
  })

  it('工作台外壳不命名：按父路由名跳转会漏掉空路径子路由，留着名字就是留着坑', () => {
    const shell = router
      .getRoutes()
      .find((r) => r.path === '/admin/events/:id' && r.children.length > 0)
    expect(shell).toBeDefined()
    expect(shell?.name).toBeUndefined()
  })

  it('展会卡片用的路由名同样匹配到子路由', () => {
    const resolved = router.resolve({ name: 'admin-event-workbench-index', params: { id: 5 } })
    expect(resolved.fullPath).toBe('/admin/events/5')
    expect(resolved.matched).toHaveLength(3)
  })

  it.each(CHILD_ROUTES.filter(([name]) => name !== 'admin-event-stats'))(
    '%s 的 id prop 是数字（不是路由参数原样的字符串）',
    (_name, path) => {
      const resolved = router.resolve(path)
      const record = resolved.matched[resolved.matched.length - 1]
      const props = record.props.default
      expect(typeof props).toBe('function')
      expect((props as (r: typeof resolved) => { id: unknown })(resolved)).toEqual({ id: 5 })
    }
  )
})
