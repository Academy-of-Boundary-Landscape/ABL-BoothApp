// src/utils/legacyExport.js
//
// v1 历史数据 xlsx 的下载逻辑。抽出来是因为它有两个出口：
// 首启弹窗（MigrationNotice.vue，只弹一次）和「历史数据（v1）」常驻 section。
// 弹窗只保证第一次提醒，常驻入口保证那之后还能导出——两份下载逻辑不许分叉。

import { toAbsoluteApiUrl } from '@/services/url'
import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

/** 默认落盘文件名，两个出口保持一致。 */
export const LEGACY_EXPORT_FILENAME = 'legacy_v1_export.xlsx'

/**
 * 从 `/api/legacy/export.xlsx` 下载 v1 备份并保存/触发下载。
 *
 * 照抄 AdminEventStat.vue 的既有写法：手动拼绝对 URL、手动加 Authorization 头、
 * Tauri 走 tauriFetch 浏览器走 fetch。不能走 axios 实例——它在 Tauri 下用的是
 * 自定义 adapter，二进制下载不适合经过它。
 *
 * @returns {Promise<boolean>} 真正写出了文件返回 true；用户在保存对话框取消返回 false。
 *   请求/读取失败会抛错，由调用方决定怎么提示。
 */
export async function exportLegacyXlsx() {
  const isTauri = window.__TAURI_INTERNALS__ !== undefined
  const token = sessionStorage.getItem('access_token')
  const url = toAbsoluteApiUrl('/api/legacy/export.xlsx')

  if (isTauri) {
    const headers = {
      Accept: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    }
    if (token) headers['Authorization'] = `Bearer ${token}`

    const resp = await tauriFetch(url, { method: 'GET', headers })

    if (!resp.ok) {
      const text = await resp.text().catch(() => '')
      throw new Error(`下载失败: ${resp.status} ${resp.statusText} ${text.slice(0, 200)}`)
    }

    const ab = await resp.arrayBuffer()
    const bytes = new Uint8Array(ab)

    const filePath = await save({
      defaultPath: LEGACY_EXPORT_FILENAME,
      filters: [{ name: 'Excel Files', extensions: ['xlsx'] }],
    })
    if (!filePath) return false

    await writeFile(filePath, bytes)
    return true
  }

  // 浏览器环境
  const headers = {}
  if (token) headers['Authorization'] = `Bearer ${token}`

  const response = await fetch(url, {
    method: 'GET',
    credentials: 'include',
    headers,
  })

  if (!response.ok) {
    const text = await response.text().catch(() => '')
    throw new Error(`下载失败: ${response.status} ${text.slice(0, 200)}`)
  }

  const blob = await response.blob()
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
