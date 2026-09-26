import { ref } from 'vue'
import type { Ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { open } from '@tauri-apps/plugin-shell'
import type { Update } from '@tauri-apps/plugin-updater'
import type { Platform } from '@tauri-apps/plugin-os'
import { copyLink } from '@/services/clipboard'

const GITHUB_USER = 'Academy-of-Boundary-Landscape'
const GITHUB_REPO = 'ABL-BoothApp'
const DOWNLOAD_PAGE_URL = `https://github.com/${GITHUB_USER}/${GITHUB_REPO}/releases/latest`

/** 下载进度（字节 + 百分比）。 */
export interface DownloadProgress {
  downloaded: number
  total: number
  percent: number
}

/** `useUpdateCheck` 的返回值。 */
export interface UpdateCheckState {
  loading: Ref<boolean>
  error: Ref<string | null>
  hasUpdate: Ref<boolean>
  currentVersion: Ref<string>
  latestVersion: Ref<string>
  releaseNote: Ref<string>
  releaseDate: Ref<string>
  platform: Ref<string>

  isDownloading: Ref<boolean>
  downloadProgress: Ref<DownloadProgress>
  isInstalled: Ref<boolean>

  checkUpdate: () => Promise<void>
  downloadAndInstall: () => Promise<void>
  restartApp: () => Promise<void>
  goToDownload: () => Promise<void>
  canAutoUpdate: () => Promise<boolean>
}

/** 从外部（GitHub API）拿到的未知 JSON 里安全读一个字符串字段。 */
function readStringField(source: unknown, key: string): string | undefined {
  if (source === null || typeof source !== 'object') return undefined
  const value = (source as Record<string, unknown>)[key]
  return typeof value === 'string' ? value : undefined
}

// Desktop-only modules — lazily loaded, will throw on Android.
async function loadDesktopModules() {
  const [{ check }, { relaunch }] = await Promise.all([
    import('@tauri-apps/plugin-updater'),
    import('@tauri-apps/plugin-process'),
  ])
  return { check, relaunch }
}

async function detectPlatform(): Promise<Platform | 'web'> {
  try {
    const { platform } = await import('@tauri-apps/plugin-os')
    return await platform()
  } catch {
    return 'web'
  }
}

function compareVersions(v1: string, v2: string): number {
  const a = v1.split('.').map(Number)
  const b = v2.split('.').map(Number)
  const len = Math.max(a.length, b.length)
  for (let i = 0; i < len; i++) {
    const x = a[i] || 0
    const y = b[i] || 0
    if (x > y) return 1
    if (x < y) return -1
  }
  return 0
}

export function useUpdateCheck(): UpdateCheckState {
  const loading = ref(false)
  const error = ref<string | null>(null)
  const hasUpdate = ref(false)
  const currentVersion = ref('')
  const latestVersion = ref('')
  const releaseNote = ref('')
  const releaseDate = ref('')
  const platform = ref('')

  // 下载 / 安装状态
  const isDownloading = ref(false)
  const downloadProgress = ref<DownloadProgress>({ downloaded: 0, total: 0, percent: 0 })
  const isInstalled = ref(false)

  // 当 checkUpdate 通过 Tauri updater 拿到新版本时，保存 Update 对象给后续下载用
  let pendingUpdate: Update | null = null

  const isTauri = (): boolean =>
    typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined

  const canAutoUpdate = async (): Promise<boolean> => {
    if (!isTauri()) return false
    const p = await detectPlatform()
    platform.value = p
    // Tauri updater 仅桌面平台可用
    return p === 'windows' || p === 'macos' || p === 'linux'
  }

  // Android/iOS fallback: 沿用旧的 GitHub API 查版本（仅展示，不下载）
  const checkViaGithubApi = async (): Promise<void> => {
    const { fetch } = await import('@tauri-apps/plugin-http')
    const url = `https://api.github.com/repos/${GITHUB_USER}/${GITHUB_REPO}/releases/latest`
    const response = await fetch(url, {
      method: 'GET',
      headers: { 'User-Agent': 'Tauri-App-Updater' },
    })
    if (!response.ok) {
      if (response.status === 403) throw new Error('检查过于频繁，请稍后再试')
      throw new Error(`请求失败: ${response.status} ${response.statusText}`)
    }
    const data: unknown = await response.json()
    const remoteTag = (readStringField(data, 'tag_name') || '').replace(/^v/, '')
    latestVersion.value = remoteTag
    releaseNote.value = readStringField(data, 'body') || '暂无更新日志'
    releaseDate.value = readStringField(data, 'published_at') || ''
    hasUpdate.value = compareVersions(remoteTag, currentVersion.value) === 1
  }

  const checkUpdate = async (): Promise<void> => {
    if (!isTauri()) {
      error.value = '请在 App 环境中运行'
      return
    }

    loading.value = true
    error.value = null
    hasUpdate.value = false
    pendingUpdate = null
    isInstalled.value = false

    try {
      currentVersion.value = await getVersion()

      const auto = await canAutoUpdate()
      if (!auto) {
        await checkViaGithubApi()
        return
      }

      const { check } = await loadDesktopModules()
      const upd = await check()

      if (upd === null) {
        hasUpdate.value = false
        latestVersion.value = currentVersion.value
        return
      }

      pendingUpdate = upd
      hasUpdate.value = true
      latestVersion.value = upd.version
      releaseNote.value = upd.body || '暂无更新日志'
      releaseDate.value = upd.date || ''
    } catch (e) {
      console.error('更新检查出错:', e)
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  const downloadAndInstall = async (): Promise<void> => {
    if (!pendingUpdate) {
      error.value = '没有待下载的更新'
      return
    }
    isDownloading.value = true
    error.value = null
    downloadProgress.value = { downloaded: 0, total: 0, percent: 0 }
    isInstalled.value = false

    try {
      let totalBytes = 0
      let downloadedBytes = 0

      await pendingUpdate.downloadAndInstall((ev) => {
        switch (ev.event) {
          case 'Started':
            totalBytes = ev.data?.contentLength || 0
            downloadedBytes = 0
            downloadProgress.value = {
              downloaded: 0,
              total: totalBytes,
              percent: 0,
            }
            break
          case 'Progress':
            downloadedBytes += ev.data?.chunkLength || 0
            downloadProgress.value = {
              downloaded: downloadedBytes,
              total: totalBytes,
              percent:
                totalBytes > 0
                  ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100))
                  : 0,
            }
            break
          case 'Finished':
            downloadProgress.value = {
              downloaded: totalBytes,
              total: totalBytes,
              percent: 100,
            }
            break
          default:
            break
        }
      })
      isInstalled.value = true
    } catch (e) {
      console.error('下载/安装失败:', e)
      error.value = e instanceof Error ? e.message : String(e)
      isInstalled.value = false
    } finally {
      isDownloading.value = false
    }
  }

  const restartApp = async (): Promise<void> => {
    const { relaunch } = await loadDesktopModules()
    await relaunch()
  }

  const goToDownload = async (): Promise<void> => {
    try {
      await copyLink(DOWNLOAD_PAGE_URL)
    } catch (e) {
      console.error('复制链接失败:', e)
    }
    if (isTauri()) {
      await open(DOWNLOAD_PAGE_URL)
    } else {
      window.open(DOWNLOAD_PAGE_URL, '_blank')
    }
  }

  return {
    loading,
    error,
    hasUpdate,
    currentVersion,
    latestVersion,
    releaseNote,
    releaseDate,
    platform,

    isDownloading,
    downloadProgress,
    isInstalled,

    checkUpdate,
    downloadAndInstall,
    restartApp,
    goToDownload,
    canAutoUpdate,
  }
}
