<!--
  管理端「收摊 · 结算」（spec §5.4）。

  页面顺序按 P4 裁定：**对账警示条在页内首位** → 垫付 / 结算调整 / 收摊清点三个编辑区块 →
  只读结算单（`SettlementReportView`）。只读结算单里不再重复警示条（`show-warnings=false`）。

  三个编辑区块是表单 + 数据列表：列表一律 `n-data-table`，原生 `<table>` 只留给
  `components/settlement/**` 的报表。金额框里填「元」，提交时换算成「分」；
  业务规则由后端判定，这里只负责把后端那句话原样显示出来。
-->
<template>
  <PageShell embedded width="full">
    <!-- 对账警示条：页内首位。数据来自结算单的 warnings。 -->
    <SettlementWarnings :warnings="warnings" />

    <div class="page-hint-row">
      <p class="page-hint">结算后仍可补录垫付、结算调整与收摊清点；结算单在页面最下方。</p>
      <HelpBubble page="event-settlement" />
    </div>

    <!-- 垫付 -->
    <SectionCard title="垫付" class="block">
      <p class="block-note">展会结算之后，这两项仍然可以增删</p>
      <p class="block-hint">
        摊主先掏钱替某个社团垫的费用（摊位费、打印费等）。这笔钱从「我应转给」里减掉。
      </p>

      <div class="form-grid">
        <div class="field">
          <span class="field-label">社团</span>
          <n-select
            v-model:value="advanceForm.societyId"
            :options="societyOptions"
            filterable
            placeholder="选择垫付给哪个社团"
          />
        </div>
        <label class="field">
          <span class="field-label">名目</span>
          <n-input v-model:value="advanceForm.label" placeholder="如 摊位费" />
        </label>
        <label class="field">
          <span class="field-label">金额（元）</span>
          <n-input-number
            v-model:value="advanceForm.amountYuan"
            :min="0"
            :precision="2"
            placeholder="只填正数"
          />
        </label>
        <div class="field actions">
          <n-button type="primary" :disabled="isBusy" @click="submitAdvance">新增垫付</n-button>
        </div>
      </div>

      <n-data-table
        v-if="store.advances.length"
        :columns="advanceColumns"
        :data="store.advances"
        :row-key="(row) => row.id"
        :bordered="false"
        :scroll-x="640"
        size="small"
      />
      <EmptyState v-else compact title="暂无垫付" />
    </SectionCard>

    <!-- 结算调整 -->
    <SectionCard title="结算调整" class="block">
      <p class="block-note">展会结算之后，这两项仍然可以增删</p>
      <p class="block-hint">方向用按钮选，金额只填正数——不要自己判断该填正号还是负号。</p>

      <div class="form-grid">
        <div class="field field-wide">
          <span class="field-label">方向</span>
          <n-radio-group v-model:value="adjustmentForm.direction">
            <n-space>
              <n-radio-button value="to_them">我要多给他们</n-radio-button>
              <n-radio-button value="to_me">他们要多给我</n-radio-button>
            </n-space>
          </n-radio-group>
        </div>
        <div class="field">
          <span class="field-label">社团</span>
          <n-select
            v-model:value="adjustmentForm.societyId"
            :options="societyOptions"
            filterable
            placeholder="选择调整给哪个社团"
          />
        </div>
        <label class="field">
          <span class="field-label">名目</span>
          <n-input v-model:value="adjustmentForm.label" placeholder="如 清点少一本按成本赔" />
        </label>
        <label class="field">
          <span class="field-label">金额（元）</span>
          <n-input-number
            v-model:value="adjustmentForm.amountYuan"
            :min="0"
            :precision="2"
            placeholder="只填正数"
          />
        </label>
        <div class="field actions">
          <n-button type="primary" :disabled="isBusy" @click="submitAdjustment">
            新增结算调整
          </n-button>
        </div>
      </div>

      <n-data-table
        v-if="store.adjustments.length"
        :columns="adjustmentColumns"
        :data="store.adjustments"
        :row-key="(row) => row.id"
        :bordered="false"
        :scroll-x="680"
        size="small"
      />
      <EmptyState v-else compact title="暂无结算调整" />
    </SectionCard>

    <!-- 收摊清点 -->
    <SectionCard v-if="store.report" title="收摊清点" class="block">
      <p class="block-hint">
        收全量：本场用过的每个渠道都要报数，<strong>数过一致的也要报</strong>——
        「我数了，一致」和「我没数」是两件事。
      </p>

      <n-data-table
        v-if="channelRows.length"
        :columns="channelColumns"
        :data="channelRows"
        :row-key="(row) => row.channel"
        :bordered="false"
        :scroll-x="760"
        size="small"
      />
      <EmptyState v-else compact title="本场还没有用过的收款渠道，无需清点。" />

      <div v-if="hasDiff" class="shortfall-note">
        差额由摊主自己承担，不进任何货主的结算。要推给某个货主，请到上面加一条结算调整。
      </div>

      <template #footer>
        <n-button type="primary" :disabled="isBusy || !channelRows.length" @click="submitReconcile">
          提交清点
        </n-button>
      </template>
    </SectionCard>

    <!-- 只读结算单（管理端与摊主端共用）。警示条已在页内首位、清点有可编辑区块，
         这两块都不在结算单里重复。 -->
    <SettlementReportView :event-id="eventId" :show-warnings="false" :show-channels="false" />
  </PageShell>
