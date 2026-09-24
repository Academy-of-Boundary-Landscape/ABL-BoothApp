<template>
  <SectionCard title="商品数据包（.boothpack）" collapsible v-model:collapsed="isCollapsed">
    <template #extra>
      <n-button text class="toggle-btn" @click="isCollapsed = !isCollapsed">
        {{ isCollapsed ? '展开' : '折叠' }}
      </n-button>
    </template>

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
        accept=".boothpack,.zip,application/zip,application/octet-stream,application/x-zip-compressed"
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
  </SectionCard>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { NAlert, NButton, NProgress } from 'naive-ui'
import { SectionCard } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'

import { useSyncStore } from '@/stores/syncStore'
import {
  SYNC_IMPORT_LIMIT_MB,
  normalizeUploadError,
  showUploadDialog,
  validateFileSize,
} from '@/utils/upload'

const emit = defineEmits<{ (e: 'imported'): void }>()

const syncStore = useSyncStore()
const fb = useFeedback()

const isCollapsed = ref(false)
const importFileInputRef = ref<HTMLInputElement | null>(null)
const syncMessage = ref('')
const syncError = ref('')
const isDragging = ref(false)

const isExporting = computed(() => syncStore.isExporting)
const isImporting = computed(() => syncStore.isImporting)
const importProgress = ref(0)
const importStatus = ref('')
let importProgressTimer: ReturnType<typeof setInterval> | null = null

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

function stopImportProgress(success: boolean) {
  if (importProgressTimer !== null) clearInterval(importProgressTimer)
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
let tauriUnlisten: (() => void) | null = null
let globalDropCleanup: (() => void) | null = null

function clearSyncHints() {
  syncMessage.value = ''
  syncError.value = ''
}

function isAllowedPackName(name: string) {
  const lowered = String(name || '').toLowerCase()
  return lowered.endsWith('.boothpack') || lowered.endsWith('.zip')
}

function rejectInvalidFile(name: string) {
  syncError.value = '请选择 .boothpack 或 .zip 文件'
  showUploadDialog(
    '文件类型不支持',
    `文件“${name || 'unknown'}”不是有效的 .boothpack/.zip 数据包。`
  )
}

function validatePackFile(file: File, displayName: string) {
  if (!isAllowedPackName(displayName)) {
    rejectInvalidFile(displayName)
    return false
  }

  const validation = validateFileSize(file, SYNC_IMPORT_LIMIT_MB)
  if (!validation.ok) {
    const validationMessage = validation.message ?? ''
    syncError.value = validationMessage
    showUploadDialog('导入文件过大', validationMessage)
    return false
  }

  return true
}

function triggerImport() {
  clearSyncHints()
  importFileInputRef.value?.click?.()
}

async function handleImportFile(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  if (validatePackFile(file, file.name)) {
    await confirmAndImport({ kind: 'file', file, displayName: file.name })
  }
  input.value = ''
}

async function handleExport() {
  clearSyncHints()
  try {
    const { filename } = await syncStore.exportProducts()
    syncMessage.value = filename ? `已导出：${filename}` : '已取消导出'
    if (filename) {
      fb.success(`已成功导出商品包：${filename}`)
    }
  } catch (error) {
    const msg = error instanceof Error && error.message ? error.message : '导出失败'
    syncError.value = msg
    fb.error(`导出失败：${msg}`)
  }
}

type ImportTarget =
  | { kind: 'file'; file: File; displayName: string }
  | { kind: 'path'; path: string; displayName: string }

function runImport(target: ImportTarget) {
  return target.kind === 'file'
    ? syncStore.importProducts(target.file)
    : syncStore.importProductsFromPath(target.path)
}

async function confirmAndImport(target: ImportTarget) {
  const name =
    target.displayName ||
    (target.kind === 'path' ? String(target.path).split(/[/\\]/).pop() : target.file?.name) ||
    'unknown'

  const confirmed = await fb.confirm({
    title: target.kind === 'path' ? '检测到文件拖入' : '确认导入',
    content: `文件名：${name}\n\n确认要导入吗？这会覆盖或更新现有商品数据。\n建议先导出当前数据作为备份。`,
    positiveText: '确认导入',
    negativeText: '取消',
  })
  if (!confirmed) return

  if (isImporting.value) {
    fb.info('正在导入中，请稍候')
    return
  }

  clearSyncHints()
  startImportProgress()
  try {
    const result = await runImport(target)

    stopImportProgress(true)
    const pCount = result?.products_count ?? 0
    const iCount = result?.images_count ?? 0
    syncMessage.value = `导入成功，更新了 ${pCount} 条商品、${iCount} 张图片。`
    fb.success(`导入成功，已更新 ${pCount} 条商品、${iCount} 张图片`)
    emit('imported')
  } catch (error) {
    stopImportProgress(false)
    syncError.value = normalizeUploadError(error, SYNC_IMPORT_LIMIT_MB)
  }
}

function onDragEnter(event: DragEvent) {
  event.stopPropagation()
  dragCounter += 1
  isDragging.value = true
}

function onDragOver() {
  isDragging.value = true
}

function onDragLeave(event: DragEvent) {
  event.stopPropagation()
  dragCounter = Math.max(0, dragCounter - 1)
  if (dragCounter === 0) isDragging.value = false
}

async function onDrop(event: DragEvent) {
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
      const paths = Array.isArray(event.payload) ? (event.payload as unknown[]) : []
      const path = paths[0]
      if (!path) return

      const name = String(path).split(/[/\\]/).pop() || 'unknown'
      if (!isAllowedPackName(name)) {
        rejectInvalidFile(name)
        return
      }

      await confirmAndImport({ kind: 'path', path: String(path), displayName: name })
    })
  } catch (error) {
    console.warn('failed to register tauri drag-drop listener', error)
  }

  const preventDefault = (event: Event) => {
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
  if (importProgressTimer !== null) clearInterval(importProgressTimer)
  if (typeof tauriUnlisten === 'function') {
    tauriUnlisten()
  }
  if (typeof globalDropCleanup === 'function') {
    globalDropCleanup()
  }
})
</script>

<style scoped>
.toggle-btn {
  color: var(--accent-color);
}

.sync-controls {
  display: flex;
  gap: var(--space-md);
  margin: var(--space-lg) 0;
}

.hidden-input {
  display: none;
}

.drop-zone {
  border: 2px dashed var(--border-color);
  border-radius: var(--radius-md);
  padding: var(--space-lg);
  transition:
    border-color 0.2s ease,
    background-color 0.2s ease;
}

.drop-zone.is-dragging {
  border-color: var(--accent-color);
  background: var(--hover-bg-color);
}

.drop-zone-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-sm);
  color: var(--text-muted);
}

.drop-zone-icon {
  font-size: var(--font-lg);
}

.import-progress {
  margin: var(--space-lg) 0;
  padding: var(--space-sm) var(--space-lg);
  background: var(--hover-bg-color);
  border-radius: var(--radius-md);
}

.import-progress-status {
  font-size: var(--font-sm);
  color: var(--text-muted);
  margin-bottom: var(--space-sm);
}

.sync-alert {
  margin-top: var(--space-md);
}
</style>
