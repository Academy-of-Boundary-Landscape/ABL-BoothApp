<template>
  <AppModal
    v-model:show="visible"
    title="v1.2 更新了账本模型"
    size="sm"
    :mask-closable="false"
    :close-on-esc="false"
    :closable="false"
  >
    <div class="migration-box">
      <p>
        旧版的展会和订单记录没有迁移到新模型——新模型要记录每一件货的来源和去向，
        旧数据补不出这些信息。
      </p>
      <p>
        你的旧数据<strong>完整保留</strong>在
        <code>sale_system.db.v1-backup</code>，随时可以导出。
        商品库（含图片和识别数据）已经自动带过来了。
      </p>
    </div>
    <template #footer>
      <div class="migration-actions">
        <button class="export-btn" :disabled="exporting" @click="exportLegacy">
          {{ exporting ? '导出中…' : '导出旧数据为 Excel' }}
        </button>
        <button class="ok-btn" @click="dismiss">知道了</button>
      </div>
    </template>
  </AppModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAuthStore } from '@/stores/authStore'
import { api, unwrap } from '@/api/client'
import { MIGRATION_NOTICE_SEEN_KEY, shouldShowMigrationNotice } from '@/utils/migrationNotice'
import { exportLegacyXlsx } from '@/utils/legacyExport'
import { AppModal } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'

const authStore = useAuthStore()
const visible = ref(false)
const exporting = ref(false)
const fb = useFeedback()

/**
 * 探测有没有 v1 历史数据。
 *
 * 只在管理员登录态下发请求：`/api/legacy/status` 是 AdminOnly（历史数据出口不该对
 * 顾客端开放），而 App.vue 在**所有**路由下都挂着这个组件——顾客首页 `/` 是公开的，
 * 若不加判断就发请求，401/403 会被 API client 的中间件当成会话失效，把顾客踢去登录页。
 */
async function checkStatus() {
  if (!authStore.isAdmin) {
    visible.value = false
    return
  }

  const seen = !!localStorage.getItem(MIGRATION_NOTICE_SEEN_KEY)
  if (seen) return

  try {
    const data = await unwrap(api.GET('/legacy/status'))
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
 * 实际下载逻辑在 `utils/legacyExport.js`，和设置页 LegacyDataSettings 的「历史数据（v1）」
 * 常驻入口共用——这个弹窗只负责第一次提醒，不是唯一出口。
 */
async function exportLegacy() {
  exporting.value = true
  try {
    const ok = await exportLegacyXlsx()
    if (ok) fb.success('导出成功')
  } catch (e) {
    console.error('下载旧数据失败:', e)
    fb.error(e, '下载失败')
  } finally {
    exporting.value = false
  }
}
</script>

<style scoped>
.migration-box {
  color: var(--primary-text-color);
}

.migration-box p {
  margin: 0 0 var(--space-lg);
  font-size: var(--font-md);
  line-height: 1.7;
}

.migration-box code {
  background-color: var(--bg-elevated);
  padding: var(--space-xs);
  border-radius: var(--radius-sm);
}

.migration-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-md);
  margin-top: var(--space-lg);
}

.export-btn,
.ok-btn {
  border: none;
  padding: var(--space-sm) var(--space-lg);
  border-radius: var(--radius-sm);
  font-weight: var(--weight-bold);
  cursor: pointer;
  transition: background-color 0.2s;
}

.export-btn {
  background-color: var(--info-color);
  color: var(--text-white);
}
.export-btn:hover:not(:disabled) {
  background-color: var(--info-color);
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
</style>
