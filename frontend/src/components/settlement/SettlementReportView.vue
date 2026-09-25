<!--
  只读结算单（spec §5.4）。管理端「收摊 · 结算」与摊主端「收摊」tab 共用同一份渲染：
  货主块（【货】【钱】【我垫付】【调整】与我应转给）、底部合计与时间戳，外加 xlsx 导出。
  垫付 / 结算调整 / 收摊清点三个可编辑区块留在 `AdminEventSettlement.vue`，不在本组件内。

  数据自己从 `settlementStore` 取（`refresh` 会一并拉垫付与调整列表，管理端编辑区块共用），
  错误态交给 `AsyncState`——页面不再手写 `n-alert type="error"`。
-->
<template>
  <div class="settlement-report">
    <div class="report-toolbar">
      <n-space class="report-actions">
        <n-button :disabled="!store.report" @click="reloadReport">刷新</n-button>
        <n-button type="primary" ghost :disabled="!store.report" @click="exportXlsx">
          导出 Excel
        </n-button>
      </n-space>
    </div>

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

    <AsyncState
      :loading="store.isLoading && !store.report"
      :error="store.error"
      loading-text="正在加载结算数据..."
    >
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
                  <td colspan="11"><EmptyState compact title="这个货主没有上架商品" /></td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="society-line">
            <span class="line-tag">【钱】</span>
            <span class="line-body">
              商品原价 {{ formatYuan(s.gross) }} · Lot折让 {{ formatSigned(s.lot_discount) }} ·
              {{ manualDiscountLabel(s.manual_discount) }} {{ formatSigned(s.manual_discount) }} →
              净额 {{ formatYuan(s.net) }}
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
                {{ advanceSummary(s.advances) }} = {{ formatSigned(s.advances_total) }}
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
          <span
            >实际到手合计 <strong>{{ formatYuan(store.report.actual_total) }}</strong></span
          >
          <span
            >Σ 我应转给 <strong>{{ formatYuan(store.report.transfer_total) }}</strong></span
          >
          <span
            >摊主留存 <strong>{{ formatYuan(store.report.vendor_retained) }}</strong></span
          >
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
    </AsyncState>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { NAlert, NButton, NSpace, NTag } from 'naive-ui'
import { useSettlementStore } from '@/stores/settlementStore'
import { AsyncState, EmptyState } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import { formatYuan, cents, type Cents } from '@/utils/money'
import { describeReportAdjustment, manualDiscountLabel } from '@/utils/settlementSigns'
import { toAbsoluteApiUrl } from '@/services/url'
import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'
import type { Schemas } from '@/api/client'

const props = defineProps<{ eventId: number }>()

const store = useSettlementStore()
const fb = useFeedback()

// 【货】默认收起，展开状态只是界面状态，不落库。
const expanded = ref<Record<number, boolean>>({})

function toggleGoods(societyId: number) {
  expanded.value = { ...expanded.value, [societyId]: !expanded.value[societyId] }
}

/**
 * 折让显示成带符号的减/加：折让为正（真折让）→ `−¥x`，加价为负 → `+¥x`。
 * 直接 `formatYuan(cents)` 对负数会打印成 `¥-60.00`（符号在货币号之后），
 * 加价那一行必须在标签下面读得出是个加项，不能靠摊主自己看负号。
 */
function formatSigned(amount: Cents) {
  if (amount > 0) return `−${formatYuan(amount)}`
  if (amount < 0) return `+${formatYuan(cents(-amount))}`
  return formatYuan(cents(0))
}

function advanceSummary(entries: Schemas['SettlementEntry'][]) {
  // 垫付是「我应转给」的减项，逐条也带上符号，和右边那个 = 合计对得上。
  return entries.map((a) => `${a.label} ${formatSigned(a.amount)}`).join(' + ')
}

function adjustmentSummary(entries: Schemas['SettlementEntry'][]) {
  return entries
    .map((a) => {
      const at = a.at ? `（${a.at}）` : ''
      return `${a.label} ${describeReportAdjustment(a.amount)}${at}`
    })
    .join('；')
}

