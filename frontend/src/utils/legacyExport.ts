// src/utils/legacyExport.ts
//
// v1 历史数据 xlsx 的下载逻辑。抽出来是因为它有两个出口：
// 首启弹窗（MigrationNotice.vue，只弹一次）和「历史数据（v1）」常驻 section。
// 弹窗只保证第一次提醒，常驻入口保证那之后还能导出——两份下载逻辑不许分叉。

import { ApiRequestError, api, unwrap } from '@/api/client'
import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'

/** 默认落盘文件名，两个出口保持一致。 */
export const LEGACY_EXPORT_FILENAME = 'legacy_v1_export.xlsx'

/**
 * 从 `/api/legacy/export.xlsx` 下载 v1 备份的原始字节。
 *
 * HTTP 统一走新 client（Bearer、超时、Tauri/浏览器传输切换都由它处理），二进制响应加
 * `parseAs: 'blob'`。失败时保留旧文案「下载失败: 状态码 + 错误体」的形状。
 */
async function downloadLegacyXlsx(): Promise<Blob> {
  try {
    // 显式给出响应类型：见 authStore.login 的同款说明（unwrap 会推断出 `T | undefined`）。
    return await unwrap(api.GET('/legacy/export.xlsx', { parseAs: 'blob' }))
  } catch (e) {
    if (e instanceof ApiRequestError) {
      // 旧代码是手动 fetch + `下载失败: <status> <text>`；这里保持同样的形状，
      // JSON 错误体（401/403 的 `{error}`）也序列化进去。
      const bodyText =
        typeof e.body === 'string' ? e.body : e.body == null ? '' : JSON.stringify(e.body)
      throw new Error(`下载失败: ${e.status} ${bodyText.slice(0, 200)}`.trim())
    }
    throw e
  }
}

/**
 * 下载 v1 备份并保存/触发下载。
 *
 * @returns 真正写出了文件返回 true；用户在保存对话框取消返回 false。
 *   请求/读取失败会抛错，由调用方决定怎么提示。
 */
export async function exportLegacyXlsx(): Promise<boolean> {
  const isTauri = window.__TAURI_INTERNALS__ !== undefined
  const blob = await downloadLegacyXlsx()

  if (isTauri) {
    const bytes = new Uint8Array(await blob.arrayBuffer())

    const filePath = await save({
      defaultPath: LEGACY_EXPORT_FILENAME,
      filters: [{ name: 'Excel Files', extensions: ['xlsx'] }],
    })
    if (!filePath) return false

    await writeFile(filePath, bytes)
    return true
  }

  // 浏览器环境
  const dl = window.URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.style.display = 'none'
  a.href = dl
  a.download = LEGACY_EXPORT_FILENAME
  document.body.appendChild(a)
  a.click()
  setTimeout(() => {
    document.body.removeChild(a)
    window.URL.revokeObjectURL(dl)
  }, 100)
  return true
}
