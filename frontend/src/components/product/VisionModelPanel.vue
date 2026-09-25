<template>
  <SectionCard
    class="vision-container"
    title="AI 视觉识别"
    collapsible
    v-model:collapsed="isCollapsed"
  >
    <template #extra>
      <span class="help-wrap" @click.stop>
        <HelpBubble page="vision" />
      </span>
    </template>

    <div class="vision-body">
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
          <n-tag
            :type="status.execution_provider?.includes('DirectML') ? 'success' : 'default'"
            size="small"
          >
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

      <!-- 行动指引（基于当前状态） -->
      <n-alert v-if="nextAction" :type="nextAction.type" :bordered="false" class="next-action-hint">
        <div class="next-action-content">
          <span>{{ nextAction.text }}</span>
          <n-button
            v-if="nextAction.link"
            size="tiny"
            type="primary"
            @click="goToLink(nextAction.link)"
          >
            {{ nextAction.linkLabel }}
          </n-button>
        </div>
      </n-alert>

      <!-- 操作按钮 -->
      <div class="action-row">
        <n-tooltip trigger="hover" placement="top">
          <template #trigger>
            <n-button
              type="primary"
              :loading="isRebuilding"
              :disabled="isRebuilding"
              @click="handleRebuild(false)"
            >
              增量构建索引
            </n-button>
          </template>
          只处理新上传 / 未嵌入的图片，日常用这个
        </n-tooltip>
        <n-tooltip trigger="hover" placement="top">
          <template #trigger>
            <n-button
              type="warning"
              secondary
              :loading="isRebuilding"
              :disabled="isRebuilding"
              @click="handleRebuild(true)"
            >
              全量重建索引
            </n-button>
          </template>
          清空索引后重新处理所有图片。换模型或索引出错时使用
        </n-tooltip>
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

      <n-alert
        v-if="actionMsg"
        :type="actionMsgType"
        :bordered="false"
        closable
        class="action-alert"
        @close="actionMsg = ''"
      >
        {{ actionMsg }}
      </n-alert>

      <!-- 模型列表 -->
      <div class="model-list">
        <h3 class="sub-title">可用模型</h3>
        <div v-if="loadError" class="model-error">
          <span>{{ loadError }}</span>
          <n-button size="tiny" secondary @click="refreshStatus">重新加载</n-button>
        </div>
        <AsyncState
          v-else
          :loading="modelsLoading && models.length === 0"
          :empty="models.length === 0"
          loading-text="加载中..."
        >
          <template #empty>
            <EmptyState compact title="暂无可用模型，请检查后端服务是否正常运行" />
          </template>

          <div
            v-for="m in models"
            :key="m.model_id"
            class="model-card"
            :class="{ 'model-card--active': m.is_active }"
          >
            <div class="model-main">
              <div class="model-header">
                <span class="model-name">{{ m.model_id }}</span>
                <n-tag v-if="m.is_active" type="success" size="tiny" round>使用中</n-tag>
                <n-tag v-if="m.tier === 'builtin'" size="tiny" :bordered="false">内嵌</n-tag>
                <n-tag v-else-if="m.installed" type="info" size="tiny" :bordered="false"
                  >已安装</n-tag
                >
                <n-tag v-else size="tiny" type="default" :bordered="false">可下载</n-tag>
              </div>
              <div class="model-desc" v-if="m.description">{{ m.description }}</div>
              <div class="model-specs">
                <span v-if="m.size_mb" class="spec-chip spec-chip--size">{{
                  formatSize(m.size_mb)
                }}</span>
                <span class="spec-chip">{{ m.dim }} 维</span>
                <span class="spec-chip">{{ m.input_size }}×{{ m.input_size }} 输入</span>
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
        </AsyncState>
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
          :status="
            installTask.status === 'failed'
              ? 'error'
              : installTask.status === 'completed'
                ? 'success'
                : 'default'
          "
        />
        <div v-if="installTask.error" class="progress-error">{{ installTask.error }}</div>
      </div>
    </div>
  </SectionCard>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRouter, type RouteLocationRaw } from 'vue-router'
import { NButton, NTag, NAlert, NProgress, NSelect, NTooltip, type SelectOption } from 'naive-ui'
import { AsyncState, EmptyState, SectionCard } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import {
  getVisionStatus,
  listModels,
  rebuildIndex,
  installModel,
  getInstallTask,
  activateModel,
} from '@/services/vision'
import { api, unwrap, errorMessage, type Schemas } from '@/api/client'
import HelpBubble from '@/components/shared/HelpBubble.vue'

const router = useRouter()
const fb = useFeedback()

function goToLink(to: RouteLocationRaw) {
  router.push(to)
}

const isCollapsed = ref(false)

// ===== EP 设备选择 =====
const epConfigured = ref('auto')
const epOptions = ref<SelectOption[]>([
  { label: '自动 (GPU 优先)', value: 'auto' },
  { label: '仅 CPU', value: 'cpu' },
])

