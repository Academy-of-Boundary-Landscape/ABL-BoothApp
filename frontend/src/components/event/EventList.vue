<template>
  <div class="event-list">
    <div class="event-toolbar">
      <div class="filters">
        <label class="filter-field" for="event-search-name">
          <span class="filter-label">按名称搜索</span>
          <n-input
            id="event-search-name"
            v-model:value="searchName"
            clearable
            placeholder="输入展会名称关键字..."
          />
        </label>
        <div class="filter-field">
          <span class="filter-label">日期范围</span>
          <div class="date-range">
            <n-date-picker
              v-model:value="dateRangeStart"
              type="date"
              clearable
              value-format="yyyy-MM-dd"
              placeholder="开始日期"
            />
            <span class="date-range__sep">至</span>
            <n-date-picker
              v-model:value="dateRangeEnd"
              type="date"
              clearable
              value-format="yyyy-MM-dd"
              placeholder="结束日期"
            />
          </div>
        </div>
        <n-button v-if="hasFilters" tertiary @click="clearFilters">清空筛选</n-button>
      </div>

      <n-button type="primary" class="touch-target" @click="openCreate">新建展会</n-button>
    </div>

    <AsyncState
      :loading="store.isLoading"
      :error="store.error"
      :empty="!store.events.length"
      loading-text="正在加载展会数据..."
    >
      <template v-if="!filteredEvents.length">
        <EmptyState
          icon="🔍"
          title="没有找到符合筛选条件的展会"
          hint="换个关键字，或清空筛选再试。"
        >
          <template #action>
            <n-button @click="clearFilters">清空筛选</n-button>
          </template>
        </EmptyState>
      </template>

      <template v-else>
        <section v-for="group in groups" :key="group.status" class="event-group">
          <div
            class="group-header"
            role="button"
            tabindex="0"
            :aria-expanded="!isCollapsed(group.status)"
            @click="toggleGroup(group.status)"
            @keydown.enter.prevent="toggleGroup(group.status)"
            @keydown.space.prevent="toggleGroup(group.status)"
          >
            <span class="group-title">{{ group.label }}</span>
            <span class="group-count">{{ group.events.length }}</span>
            <span
              class="group-arrow"
              :class="{ 'group-arrow--collapsed': isCollapsed(group.status) }"
              >▾</span
            >
          </div>

          <div v-show="!isCollapsed(group.status)" class="event-grid">
            <article
              v-for="event in group.events"
              :key="event.id"
              class="event-card"
              role="link"
              tabindex="0"
              @click="openWorkbench(event)"
              @keydown.enter.prevent="openWorkbench(event)"
            >
              <div class="event-card__head">
                <h3 class="event-card__name">{{ event.name }}</h3>
                <div class="event-card__tools" @click.stop @keydown.stop>
                  <n-tag :type="statusType(event.status)" size="small">{{ event.status }}</n-tag>
                  <n-dropdown
                    trigger="click"
                    placement="bottom-end"
                    :options="cardMenuOptions"
                    @select="(key) => onCardAction(key, event)"
                  >
                    <n-button quaternary circle size="small" aria-label="更多操作">⋯</n-button>
                  </n-dropdown>
                </div>
              </div>
              <p class="event-card__meta">日期：{{ event.date }}</p>
              <p class="event-card__meta">地点：{{ event.location || '未指定' }}</p>
            </article>
          </div>
        </section>
      </template>

      <template #empty>
        <EmptyState
          icon="📋"
          title="还没有创建展会"
          desc="展会是管理摊位的核心单位。每场漫展创建一个展会，然后在其中管理商品库存和订单。"
          hint="点右上角「新建展会」开始吧"
        >
          <template #action>
            <n-button type="primary" @click="openCreate">新建展会</n-button>
          </template>
        </EmptyState>
      </template>
    </AsyncState>

    <AppModal
      v-model:show="formVisible"
      :title="editingEvent ? '编辑展会' : '新建展会'"
      size="sm"
      :mask-closable="false"
    >
      <EventForm
        :key="formKey"
        ref="eventFormRef"
        :mode="editingEvent ? 'edit' : 'create'"
        :event="editingEvent ?? undefined"
        @saved="onSaved"
      />
      <template #footer>
        <n-button @click="closeForm">取消</n-button>
        <n-button type="primary" @click="submitForm">
          {{ editingEvent ? '保存更改' : '创建' }}
        </n-button>
      </template>
    </AppModal>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NDatePicker, NDropdown, NInput, NTag } from 'naive-ui'
