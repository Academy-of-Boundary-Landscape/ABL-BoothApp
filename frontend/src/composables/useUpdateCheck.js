import { ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { open } from '@tauri-apps/plugin-shell'
import { copyLink } from '@/services/clipboard'

const GITHUB_USER = 'Academy-of-Boundary-Landscape'
const GITHUB_REPO = 'ABL-BoothApp'
const DOWNLOAD_PAGE_URL = `https://github.com/${GITHUB_USER}/${GITHUB_REPO}/releases/latest`

// Desktop-only modules — lazily loaded, will throw on Android.
async function loadDesktopModules() {
  const [{ check }, { relaunch }] = await Promise.all([
    import('@tauri-apps/plugin-updater'),
    import('@tauri-apps/plugin-process'),
  ])
  return { check, relaunch }
}

async function detectPlatform() {
  try {
    const { platform } = await import('@tauri-apps/plugin-os')
    return await platform()
  } catch {
    return 'web'
  }
}

function compareVersions(v1, v2) {
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

export function useUpdateCheck() {
  const loading = ref(false)
  const error = ref(null)
  const hasUpdate = ref(false)
  const currentVersion = ref('')
  const latestVersion = ref('')
  const releaseNote = ref('')
  const releaseDate = ref('')
  const platform = ref('')

  // 下载 / 安装状态
  const isDownloading = ref(false)
  const downloadProgress = ref({ downloaded: 0, total: 0, percent: 0 })
  const isInstalled = ref(false)

  // 当 checkUpdate 通过 Tauri updater 拿到新版本时，保存 Update 对象给后续下载用
  let pendingUpdate = null

  const isTauri = () => typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined

  const canAutoUpdate = async () => {
    if (!isTauri()) return false
    const p = await detectPlatform()
    platform.value = p
    // Tauri updater 仅桌面平台可用
    return p === 'windows' || p === 'macos' || p === 'linux'
  }

  // Android/iOS fallback: 沿用旧的 GitHub API 查版本（仅展示，不下载）
  const checkViaGithubApi = async () => {
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
    const data = await response.json()
    const remoteTag = (data.tag_name || '').replace(/^v/, '')
    latestVersion.value = remoteTag
    releaseNote.value = data.body || '暂无更新日志'
    releaseDate.value = data.published_at || ''
    hasUpdate.value = compareVersions(remoteTag, currentVersion.value) === 1
  }

  const checkUpdate = async () => {
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

  const downloadAndInstall = async () => {
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

  const restartApp = async () => {
    const { relaunch } = await loadDesktopModules()
    await relaunch()
  }

  const goToDownload = async () => {
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
