<template>
  <div class="sync-container">
    <div class="section-header" @click="isCollapsed = !isCollapsed">
      <h2>商品数据包（.boothpack）</h2>
      <n-button text class="toggle-btn">
        {{ isCollapsed ? '展开' : '折叠' }}
      </n-button>
    </div>

    <transition name="expand">
      <div v-show="!isCollapsed" class="section-body">
        <n-alert type="info" :bordered="false" class="info-alert">
          <div class="info-text">
            你可以导出当前商品库为 <code>.boothpack</code> 备份，也可以在其他设备导入。
            <br />
            <strong>注意：</strong>导入会覆盖同编号商品，建议先导出当前数据做备份。
          </div>
        </n-alert>

        <n-alert
          v-if="syncMessage"
          type="success"
          :bordered="false"
          class="sync-alert"
          closable
          @close="syncMessage = ''"
        >
          {{ syncMessage }}
        </n-alert>

        <n-alert
          v-if="syncError"
          type="error"
          :bordered="false"
          class="sync-alert"
          closable
          @close="syncError = ''"
        >
          {{ syncError }}
        </n-alert>

        <div class="sync-controls">
          <n-button size="large" type="success" :loading="isExporting" @click="handleExport">
            导出 .boothpack
          </n-button>

          <n-button size="large" type="info" :loading="isImporting" @click="triggerImport">
            导入 .boothpack
          </n-button>

          <input
            ref="importFileInputRef"
            type="file"
            class="hidden-input"
            accept=".boothpack,.zip"
            @change="handleImportFile"
          />
        </div>

        <div v-if="isImporting" class="import-progress">
          <div class="import-progress-status">{{ importStatus }}</div>
          <n-progress
            type="line"
            :percentage="importProgress"
            :show-indicator="true"
            :status="importProgress >= 100 ? 'success' : 'default'"
            :height="20"
            :border-radius="8"
          />
        </div>

        <div
          class="drop-zone"
          :class="{ 'is-dragging': isDragging }"
          @dragenter.prevent="onDragEnter"
          @dragover.prevent="onDragOver"
          @dragleave.prevent="onDragLeave"
          @drop.prevent="onDrop"
        >
          <div class="drop-zone-content">
            <span class="drop-zone-icon">拖</span>
            <span class="drop-zone-text">把 .boothpack 或 .zip 文件拖到这里导入</span>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup>