async function loadEpSetting() {
  try {
    const data = await unwrap(api.GET('/vision/settings/ep'))
    epConfigured.value = data.configured || 'auto'

    const opts: SelectOption[] = [
      { label: '自动（最佳加速）', value: 'auto' },
      { label: '仅 CPU', value: 'cpu' },
    ]

    if (data.platform === 'windows' && data.gpu_devices?.length) {
      // Windows: 列出 DirectML GPU 设备
      for (const dev of data.gpu_devices) {
        const lower = dev.name.toLowerCase()
        const isVirtual =
          lower.includes('virtual') || lower.includes('basic') || lower.includes('remote')
        opts.push({
          label: `GPU ${dev.device_id}: ${dev.name}${isVirtual ? ' (虚拟)' : ''}`,
          value: `gpu:${dev.device_id}`,
          disabled: isVirtual,
        })
      }
    } else if (data.platform === 'android') {
      // Android: auto 走 CPU；NNAPI 只作手动选项（对这些模型大多比 CPU 慢）
      opts.push({ label: 'NNAPI (NPU/GPU，多数机型比 CPU 慢)', value: 'nnapi' })
    }

    epOptions.value = opts
  } catch {
    /* ignore */
  }
}

async function handleEpChange(val: string) {
  try {
    const data = await unwrap(api.PUT('/vision/settings/ep', { body: { execution_provider: val } }))
    epConfigured.value = val
    actionMsg.value = data.message || '推理设备已切换'
    actionMsgType.value = 'success'
    // 刷新状态以反映新的 EP
    await refreshStatus()
  } catch (err) {
    actionMsg.value = errorMessage(err, '切换失败')
    actionMsgType.value = 'error'
  }
}

// ===== 状态 =====
const status = ref<Partial<Schemas['VisionStatusResponse']>>({})
const models = ref<Schemas['VisionModelItem'][]>([])
const actionMsg = ref('')
const actionMsgType = ref<'success' | 'error' | 'info'>('success')
const modelsLoading = ref(true)
const loadError = ref('')

const activeModel = computed(() => models.value.find((m) => m.is_active))
const hasInstalledModel = computed(() => models.value.some((m) => m.installed))

const statusText = computed(() => {
  if (status.value.is_rebuilding) return '正在构建索引...'
  if (status.value.is_ready) return '就绪'
  if (status.value.reason === 'VISION_INDEX_EMPTY') return '索引为空'
  if (status.value.reason === 'VISION_REBUILD_REQUIRED') return '需要重建'
  if (!activeModel.value) return '未激活模型'
  return '未就绪'
})

interface NextAction {
  type: 'info' | 'warning'
  text: string
  link?: RouteLocationRaw
  linkLabel?: string
}

// 基于当前状态给出"下一步该做什么"的行动提示
const nextAction = computed<NextAction | null>(() => {
  if (loadError.value) return null
  if (status.value.is_rebuilding) return null
  if (status.value.is_ready) return null
  if (!hasInstalledModel.value) {
    return {
      type: 'info',
      text: '第 1 步：从下方「可用模型」中选择一个下载（推荐 ⭐ 标注的版本）',
    }
  }
  if (!activeModel.value) {
    return {
      type: 'info',
      text: '第 2 步：点击已下载模型上的「激活」按钮，系统才会使用该模型进行识别',
    }
  }
  if (status.value.reason === 'VISION_INDEX_EMPTY') {
    return {
      type: 'warning',
      text: '索引为空：请先给商品上传识别图，然后回来点击上方「增量构建索引」',
      link: { path: '/admin/master-products', query: { filter: 'need_vision_images' } },
      linkLabel: '去上传识别图',
    }
  }
  if (status.value.reason === 'VISION_REBUILD_REQUIRED') {
    return {
      type: 'warning',
      text: '有新照片未处理 / 模型已切换，请点击上方「增量构建索引」',
    }
  }
  return null
})

const statusTagType = computed<'warning' | 'success' | 'error'>(() => {
  if (status.value.is_rebuilding) return 'warning'
  if (status.value.is_ready) return 'success'
  return 'error'
})

function formatSize(mb: number | null | undefined) {
  if (mb == null) return ''
  return mb < 10 ? `${mb.toFixed(1)} MB` : `${Math.round(mb)} MB`
}