</template>

<script setup lang="ts">
import { ref, computed, h, watch, onMounted } from 'vue'
import {
  NButton,
  NDataTable,
  NInput,
  NInputNumber,
  NRadioButton,
  NRadioGroup,
  NSelect,
  NSpace,
  type DataTableColumns,
} from 'naive-ui'
import { useSettlementStore } from '@/stores/settlementStore'
import { useSocietyStore } from '@/stores/societyStore'
import { formatYuan, fromCents, cents, toCents, type Cents } from '@/utils/money'
import { describeEntryAdjustment } from '@/utils/settlementSigns'
import type { Schemas } from '@/api/client'
import { PageShell, EmptyState, SectionCard } from '@/components/ui'
import SettlementReportView from '@/components/settlement/SettlementReportView.vue'
import SettlementWarnings from '@/components/settlement/SettlementWarnings.vue'
import HelpBubble from '@/components/shared/HelpBubble.vue'
import { useFeedback } from '@/composables/useFeedback'

const props = defineProps<{ id: string | number }>()

const eventId = computed(() => Number(props.id))

const store = useSettlementStore()
const societyStore = useSocietyStore()
const fb = useFeedback()

const isBusy = ref(false)

const warnings = computed(() => store.report?.warnings ?? [])

const defaultAdvanceForm = (): {
  societyId: number | null
  label: string
  amountYuan: number | null
} => ({ societyId: null, label: '', amountYuan: null })
const defaultAdjustmentForm = (): {
  societyId: number | null
  label: string
  amountYuan: number | null
  // 不给默认值：方向是「我要多给他们」还是「他们要多给我」必须由人明确选一次，
  // 默认 to_them 会让累了一天的摊主根本没看控件就提交，然后钱付反。
  direction: Schemas['AdjustmentDirection'] | null
} => ({
  societyId: null,
  label: '',
  amountYuan: null,
  direction: null,
})

const advanceForm = ref(defaultAdvanceForm())
const adjustmentForm = ref(defaultAdjustmentForm())

const societyOptions = computed(() =>
  societyStore.societies.map((s) => ({
    label: s.is_home ? `${s.name}（本社团）` : s.name,
    value: s.id,
  }))
)

// --- 收摊清点 ---
// 输入框收「元」，提交时才换算成「分」。
const counts = ref<Record<string, number | null>>({})

// 清点表的行：把当前输入值摊进行里，输入变化时 `:data` 跟着变，表格会重渲染。
type ChannelRow = Schemas['ChannelLine'] & { entered: number | null }
const channelRows = computed<ChannelRow[]>(() =>
  (store.report?.channels ?? []).map((c) => ({ ...c, entered: counts.value[c.channel] ?? null }))
)

// 只有**已经清点过**的渠道才预填上次的人工实收（重新清点时保留已知值有意义）。
// 从未清点过的行必须留空（「收全量」原则）：build_report 对它们回落 actual = book，
// 预填账面值会让一键提交等于声称「每个渠道我都数了且都对」，而收全量这个设计的
// 全部意义就是分开「我数了，一致」和「我没数这个」。已经动过的值不被后续刷新覆盖。
watch(
  () => store.report?.channels,
  (rows) => {
    if (!rows) return
    const next: Record<string, number | null> = {}
    for (const c of rows) {
      const existing = counts.value[c.channel]
      next[c.channel] = Number.isFinite(existing)
        ? existing
        : c.counted
          ? fromCents(c.actual)
          : null
    }
    counts.value = next
  },
  { immediate: true }
)

