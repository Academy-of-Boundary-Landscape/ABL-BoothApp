<template>
  <n-modal
    v-model:show="showModal"
    preset="card"
    title="检查更新"
    style="max-width: 400px"
    :mask-closable="!loading && isTauriEnv"
    :close-on-esc="!loading && isTauriEnv"
    @after-enter="handleEnter"
  >
    <!-- 状态 0: 浏览器环境 -->
    <div v-if="!isTauriEnv" class="state-container">
      <n-result
        status="info"
        title="功能不可用"
        description="您正在使用浏览器访问，检查更新功能仅在客户端 App 内可用。"
      >
        <template #footer>
          <n-button @click="close">好的</n-button>
        </template>
      </n-result>
    </div>

    <!-- 状态 1: 加载中 -->
    <div v-else-if="loading" class="state-container">
      <n-spin size="large" />
      <p class="mt-4 text-muted">正在获取最新版本信息...</p>
    </div>

    <!-- 状态 2: 出错 -->
    <div v-else-if="error" class="state-container">
      <n-result status="warning" title="检查失败" :description="error">
        <template #footer>
          <n-space justify="center">
            <n-button @click="retry">重试</n-button>
            <n-button type="primary" @click="handleDownload">前往发布页手动下载</n-button>
          </n-space>
        </template>
      </n-result>
      <p class="error-hint">
        若反复失败，可能是网络问题或本版本暂未配置自动更新清单。 点击「前往发布页」直接到 GitHub
        看最新版本。
      </p>
    </div>

    <!-- 状态 3: 已经是最新版 -->
    <div v-else-if="!hasUpdate" class="state-container">
      <n-result
        status="success"
        title="当前已是最新版本"
        :description="`版本号: v${currentVersion}`"
      >
        <template #footer>
          <n-button @click="close">关闭</n-button>
        </template>
      </n-result>
    </div>

    <!-- 状态 4: 发现新版本 -->
    <div v-else class="update-content">
      <div class="header-section">
        <n-tag type="success" size="large">新版本 v{{ latestVersion }}</n-tag>
        <span class="date">{{ formatDate(releaseDate) }}</span>
      </div>

      <div class="current-ver-tip">当前版本: v{{ currentVersion }}</div>

      <n-divider title-placement="left" style="margin: 12px 0">更新内容</n-divider>

      <n-scrollbar style="max-height: 200px" class="log-scroll">
        <div class="release-note">{{ releaseNote }}</div>
      </n-scrollbar>

      <!-- 下载进度 -->
      <div v-if="isDownloading" class="progress-section">
        <n-progress
          type="line"
          :percentage="downloadProgress.percent"
          indicator-placement="inside"
        />
        <p class="progress-text">
          正在下载 {{ formatBytes(downloadProgress.downloaded) }} /
          {{ formatBytes(downloadProgress.total) }}
        </p>
      </div>

      <!-- 安装完成提示 -->
      <div v-else-if="isInstalled" class="installed-hint">
        <n-alert type="success" :bordered="false">
          新版本已安装完成。点击「立即重启」以完成更新。
        </n-alert>
      </div>

      <div class="actions">
        <template v-if="isInstalled">
          <n-button @click="close" ghost>稍后重启</n-button>
          <n-button type="primary" @click="confirmRestart">立即重启</n-button>
        </template>
        <template v-else-if="isDownloading">
          <n-button disabled>下载中... {{ downloadProgress.percent }}%</n-button>
        </template>
        <template v-else>
          <n-button @click="close" ghost>暂不更新</n-button>
          <n-button v-if="canAuto" type="primary" @click="handleAutoInstall"> 下载并安装 </n-button>
          <n-button v-else type="primary" @click="handleDownload"> 前往下载 </n-button>
        </template>
      </div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useUpdateCheck } from '@/composables/useUpdateCheck'
