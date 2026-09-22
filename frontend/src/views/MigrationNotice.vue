<template>
  <Transition name="migration-fade">
    <div v-if="visible" class="migration-overlay">
      <div class="migration-box">
        <h3>v1.2 更新了账本模型</h3>
        <p>
          旧版的展会和订单记录没有迁移到新模型——新模型要记录每一件货的来源和去向，
          旧数据补不出这些信息。
        </p>
        <p>
          你的旧数据<strong>完整保留</strong>在
          <code>sale_system.db.v1-backup</code>，随时可以导出。
          商品库（含图片和识别数据）已经自动带过来了。
        </p>
        <div class="migration-actions">
          <button class="export-btn" :disabled="exporting" @click="exportLegacy">
            {{ exporting ? '导出中…' : '导出旧数据为 Excel' }}
          </button>
          <button class="ok-btn" @click="dismiss">知道了</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/authStore'
import api from '@/services/api'
import { toAbsoluteApiUrl } from '@/services/url'
import { MIGRATION_NOTICE_SEEN_KEY, shouldShowMigrationNotice } from '@/utils/migrationNotice'
import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

const authStore = useAuthStore()
const visible = ref(false)
const exporting = ref(false)

/**
 * 探测有没有 v1 历史数据。
 *
 * 只在管理员登录态下发请求：`/api/legacy/status` 是 AdminOnly（历史数据出口不该对
 * 顾客端开放），而 App.vue 在**所有**路由下都挂着这个组件——顾客首页 `/` 是公开的，
 * 若不加判断就发请求，401/403 会被 axios 全局拦截器当成会话失效，把顾客踢去登录页。
 */
async function checkStatus() {
  if (!authStore.isAdmin) {
    visible.value = false
    return
  }

  const seen = !!localStorage.getItem(MIGRATION_NOTICE_SEEN_KEY)
  if (seen) return

  try {
    const { data } = await api.get('/legacy/status')
    if (shouldShowMigrationNotice(data, seen)) {
      // 「弹过之后」立刻标记：这份 v1 备份不会再变，提示只打扰一次。
      localStorage.setItem(MIGRATION_NOTICE_SEEN_KEY, '1')
      visible.value = true
    }
  } catch (e) {
    // 探测失败只是「这次不提示」，绝不能影响主流程
    console.warn('[MigrationNotice] 读取历史数据状态失败', e)
  }
}

// immediate 覆盖「组件挂载时就已是管理员」；watch 覆盖「先打开首页、之后才登录」。
watch(() => authStore.isAdmin, checkStatus, { immediate: true })

function dismiss() {
  visible.value = false
}

/**
 * 下载旧数据 xlsx。
 *
 * 照抄 AdminEventStat.vue 的既有写法：手动拼绝对 URL、手动加 Authorization 头、
 * Tauri 走 tauriFetch 浏览器走 fetch。不能走 axios 实例——它在 Tauri 下用的是
 * 自定义 adapter，二进制下载不适合经过它。
 */
async function exportLegacy() {
  const isTauri = window.__TAURI_INTERNALS__ !== undefined
  const token = sessionStorage.getItem('access_token')
  const fileName = 'legacy_v1_export.xlsx'
  const url = toAbsoluteApiUrl('/api/legacy/export.xlsx')

  exporting.value = true
  try {
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
        defaultPath: fileName,
        filters: [{ name: 'Excel Files', extensions: ['xlsx'] }],
      })
      if (!filePath) return

      await writeFile(filePath, bytes)
      alert('导出成功')
      return
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
    a.download = fileName
    document.body.appendChild(a)
    a.click()
    setTimeout(() => {
      document.body.removeChild(a)
      window.URL.revokeObjectURL(dl)
    }, 100)
  } catch (e) {
    console.error('下载旧数据失败:', e)
    alert(e?.message || '下载失败')
  } finally {
    exporting.value = false
  }
}
</script>

<style scoped>
.migration-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: var(--overlay-color);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 999998;
}

.migration-box {
  background-color: var(--alert-bg);
  color: var(--primary-text-color);
  border-radius: var(--radius-md);
  width: 90%;
  max-width: 460px;
  padding: 1.5rem;
  box-shadow: var(--shadow-xl);
  border-top: 4px solid var(--info-color);
}

.migration-box h3 {
  margin: 0 0 1rem;
  font-size: var(--font-lg);
  font-weight: 600;
}

.migration-box p {
  margin: 0 0 1rem;
  font-size: var(--font-md);
  line-height: 1.7;
}

.migration-box code {
  background-color: var(--bg-elevated);
  padding: 0.1rem 0.3rem;
  border-radius: var(--radius-sm);
}

.migration-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-top: 1.25rem;
}

.export-btn,
.ok-btn {
  border: none;
  padding: 0.65rem 1.25rem;
  border-radius: var(--radius-sm);
  font-weight: bold;
  cursor: pointer;
  transition: background-color 0.2s;
}

.export-btn {
  background-color: var(--info-color);
  color: var(--text-white);
}
.export-btn:hover:not(:disabled) {
  background-color: var(--info-color-hover);
}
.export-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.ok-btn {
  background-color: var(--bg-elevated);
  color: var(--primary-text-color);
}
.ok-btn:hover {
  background-color: var(--border-color-light);
}

.migration-fade-enter-active,
.migration-fade-leave-active {
  transition: opacity 0.3s ease;
}
.migration-fade-enter-from,
.migration-fade-leave-to {
  opacity: 0;
}
</style>
