/**
 * API 返回 401/403（会话失效或越权）时跳去哪。
 *
 * 登录路由是 `/login/:role`——旧版直接 `router.push('/login')`，那个路由不存在，
 * 结果落到 404 页。角色取当前页面路由的 `meta.role`，缺省 vendor，与路由守卫一致。
 */
export interface RouteLike {
  fullPath: string
  meta: { role?: unknown }
}

export function sessionExpiredTarget(route: RouteLike) {
  const role = route.meta.role === 'admin' ? 'admin' : 'vendor'
  return { name: 'login', params: { role }, query: { redirect: route.fullPath } }
}
