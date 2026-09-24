/**
 * Tauri 内后端的实际地址。
 *
 * 后端首选 127.0.0.1:5140，被占时会回退到别的端口（src-tauri/src/server.rs 的
 * http_port_candidates）。实际地址经 Tauri 命令 `get_backend_url` 取得。
 * main.ts 在加载任何业务模块**之前**调用 initBackendOrigin()，所以 client.ts 与 url.ts
 * 在模块初始化时读到的已经是实际地址。
 *
 * 浏览器（LAN 顾客端）里不问 Tauri：API 走同源的 `/api`（见 client.ts）。
 */
const DEFAULT_ORIGIN = 'http://127.0.0.1:5140'

let origin = DEFAULT_ORIGIN

export function backendOrigin(): string {
  return origin
}

export async function initBackendOrigin(): Promise<void> {
  if (window.__TAURI_INTERNALS__ === undefined) return
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const url = await invoke<string>('get_backend_url')
    if (url) origin = url.replace(/\/+$/, '')
  } catch (e) {
    // 取不到就用默认端口——不能因此卡住启动；真出问题时后端已经弹过对话框了
    console.warn('[backendOrigin] get_backend_url failed, falling back to default', e)
  }
}
