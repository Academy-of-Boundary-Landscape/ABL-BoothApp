import { defineStore } from 'pinia'
import api from '@/services/api'
import { ref } from 'vue'
import { SYNC_IMPORT_LIMIT_MB, validateFileSize } from '@/utils/upload'

function detectEnv() {
  const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined
  const ua = typeof navigator !== 'undefined' ? (navigator.userAgent || '') : ''
  const isMobileUA = /android|iphone|ipad|ipod/i.test(ua)
  return {
    isTauri,
    isMobileUA,
    isTauriDesktop: isTauri && !isMobileUA,
    isTauriMobile: isTauri && isMobileUA
  }
}

// 解析 Content-Disposition 的 filename / filename*
function parseFilenameFromDisposition(disposition, fallback = 'booth_catalog.boothpack') {
  const d = String(disposition || '')

  // filename*=UTF-8''xxx
  const mStar = d.match(/filename\*\s*=\s*([^']*)''([^;]+)/i)
  if (mStar && mStar[2]) {
    try {
      return decodeURIComponent(mStar[2].trim().replace(/(^"|"$)/g, ''))
    } catch {
      return mStar[2].trim().replace(/(^"|"$)/g, '')
    }
  }

  // filename="xxx"
  const m = d.match(/filename\s*=\s*"?([^";]+)"?/i)
  if (m && m[1]) return m[1].trim()
  return fallback
}

function toBlob(data, mime = 'application/zip') {
  if (data instanceof Blob) return data
  if (data instanceof ArrayBuffer) return new Blob([new Uint8Array(data)], { type: mime })
  // axios 在某些环境可能给 Uint8Array
  if (data instanceof Uint8Array) return new Blob([data], { type: mime })
  return new Blob([data], { type: mime })
}

function triggerBrowserDownload(blob, filename) {
  const url = window.URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  window.URL.revokeObjectURL(url)
}

export const useSyncStore = defineStore('sync', () => {
  const isExporting = ref(false)
  const isImporting = ref(false)
  const lastError = ref(null)

  const env = detectEnv()

  async function exportProducts() {
    isExporting.value = true
    lastError.value = null

    try {
      // 统一用 arraybuffer 更省心（blob/arraybuffer 在不同端差异多）
      const response = await api.get('/sync/export-products', { responseType: 'arraybuffer' })
      const disposition = response.headers?.['content-disposition'] || response.headers?.['Content-Disposition'] || ''
      const filename = parseFilenameFromDisposition(disposition, 'booth_catalog.boothpack')
      const blob = toBlob(response.data, 'application/zip')

      // ✅ Tauri Desktop：保存对话框 + 写文件
      if (env.isTauriDesktop) {
        const [dialogModule, fsModule] = await Promise.all([
          import('@tauri-apps/plugin-dialog'),
          import('@tauri-apps/plugin-fs')
        ])

        const filePath = await dialogModule.save({
          defaultPath: filename,
          filters: [{ name: 'Booth Pack', extensions: ['boothpack', 'zip'] }]
        })

        if (!filePath) {
          // 用户取消不算“错误”
          return { filename: null, cancelled: true }
        }

        // 把 arraybuffer 写入
        const bytes = response.data instanceof ArrayBuffer ? new Uint8Array(response.data) : new Uint8Array(await blob.arrayBuffer())
        await fsModule.writeFile(filePath, bytes)
        return { filename: filePath }
      }

      // ✅ Tauri Mobile：优先尝试 dialog.save（如果可用），否则再走 share/web
      if (env.isTauriMobile) {
        // 1) 尝试用 dialog.save + fs.writeFile（如果你的移动端插件支持）
        try {
          const [dialogModule, fsModule] = await Promise.all([
            import('@tauri-apps/plugin-dialog'),
            import('@tauri-apps/plugin-fs')
          ])
          const filePath = await dialogModule.save({
            defaultPath: filename,
            filters: [{ name: 'Booth Pack', extensions: ['boothpack', 'zip'] }]
          })
          if (filePath) {
            const bytes = response.data instanceof ArrayBuffer ? new Uint8Array(response.data) : new Uint8Array(await blob.arrayBuffer())
            await fsModule.writeFile(filePath, bytes)
            return { filename: filePath }
          }
          // 若用户取消
          return { filename: null, cancelled: true }
        } catch (e) {
          // 2) 如果移动端不支持 save/writeFile，则尝试 Web Share
          // （注意：Tauri WebView 未必支持 files share）
          console.warn('[export] tauri mobile save/writeFile not available, fallback to share/web', e)
        }
      }

      // ✅ 浏览器 or 移动端 fallback：优先 Web Share(files) 再下载
      const canUseShare = typeof navigator !== 'undefined'
        && typeof navigator.share === 'function'
        && typeof navigator.canShare === 'function'

      if (canUseShare) {
        try {
          const file = new File([blob], filename, { type: 'application/zip' })
          if (navigator.canShare({ files: [file] })) {
            await navigator.share({
              files: [file],
              title: 'Booth Tool 制品包',
              text: '导出的制品包，可在设备上保存或分享'
            })
            return { filename }
          }
        } catch (shareErr) {
          console.warn('[export] Web Share failed, fallback to download', shareErr)
        }
      }

      // 最终回退：浏览器下载（Tauri Mobile 上可能不工作，但至少不 silently succeed）
      if (typeof window !== 'undefined') {
        triggerBrowserDownload(blob, filename)
        return { filename }
      }

      throw new Error('当前环境不支持下载/保存')
    } catch (err) {
      console.error(err)
      lastError.value = err

      // 尽量保留真实错误
      const msg = err?.message || '导出失败，请稍后重试'
      throw new Error(msg)
    } finally {
      isExporting.value = false
    }
  }

  async function importProducts(file) {
    if (!file) throw new Error('请选择要导入的文件')

    isImporting.value = true
    lastError.value = null
    try {
      const formData = new FormData()
      formData.append('file', file)
      const response = await api.post('/sync/import-products', formData)
      return response.data
    } catch (err) {
      console.error(err)
      lastError.value = err
      const msg =
        err?.response?.data?.error ||
        err?.response?.data?.message ||
        err?.message ||
        '导入失败，请检查文件格式或稍后重试'
      throw new Error(msg)
    } finally {
      isImporting.value = false
    }
  }

  // ✅ 修复：不再调用 importProducts() 以免 isImporting 双重管理
  async function importProductsFromPath(filePath) {
    if (!filePath) throw new Error('未提供文件路径')
    if (!env.isTauri) throw new Error('仅在 Tauri 环境可用')

    isImporting.value = true
    lastError.value = null
    try {
      const fsModule = await import('@tauri-apps/plugin-fs')
      const data = await fsModule.readFile(filePath) // Uint8Array
      const blob = new Blob([data], { type: 'application/zip' })
      const fileName = filePath.split(/[/\\]/).pop() || 'import.boothpack'
      const file = new File([blob], fileName, { type: 'application/zip' })

      const formData = new FormData()
      formData.append('file', file)
      const response = await api.post('/sync/import-products', formData)
      return response.data
    } catch (err) {
      console.error('[Tauri] importProductsFromPath error:', err)
      lastError.value = err
      const msg =
        err?.response?.data?.error ||
        err?.response?.data?.message ||
        err?.message ||
        '导入失败'
      throw new Error(msg)
    } finally {
      isImporting.value = false
    }
  }

  return {
    isExporting,
    isImporting,
    lastError,
    exportProducts,
    importProducts,
    importProductsFromPath
  }
})
