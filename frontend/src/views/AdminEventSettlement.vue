<template>
  <div class="page">
    <header class="page-header">
      <h1>展会结算</h1>
      <p>
        录垫付、结算调整和收摊清点。金额框里填「元」，提交时换算成「分」；
        业务规则由后端判定，这里只负责把后端那句话原样显示出来。
      </p>
    </header>

    <div v-if="store.isLoading && !store.report" class="loading-message">
      正在加载结算数据...
    </div>

    <template v-else>
      <n-alert v-if="store.error" type="error" class="store-error" :bordered="false">
        {{ store.error }}
      </n-alert>

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

        <div v-if="store.advances.length" class="table-wrapper">
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
                  <n-button size="small" type="error" quaternary :disabled="isBusy" @click="removeAdvance(a)">
                    删除
                  </n-button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <p v-else class="empty-line">暂无垫付</p>
      </section>

      <!-- 结算调整 -->
      <section class="block">
        <h2>结算调整</h2>
        <p class="block-note">展会结算之后，这两项仍然可以增删</p>
        <p class="block-hint">
          方向用按钮选，金额只填正数——不要自己判断该填正号还是负号。
        </p>

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

        <div v-if="store.adjustments.length" class="table-wrapper">
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
                  <n-button size="small" type="error" quaternary :disabled="isBusy" @click="removeAdjustment(a)">
                    删除
                  </n-button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <p v-else class="empty-line">暂无结算调整</p>
      </section>

      <!-- 收摊清点 -->
      <section v-if="store.report" class="block">
        <h2>收摊清点</h2>
        <p class="block-hint">
          收全量：本场用过的每个渠道都要报数，<strong>数过一致的也要报</strong>——
          「我数了，一致」和「我没数」是两件事。
        </p>

        <div class="table-wrapper">
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

        <p v-if="!store.report.channels.length" class="empty-line">
          本场还没有用过的收款渠道，无需清点。
        </p>

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
    </template>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import {
  NAlert,
  NButton,
  NInput,
  NInputNumber,
  NSelect,
  NSpace,
  NRadioGroup,
  NRadioButton,
  useDialog,
  useMessage,
} from 'naive-ui'
import { useSettlementStore } from '@/stores/settlementStore'
import { useSocietyStore } from '@/stores/societyStore'
import { formatYuan, toCents, fromCents } from '@/utils/money'

const props = defineProps({ id: { type: [String, Number], required: true } })

const store = useSettlementStore()
const societyStore = useSocietyStore()
const dialog = useDialog()
const message = useMessage()

const isBusy = ref(false)