import { AsyncState, EmptyState, AppModal } from '@/components/ui'
import { useEventStore } from '@/stores/eventStore'
import { useFeedback } from '@/composables/useFeedback'
import EventForm from '@/components/event/EventForm.vue'
import type { Schemas } from '@/api/client'

const store = useEventStore()
const fb = useFeedback()
const router = useRouter()

// ===================== 搜索 / 过滤 =====================
const searchName = ref('')
// n-date-picker 的 v-model:value 带 value-format 时，onUpdate:value 仍发时间戳（number）。
const dateRangeStart = ref<number | null>(null)
const dateRangeEnd = ref<number | null>(null)

const filteredEvents = computed(() => {
  let events = store.events

  if (searchName.value.trim()) {
    const lowerCaseQuery = searchName.value.toLowerCase()
    events = events.filter((event) => event.name.toLowerCase().includes(lowerCaseQuery))
  }

  if (dateRangeStart.value) {
    const start = dateRangeStart.value
    events = events.filter((event) => new Date(event.date) >= new Date(start))
  }

  if (dateRangeEnd.value) {
    // 设置到当天的最后一刻，确保包含选定的结束日期。
    const endDate = new Date(dateRangeEnd.value)
    endDate.setHours(23, 59, 59, 999)
    events = events.filter((event) => new Date(event.date) <= endDate)
  }

  return events
})

const hasFilters = computed(() =>
  Boolean(searchName.value.trim() || dateRangeStart.value || dateRangeEnd.value)
)

function clearFilters() {
  searchName.value = ''
  dateRangeStart.value = null
  dateRangeEnd.value = null
}

// ===================== 分组（进行中 → 筹备 → 已结算） =====================
const STATUS_ORDER = ['进行中', '筹备', '已结算'] as const

const groups = computed(() =>
  STATUS_ORDER.map((status) => ({
    status,
    label: status,
    events: filteredEvents.value.filter((event) => event.status === status),
  })).filter((group) => group.events.length > 0)
)

const collapsedGroups = ref<Record<string, boolean>>({ 已结算: true })

function isCollapsed(status: string) {
  return collapsedGroups.value[status] === true
}

function toggleGroup(status: string) {
  collapsedGroups.value = { ...collapsedGroups.value, [status]: !isCollapsed(status) }
}

// ===================== 卡片 =====================
const cardMenuOptions = [
  { label: '编辑', key: 'edit' },
  { label: '删除', key: 'delete' },
]

function statusType(status: Schemas['EventStatus']): 'warning' | 'default' | 'success' {
  if (status === '进行中') return 'warning'
  if (status === '已结算') return 'default'
  return 'success' // 筹备
}

function openWorkbench(event: Schemas['EventResponse']) {
  void router.push({ name: 'admin-event-workbench', params: { id: event.id } })
}

function onCardAction(key: string | number, event: Schemas['EventResponse']) {
  if (key === 'edit') {
    openEdit(event)
  } else if (key === 'delete') {
    void confirmDelete(event)
  }
}

