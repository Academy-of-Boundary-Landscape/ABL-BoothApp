<template>
  <PageShell embedded width="wide">
    <p class="page-hint">
      录垫付、结算调整和收摊清点。金额框里填「元」，提交时换算成「分」；
      业务规则由后端判定，这里只负责把后端那句话原样显示出来。
    </p>

    <!-- 只读结算单 + 导出（管理端与摊主端共用，spec §5.4）。 -->
    <SettlementReportView :event-id="eventId" />

    <!-- 垫付 -->
    <section class="block">
      <h2>垫付</h2>
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

      <div v-if="store.advances.length" class="table-scroll">
        <table class="data-table">
          <thead>
            <tr>
              <th>社团</th>
              <th>名目</th>
              <th class="text-right">金额</th>
              <th class="text-right">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="a in store.advances" :key="a.id">
              <td>{{ a.society_name }}</td>
              <td>{{ a.label }}</td>
              <td class="text-right amount-cell">{{ formatYuan(a.amount) }}</td>
              <td class="text-right">
                <n-button
                  size="small"
                  type="error"
                  quaternary
                  :disabled="isBusy"
                  @click="removeAdvance(a)"
                >
                  删除
                </n-button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <EmptyState v-else compact title="暂无垫付" />
    </section>

    <!-- 结算调整 -->
    <section class="block">
      <h2>结算调整</h2>
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

      <div v-if="store.adjustments.length" class="table-scroll">
        <table class="data-table">
          <thead>
            <tr>
              <th>社团</th>
              <th>名目</th>
              <th class="text-right">方向与金额</th>
              <th class="text-right">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="a in store.adjustments" :key="a.id">
              <td>{{ a.society_name }}</td>
              <td>{{ a.label }}</td>
              <td class="text-right amount-cell">{{ describeEntryAdjustment(a.amount) }}</td>
              <td class="text-right">
                <n-button
                  size="small"
                  type="error"
                  quaternary
                  :disabled="isBusy"
                  @click="removeAdjustment(a)"
                >
                  删除
                </n-button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <EmptyState v-else compact title="暂无结算调整" />
    </section>

    <!-- 收摊清点 -->
    <section v-if="store.report" class="block">
      <h2>收摊清点</h2>
      <p class="block-hint">
        收全量：本场用过的每个渠道都要报数，<strong>数过一致的也要报</strong>——
        「我数了，一致」和「我没数」是两件事。
      </p>

      <div class="table-scroll">
        <table class="data-table">
          <thead>
            <tr>
              <th>渠道</th>
              <th class="text-right">账面应有</th>
              <th class="text-right">实际到手（元）</th>
              <th class="text-right">差额</th>
              <th>是否清点</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="c in store.report.channels" :key="c.channel">
              <td>{{ c.channel }}</td>
              <td class="text-right amount-cell">{{ formatYuan(c.book) }}</td>
              <td class="text-right">
                <n-input-number
                  v-model:value="counts[c.channel]"
                  :min="0"
                  :precision="2"
                  class="count-input"
                />
              </td>
              <td class="text-right amount-cell" :class="diffClass(c)">
                {{ formatYuan(rowDiff(c)) }}
              </td>
              <td>{{ c.counted ? '已清点' : '未清点' }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <EmptyState
        v-if="!store.report.channels.length"
        compact
        title="本场还没有用过的收款渠道，无需清点。"
      />

      <div v-if="hasDiff" class="shortfall-note">
        差额由摊主自己承担，不进任何货主的结算。要推给某个货主，请到上面加一条结算调整。
      </div>

      <div class="actions-row">
        <n-button
          type="primary"
          :disabled="isBusy || !store.report.channels.length"
          @click="submitReconcile"
        >
          提交清点
        </n-button>
      </div>
    </section>
  </PageShell>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { NButton, NInput, NInputNumber, NSelect, NSpace, NRadioGroup, NRadioButton } from 'naive-ui'
import { useSettlementStore } from '@/stores/settlementStore'
import { useSocietyStore } from '@/stores/societyStore'
import { formatYuan, cents, toCents, fromCents, type Cents } from '@/utils/money'
import { describeEntryAdjustment } from '@/utils/settlementSigns'
import type { Schemas } from '@/api/client'
import { PageShell, EmptyState } from '@/components/ui'
import SettlementReportView from '@/components/settlement/SettlementReportView.vue'
import { useFeedback } from '@/composables/useFeedback'

const props = defineProps<{ id: string | number }>()

const eventId = computed(() => Number(props.id))

const store = useSettlementStore()
const societyStore = useSocietyStore()
const fb = useFeedback()

const isBusy = ref(false)

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

// 只有**已经清点过**的渠道才预填已知的实际到手（重新清点时要保留已知值有意义）。
// 从未清点过的行必须留空：build_report 对它们回落 actual = book，预填账面值会让
// 一键提交等于声称「每个渠道我都数了且都对」，而收全量这个设计的全部意义就是
// 分开「我数了，一致」和「我没数这个」。已经动过的值不被后续刷新覆盖。
watch(
  () => store.report?.channels,
  (rows) => {
    if (!rows) return
    const next: Record<string, number | null> = {}
    for (const c of rows) {
      const existing = counts.value[c.channel]
      if (Number.isFinite(existing)) next[c.channel] = existing ?? null
      else next[c.channel] = c.counted ? fromCents(c.actual) : null
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
.page-hint {
  margin: 0 0 var(--space-lg);
  color: var(--text-muted);
  font-size: var(--font-base);
  line-height: var(--leading-base);
}

.block {
  margin-bottom: var(--space-2xl);
}
.block h2 {
  margin: 0 0 var(--space-xs);
  font-size: var(--font-lg);
  color: var(--primary-text-color);
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
.actions-row {
  display: flex;
  justify-content: flex-end;
  margin-top: var(--space-md);
}
@media (--phone) {
  .form-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}

.data-table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
  font-size: var(--font-base);
}
.data-table th {
  padding: var(--space-sm) var(--space-md);
  background-color: var(--card-bg-color);
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
  border-bottom: 2px solid var(--accent-color);
  white-space: nowrap;
}
.data-table td {
  padding: var(--space-sm) var(--space-md);
  border-bottom: 1px solid var(--border-color);
  color: var(--text-placeholder);
  vertical-align: middle;
}
.text-right {
  text-align: right;
}
.amount-cell {
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.count-input {
  width: 140px;
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
