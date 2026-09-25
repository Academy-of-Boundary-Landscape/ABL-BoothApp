<template>
  <PageShell width="wide">
    <template #title>{{ pageTitle }}</template>
    <template #subtitle>
      <span v-if="event">
        {{ event.date }}<template v-if="event.location"> · {{ event.location }}</template>
      </span>
      <span v-else-if="error">{{ error }}</span>
      <span v-else>正在加载展会…</span>
    </template>
    <template #actions>
      <n-button v-if="event" @click="openEdit">编辑</n-button>
    </template>

    <template v-if="event">
      <!-- 状态条：当前段高亮；只有「筹备」有操作，「进行中 → 已结算」只能走摊主端收摊。 -->
      <div class="status-bar">
        <div class="status-steps">
          <template v-for="(step, i) in STATUS_STEPS" :key="step">
            <span
              class="status-step"
              :class="{
                'status-step--current': step === event.status,
                'status-step--done': i < currentStepIndex,
              }"
            >
              <span class="status-dot">●</span>{{ step }}
            </span>
            <span v-if="i < STATUS_STEPS.length - 1" class="status-link">───</span>
          </template>
        </div>

        <div class="status-action">
          <n-button
            v-if="event.status === '筹备'"
            type="primary"
            :loading="starting"
            @click="startEvent"
          >
            开始展会
          </n-button>
          <span v-else-if="event.status === '进行中'" class="status-hint">收摊由摊主端完成</span>
        </div>
      </div>

      <!-- tab 行：三组之间带组标签。组标签是带框的色块、不可点，和可点的 tab 区分开；
           展会当前所处阶段那一组的标签实心填色，和上面的状态条呼应。窄屏整行横滑不折行。 -->
      <nav class="workbench-tabs">
        <div
          v-for="group in TAB_GROUPS"
          :key="group.label"
          class="tab-group"
          :class="[
            `tab-group--${group.tone}`,
            { 'tab-group--current': group.status === event.status },
          ]"
        >
          <span class="tab-group__label">{{ group.label }}</span>
          <RouterLink
            v-for="tab in group.tabs"
            :key="tab.route"
            class="tab"
            :to="{ name: tab.route, params: { id: event.id } }"
          >
            {{ tab.label }}
          </RouterLink>
        </div>
      </nav>
    </template>

    <!-- 外壳加载失败（不存在 / 已删除）：只渲染错误态与出口，不渲染子页，免得各子页各报一遍。 -->
    <template v-if="error">
      <AsyncState :error="error" />
      <div class="workbench-back">
        <n-button @click="goEvents">返回展会列表</n-button>
      </div>
    </template>
    <router-view v-else />

    <AppModal v-model:show="isEditModalVisible" title="编辑展会" size="sm" :mask-closable="false">
      <EventForm
        v-if="editTarget"
        ref="editForm"
        mode="edit"
        :event="editTarget"
        @saved="onEditSaved"
      />
      <template #footer>
        <n-button @click="closeEditModal">取消</n-button>
        <n-button type="primary" @click="handleUpdateEvent">保存更改</n-button>
      </template>
    </AppModal>
  </PageShell>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { NButton } from 'naive-ui'
import { AppModal, AsyncState, PageShell } from '@/components/ui'
import { useEventStore } from '@/stores/eventStore'
import { useFeedback } from '@/composables/useFeedback'
import { provideWorkbenchEvent } from '@/composables/useWorkbenchEvent'
import type { Schemas } from '@/api/client'
import EventForm from '@/components/event/EventForm.vue'

const STATUS_STEPS = ['筹备', '进行中', '已结算'] as const

/** tab 分组。`status` 是该组对应的展会状态（用来高亮当前阶段），`tone` 决定组标签的颜色。 */
const TAB_GROUPS = [
  {
    label: '展前',
    status: '筹备',
    tone: 'info',
    tabs: [
      { label: '商品', route: 'admin-event-products' },
      { label: '套装', route: 'admin-event-lots' },
    ],
  },
  {
    label: '现场',
    status: '进行中',
    tone: 'success',
    tabs: [
      { label: '订单', route: 'admin-event-orders' },
      { label: '统计', route: 'admin-event-stats' },
    ],
  },
  {
    label: '收摊',
    status: '已结算',
    tone: 'warning',
    tabs: [{ label: '结算', route: 'admin-event-settlement' }],
  },
] as const

const route = useRoute()
const router = useRouter()
const eventStore = useEventStore()
const fb = useFeedback()

