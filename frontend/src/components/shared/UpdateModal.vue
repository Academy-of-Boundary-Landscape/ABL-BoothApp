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
          <n-button @click="retry">重试</n-button>
        </template>
      </n-result>
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

      <div class="current-ver-tip">
        当前版本: v{{ currentVersion }}
      </div>

      <n-divider title-placement="left" style="margin: 12px 0;">更新内容</n-divider>

      <!-- 滚动区域 -->
      <n-scrollbar style="max-height: 200px" class="log-scroll">
        <!-- 使用 pre-wrap 保留 GitHub Release 说明的格式 -->
        <div class="release-note">{{ releaseNote }}</div>
      </n-scrollbar>

      <div class="actions">
        <n-button @click="close" ghost>暂不更新</n-button>
        <n-button type="primary" @click="handleDownload">
          前往下载
        </n-button>
      </div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useUpdateCheck } from '@/composables/useUpdateCheck';
import { 
  NModal, NSpin, NResult, NButton, NTag, NDivider, NScrollbar 
} from 'naive-ui';

const props = defineProps<{ show: boolean }>();
const emit = defineEmits(['update:show']);

// 判断环境
const isTauriEnv = ref(window.__TAURI_INTERNALS__ !== undefined);

const showModal = computed({
  get: () => props.show,
  set: (val) => emit('update:show', val),
});

const {
  loading,
  error,
  hasUpdate,
  currentVersion,
  latestVersion,
  releaseNote,
  releaseDate,
  checkUpdate,
  goToDownload,
} = useUpdateCheck();

const handleEnter = () => {
  if (isTauriEnv.value) {
    checkUpdate();
  }
};

const retry = () => {
  if (isTauriEnv.value) {
    checkUpdate();
  }
};

const close = () => {
  showModal.value = false;
};

const handleDownload = () => {
  goToDownload();
  // 可选：点击下载后是否关闭弹窗？通常保留让用户知道发生了什么
  // close(); 
};

const formatDate = (dateStr: string) => {
  if (!dateStr) return '';
  return new Date(dateStr).toLocaleDateString();
};
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
</style>