function rowDiff(c: Schemas['ChannelLine']): Cents | null {
  const v = counts.value[c.channel]
  if (!Number.isFinite(v)) return null
  return cents(toCents(v ?? 0) - c.book)
}

const hasDiff = computed(() =>
  (store.report?.channels ?? []).some((c) => {
    const d = rowDiff(c)
    return d !== null && d !== 0
  })
)

function diffClass(c: Schemas['ChannelLine']) {
  const d = rowDiff(c)
  if (d === null || d === 0) return ''
  return d < 0 ? 'diff-short' : 'diff-over'
}

// --- 三个编辑区块的数据列表 ---
const advanceColumns = computed<DataTableColumns<Schemas['LedgerEntryRow']>>(() => [
  { title: '社团', key: 'society_name' },
  { title: '名目', key: 'label' },
  {
    title: '金额',
    key: 'amount',
    align: 'right',
    render: (row) => h('span', { class: 'amount-cell' }, formatYuan(row.amount)),
  },
  {
    title: '操作',
    key: 'actions',
    align: 'right',
    width: 96,
    render: (row) =>
      h(
        NButton,
        {
          size: 'small',
          type: 'error',
          quaternary: true,
          disabled: isBusy.value,
          onClick: () => removeAdvance(row),
        },
        { default: () => '删除' }
      ),
  },
])

const adjustmentColumns = computed<DataTableColumns<Schemas['LedgerEntryRow']>>(() => [
  { title: '社团', key: 'society_name' },
  { title: '名目', key: 'label' },
  {
    title: '方向与金额',
    key: 'amount',
    align: 'right',
    render: (row) => h('span', { class: 'amount-cell' }, describeEntryAdjustment(row.amount)),
  },
  {
    title: '操作',
    key: 'actions',
    align: 'right',
    width: 96,
    render: (row) =>
      h(
        NButton,
        {
          size: 'small',
          type: 'error',
          quaternary: true,
          disabled: isBusy.value,
          onClick: () => removeAdjustment(row),
        },
        { default: () => '删除' }
      ),
  },
])

const channelColumns = computed<DataTableColumns<ChannelRow>>(() => [
  { title: '渠道', key: 'channel' },
  {
    title: '账面应有',
    key: 'book',
    align: 'right',
    render: (row) => h('span', { class: 'amount-cell' }, formatYuan(row.book)),
  },
  {
    title: '实际到手（元）',
    key: 'entered',
    width: 180,
    render: (row) =>
      h(NInputNumber, {
        value: row.entered,
        min: 0,
        precision: 2,
        placeholder: '未填',
        'onUpdate:value': (value: number | null) => {
          counts.value[row.channel] = value
        },
      }),
  },
  {
    title: '差额',
    key: 'diff',
    align: 'right',
    render: (row) => h('span', { class: diffClass(row) }, formatYuan(rowDiff(row))),
  },
  { title: '是否清点', key: 'counted', render: (row) => (row.counted ? '已清点' : '未清点') },
])

async function submitAdvance() {
  const { societyId, label: rawLabel, amountYuan } = advanceForm.value
  if (!societyId) return fb.warning('请选择社团')
  const label = rawLabel.trim()
  if (!label) return fb.warning('请填写名目')
  if (amountYuan === null || !Number.isFinite(amountYuan) || amountYuan <= 0) {
    return fb.warning('垫付金额必须大于 0')
  }

  isBusy.value = true
  try {
    await store.createAdvance(eventId.value, {
      society_id: societyId,
      label,
      amount: toCents(amountYuan),
    })
    fb.success('垫付已记录')
    advanceForm.value = defaultAdvanceForm()
  } catch (error) {
    fb.error(error, '新增垫付失败')
  } finally {
    isBusy.value = false
  }
}

async function removeAdvance(entry: Schemas['LedgerEntryRow']) {
  await fb.confirm({
    title: '确认删除',
    content: `删除垫付「${entry.label}」（${formatYuan(entry.amount)}）？`,
    positiveText: '确认删除',
    negativeText: '取消',
    danger: true,
    onConfirm: async () => {
      isBusy.value = true
      try {
        await store.deleteAdvance(eventId.value, entry.id)
        fb.success('垫付已删除')
      } catch (error) {
        fb.error(error, '删除垫付失败')
      } finally {
        isBusy.value = false
      }
    },
  })
}

