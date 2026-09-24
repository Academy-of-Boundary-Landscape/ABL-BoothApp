import { describe, it, expect } from 'vitest'
import { sessionExpiredTarget } from './loginRedirect'

describe('sessionExpiredTarget', () => {
  it('管理端页面 → /login/admin，并带回跳地址', () => {
    expect(sessionExpiredTarget({ fullPath: '/admin/events/3', meta: { role: 'admin' } })).toEqual({
      name: 'login',
      params: { role: 'admin' },
      query: { redirect: '/admin/events/3' },
    })
  })

  it('摊主页面或未标角色的页面 → /login/vendor（与路由守卫的默认一致）', () => {
    expect(
      sessionExpiredTarget({ fullPath: '/vendor/3', meta: { role: 'vendor' } }).params.role
    ).toBe('vendor')
    expect(sessionExpiredTarget({ fullPath: '/', meta: {} }).params.role).toBe('vendor')
  })
})
