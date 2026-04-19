<template>
  <div class="vision-container">
    <div class="section-header" @click="isCollapsed = !isCollapsed">
      <h2>AI 视觉识别</h2>
      <n-button text class="toggle-btn">
        {{ isCollapsed ? '展开' : '折叠' }}
      </n-button>
    </div>

    <transition name="expand">
      <div v-show="!isCollapsed" class="section-body">

        <!-- 运行时状态 -->
        <div class="status-bar">
          <div class="status-item">
            <span class="status-label">状态</span>
            <n-tag :type="statusTagType" size="small" round>{{ statusText }}</n-tag>
          </div>
          <div class="status-item">
            <span class="status-label">当前模型</span>
            <span class="status-value">{{ status.model_id || '-' }}</span>
          </div>
          <div class="status-item">
            <span class="status-label">索引数量</span>
            <span class="status-value">{{ status.index_size ?? '-' }}</span>
          </div>
          <div class="status-item">
            <span class="status-label">最后构建</span>
            <span class="status-value">{{ status.last_rebuild_at || '-' }}</span>
          </div>
          <div class="status-item status-item--wide">
            <span class="status-label">推理设备</span>
            <n-tag :type="status.execution_provider?.includes('DirectML') ? 'success' : 'default'" size="small">
              {{ status.execution_provider || '-' }}
            </n-tag>
          </div>
          <div class="status-item status-item--wide">
            <span class="status-label">设备选择</span>
            <n-select
              size="small"
              :value="epConfigured"
              :options="epOptions"
              :style="{ minWidth: '220px' }"
              @update:value="handleEpChange"
            />
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="action-row">
          <n-button
            type="primary"
            :loading="isRebuilding"
            :disabled="isRebuilding"
            @click="handleRebuild(false)"
          >
            增量构建索引
          </n-button>
          <n-button
            type="warning"
            secondary
            :loading="isRebuilding"
            :disabled="isRebuilding"
            @click="handleRebuild(true)"
          >
            全量重建索引
          </n-button>
          <n-button secondary @click="refreshStatus">刷新状态</n-button>
        </div>

        <!-- 重建进度条 -->
        <div v-if="isRebuilding && rebuildTotal > 0" class="rebuild-progress">
          <div class="progress-label">
            正在处理图片嵌入... {{ rebuildProcessed }} / {{ rebuildTotal }}
          </div>
          <n-progress
            type="line"
            :percentage="rebuildPercentage"
            :show-indicator="true"
            indicator-placement="inside"
          />
        </div>

        <n-alert v-if="actionMsg" :type="actionMsgType" :bordered="false" closable class="action-alert" @close="actionMsg = ''">
          {{ actionMsg }}
        </n-alert>

        <!-- 模型列表 -->
        <div class="model-list">
          <h3 class="sub-title">可用模型</h3>
          <div v-if="models.length === 0" class="empty-hint">加载中...</div>

          <div v-for="m in models" :key="m.model_id" class="model-card" :class="{ 'model-card--active': m.is_active }">
            <div class="model-main">
              <div class="model-header">
                <span class="model-name">{{ m.model_id }}</span>
                <n-tag v-if="m.is_active" type="success" size="tiny" round>使用中</n-tag>
                <n-tag v-if="m.tier === 'builtin'" size="tiny" :bordered="false">内嵌</n-tag>
                <n-tag v-else-if="m.installed" type="info" size="tiny" :bordered="false">已安装</n-tag>
                <n-tag v-else size="tiny" type="default" :bordered="false">可下载</n-tag>
              </div>
              <div class="model-desc" v-if="m.description">{{ m.description }}</div>
              <div class="model-specs">
                <span class="spec-chip">{{ m.dim }} 维</span>
                <span class="spec-chip">{{ m.input_size }}×{{ m.input_size }} 输入</span>
                <span class="spec-chip">{{ m.model_version }}</span>
              </div>
            </div>
            <div class="model-actions">
              <template v-if="m.installed">
                <n-button
                  v-if="!m.is_active"
                  size="small"
                  type="primary"
                  :loading="activatingId === m.model_id"
                  @click="handleActivate(m.model_id)"
                >
                  激活
                </n-button>
                <n-button
                  v-if="!m.is_active && m.tier !== 'builtin'"
                  size="small"
                  type="error"
                  tertiary
                  @click="handleDelete(m.model_id)"
                >
                  删除
                </n-button>
              </template>
              <template v-else>
                <n-button
                  size="small"
                  type="info"
                  :loading="installingId === m.model_id"
                  @click="handleInstall(m.model_id)"
                >
                  下载安装
                </n-button>
              </template>
            </div>
          </div>
        </div>

        <!-- 安装进度 -->
        <div v-if="installTask" class="install-progress">
          <div class="progress-label">
            安装 {{ installTask.model_id }}:
            <span :class="'progress-status--' + installTask.status">{{ installTask.status }}</span>
          </div>
          <n-progress
            type="line"
            :percentage="installTask.progress"
            :status="installTask.status === 'failed' ? 'error' : installTask.status === 'completed' ? 'success' : 'default'"
          />
          <div v-if="installTask.error" class="progress-error">{{ installTask.error }}</div>
        </div>

      </div>
    </transition>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { NButton, NTag, NAlert, NProgress, NSelect } from 'naive-ui'
