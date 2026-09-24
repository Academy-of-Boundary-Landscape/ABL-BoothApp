<template>
  <div class="page">
    <!-- warnings 必须最显眼：这里非空意味着业务表加出来的数和账本对不上，
         也就是某笔账记错了。不折叠、不放底部，逐条列在整张单最上方。 -->
    <n-alert
      v-if="store.report && store.report.warnings.length"
      type="error"
      :bordered="false"
      title="这张结算单和账本对不上，先别急着导出"
      class="warnings-block"
    >
      <p v-for="(w, i) in store.report.warnings" :key="i" class="warning-line">⚠ {{ w }}</p>
      <p class="warning-line muted">说明某笔账记错了，核对无误后再导出。</p>
    </n-alert>

    <header class="page-header">
      <div class="header-main">
        <h1>展会结算</h1>
        <p v-if="store.report" class="event-title">
          {{ store.report.event_name }} · {{ store.report.event_date }}
        </p>
        <p>
          录垫付、结算调整和收摊清点。金额框里填「元」，提交时换算成「分」；
          业务规则由后端判定，这里只负责把后端那句话原样显示出来。
        </p>
      </div>
      <n-space class="header-actions">
        <n-button :disabled="!store.report" @click="reloadReport">刷新</n-button>
        <n-button type="primary" ghost :disabled="!store.report" @click="exportXlsx">
          导出 Excel
        </n-button>
      </n-space>
    </header>

    <div v-if="store.isLoading && !store.report" class="loading-message">
      正在加载结算数据...
    </div>

    <template v-else>
      <n-alert v-if="store.error" type="error" class="store-error" :bordered="false">
        {{ store.error }}
      </n-alert>

      <!-- ── 结算单主体（母 spec 第 7 节）───────────────────────── -->
      <section v-if="store.report" class="block report-block">
        <div v-for="s in store.report.societies" :key="s.society_id" class="society-card">
          <div class="society-head">
            <span class="society-name">货主：{{ s.name }}</span>
            <n-tag v-if="s.is_home" size="small" type="success" round>本社团</n-tag>
          </div>

          <!-- 【货】收起时是合计，点「展开明细」到每个商品。 -->
          <div class="society-line">
            <span class="line-tag">【货】</span>
            <span class="line-body">
              带去 {{ s.totals.brought_in }} → 卖出 {{ s.totals.sold }} / 赠送
              {{ s.totals.gifted }} / 报废 {{ s.totals.scrapped }} / 带回
              {{ s.totals.taken_back }}
              <template v-if="s.totals.on_site">／现场仓 {{ s.totals.on_site }}</template>
              <span class="variance">盘点差异 {{ s.totals.variance }}</span>
              <!-- 未盘点时不能安静地按账面推算，必须标出来。 -->
              <n-tag
                v-if="!store.report.stocktaken"
                size="small"
                type="warning"
                class="stocktake-tag"
              >
                未盘点，剩余数为账面推算
              </n-tag>
              <n-button text size="tiny" class="detail-toggle" @click="toggleGoods(s.society_id)">
                {{ expanded[s.society_id] ? '收起明细' : `展开明细（${s.goods.length} 项）` }}
              </n-button>
            </span>
          </div>

          <div v-if="expanded[s.society_id]" class="goods-detail">
            <table class="data-table">
              <thead>
                <tr>
                  <th>商品</th>
                  <th class="text-right">带去</th>
                  <th class="text-right">卖出</th>
                  <th class="text-right">赠送</th>
                  <th class="text-right">报废</th>
                  <th class="text-right">差异</th>
                  <th class="text-right">带回</th>
                  <th class="text-right">现场仓</th>
                  <th class="text-right">原价</th>
                  <th class="text-right">折让</th>
                  <th class="text-right">净额</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="g in s.goods" :key="g.event_product_id">
                  <td>{{ g.product_code }} {{ g.name }}</td>
                  <td class="text-right">{{ g.brought_in }}</td>
                  <td class="text-right">{{ g.sold }}</td>
                  <td class="text-right">{{ g.gifted }}</td>
                  <td class="text-right">{{ g.scrapped }}</td>
                  <td class="text-right">{{ g.variance }}</td>
                  <td class="text-right">{{ g.taken_back }}</td>
                  <td class="text-right">{{ g.on_site }}</td>
                  <td class="text-right amount-cell">{{ formatYuan(g.gross) }}</td>
                  <td class="text-right amount-cell">{{ formatSigned(g.lot_discount) }}</td>
                  <td class="text-right amount-cell">{{ formatYuan(g.allocated) }}</td>
                </tr>
                <tr v-if="!s.goods.length">
                  <td colspan="11" class="empty-line">这个货主没有上架商品</td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="society-line">
            <span class="line-tag">【钱】</span>
            <span class="line-body">
              商品原价 {{ formatYuan(s.gross) }} · Lot折让 {{ formatSigned(s.lot_discount) }} ·
              手工折让 {{ formatSigned(s.manual_discount) }} → 净额 {{ formatYuan(s.net) }}
              <!-- 这两项是「我应转给」的加项，xlsx 里有、网页上原来漏了。
                   非零时才出现，否则默认全 0 的行会淹没真正的数字。 -->
              <template v-if="s.refund_kept"> · 退货保留 {{ formatYuan(s.refund_kept) }}</template>
              <template v-if="s.gift_self_paid">
                · 自掏赠品 {{ formatYuan(s.gift_self_paid) }}
              </template>
            </span>
          </div>

          <div class="society-line">
            <span class="line-tag">【我垫付】</span>
            <span class="line-body">
              <template v-if="s.advances.length">
                {{ advanceSummary(s.advances) }} = {{ formatYuan(s.advances_total) }}
              </template>
              <template v-else>（无）</template>
            </span>
          </div>

          <div class="society-line">
            <span class="line-tag">【调整】</span>
            <span class="line-body">
              <template v-if="s.adjustments.length">
                {{ adjustmentSummary(s.adjustments) }}
                <!-- 单条明细时合计和它上面那句完全一样，没必要念两遍；
                     多条时才需要合计，且合计也必须说人话，不能露原始符号。 -->
                <template v-if="s.adjustments.length > 1">
                  = {{ describeReportAdjustment(s.adjustments_total) }}
                </template>
              </template>
              <template v-else>（无）</template>
            </span>
          </div>

          <!-- 这一块唯一要被记住的数字，视觉上压过其它行。 -->
          <div class="transfer-row">
            我应转给{{ s.name }}：
            <span class="transfer-amount">{{ formatYuan(s.transfer) }}</span>
          </div>
        </div>

        <div class="report-totals">
          <span>实际到手合计 <strong>{{ formatYuan(store.report.actual_total) }}</strong></span>
          <span>Σ 我应转给 <strong>{{ formatYuan(store.report.transfer_total) }}</strong></span>
          <span>摊主留存 <strong>{{ formatYuan(store.report.vendor_retained) }}</strong></span>
        </div>

        <!-- 两个时间戳平铺，不加警告语气。generated_at（此刻）和 last_changed_at
             （最新 journal 的时间）几乎恒不相等，按字面提示「导出前请刷新」会基本
             常亮；常亮的警告会训练人忽略警告，而这一页顶部有个真正不能被稀释的
             红色 warnings 块。摊主自己看得出新旧。 -->
        <p class="report-meta">
          生成于 {{ store.report.generated_at }} · 账本最后变动于
          {{ store.report.last_changed_at || '（无记录）' }}
        </p>
      </section>

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
  NTag,
  useDialog,
  useMessage,
} from 'naive-ui'
import { useSettlementStore } from '@/stores/settlementStore'
import { useSocietyStore } from '@/stores/societyStore'
import { formatYuan, toCents, fromCents } from '@/utils/money'
import {
  describeEntryAdjustment,
  describeReportAdjustment,
} from '@/utils/settlementSigns'
import { toAbsoluteApiUrl } from '@/services/url'
import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

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
  // 不给默认值：方向是「我要多给他们」还是「他们要多给我」必须由人明确选一次，
  // 默认 to_them 会让累了一天的摊主根本没看控件就提交，然后钱付反。
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
const counts = ref({})