const defaultAdvanceForm = () => ({ societyId: null, label: '', amountYuan: null })
const defaultAdjustmentForm = () => ({
  societyId: null,
  label: '',
  amountYuan: null,
  direction: 'to_them',
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
const counts = ref({})

// 预填当前实际到手；已经动过的值不被后续刷新覆盖（重拉报告时保留摊主刚敲的数）。
watch(
  () => store.report?.channels,
  (rows) => {
    if (!rows) return
    const next = {}
    for (const c of rows) {
      const existing = counts.value[c.channel]
      next[c.channel] = Number.isFinite(existing) ? existing : fromCents(c.actual)
    }
    counts.value = next
  },
  { immediate: true }
)

function rowDiff(c) {
  const v = counts.value[c.channel]
  if (!Number.isFinite(v)) return null
  return toCents(v) - c.book
}

const hasDiff = computed(() =>
  (store.report?.channels ?? []).some((c) => {
    const d = rowDiff(c)
    return d !== null && d !== 0
  })
)

function diffClass(c) {
  const d = rowDiff(c)
  if (d === null || d === 0) return ''
  return d < 0 ? 'diff-short' : 'diff-over'
}

/**
 * 结算调整列表里的方向。
 *
 * `GET /adjustments` 返回的就是 `settlement_adjustments.amount`：**负 = 我要多给他们**
 * （`to_them`），正 = 他们要多给我（`to_me`）。不要把原始符号摆出来，按方向说人话。
 */
function describeEntryAdjustment(cents) {
  if (cents < 0) return `我多给 ${formatYuan(-cents)}`
  if (cents > 0) return `他们多给 ${formatYuan(cents)}`
  return '—'
}

async function submitAdvance() {
  if (!advanceForm.value.societyId) return message.warning('请选择社团')
  const label = advanceForm.value.label.trim()
  if (!label) return message.warning('请填写名目')
  if (!Number.isFinite(advanceForm.value.amountYuan) || advanceForm.value.amountYuan <= 0) {
    return message.warning('垫付金额必须大于 0')
  }

  isBusy.value = true
  try {
    await store.createAdvance(props.id, {
      society_id: advanceForm.value.societyId,
      label,
      amount: toCents(advanceForm.value.amountYuan),
    })
    message.success('垫付已记录')
    advanceForm.value = defaultAdvanceForm()
  } catch (error) {
    message.error(error.message || '新增垫付失败')
  } finally {
    isBusy.value = false
  }
}

function removeAdvance(entry) {
  dialog.warning({
    title: '确认删除',
    content: `删除垫付「${entry.label}」（${formatYuan(entry.amount)}）？`,
    positiveText: '确认删除',
    negativeText: '取消',
    async onPositiveClick() {
      isBusy.value = true
      try {
        await store.deleteAdvance(props.id, entry.id)
        message.success('垫付已删除')
      } catch (error) {
        message.error(error.message || '删除垫付失败')
      } finally {
        isBusy.value = false
      }
    },
  })
}

async function submitAdjustment() {
  if (!adjustmentForm.value.societyId) return message.warning('请选择社团')
  const label = adjustmentForm.value.label.trim()
  if (!label) return message.warning('请填写名目')
  if (!Number.isFinite(adjustmentForm.value.amountYuan) || adjustmentForm.value.amountYuan <= 0) {
    return message.warning('调整金额必须大于 0')
  }

  isBusy.value = true
  try {
    await store.createAdjustment(props.id, {
      society_id: adjustmentForm.value.societyId,
      label,
      direction: adjustmentForm.value.direction,
      amount: toCents(adjustmentForm.value.amountYuan),
    })
    message.success('结算调整已记录')
    adjustmentForm.value = defaultAdjustmentForm()
  } catch (error) {
    message.error(error.message || '新增结算调整失败')
  } finally {
    isBusy.value = false
  }
}

function removeAdjustment(entry) {
  dialog.warning({
    title: '确认删除',
    content: `删除结算调整「${entry.label}」（${describeEntryAdjustment(entry.amount)}）？`,
    positiveText: '确认删除',
    negativeText: '取消',
    async onPositiveClick() {
      isBusy.value = true
      try {
        await store.deleteAdjustment(props.id, entry.id)
        message.success('结算调整已删除')
      } catch (error) {
        message.error(error.message || '删除结算调整失败')
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
    message.warning(`这些渠道还没填实际到手：${blank.map((c) => c.channel).join('、')}`)
    return
  }
  const payload = rows.map((c) => ({
    channel: c.channel,
    actual: toCents(counts.value[c.channel]),
  }))

  isBusy.value = true
  try {
    await store.reconcile(props.id, payload)
    message.success('清点已提交')
  } catch (error) {
    // 漏渠道、重复渠道、本场没用过的渠道，后端 400 的原文原样显示。
    message.error(error.message || '提交清点失败')
  } finally {
    isBusy.value = false
  }
}

onMounted(async () => {
  if (!societyStore.societies.length) await societyStore.fetchSocieties()
  await store.refresh(props.id)
})

onUnmounted(() => {
  store.resetStore()
})
</script>

<style scoped>
.page {
  max-width: 1080px;
}
.page-header {
  margin-bottom: 1.5rem;
}
.page-header h1 {
  margin: 0 0 0.25rem;
  font-size: var(--font-xl);
  color: var(--accent-color);
}
.page-header p {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--font-base);
  line-height: 1.6;
}

.store-error {
  margin-bottom: 1rem;
}

.block {
  margin-bottom: 2rem;
}
.block h2 {
  margin: 0 0 0.25rem;
  font-size: var(--font-lg, 1.125rem);
  color: var(--primary-text-color);
}
/* 「展会结算之后，这两项仍然可以增删」是冻结例外的用户可见部分，
   单列一行、不用 muted 灰掉，免得摊主以为结算之后就补不了了。 */
.block-note {
  margin: 0 0 0.25rem;
  color: var(--warning-color);
  font-size: var(--font-sm);
}
.block-hint {
  margin: 0 0 0.75rem;
  color: var(--text-muted);
  font-size: var(--font-sm);
  line-height: 1.6;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.9rem 1rem;
  align-items: end;
  margin-bottom: 1rem;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
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
  margin-top: 0.75rem;
}
@media (max-width: 640px) {
  .form-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}

.table-wrapper {
  width: 100%;
  overflow-x: auto;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
}
.data-table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
  font-size: var(--font-base);
}
.data-table th {
  padding: 10px 14px;
  background-color: var(--card-bg-color);
  color: var(--primary-text-color);
  font-weight: 600;
  border-bottom: 2px solid var(--accent-color);
  white-space: nowrap;
}
.data-table td {
  padding: 10px 14px;
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
  font-weight: 600;
}
.diff-over {
  color: var(--warning-color);
  font-weight: 600;
}

/* 短款第一反应是「能不能算到某个货主头上」，把母 spec 第 5 节那句话直接摆出来。 */
.shortfall-note {
  margin-top: 0.75rem;
  padding: 0.6rem 0.9rem;
  border-left: 3px solid var(--error-color);
  background-color: var(--card-bg-color);
  color: var(--error-color);
  font-size: var(--font-sm);
  line-height: 1.6;
}

.empty-line {
  margin: 0.5rem 0 0;
  color: var(--text-muted);
  font-size: var(--font-sm);
}

.loading-message {
  padding: 1rem;
  text-align: center;
  color: var(--text-muted);
}
</style>
