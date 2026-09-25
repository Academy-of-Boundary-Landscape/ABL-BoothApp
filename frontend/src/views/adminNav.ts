// views/adminNav.ts —— 管理端侧栏的菜单 key 与高亮规则（纯函数，便于单测）。
// 菜单 key 用路径前缀表示归属：/admin/events/5/orders 高亮到 /admin/events。

export const ADMIN_MENU_KEYS = [
  '/admin/events',
  '/admin/master-products',
  '/admin/societies',
  '/admin/settings',
  '/admin/help',
] as const

/** 取「是当前路径前缀的最长菜单 key」；没有匹配返回空串（n-menu 不高亮任何项）。 */
export function resolveActiveKey(path: string): string {
  let best = ''
  for (const key of ADMIN_MENU_KEYS) {
    if ((path === key || path.startsWith(`${key}/`)) && key.length > best.length) {
      best = key
    }
  }
  return best
}