import {
  getVisionStatus,
  listModels,
  rebuildIndex,
  installModel,
  getInstallTask,
  activateModel,
} from '@/services/vision'
import apiClient from '@/services/api'

const isCollapsed = ref(false)

// ===== EP 设备选择 =====
const epConfigured = ref('auto')
const epOptions = ref([
  { label: '自动 (GPU 优先)', value: 'auto' },
  { label: '仅 CPU', value: 'cpu' },
])

async function loadEpSetting() {
  try {
    const { data } = await apiClient.get('/vision/settings/ep')
    epConfigured.value = data.configured || 'auto'

    const opts = [
      { label: '自动（最佳加速）', value: 'auto' },
      { label: '仅 CPU', value: 'cpu' },
    ]

    if (data.platform === 'windows' && data.gpu_devices?.length) {
      // Windows: 列出 DirectML GPU 设备
      for (const dev of data.gpu_devices) {
        const lower = dev.name.toLowerCase()
        const isVirtual = lower.includes('virtual') || lower.includes('basic') || lower.includes('remote')
        opts.push({
          label: `GPU ${dev.device_id}: ${dev.name}${isVirtual ? ' (虚拟)' : ''}`,
          value: `gpu:${dev.device_id}`,
          disabled: isVirtual,
        })
      }
    } else if (data.platform === 'android') {
      // Android: NNAPI 选项
      opts.push({ label: 'NNAPI (NPU/GPU)', value: 'nnapi' })
    }

    epOptions.value = opts
  } catch { /* ignore */ }
}

async function handleEpChange(val) {
  try {
    const { data } = await apiClient.put('/vision/settings/ep', { execution_provider: val })
    epConfigured.value = val
    actionMsg.value = data.message || '推理设备已切换'
    actionMsgType.value = 'success'
    // 刷新状态以反映新的 EP
    await refreshStatus()
  } catch (err) {
    actionMsg.value = err.response?.data?.error || '切换失败'
    actionMsgType.value = 'error'
  }
}

// ===== 状态 =====
const status = ref({})
const models = ref([])
const actionMsg = ref('')
const actionMsgType = ref('success')

const statusText = computed(() => {
  if (status.value.is_rebuilding) return '正在构建索引...'
  if (status.value.is_ready) return '就绪'
  if (status.value.reason === 'VISION_INDEX_EMPTY') return '索引为空'
  if (status.value.reason === 'VISION_REBUILD_REQUIRED') return '需要重建'
  return '未就绪'
})

const statusTagType = computed(() => {
  if (status.value.is_rebuilding) return 'warning'
  if (status.value.is_ready) return 'success'
  return 'error'
})

async function refreshStatus() {
  try {
    status.value = await getVisionStatus()
  } catch {
    status.value = {}
  }
  try {
    const resp = await listModels()
    models.value = resp.models || []
  } catch {
    models.value = []
  }
}

// ===== 索引重建 =====
const isRebuilding = ref(false)
const rebuildProcessed = ref(0)
const rebuildTotal = ref(0)
const rebuildPercentage = computed(() => {
  if (rebuildTotal.value <= 0) return 0
  return Math.round((rebuildProcessed.value / rebuildTotal.value) * 100)
})
let rebuildPollTimer = null

async function handleRebuild(forceFull) {
  isRebuilding.value = true
  rebuildProcessed.value = 0
  rebuildTotal.value = 0
  actionMsg.value = ''
  try {
    await rebuildIndex(forceFull)
    actionMsg.value = forceFull ? '全量重建已启动' : '增量构建已启动'
    actionMsgType.value = 'info'
    startRebuildPoll()
  } catch (err) {
    actionMsg.value = err.response?.data?.error || '重建失败'
    actionMsgType.value = 'error'
    isRebuilding.value = false
  }
}

function startRebuildPoll() {
  stopRebuildPoll()
  rebuildPollTimer = setInterval(async () => {
    await refreshStatus()
    rebuildProcessed.value = status.value.rebuild_processed ?? 0
    rebuildTotal.value = status.value.rebuild_total ?? 0
    if (!status.value.is_rebuilding) {
      stopRebuildPoll()
      isRebuilding.value = false
      rebuildProcessed.value = 0
      rebuildTotal.value = 0
      actionMsg.value = `索引构建完成，共 ${status.value.index_size ?? 0} 条嵌入`
      actionMsgType.value = 'success'
    }
  }, 1000)
}

function stopRebuildPoll() {
  if (rebuildPollTimer) {
    clearInterval(rebuildPollTimer)
    rebuildPollTimer = null
  }
}

// ===== 模型安装 =====
const installingId = ref(null)
const installTask = ref(null)
let installPollTimer = null