async function refreshStatus() {
  modelsLoading.value = true
  loadError.value = ''
  try {
    status.value = await getVisionStatus()
  } catch {
    status.value = {}
  }
  try {
    const resp = await listModels()
    models.value = resp.models || []
  } catch (err) {
    models.value = []
    loadError.value = errorMessage(err, '加载模型列表失败，请检查后端服务后重试')
  } finally {
    modelsLoading.value = false
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
let rebuildPollTimer: ReturnType<typeof setInterval> | null = null

async function handleRebuild(forceFull: boolean) {
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
    actionMsg.value = errorMessage(err, '重建失败')
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
const installingId = ref<string | null>(null)
const installTask = ref<Schemas['VisionInstallTaskResponse'] | null>(null)
let installPollTimer: ReturnType<typeof setInterval> | null = null

async function handleInstall(modelId: string) {
  installingId.value = modelId
  actionMsg.value = ''
  try {
    const resp = await installModel(modelId)
    installTask.value = {
      task_id: resp.task_id,
      model_id: modelId,
      status: 'downloading',
      progress: 1,
      error: null,
    }
    startInstallPoll(resp.task_id)
  } catch (err) {
    actionMsg.value = errorMessage(err, '安装失败')
    actionMsgType.value = 'error'
    installingId.value = null
  }
}

function startInstallPoll(taskId: string) {
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
const activatingId = ref<string | null>(null)

async function handleActivate(modelId: string) {
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
    actionMsg.value = errorMessage(err, '激活失败')
    actionMsgType.value = 'error'
  } finally {
    activatingId.value = null
  }
}

// ===== 模型删除 =====
async function handleDelete(modelId: string) {
  const ok = await fb.confirm({ title: `确认删除模型 ${modelId}？`, danger: true })
  if (!ok) return
  actionMsg.value = ''
  try {
    await unwrap(
      api.DELETE('/vision/models/{model_id}', { params: { path: { model_id: modelId } } })
    )
    actionMsg.value = `已删除 ${modelId}`
    actionMsgType.value = 'success'
    await refreshStatus()
  } catch (err) {
    actionMsg.value = errorMessage(err, '删除失败')
    actionMsgType.value = 'error'
  }
}

// ===== 生命周期 =====
onMounted(() => {
  refreshStatus()
  loadEpSetting()
})
onBeforeUnmount(() => {
  stopRebuildPoll()
  stopInstallPoll()
})
</script>

<style scoped>
.help-wrap {
  display: inline-flex;
  align-items: center;
}

.vision-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

/* 状态栏 */
.status-bar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-lg) var(--space-xl);
  padding: var(--space-sm) var(--space-md);
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
}
.status-item {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
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
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
}

/* 操作 */
.action-row {
  display: flex;
  gap: var(--space-sm);
  flex-wrap: wrap;
}
.action-alert {
  margin: 0;
}

/* 模型列表 */
.sub-title {
  margin: 0;
  font-size: var(--font-base);
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
}
.model-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}
.model-card {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-md);
  padding: var(--space-md) var(--space-lg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-color);
  transition: border-color 0.15s;
}
.model-card--active {
  border-color: var(--accent-color);
  background: color-mix(in srgb, var(--accent-color) 6%, var(--bg-color));
}
.model-main {
  flex: 1;
  min-width: 0;
}
.model-header {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  flex-wrap: wrap;
}
.model-name {
  font-weight: var(--weight-bold);
  font-size: var(--font-md);
  color: var(--primary-text-color);
}
.model-desc {
  font-size: var(--font-sm);
  color: var(--text-muted);
  margin-top: var(--space-xs);
  line-height: 1.4;
}
.model-specs {
  display: flex;
  gap: var(--space-sm);
  margin-top: var(--space-sm);
  flex-wrap: wrap;
}
.spec-chip {
  font-size: var(--font-xs);
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-pill);
  background: var(--bg-secondary);
  color: var(--text-muted);
  font-weight: var(--weight-medium);
  white-space: nowrap;
}
.spec-chip--size {
  background: var(--accent-color-light);
  color: var(--primary-text-color);
  font-variant-numeric: tabular-nums;
}
.model-actions {
  display: flex;
  gap: var(--space-sm);
  flex-shrink: 0;
  align-self: center;
}
.model-error {
  color: var(--error-color);
  font-size: var(--font-sm);
  padding: var(--space-sm) 0;
  display: flex;
  align-items: center;
  gap: var(--space-md);
}
.next-action-hint {
  margin: var(--space-sm) 0;
}
.next-action-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  flex-wrap: wrap;
}

/* 重建进度 */
.rebuild-progress {
  padding: var(--space-sm) var(--space-md);
  border: 1px solid var(--accent-color);
  border-radius: var(--radius-md);
  background: var(--hover-bg-color);
}
.rebuild-progress .progress-label {
  font-size: var(--font-sm);
  margin-bottom: var(--space-sm);
  color: var(--primary-text-color);
  font-weight: var(--weight-medium);
}

/* 安装进度 */
.install-progress {
  padding: var(--space-sm) var(--space-md);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.progress-label {
  font-size: var(--font-sm);
  margin-bottom: var(--space-sm);
  color: var(--primary-text-color);
}
.progress-status--downloading {
  color: var(--accent-color);
}
.progress-status--completed {
  color: var(--success-color);
}
.progress-status--failed {
  color: var(--error-color);
}
.progress-error {
  margin-top: var(--space-xs);
  font-size: var(--font-sm);
  color: var(--error-color);
}
</style>
