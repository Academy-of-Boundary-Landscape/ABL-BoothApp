/**
 * 后端 API 的唯一入口。类型来自 schema.d.ts（由 src-tauri/openapi.json 生成）。
 *
 * 调用写法：
 *   import { api, unwrap, errorMessage, type Schemas } from '@/api/client'
 *   const report = await unwrap(
 *     api.GET('/events/{event_id}/settlement', { params: { path: { event_id } } })
 *   )
 *   try { … } catch (e) { message.error(errorMessage(e, '加载失败')) }
 *
 * - 路径就是 openapi.json 里的路径，**不带** `/api` 前缀。
 * - FormData 直接作为 body 传；原始字节要显式带 `headers: { 'Content-Type': … }`。
 * - 二进制下载加 `parseAs: 'blob'`。
 *
 * 传输层沿用旧 axios adapter 的切换逻辑：Tauri 内请求 localhost 走 WebView 原生 fetch
 * （绕开 plugin-http 的 IPC 序列化，大 body 时 ~1.5 MB/s 瓶颈），其它走 plugin-http（保留跨域能力）。
 */
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

import router from '@/router'
import {
  IMAGE_UPLOAD_LIMIT_MB,
  SYNC_IMPORT_LIMIT_MB,
  normalizeUploadError,
  showUploadDialog,
} from '@/utils/upload'
import { backendOrigin } from './backendOrigin'
import { createApiClient } from './core'
import { sessionExpiredTarget } from './loginRedirect'
import type { components } from './schema'

export { ApiRequestError, errorMessage, unwrap, type ApiClient } from './core'
export type Schemas = components['schemas']

/** 是否运行在 Tauri 壳里（桌面 / Android）；LAN 顾客端浏览器里为 false。 */
export const isTauri = window.__TAURI_INTERNALS__ !== undefined
// Tauri 内是后端的实际地址（5140 被占时会换端口，见 backendOrigin.ts）；浏览器里走同源
const baseUrl = isTauri ? `${backendOrigin()}/api` : '/api'

const isLocalhostUrl = (u: string) => {
  try {
    const h = new URL(u).hostname
    return h === '127.0.0.1' || h === 'localhost' || h === '::1'
  } catch {
    return false
  }
}

let reqSeq = 0
async function transportFetch(req: Request, signal: AbortSignal): Promise<Response> {
  const id = ++reqSeq
  const t0 = performance.now()
  const native = !isTauri || isLocalhostUrl(req.url)
  const res = native ? await window.fetch(req, { signal }) : await tauriFetch(req, { signal })
  const ms = (performance.now() - t0).toFixed(2)
  console.log(
    `[Req #${id}] ${req.method} ${req.url} -> ${res.status} (${ms}ms) [${native ? 'native' : 'plugin-http'}]`
  )
  return res
}

export const api = createApiClient({
  baseUrl,
  fetch: transportFetch,
  // 旧版在 Tauri 里挂的是自定义 axios adapter，而 axios 的 timeout 只在它内置的 adapter 里实现——
  // 所以桌面 / Android 以前请求从不超时（大号 .boothpack 导入、导出要跑很久）。只在 LAN 浏览器里保留 30 秒。
  timeoutMs: isTauri ? null : 30_000,
  getToken: () => sessionStorage.getItem('access_token'),
  currentPath: () => router.currentRoute.value.path,
  onUnauthorized: () => {
    router.push(sessionExpiredTarget(router.currentRoute.value)).catch(() => {})
  },
  onUploadError: (url, errLike) => {
    // 三条规则与旧 services/api.js 的响应拦截器一致
    if (url.includes('/sync/import-products')) {
      showUploadDialog('商品包导入失败', normalizeUploadError(errLike, SYNC_IMPORT_LIMIT_MB))
    } else if (url.includes('/events')) {
      showUploadDialog('付款二维码上传失败', normalizeUploadError(errLike, IMAGE_UPLOAD_LIMIT_MB))
    } else if (url.includes('/master-products')) {
      showUploadDialog('商品预览图上传失败', normalizeUploadError(errLike, IMAGE_UPLOAD_LIMIT_MB))
    }
  },
})