// 只有**已经清点过**的渠道才预填已知的实际到手（重新清点时要保留已知值有意义）。
// 从未清点过的行必须留空：build_report 对它们回落 actual = book，预填账面值会让
// 一键提交等于声称「每个渠道我都数了且都对」，而收全量这个设计的全部意义就是
// 分开「我数了，一致」和「我没数这个」。已经动过的值不被后续刷新覆盖。
watch(
  () => store.report?.channels,
  (rows) => {
    if (!rows) return
    const next = {}
    for (const c of rows) {
      const existing = counts.value[c.channel]
      if (Number.isFinite(existing)) next[c.channel] = existing
      else next[c.channel] = c.counted ? fromCents(c.actual) : null
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

// --- 结算单主体 ---
// 【货】默认收起，展开状态只是界面状态，不落库。
const expanded = ref({})

function toggleGoods(societyId) {
  expanded.value = { ...expanded.value, [societyId]: !expanded.value[societyId] }
}

/**
 * 折让显示成带符号的减/加：折让为正（真折让）→ `−¥x`，加价为负 → `+¥x`。
 * 直接 `formatYuan(cents)` 对负数会打印成 `¥-60.00`（符号在货币号之后），
 * 加价那一行必须在标签下面读得出是个加项，不能靠摊主自己看负号。
 */
function formatSigned(cents) {
  if (cents > 0) return `−${formatYuan(cents)}`
  if (cents < 0) return `+${formatYuan(-cents)}`
  return formatYuan(0)
}

function advanceSummary(entries) {
  return entries.map((a) => `${a.label} ${formatYuan(a.amount)}`).join(' + ')
}

function adjustmentSummary(entries) {
  return entries
    .map((a) => {
      const at = a.at ? `（${a.at}）` : ''
      return `${a.label} ${describeReportAdjustment(a.amount)}${at}`
    })
    .join('；')
}

async function reloadReport() {
  await store.refresh(props.id)
}

/**
 * 导出 xlsx。**照 AdminEventStat 那条验过的路径抄**：Tauri 里 `window.open`
 * 不一定触发下载，得走 plugin-http 取字节 + 保存对话框 + 写文件。
 */
async function exportXlsx() {
  if (!store.report) return

  const isTauri = window.__TAURI_INTERNALS__ !== undefined
  const token = sessionStorage.getItem('access_token')
  const safeName = (store.report.event_name || 'settlement').replace(/[\\/:*?"<>|]/g, '_')
  const fileName = `settlement_${safeName}.xlsx`
  const url = toAbsoluteApiUrl(`/api/events/${props.id}/settlement.xlsx`)

  try {
    if (isTauri) {
      const headers = {
        Accept: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
      }
      if (token) headers['Authorization'] = `Bearer ${token}`

      const resp = await tauriFetch(url, { method: 'GET', headers })
      if (!resp.ok) {
        const text = await resp.text().catch(() => '')
        throw new Error(`下载失败: ${resp.status} ${resp.statusText} ${text.slice(0, 200)}`)
      }

      const ab = await resp.arrayBuffer()
      const bytes = new Uint8Array(ab)
      const filePath = await save({
        defaultPath: fileName,
        filters: [{ name: 'Excel Files', extensions: ['xlsx'] }],
      })
      if (!filePath) return

      await writeFile(filePath, bytes)
      alert('导出成功')
      return
    }

    const headers = {}
    if (token) headers['Authorization'] = `Bearer ${token}`
    const response = await fetch(url, { method: 'GET', credentials: 'include', headers })
    if (!response.ok) {
      const text = await response.text().catch(() => '')
      throw new Error(`下载失败: ${response.status} ${text.slice(0, 200)}`)
    }

    const blob = await response.blob()
    const dl = window.URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.style.display = 'none'
    a.href = dl
    a.download = fileName
    document.body.appendChild(a)
    a.click()
    setTimeout(() => {
      document.body.removeChild(a)
      window.URL.revokeObjectURL(dl)
    }, 100)
  } catch (error) {
    console.error('下载结算单失败:', error)
    alert(error?.message || '下载失败')
  }
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
  if (!adjustmentForm.value.direction) return message.warning('请选择方向')
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
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
  margin-bottom: 1.5rem;
}
.header-main {
  min-width: 0;
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
.page-header .event-title {
  color: var(--primary-text-color);
  font-weight: 600;
}
.header-actions {
  flex: 0 0 auto;
}

/* warnings 是「业务表加出来的数和账本对不上」，n-alert 自带红底，这里只压间距。 */
.warnings-block {
  margin-bottom: 1.25rem;
}
.warning-line {
  margin: 0.2rem 0;
  line-height: 1.6;
  word-break: break-word;
}
.warning-line.muted {
  color: var(--text-muted);
}

/* ── 结算单主体 ── */
.report-block {
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  padding: 1rem 1.25rem;
  background-color: var(--card-bg-color);
}
.society-card {
  border-bottom: 1px solid var(--border-color);
  padding-bottom: 0.75rem;
  margin-bottom: 0.75rem;
}
.society-card:last-of-type {
  border-bottom: none;
}
.society-head {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.35rem;
}
.society-name {
  font-weight: 700;
  color: var(--primary-text-color);
}
.society-line {
  display: flex;
  gap: 0.4rem;
  padding: 0.15rem 0;
  font-size: var(--font-sm);
  line-height: 1.7;
  color: var(--text-placeholder);
}
.line-tag {
  flex: 0 0 auto;
  color: var(--accent-color);
  font-weight: 600;
}
.line-body {
  min-width: 0;
}
.variance {
  margin-left: 0.75rem;
}
.stocktake-tag {
  margin-left: 0.5rem;
}
.detail-toggle {
  margin-left: 0.5rem;
}
.goods-detail {
  margin: 0.5rem 0;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  overflow-x: auto;
}
.transfer-row {
  margin-top: 0.5rem;
  padding-top: 0.5rem;
  border-top: 1px solid var(--border-color);
  font-size: var(--font-base);
  color: var(--primary-text-color);
}
/* 这一块唯一要被记住的数字，视觉上压过其它行。 */
.transfer-amount {
  margin-left: 0.35rem;
  font-size: 1.6rem;
  font-weight: 700;
  color: var(--accent-color);
  font-variant-numeric: tabular-nums;
}
.report-totals {
  display: flex;
  flex-wrap: wrap;
  gap: 1.25rem;
  justify-content: flex-end;
  padding-top: 0.5rem;
  border-top: 2px solid var(--accent-color);
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.report-totals strong {
  color: var(--primary-text-color);
  font-variant-numeric: tabular-nums;
}
.report-meta {
  margin: 0.75rem 0 0;
  font-size: var(--font-xs, 0.75rem);
  color: var(--text-muted);
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
