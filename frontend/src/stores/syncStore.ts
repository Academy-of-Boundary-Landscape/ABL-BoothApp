import { defineStore } from 'pinia'
import { api, unwrap, ApiRequestError, type Schemas } from '@/api/client'
import { ref } from 'vue'

function detectEnv() {
  const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined
  const ua = typeof navigator !== 'undefined' ? navigator.userAgent || '' : ''
  const isMobileUA = /android|iphone|ipad|ipod/i.test(ua)
  return {
    isTauri,
    isMobileUA,
    isTauriDesktop: isTauri && !isMobileUA,
    isTauriMobile: isTauri && isMobileUA,
  }
}

// 解析 Content-Disposition 的 filename / filename*
function parseFilenameFromDisposition(disposition: string, fallback = 'booth_catalog.boothpack') {
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

function toBlob(data: Blob | ArrayBuffer | Uint8Array, mime = 'application/zip'): Blob {
  if (data instanceof Blob) return data
  if (data instanceof ArrayBuffer) return new Blob([new Uint8Array(data)], { type: mime })
  // axios 在某些环境可能给 Uint8Array
  if (data instanceof Uint8Array) return new Blob([data], { type: mime })
  return new Blob([data], { type: mime })
}

function triggerBrowserDownload(blob: Blob, filename: string) {
  const url = window.URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  window.URL.revokeObjectURL(url)
}

/** 从错误对象上取一句话，等价于旧写法 `err?.message || err`。 */
function errText(err: unknown): string {
  return err instanceof Error && err.message ? err.message : String(err)
}

export const useSyncStore = defineStore('sync', () => {
  const isExporting = ref(false)
  const isImporting = ref(false)
  const lastError = ref<unknown>(null)

  const env = detectEnv()

  async function exportProducts(): Promise<{ filename: string | null; cancelled?: boolean }> {
    isExporting.value = true
    lastError.value = null

    try {
      // 二进制下载：parseAs blob，数据即以 Blob 返回。
      const result = await api.GET('/sync/export-products', { parseAs: 'blob' })
      if (!result.response.ok || result.data === undefined) {
        throw new ApiRequestError(result.response.status, result.error)
      }
      const disposition = result.response.headers.get('content-disposition') ?? ''
      const filename = parseFilenameFromDisposition(disposition, 'booth_catalog.boothpack')
      const blob = toBlob(result.data, 'application/zip')

      // ✅ Tauri Desktop：保存对话框 + 写文件
      if (env.isTauriDesktop) {
        const [dialogModule, fsModule] = await Promise.all([
          import('@tauri-apps/plugin-dialog'),
          import('@tauri-apps/plugin-fs'),
        ])

        const filePath = await dialogModule.save({
          defaultPath: filename,
          filters: [{ name: 'Booth Pack', extensions: ['boothpack', 'zip'] }],
        })

        if (!filePath) {
          // 用户取消不算“错误”
          return { filename: null, cancelled: true }
        }

        // 把字节写入（parseAs blob 后 data 恒是 Blob）
        const bytes = new Uint8Array(await blob.arrayBuffer())
        await fsModule.writeFile(filePath, bytes)
        return { filename: filePath }
      }

      // ✅ Tauri Mobile：优先尝试 dialog.save（如果可用），否则再走 share/web
      if (env.isTauriMobile) {
        // 1) 尝试用 dialog.save + fs.writeFile（如果你的移动端插件支持）
        try {
          const [dialogModule, fsModule] = await Promise.all([
            import('@tauri-apps/plugin-dialog'),
            import('@tauri-apps/plugin-fs'),
          ])
          const filePath = await dialogModule.save({
            defaultPath: filename,
            filters: [{ name: 'Booth Pack', extensions: ['boothpack', 'zip'] }],
          })
          if (filePath) {
            const bytes = new Uint8Array(await blob.arrayBuffer())
            await fsModule.writeFile(filePath, bytes)
            return { filename: filePath }
          }
          // 若用户取消
          return { filename: null, cancelled: true }
        } catch (e) {
          // 2) 如果移动端不支持 save/writeFile，则尝试 Web Share
          // （注意：Tauri WebView 未必支持 files share）
          console.warn(
            '[export] tauri mobile save/writeFile not available, fallback to share/web',
            e
          )
        }
      }

      // ✅ 浏览器 or 移动端 fallback：优先 Web Share(files) 再下载
      const canUseShare =
        typeof navigator !== 'undefined' &&
        typeof navigator.share === 'function' &&
        typeof navigator.canShare === 'function'

      if (canUseShare) {
        try {
          const file = new File([blob], filename, { type: 'application/zip' })
          if (navigator.canShare({ files: [file] })) {
            await navigator.share({
              files: [file],
              title: 'Booth Tool 制品包',
              text: '导出的制品包，可在设备上保存或分享',
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

      // 原样抛给组件：组件读 ApiRequestError.response / message 自行兜底。
      throw err
    } finally {
      isExporting.value = false
    }
  }

  // 走 raw endpoint：把整个 zip 当字节流直接当 HTTP body 发出去。
  //
  // 注意：旧的 multipart endpoint /sync/import-products 仍然保留在后端，
  // 用于 LAN 浏览器或任何不走 Tauri webview 的客户端，向后兼容。
  //
  // [sync-fe] 前端时序日志，对应后端的 [sync] 标签。
  // 用 performance.now() 拿毫秒时间戳，分阶段打印帮排查"卡 30 秒"问题。
  // 总是打印（不分 dev/release），原因和后端一样：用户是在 release 里遇到 bug。
  function feLog(msg: string) {
    console.log(`[sync-fe] ${msg}`)
  }

  async function postRawZip(uint8: Uint8Array): Promise<Schemas['SyncImportResponse']> {
    const t0 = performance.now()
    feLog(`postRawZip: about to api.post (${uint8.byteLength} bytes)`)
    const response = await unwrap<Schemas['SyncImportResponse']>(
      api.POST('/sync/import-products-raw', {
        // 原始字节：契约把 application/octet-stream 生成为 number[]，
        // 但只有原样发 Uint8Array 才会走 client 的 raw bodySerializer。
        body: uint8 as never,
        headers: { 'Content-Type': 'application/zip' },
      })
    )
    feLog(`postRawZip: api.post returned in ${(performance.now() - t0).toFixed(0)}ms`)
    return response
  }

  async function importProducts(file: File) {
    if (!file) throw new Error('请选择要导入的文件')

    isImporting.value = true
    lastError.value = null
    const t0 = performance.now()
    feLog(`importProducts: start (file=${file.name}, size=${file.size}B)`)
    try {
      const tA = performance.now()
      const buffer = await file.arrayBuffer()
      feLog(`importProducts: file.arrayBuffer() took ${(performance.now() - tA).toFixed(0)}ms`)
      const tB = performance.now()
      const u8 = new Uint8Array(buffer)
      feLog(`importProducts: new Uint8Array() took ${(performance.now() - tB).toFixed(0)}ms`)
      const result = await postRawZip(u8)
      feLog(`importProducts: success, total ${(performance.now() - t0).toFixed(0)}ms`)
      return result
    } catch (err) {
      feLog(
        `importProducts: error after ${(performance.now() - t0).toFixed(0)}ms — ${errText(err)}`
      )
      console.error(err)
      lastError.value = err
      throw err
    } finally {
      isImporting.value = false
    }
  }

  // ✅ 修复：不再调用 importProducts() 以免 isImporting 双重管理
  async function importProductsFromPath(filePath: string) {
    if (!filePath) throw new Error('未提供文件路径')
    if (!env.isTauri) throw new Error('仅在 Tauri 环境可用')

    isImporting.value = true
    lastError.value = null
    const t0 = performance.now()
    feLog(`importProductsFromPath: start (path=${filePath})`)
    try {
      const tA = performance.now()
      const fsModule = await import('@tauri-apps/plugin-fs')
      feLog(
        `importProductsFromPath: import('@tauri-apps/plugin-fs') took ${(performance.now() - tA).toFixed(0)}ms`
      )
      const tB = performance.now()
      const data = await fsModule.readFile(filePath) // 已经是 Uint8Array
      feLog(
        `importProductsFromPath: fs.readFile() took ${(performance.now() - tB).toFixed(0)}ms (size=${data.byteLength}B)`
      )
      // 不再包 Blob、不再包 File、不再 FormData ——直接当 body 发
      const result = await postRawZip(data)
      feLog(`importProductsFromPath: success, total ${(performance.now() - t0).toFixed(0)}ms`)
      return result
    } catch (err) {
      feLog(
        `importProductsFromPath: error after ${(performance.now() - t0).toFixed(0)}ms — ${errText(err)}`
      )
      console.error('[Tauri] importProductsFromPath error:', err)
      lastError.value = err
      throw err
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
    importProductsFromPath,
  }
})
