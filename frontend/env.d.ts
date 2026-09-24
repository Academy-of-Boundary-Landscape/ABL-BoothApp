/// <reference types="vite/client" />

interface Window {
  /** Tauri 注入；浏览器（LAN 顾客端）里不存在。用它判断运行环境。 */
  __TAURI_INTERNALS__?: unknown
}