async function handleInstall(modelId) {
  installingId.value = modelId
  actionMsg.value = ''
  try {
    const resp = await installModel(modelId)
    installTask.value = { task_id: resp.task_id, model_id: modelId, status: 'downloading', progress: 1, error: null }
    startInstallPoll(resp.task_id)
  } catch (err) {
    actionMsg.value = err.response?.data?.error || '安装失败'
    actionMsgType.value = 'error'
    installingId.value = null
  }
}

function startInstallPoll(taskId) {
  stopInstallPoll()
  installPollTimer = setInterval(async () => {
    try {
      const task = await getInstallTask(taskId)
      installTask.value = task
      if (task.status === 'completed' || task.status === 'failed') {
        stopInstallPoll()
        installingId.value = null
        if (task.status === 'completed') {
          actionMsg.value = '模型安装完成'
          actionMsgType.value = 'success'
        }
        await refreshStatus()
      }
    } catch {
      stopInstallPoll()
      installingId.value = null
    }
  }, 1500)
}

function stopInstallPoll() {
  if (installPollTimer) {
    clearInterval(installPollTimer)
    installPollTimer = null
  }
}

// ===== 模型激活 =====
const activatingId = ref(null)

async function handleActivate(modelId) {
  activatingId.value = modelId
  actionMsg.value = ''
  try {
    await activateModel(modelId)
    actionMsg.value = `已激活 ${modelId}，正在重建索引...`
    actionMsgType.value = 'info'
    isRebuilding.value = true
    startRebuildPoll()
    await refreshStatus()
  } catch (err) {
    actionMsg.value = err.response?.data?.error || '激活失败'
    actionMsgType.value = 'error'
  } finally {
    activatingId.value = null
  }
}

// ===== 模型删除 =====
async function handleDelete(modelId) {
  if (!confirm(`确认删除模型 ${modelId}？`)) return
  actionMsg.value = ''
  try {
    await apiClient.delete(`/vision/models/${modelId}`)
    actionMsg.value = `已删除 ${modelId}`
    actionMsgType.value = 'success'
    await refreshStatus()
  } catch (err) {
    actionMsg.value = err.response?.data?.error || '删除失败'
    actionMsgType.value = 'error'
  }
}

// ===== 生命周期 =====
onMounted(() => { refreshStatus(); loadEpSetting() })
onBeforeUnmount(() => { stopRebuildPoll(); stopInstallPoll() })
</script>

<style scoped>
.vision-container {
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
.toggle-btn { color: var(--accent-color); }

.section-body {
  padding: 1rem;
  border-top: 2px solid var(--border-color);
  display: flex;
  flex-direction: column;
  gap: 14px;
}

/* 状态栏 */
.status-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 16px 24px;
  padding: 10px 14px;
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
}
.status-item {
  display: flex;
  align-items: center;
  gap: 6px;
}
.status-item--wide {
  flex-basis: 100%;
}
.status-label {
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.status-value {
  font-size: var(--font-sm);
  font-weight: 600;
  color: var(--primary-text-color);
}

/* 操作 */
.action-row {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}
.action-alert { margin: 0; }

/* 模型列表 */
.sub-title {
  margin: 0;
  font-size: var(--font-base);
  font-weight: 600;
  color: var(--primary-text-color);
}
.model-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.model-card {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 16px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-color);
  transition: border-color 0.15s;
}
.model-card--active {
  border-color: var(--accent-color);
  background: color-mix(in srgb, var(--accent-color) 6%, var(--bg-color));
}
.model-main { flex: 1; min-width: 0; }
.model-header {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.model-name {
  font-weight: 700;
  font-size: var(--font-md);
  color: var(--primary-text-color);
}
.model-desc {
  font-size: var(--font-sm);
  color: var(--text-muted);
  margin-top: 4px;
  line-height: 1.4;
}
.model-specs {
  display: flex;
  gap: 6px;
  margin-top: 8px;
  flex-wrap: wrap;
}
.spec-chip {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: var(--radius-pill);
  background: var(--bg-secondary);
  color: var(--text-muted);
  font-weight: 500;
  white-space: nowrap;
}
.model-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
  align-self: center;
}
.empty-hint {
  color: var(--text-muted);
  font-size: var(--font-sm);
  padding: 8px 0;
}

/* 重建进度 */
.rebuild-progress {
  padding: 10px 14px;
  border: 1px solid var(--accent-color);
  border-radius: var(--radius-md);
  background: var(--hover-bg-color);
}
.rebuild-progress .progress-label {
  font-size: var(--font-sm);
  margin-bottom: 6px;
  color: var(--primary-text-color);
  font-weight: 500;
}

/* 安装进度 */
.install-progress {
  padding: 10px 14px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.progress-label {
  font-size: var(--font-sm);
  margin-bottom: 6px;
  color: var(--primary-text-color);
}
.progress-status--downloading { color: var(--accent-color); }
.progress-status--completed { color: var(--success-color); }
.progress-status--failed { color: var(--error-color); }
.progress-error {
  margin-top: 4px;
  font-size: var(--font-sm);
  color: var(--error-color);
}

/* 展开动画 */
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
  max-height: 1200px;
  opacity: 1;
}
</style>