async function reloadReport() {
  await store.refresh(props.eventId)
}

/**
 * 导出 xlsx。**照 AdminEventStat 那条验过的路径抄**：Tauri 里 `window.open`
 * 不一定触发下载，得走 plugin-http 取字节 + 保存对话框 + 写文件。
 */
async function exportXlsx() {
  const report = store.report
  if (!report) return

  const isTauri = window.__TAURI_INTERNALS__ !== undefined
  const token = sessionStorage.getItem('access_token')
  const safeName = (report.event_name || 'settlement').replace(/[\\/:*?"<>|]/g, '_')
  const fileName = `settlement_${safeName}.xlsx`
  const url = toAbsoluteApiUrl(`/api/events/${props.eventId}/settlement.xlsx`)

  try {
    if (isTauri) {
      const headers: Record<string, string> = {
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
      fb.success('导出成功')
      return
    }

    const headers: Record<string, string> = {}
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
    fb.error(error, '下载失败')
  }
}

onMounted(() => {
  void store.refresh(props.eventId)
})

watch(
  () => props.eventId,
  (id) => {
    void store.refresh(id)
  }
)

onUnmounted(() => {
  store.resetStore()
})
</script>

<style scoped>
/* 只读组件没有自己的页头；刷新 / 导出靠右，与管理端原来的工具栏一致。 */
.report-toolbar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-lg);
  flex-wrap: wrap;
  margin-bottom: var(--space-lg);
}
.report-actions {
  flex: 0 0 auto;
}

/* warnings 是「业务表加出来的数和账本对不上」，n-alert 自带红底，这里只压间距。 */
.warnings-block {
  margin-bottom: var(--space-lg);
}
.warning-line {
  margin: var(--space-xs) 0;
  line-height: 1.6;
  /* stylelint-disable-next-line declaration-property-value-keyword-no-deprecated -- 保留原关键字，不做行为变更 */
  word-break: break-word;
}
.warning-line.muted {
  color: var(--text-muted);
}

/* ── 结算单主体 ── */
.report-block {
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  padding: var(--space-lg) var(--space-xl);
  background-color: var(--card-bg-color);
}
.society-card {
  border-bottom: 1px solid var(--border-color);
  padding-bottom: var(--space-md);
  margin-bottom: var(--space-md);
}
.society-card:last-of-type {
  border-bottom: none;
}
.society-head {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  margin-bottom: var(--space-xs);
}
.society-name {
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
}
.society-line {
  display: flex;
  gap: var(--space-sm);
  padding: var(--space-xs) 0;
  font-size: var(--font-sm);
  line-height: 1.7;
  color: var(--text-placeholder);
}
.line-tag {
  flex: 0 0 auto;
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}
.line-body {
  min-width: 0;
}
.variance {
  margin-left: var(--space-md);
}
.stocktake-tag {
  margin-left: var(--space-sm);
}
.detail-toggle {
  margin-left: var(--space-sm);
}
.goods-detail {
  margin: var(--space-sm) 0;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  overflow-x: auto;
}
.transfer-row {
  margin-top: var(--space-sm);
  padding-top: var(--space-sm);
  border-top: 1px solid var(--border-color);
  font-size: var(--font-base);
  color: var(--primary-text-color);
}
/* 这一块唯一要被记住的数字，视觉上压过其它行。 */
.transfer-amount {
  margin-left: var(--space-xs);
  font-size: var(--font-xl);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
  font-variant-numeric: tabular-nums;
}
.report-totals {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-lg);
  justify-content: flex-end;
  padding-top: var(--space-sm);
  border-top: 2px solid var(--accent-color);
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.report-totals strong {
  color: var(--primary-text-color);
  font-variant-numeric: tabular-nums;
}
.report-meta {
  margin: var(--space-md) 0 0;
  font-size: var(--font-xs);
  color: var(--text-muted);
  line-height: 1.6;
}

.block {
  margin-bottom: var(--space-2xl);
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
</style>