import {
  NModal,
  NSpin,
  NResult,
  NButton,
  NTag,
  NDivider,
  NScrollbar,
  NAlert,
  NProgress,
  NSpace,
  useDialog,
} from 'naive-ui'

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ (e: 'update:show', v: boolean): void }>()

const isTauriEnv = ref(typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined)
const dialog = useDialog()

const showModal = computed({
  get: () => props.show,
  set: (val) => emit('update:show', val),
})

const {
  loading,
  error,
  hasUpdate,
  currentVersion,
  latestVersion,
  releaseNote,
  releaseDate,

  isDownloading,
  downloadProgress,
  isInstalled,

  checkUpdate,
  downloadAndInstall,
  restartApp,
  goToDownload,
  canAutoUpdate,
} = useUpdateCheck()

const canAuto = ref(false)

onMounted(async () => {
  if (isTauriEnv.value) {
    canAuto.value = await canAutoUpdate()
  }
})

const handleEnter = () => {
  if (isTauriEnv.value) {
    checkUpdate()
  }
}

const retry = () => {
  if (isTauriEnv.value) {
    checkUpdate()
  }
}

const close = () => {
  showModal.value = false
}

const handleDownload = () => {
  goToDownload()
}

const handleAutoInstall = async () => {
  await downloadAndInstall()
  // 成功时 isInstalled 变 true，UI 自动切换到"立即重启"。
  // 失败时 error.value 已被设置；UI 会回退到错误分支让用户重试。
}

const confirmRestart = () => {
  dialog.warning({
    title: '即将重启摊盒',
    content: '重启会关闭应用以完成安装。请确认当前没有未保存的订单或编辑。继续吗？',
    positiveText: '确认重启',
    negativeText: '再等等',
    onPositiveClick: async () => {
      await restartApp()
    },
  })
}

const formatDate = (dateStr: string) => {
  if (!dateStr) return ''
  return new Date(dateStr).toLocaleDateString()
}

const formatBytes = (bytes: number) => {
  if (!bytes || bytes < 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let i = 0
  let n = bytes
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024
    i += 1
  }
  return `${n.toFixed(i === 0 ? 0 : 1)} ${units[i]}`
}
</script>

<style scoped>
.state-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 2rem 1rem;
  min-height: 220px;
}

.text-muted {
  color: var(--text-muted);
  font-size: var(--font-base);
  margin-top: 1rem;
}

.update-content {
  padding: 0.5rem 0;
}

.header-section {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}

.date {
  font-size: var(--font-sm);
  color: var(--text-muted);
}

.current-ver-tip {
  font-size: var(--font-sm);
  color: var(--secondary-text-color);
  margin-top: 1rem;
  padding: 0.5rem 0.75rem;
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  border-left: 4px solid var(--accent-color);
}

.log-scroll {
  margin: 0.5rem 0;
  border: 1px solid var(--divider-color, rgba(255, 255, 255, 0.09));
  border-radius: var(--radius-md);
}

.release-note {
  white-space: pre-wrap;
  line-height: 1.6;
  padding: 1rem;
  font-size: var(--font-base);
  color: var(--text-color-1, inherit);
  background: var(--card-color, transparent);
  word-break: break-word;
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-top: 1.5rem;
}

/* 移动端适配 */
@media (max-width: 600px) {
  .header-section {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
  }

  .actions {
    flex-direction: column; /* 手机上按钮垂直排列更易点击 */
  }

  .actions button {
    width: 100%;
  }
}

.progress-section {
  margin-top: 1rem;
}
.progress-text {
  font-size: var(--font-sm);
  color: var(--text-muted);
  margin-top: 0.5rem;
  text-align: center;
}
.installed-hint {
  margin-top: 1rem;
}

.error-hint {
  font-size: var(--font-sm);
  color: var(--text-muted);
  text-align: center;
  margin-top: 1rem;
  padding: 0.5rem 1rem;
  line-height: 1.5;
}
</style>
