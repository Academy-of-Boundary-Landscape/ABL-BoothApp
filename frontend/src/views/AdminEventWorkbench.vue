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

      <!-- tab 行：三组之间带组标题，窄屏横向滑动不折行。 -->
      <nav class="workbench-tabs">
        <div class="tab-group">
          <span class="tab-group__label">展前</span>
          <RouterLink class="tab" :to="{ name: 'admin-event-products', params: { id: event.id } }">
            商品
          </RouterLink>
          <RouterLink class="tab" :to="{ name: 'admin-event-lots', params: { id: event.id } }">
            套装
          </RouterLink>
        </div>
        <div class="tab-group">
          <span class="tab-group__label">现场</span>
          <RouterLink class="tab" :to="{ name: 'admin-event-orders', params: { id: event.id } }">
            订单
          </RouterLink>
          <RouterLink class="tab" :to="{ name: 'admin-event-stats', params: { id: event.id } }">
            统计
          </RouterLink>
        </div>
        <div class="tab-group">
          <span class="tab-group__label">收摊</span>
          <RouterLink
            class="tab"
            :to="{ name: 'admin-event-settlement', params: { id: event.id } }"
          >
            结算
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
      <EditEventForm v-if="editTarget" ref="editForm" :event="editTarget" />
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
import EditEventForm from '@/components/event/EditEventForm.vue'

const STATUS_STEPS = ['筹备', '进行中', '已结算'] as const

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

// ── 编辑：本 task 先挂现有 EditEventForm（Task 4 P2 换成 EventForm）。──
const isEditModalVisible = ref(false)
const editTarget = ref<Schemas['EventResponse'] | null>(null)
const editForm = ref<InstanceType<typeof EditEventForm> | null>(null)

function openEdit() {
  editTarget.value = event.value
  isEditModalVisible.value = true
}

function closeEditModal() {
  isEditModalVisible.value = false
  editTarget.value = null
}

async function handleUpdateEvent() {
  if (!editForm.value || !editTarget.value) return
  const formData = editForm.value.submit()
  if (!formData) return
  try {
    await eventStore.updateEvent(editTarget.value.id, formData)
    closeEditModal()
    await reload()
  } catch (e) {
    fb.error(e, '保存失败')
  }
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
  gap: var(--space-xl);
  overflow-x: auto;
  flex-wrap: nowrap;
  padding-bottom: var(--space-sm);
  border-bottom: 1px solid var(--border-color);
  margin-bottom: var(--space-lg);
}

.tab-group {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  white-space: nowrap;
}

.tab-group__label {
  font-size: var(--font-xs);
  color: var(--text-muted);
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