async function submitAdjustment() {
  const { societyId, direction, label: rawLabel, amountYuan } = adjustmentForm.value
  if (!societyId) return fb.warning('请选择社团')
  if (!direction) return fb.warning('请选择方向')
  const label = rawLabel.trim()
  if (!label) return fb.warning('请填写名目')
  if (amountYuan === null || !Number.isFinite(amountYuan) || amountYuan <= 0) {
    return fb.warning('调整金额必须大于 0')
  }

  isBusy.value = true
  try {
    await store.createAdjustment(eventId.value, {
      society_id: societyId,
      label,
      direction,
      amount: toCents(amountYuan),
    })
    fb.success('结算调整已记录')
    adjustmentForm.value = defaultAdjustmentForm()
  } catch (error) {
    fb.error(error, '新增结算调整失败')
  } finally {
    isBusy.value = false
  }
}

async function removeAdjustment(entry: Schemas['LedgerEntryRow']) {
  await fb.confirm({
    title: '确认删除',
    content: `删除结算调整「${entry.label}」（${describeEntryAdjustment(entry.amount)}）？`,
    positiveText: '确认删除',
    negativeText: '取消',
    danger: true,
    onConfirm: async () => {
      isBusy.value = true
      try {
        await store.deleteAdjustment(eventId.value, entry.id)
        fb.success('结算调整已删除')
      } catch (error) {
        fb.error(error, '删除结算调整失败')
      } finally {
        isBusy.value = false
      }
    },
  })
}

async function submitReconcile() {
  const rows = store.report?.channels ?? []
  const blank = rows.filter((c) => !Number.isFinite(counts.value[c.channel]))
  if (blank.length) {
    fb.warning(`这些渠道还没填实际到手：${blank.map((c) => c.channel).join('、')}`)
    return
  }
  const payload: Schemas['ReconcileRequest']['counts'] = rows.map((c) => ({
    channel: c.channel,
    actual: toCents(counts.value[c.channel] ?? 0),
  }))

  isBusy.value = true
  try {
    await store.reconcile(eventId.value, payload)
    fb.success('清点已提交')
  } catch (error) {
    // 漏渠道、重复渠道、本场没用过的渠道，后端 400 的原文原样显示。
    fb.error(error, '提交清点失败')
  } finally {
    isBusy.value = false
  }
}

onMounted(async () => {
  if (!societyStore.societies.length) await societyStore.fetchSocieties()
})
</script>

<style scoped>
.page-hint-row {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  margin-bottom: var(--space-lg);
}

.page-hint {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--font-base);
}

.block {
  margin-bottom: var(--space-xl);
}
/* 「展会结算之后，这两项仍然可以增删」是冻结例外的用户可见部分，
   单列一行、不用 muted 灰掉，免得摊主以为结算之后就补不了了。 */
.block-note {
  margin: 0 0 var(--space-xs);
  color: var(--warning-color);
  font-size: var(--font-sm);
}
.block-hint {
  margin: 0 0 var(--space-md);
  color: var(--text-muted);
  font-size: var(--font-sm);
  line-height: 1.6;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--space-md) var(--space-lg);
  align-items: end;
  margin-bottom: var(--space-lg);
}
.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  min-width: 0;
}
.field-wide {
  grid-column: 1 / -1;
}
.field-label {
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.field > *:not(.field-label) {
  width: 100%;
}
.actions {
  justify-content: flex-end;
}
@media (--phone) {
  .form-grid {
    grid-template-columns: minmax(0, 1fr);
  }
  /* 手机上把表单主按钮抬到 44px（「能用」标准）。 */
  .field.actions :deep(.n-button) {
    min-height: 44px;
  }
}

.amount-cell {
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.diff-short {
  color: var(--error-color);
  font-weight: var(--weight-bold);
}
.diff-over {
  color: var(--warning-color);
  font-weight: var(--weight-bold);
}

/* 短款第一反应是「能不能算到某个货主头上」，把母 spec 第 5 节那句话直接摆出来。 */
.shortfall-note {
  margin-top: var(--space-md);
  padding: var(--space-sm) var(--space-md);
  border-left: 3px solid var(--error-color);
  background-color: var(--card-bg-color);
  color: var(--error-color);
  font-size: var(--font-sm);
  line-height: 1.6;
}
</style>