import { computed, h, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { NAlert, NButton, NProgress, useDialog, useMessage } from 'naive-ui'

import { useSyncStore } from '@/stores/syncStore'
import {
  SYNC_IMPORT_LIMIT_MB,
  normalizeUploadError,
  showUploadDialog,
  validateFileSize,
} from '@/utils/upload'

const emit = defineEmits(['imported'])

const syncStore = useSyncStore()
const dialog = useDialog()
const message = useMessage()

const isCollapsed = ref(false)
const importFileInputRef = ref(null)
const syncMessage = ref('')
const syncError = ref('')
const isDragging = ref(false)

const isExporting = computed(() => syncStore.isExporting)
const isImporting = computed(() => syncStore.isImporting)
const importProgress = ref(0)
const importStatus = ref('')
let importProgressTimer = null

function startImportProgress() {
  importProgress.value = 5
  importStatus.value = '正在上传文件...'
  let tick = 0
  importProgressTimer = setInterval(() => {
    tick++
    if (tick <= 3) {
      importProgress.value = Math.min(30, 5 + tick * 8)
      importStatus.value = '正在上传文件...'
    } else if (tick <= 8) {
      importProgress.value = Math.min(70, 30 + (tick - 3) * 8)
      importStatus.value = '正在解压并写入图片...'
    } else {
      importProgress.value = Math.min(90, 70 + (tick - 8) * 3)
      importStatus.value = '正在写入商品数据...'
    }
  }, 800)
}

function stopImportProgress(success) {
  clearInterval(importProgressTimer)
  importProgressTimer = null
  if (success) {
    importProgress.value = 100
    importStatus.value = '导入完成'
  } else {
    importProgress.value = 0
    importStatus.value = ''
  }
}

watch(isImporting, (val) => {
  if (!val && importProgressTimer) {
    stopImportProgress(false)
  }
})

let dragCounter = 0
let tauriUnlisten = null
let globalDropCleanup = null

function clearSyncHints() {
  syncMessage.value = ''
  syncError.value = ''
}

function isAllowedPackName(name) {
  const lowered = String(name || '').toLowerCase()
  return lowered.endsWith('.boothpack') || lowered.endsWith('.zip')
}

function rejectInvalidFile(name) {
  syncError.value = '请选择 .boothpack 或 .zip 文件'
  showUploadDialog('文件类型不支持', `文件“${name || 'unknown'}”不是有效的 .boothpack/.zip 数据包。`)
}

function validatePackFile(file, displayName) {
  if (!isAllowedPackName(displayName)) {
    rejectInvalidFile(displayName)
    return false
  }

  const validation = validateFileSize(file, SYNC_IMPORT_LIMIT_MB)
  if (!validation.ok) {
    syncError.value = validation.message
    showUploadDialog('导入文件过大', validation.message)
    return false
  }

  return true
}

function triggerImport() {
  clearSyncHints()
  importFileInputRef.value?.click?.()
}

async function handleImportFile(event) {
  const file = event.target?.files?.[0]
  if (!file) return
  if (validatePackFile(file, file.name)) {
    await confirmAndImport({ kind: 'file', file, displayName: file.name })
  }
  event.target.value = ''
}

async function handleExport() {
  clearSyncHints()
  try {
    const { filename } = await syncStore.exportProducts()
    syncMessage.value = filename ? `已导出：${filename}` : '已取消导出'
    if (filename) {
      message.success(`已成功导出商品包：${filename}`, { duration: 5000, closable: true })
    }
  } catch (error) {
    const msg = error?.message || '导出失败'
    syncError.value = msg
    message.error(`导出失败：${msg}`, { duration: 5000, closable: true })
  }
}

async function confirmAndImport({ kind, file, path, displayName }) {
  const name =
    displayName ||
    (kind === 'path' ? String(path).split(/[/\\]/).pop() : file?.name) ||
    'unknown'

  dialog.warning({
    title: kind === 'path' ? '检测到文件拖入' : '确认导入',
    content: () =>
      h('div', { style: 'white-space: pre-line;' }, [
        `文件名：${name}`,
        '\n\n',
        '确认要导入吗？这会覆盖或更新现有商品数据。',
        '\n',
        '建议先导出当前数据作为备份。',
      ]),
    positiveText: '确认导入',
    negativeText: '取消',
    onPositiveClick: async () => {
      if (isImporting.value) {
        message.info('正在导入中，请稍候', { duration: 2000, closable: true })
        return false
      }

      clearSyncHints()
      startImportProgress()
      try {
        const result =
          kind === 'file'
            ? await syncStore.importProducts(file)
            : await syncStore.importProductsFromPath(path)

        stopImportProgress(true)
        const pCount = result?.products_count ?? 0
        const iCount = result?.images_count ?? 0
        syncMessage.value = `导入成功，更新了 ${pCount} 条商品、${iCount} 张图片。`
        message.success(`导入成功，已更新 ${pCount} 条商品、${iCount} 张图片`, {
          duration: 5000,
          closable: true,
        })
        emit('imported')
      } catch (error) {
        stopImportProgress(false)
        syncError.value = normalizeUploadError(error, SYNC_IMPORT_LIMIT_MB)
      }
    },
  })
}

function onDragEnter(event) {
  event.stopPropagation()
  dragCounter += 1
  isDragging.value = true
}

function onDragOver() {
  isDragging.value = true
}

function onDragLeave(event) {
  event.stopPropagation()
  dragCounter = Math.max(0, dragCounter - 1)
  if (dragCounter === 0) isDragging.value = false
}

async function onDrop(event) {
  dragCounter = 0
  isDragging.value = false

  const file = event.dataTransfer?.files?.[0]
  if (!file) return
  if (!validatePackFile(file, file.name)) return
  await confirmAndImport({ kind: 'file', file, displayName: file.name })
}

onMounted(async () => {
  if (window.__TAURI_INTERNALS__ === undefined) return

  try {
    const { listen } = await import('@tauri-apps/api/event')
    tauriUnlisten = await listen('boothpack-file-drop', async (event) => {
      const paths = Array.isArray(event.payload) ? event.payload : []
      const path = paths[0]
      if (!path) return

      const name = String(path).split(/[/\\]/).pop() || 'unknown'
      if (!isAllowedPackName(name)) {
        rejectInvalidFile(name)
        return
      }

      await confirmAndImport({ kind: 'path', path, displayName: name })
    })
  } catch (error) {
    console.warn('failed to register tauri drag-drop listener', error)
  }

  const preventDefault = (event) => {
    event.preventDefault()
  }

  window.addEventListener('dragover', preventDefault)
  window.addEventListener('drop', preventDefault)
  globalDropCleanup = () => {
    window.removeEventListener('dragover', preventDefault)
    window.removeEventListener('drop', preventDefault)
  }
})

onBeforeUnmount(() => {
  clearInterval(importProgressTimer)
  if (typeof tauriUnlisten === 'function') {
    tauriUnlisten()
  }
  if (typeof globalDropCleanup === 'function') {
    globalDropCleanup()
  }
})
</script>

<style scoped>
.sync-container {
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  user-select: none;
  padding: 0.75rem 1rem;
}

.section-header h2 {
  margin: 0;
  color: var(--accent-color);
  font-size: var(--font-lg);
}

.toggle-btn {
  color: var(--accent-color);
}

.section-body {
  padding: 1rem;
  border-top: 2px solid var(--border-color);
}

.sync-controls {
  display: flex;
  gap: 12px;
  margin: 1rem 0;
}

.hidden-input {
  display: none;
}

.drop-zone {
  border: 2px dashed var(--border-color);
  border-radius: var(--radius-md);
  padding: 1.25rem;
  transition: border-color 0.2s ease, background-color 0.2s ease;
}

.drop-zone.is-dragging {
  border-color: var(--accent-color);
  background: var(--hover-bg-color);
}

.drop-zone-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  color: var(--text-muted);
}

.drop-zone-icon {
  font-size: var(--font-lg);
}

.import-progress {
  margin: 1rem 0;
  padding: 0.75rem 1rem;
  background: var(--hover-bg-color);
  border-radius: var(--radius-md);
}

.import-progress-status {
  font-size: var(--font-sm, 13px);
  color: var(--text-muted);
  margin-bottom: 0.5rem;
}

.sync-alert {
  margin-top: 0.75rem;
}

.expand-enter-active,
.expand-leave-active {
  transition: all 0.3s ease;
  overflow: hidden;
}

.expand-enter-from,
.expand-leave-to {
  max-height: 0;
  opacity: 0;
}

.expand-enter-to,
.expand-leave-from {
  max-height: 1000px;
  opacity: 1;
}
</style>