const eventId = computed(() => Number(route.params.id))
const { event, error, reload } = provideWorkbenchEvent(eventId)

// 页头：有展会用展会名；加载失败给一个稳定的错误标题；否则是加载中。
const pageTitle = computed(
  () => event.value?.name || (error.value ? '展会不存在或无法加载' : '展会工作台')
)

function goEvents() {
  void router.push({ name: 'admin-events' })
}

const currentStepIndex = computed(() =>
  event.value ? STATUS_STEPS.indexOf(event.value.status) : -1
)

const starting = ref(false)

async function startEvent() {
  if (!event.value) return
  starting.value = true
  try {
    // 只走「筹备 → 进行中」这一条后端允许的迁移；失败时把后端原文交给 fb.error。
    await eventStore.updateEventStatus(event.value.id, '进行中')
    fb.success('展会已开始')
    await reload()
  } catch (e) {
    fb.error(e, '开始展会失败')
  } finally {
    starting.value = false
  }
}

// ── 编辑：EventForm 自己调 store 并在失败时把错误显示在弹窗内；saved 后关弹窗 + 刷新外壳。──
const isEditModalVisible = ref(false)
const editTarget = ref<Schemas['EventResponse'] | null>(null)
const editForm = ref<InstanceType<typeof EventForm> | null>(null)

function openEdit() {
  editTarget.value = event.value
  isEditModalVisible.value = true
}

function closeEditModal() {
  isEditModalVisible.value = false
  editTarget.value = null
}

async function handleUpdateEvent() {
  if (!editForm.value) return
  await editForm.value.submit()
}

function onEditSaved() {
  closeEditModal()
  void reload()
}
</script>

<style scoped>
.status-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-lg);
  flex-wrap: wrap;
  padding: var(--space-md) var(--space-lg);
  background: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  margin-bottom: var(--space-lg);
}

.status-steps {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.status-step {
  display: inline-flex;
  align-items: center;
  gap: var(--space-xs);
  font-size: var(--font-md);
  color: var(--text-muted);
}

.status-step--done {
  color: var(--text-muted);
}

.status-step--current {
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}

.status-dot {
  font-size: var(--font-xs);
}

.status-link {
  color: var(--border-color);
}

.status-hint {
  font-size: var(--font-sm);
  color: var(--text-muted);
}

.workbench-tabs {
  display: flex;
  gap: var(--space-md);
  overflow-x: auto;
  flex-wrap: nowrap;
  padding-bottom: var(--space-sm);
  border-bottom: 1px solid var(--border-color);
  margin-bottom: var(--space-lg);
}

.tab-group {
  /* 组色：每组只定义一个 --group-color，标签的边框 / 底色 / 字色都从它派生 */
  --group-color: var(--info-color);

  display: inline-flex;
  align-items: center;
  gap: var(--space-xs);
  white-space: nowrap;
}

.tab-group + .tab-group {
  padding-left: var(--space-md);
  border-left: 1px solid var(--border-color);
}

.tab-group--success {
  --group-color: var(--success-color);
}

.tab-group--warning {
  --group-color: var(--warning-color);
}

/* 不可点的组标签：带框的小色块，字号小一档、不加粗、没有 hover，和可点的 tab 一眼分开 */
.tab-group__label {
  margin-right: var(--space-xs);
  padding: var(--space-xs) var(--space-sm);
  border: 1px solid color-mix(in srgb, var(--group-color) 45%, transparent);
  border-radius: var(--radius-sm);
  background: color-mix(in srgb, var(--group-color) 12%, transparent);
  color: var(--group-color);
  font-size: var(--font-xs);
  line-height: var(--leading-tight);
  cursor: default;
  user-select: none;
}

/* 展会当前所处阶段：实心填色 */
.tab-group--current .tab-group__label {
  border-color: var(--group-color);
  background: var(--group-color);
  color: var(--text-white);
}

.tab {
  display: inline-flex;
  align-items: center;
  min-height: 44px;
  padding: var(--space-xs) var(--space-md);
  border-radius: var(--radius-pill);
  color: var(--secondary-text-color);
  font-size: var(--font-base);
  text-decoration: none;
}

.tab:hover {
  color: var(--accent-color);
}

.tab.router-link-active {
  background: var(--accent-color-light);
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}

@media (--phone) {
  .status-bar {
    flex-direction: column;
    align-items: flex-start;
  }
}

/* 外壳加载失败时的出口按钮：AsyncState 只渲染错误态，出口单独放在下面。 */
.workbench-back {
  margin-top: var(--space-lg);
}
</style>
