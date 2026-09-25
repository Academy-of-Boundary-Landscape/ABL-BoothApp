// src/services/url.ts
import { backendOrigin } from '@/api/backendOrigin'

const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined

// 注意：图片不在 /api 下，而在根路径的 /uploads 下。
// 后端的实际地址（5140 被占时会换端口），main.ts 在加载本模块前已经取好了。
export const SERVER_ORIGIN = backendOrigin()

/**
 * 把后端返回的相对地址补成可以直接 fetch 的地址。
 * Tauri 里页面不是 http 源，必须拼上后端的绝对地址；
 * 浏览器（LAN 设备）里保持同源的相对路径——拼成 127.0.0.1 会指向访问者自己的设备，
 * 而且在 HTTPS 页面里是混合内容，摊主在手机上导出结算单就会失败。
 */
export function toAbsoluteApiUrl(url: string): string {
  if (!url) return url
  if (url.startsWith('http://') || url.startsWith('https://')) return url
  const path = url.startsWith('/') ? url : `/${url}`
  return isTauri ? `${SERVER_ORIGIN}${path}` : path
}

/**
 * 将数据库存储的相对路径转换为完整的 URL
 * @param path - 数据库存的路径 (如 "uploads/products/abc.png" 或 "/uploads/products/abc.png")
 */
export function getImageUrl(path: string): string {
  if (!path) return ''

  // 已经是完整链接 or blob
  if (path.startsWith('http') || path.startsWith('blob:') || path.startsWith('data:')) {
    return path
  }

  // 规范成以 / 开头
  const cleanPath = path.startsWith('/') ? path : `/${path}`

  // 关键：Tauri 里前端不是 http origin，必须拼绝对地址指向你的 axum server
  if (isTauri) {
    return `${SERVER_ORIGIN}${cleanPath}`
  }

  // 浏览器环境仍保持相对路径（同源）
  return cleanPath
}