async function confirmDelete(event: Schemas['EventResponse']) {
  // 已结算的展会账已经冻结、往往还要留着对账，删除却是级联删掉整本账且不受
  // 冻结保护（后端 DELETE /events/:id 有意不守展会状态）。这里给一句明确的
  // 后果说明，不能和普通展会共用同一句「无法撤销」。
  const message =
    event.status === '已结算'
      ? `「${event.name}」已结算。删除将永久删除该展会的全部订单与账本流水，且无法恢复。确定继续吗？`
      : `您确定要删除「${event.name}」吗？此操作无法撤销。`
  await fb.confirm({
    title: '确认删除',
    content: message,
    danger: true,
    onConfirm: async () => {
      try {
        await store.deleteEvent(event.id)
      } catch (error) {
        fb.error(error, '删除失败，请稍后再试。')
      }
    },
  })
}

// ===================== 新建 / 编辑弹窗 =====================
const formVisible = ref(false)
const editingEvent = ref<Schemas['EventResponse'] | null>(null)
// 每次打开都换 key 强制 EventForm 重新挂载，避免上一次的输入 / 校验残留。
const formKey = ref(0)
const eventFormRef = ref<InstanceType<typeof EventForm> | null>(null)

function openCreate() {
  editingEvent.value = null
  formKey.value += 1
  formVisible.value = true
}

function openEdit(event: Schemas['EventResponse']) {
  editingEvent.value = event
  formKey.value += 1
  formVisible.value = true
}

function closeForm() {
  formVisible.value = false
  editingEvent.value = null
}

function onSaved() {
  closeForm()
}

function submitForm() {
  void eventFormRef.value?.submit()
}

onMounted(() => {
  store.fetchEvents()
})
</script>

<style scoped>
.event-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}

.event-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-lg);
  align-items: flex-end;
  justify-content: space-between;
  padding: var(--space-lg) var(--space-xl);
  background: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
}

.filters {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-lg);
  align-items: flex-end;
}

.filter-field {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}

.filter-label {
  font-size: var(--font-sm);
  color: var(--text-muted);
}

.date-range {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.date-range__sep {
  color: var(--text-muted);
}

.event-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

.group-header {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm) 0;
  cursor: pointer;
  user-select: none;
}

.group-title {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
}

.group-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 24px;
  height: 24px;
  padding: 0 var(--space-xs);
  border-radius: var(--radius-pill);
  background: var(--accent-color-light);
  color: var(--accent-color);
  font-size: var(--font-sm);
  font-weight: var(--weight-bold);
}

.group-arrow {
  margin-left: auto;
  color: var(--text-muted);
  transition: transform 0.2s ease;
}

.group-arrow--collapsed {
  transform: rotate(-90deg);
}

.event-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--space-lg);
}

.event-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  padding: var(--space-lg);
  background: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-sm);
  cursor: pointer;
  transition:
    transform 0.12s ease,
    box-shadow 0.12s ease,
    border-color 0.12s ease,
    background-color 0.12s ease;
}

.event-card:hover {
  transform: translateY(-4px);
  box-shadow: var(--shadow-lg);
  border-color: var(--accent-color);
  background: var(--accent-color-light);
}

.event-card:focus-visible {
  outline: 2px solid var(--accent-color);
  outline-offset: 2px;
}

.event-card__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-sm);
}

.event-card__name {
  margin: 0;
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
  overflow-wrap: anywhere;
}

.event-card__tools {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  flex-shrink: 0;
}

.event-card__meta {
  margin: 0;
  font-size: var(--font-sm);
  color: var(--text-muted);
}

@media (--phone) {
  .event-toolbar {
    padding: var(--space-md);
  }

  .filters {
    width: 100%;
  }

  .filter-field {
    flex: 1 1 100%;
  }

  .date-range {
    flex-wrap: wrap;
  }

  .event-grid {
    grid-template-columns: 1fr;
  }

  /* 手机上可点区域要求 ≥ 44px。 */
  .event-toolbar :deep(.n-button),
  .event-card__tools :deep(.n-button) {
    min-height: 44px;
  }
}
</style>
