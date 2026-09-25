<template>
  <PageShell embedded width="full">
    <div class="page-toolbar">
      <div class="page-hint-row">
        <p class="page-hint">查看当前展会的销售数据和统计分析。</p>
        <HelpBubble page="event-stats" />
      </div>
      <div v-if="statStore.stats && statStore.stats.summary.length > 0" class="download-actions">
        <n-button class="download-btn" type="default" ghost size="large" @click="downloadCsv">
          下载 CSV
        </n-button>

        <n-button class="download-btn" type="primary" ghost size="large" @click="downloadReport">
          <template #icon>
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
              <polyline points="7 10 12 15 17 10"></polyline>
              <line x1="12" y1="15" x2="12" y2="3"></line>
            </svg>
          </template>
          下载 Excel 报告
        </n-button>
      </div>
    </div>

    <!-- 错误态交给 AsyncState，不再手写「后端数据库寄了！」。error 优先于 empty。 -->
    <AsyncState
      :loading="statStore.isLoading"
      :error="statStore.error"
      loading-text="正在从数据库中提取统计信息..."
      @retry="applyFilters"
    >
      <div v-if="statStore.stats" class="stats-content">
        <SectionCard
          title="数据筛选"
          collapsible
          v-model:collapsed="isFilterCollapsed"
          class="filter-section"
        >
          <StatFilters
            :product-options="productOptions"
            :selected-product="selectedProduct"
            :start-date="startDate"
            :end-date="endDate"
            :interval-minutes="intervalMinutes"
            @update:selectedProduct="(val) => (selectedProduct = val)"
            @update:startDate="(val) => (startDate = val)"
            @update:endDate="(val) => (endDate = val)"
            @update:intervalMinutes="(val) => (intervalMinutes = val)"
            @change="applyFilters"
          />
        </SectionCard>

        <!-- 关键数据总览 -->
        <SectionCard
          title="关键数据总览"
          collapsible
          v-model:collapsed="isSummaryCollapsed"
          class="summary-section"
        >
          <div class="summary-cards">
            <StatTile label="总销售额" :value="formatCurrency(statStore.stats.total_revenue)" />
            <StatTile label="总销售件数" :value="totalItemsSold" />
            <StatTile label="销售品类数" :value="productVarietyCount" />
          </div>
        </SectionCard>

        <!-- 销售趋势图 -->
        <SectionCard
          title="销售额趋势"
          collapsible
          v-model:collapsed="isChartCollapsed"
          class="chart-section"
        >
          <div class="chart-info">
            <span v-if="statStore.stats.timeseries?.length" class="chart-subtitle">{{
              chartSubtitle
            }}</span>
          </div>
          <SalesLineChart
            v-if="statStore.stats.timeseries?.length"
            :series="chartSeries"
            :width="chartWidth"
            :height="chartHeight"
            :padding="padding"
          />
          <EmptyState v-else compact title="// 暂无趋势数据" />
        </SectionCard>

        <!-- 销售详情表格 -->
        <SectionCard
          title="销售数据表"
          collapsible
          v-model:collapsed="isTableCollapsed"
          class="table-section"
        >
          <EmptyState
            v-if="!statStore.stats.summary.length"
            compact
            title="// 无有效销售数据记录..."
          />
          <n-data-table
            v-else
            :columns="summaryColumns"
            :data="statStore.stats.summary"
            :row-key="(row) => row.product_id"
            :scroll-x="680"
            size="small"
          />
        </SectionCard>
      </div>
    </AsyncState>
  </PageShell>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, watch, computed, ref, h } from 'vue'
import { useRoute } from 'vue-router'
import { useEventStatStore } from '@/stores/eventStatStore'
import SalesLineChart from '@/components/stats/SalesLineChart.vue'
import StatFilters from '@/components/stats/StatFilters.vue'
import { NButton, NDataTable, type DataTableColumns } from 'naive-ui'
import { PageShell, SectionCard, AsyncState, EmptyState, StatTile, Money } from '@/components/ui'
import HelpBubble from '@/components/shared/HelpBubble.vue'
import { useFeedback } from '@/composables/useFeedback'
import { toAbsoluteApiUrl } from '@/services/url'
import { formatYuan, fromCents, type Cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

const route = useRoute()
const statStore = useEventStatStore()
const fb = useFeedback()
const selectedProduct = ref('')
const startDate = ref('')
const endDate = ref('')
const intervalMinutes = ref(60)
const chartWidth = ref(800)
const chartHeight = 320
const padding = 48

function updateChartWidth() {
  const el = document.querySelector('.chart-section .section-card__body')
  if (el) chartWidth.value = Math.min(el.clientWidth - padding * 2, 1200)
}
const isFilterCollapsed = ref(false)
const isSummaryCollapsed = ref(false)
const isChartCollapsed = ref(false)
const isTableCollapsed = ref(false)

const totalItemsSold = computed(
  () => statStore.stats?.summary.reduce((sum, item) => sum + item.total_quantity, 0) || 0
)
const productVarietyCount = computed(() => statStore.stats?.summary.length || 0)
const productOptions = computed(() => {
  const summary = statStore.stats?.summary || []
  const unique = new Map<string, { code: string; name: string }>()
  summary.forEach((item) => {
    if (!unique.has(item.product_code)) {
      unique.set(item.product_code, { code: item.product_code, name: item.product_name })
    }
  })
  return Array.from(unique.values())
})

// 明细表：语义化表格换成 n-data-table，窄屏在自身区域内横滑。
const summaryColumns: DataTableColumns<Schemas['StatsProductSalesItem']> = [
  {
    title: '制品编号',
    key: 'product_code',
    width: 120,
    render: (item) => `#${item.product_code}`,
  },
  { title: '制品名', key: 'product_name', minWidth: 160 },
  {
    title: '单价',
    key: 'unit_price',
    width: 120,
    align: 'right',
    render: (item) => h(Money, { value: item.unit_price }),
  },
  { title: '销售量', key: 'total_quantity', width: 96, align: 'center' },
  {
    title: '销售额',
    key: 'total_revenue_per_item',
    width: 130,
    align: 'right',
    render: (item) => h(Money, { value: item.total_revenue_per_item }),
  },
]

const chartSubtitle = computed(() => {
  const parts: string[] = []
  if (selectedProduct.value) parts.push(`制品 ${selectedProduct.value}`)
  if (startDate.value) parts.push(`自 ${startDate.value}`)
  if (endDate.value) parts.push(`至 ${endDate.value}`)
  parts.push(intervalMinutes.value === 30 ? '每 30 分钟' : '每小时')
  return parts.join(' · ')
})

function formatCurrency(value: Cents) {
  // 统计接口的金额一律是分，展示走唯一的换算入口。
  return formatYuan(value)
}

// 趋势图的内部数值按元计算（坐标轴/提示框），所以先把分换算成元再传。
const chartSeries = computed(() =>
  (statStore.stats?.timeseries || []).map((p) => ({ ...p, revenue: fromCents(p.revenue) }))
)

async function applyFilters() {
  await statStore.fetchStats({
    productCode: selectedProduct.value,
    startDate: startDate.value,
    endDate: endDate.value,
    intervalMinutes: intervalMinutes.value,
  })
}
async function downloadReport() {
  const stats = statStore.stats
  if (!stats || !stats.summary?.length) return

  const isTauri = window.__TAURI_INTERNALS__ !== undefined
  const token = sessionStorage.getItem('access_token')

  const safeName = (stats.event_name || 'sales_report').replace(/[\\/:*?"<>|]/g, '_')
  const fileName = `sales_report_${safeName}.xlsx`

  const url = toAbsoluteApiUrl(statStore.downloadUrl)

  try {
    console.log('开始请求 Excel 报告:', url, 'isTauri:', isTauri)

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

    // 浏览器环境
    const headers: Record<string, string> = {}
    if (token) headers['Authorization'] = `Bearer ${token}`

    const response = await fetch(url, {
      method: 'GET',
      credentials: 'include',
      headers,
    })

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
  } catch (e) {
    console.error('下载 Excel 报告失败:', e)
    fb.error(e, '下载失败')
  }
}

function escapeCsvCell(value: unknown) {
  const text = String(value ?? '')
  if (/[,"\n\r]/.test(text)) {
    return `"${text.replace(/"/g, '""')}"`
  }
  return text
}

async function downloadCsv() {
  const stats = statStore.stats
  const summary = stats?.summary || []
  if (!summary.length) return

  try {
    const isTauri = window.__TAURI_INTERNALS__ !== undefined
    const safeName = (stats?.event_name || 'sales_report').replace(/[\\/:*?"<>|]/g, '_')
    const fileName = `sales_report_${safeName}.csv`

    const header = ['制品编号', '制品名', '单价', '销售量', '销售额']
    const rows = summary.map((item) => [
      item.product_code ?? '',
      item.product_name ?? '',
      // 接口是分，CSV 给人看，换算成元。
      typeof item.unit_price === 'number' ? fromCents(item.unit_price).toFixed(2) : '0.00',
      item.total_quantity ?? 0,
      typeof item.total_revenue_per_item === 'number'
        ? fromCents(item.total_revenue_per_item).toFixed(2)
        : '0.00',
    ])

    const csvContent = [header, ...rows]
      .map((cols) => cols.map(escapeCsvCell).join(','))
      .join('\r\n')

    if (isTauri) {
      const bytes = new TextEncoder().encode('\uFEFF' + csvContent)
      const filePath = await save({
        defaultPath: fileName,
        filters: [{ name: 'CSV Files', extensions: ['csv'] }],
      })

      if (!filePath) return

      await writeFile(filePath, bytes)
      fb.success('CSV 导出成功')
      return
    }

    const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' })
    const url = window.URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.style.display = 'none'
    a.href = url
    a.download = fileName
    document.body.appendChild(a)
    a.click()
    setTimeout(() => {
      document.body.removeChild(a)
      window.URL.revokeObjectURL(url)
    }, 100)
  } catch (error) {
    console.error('[CSV] 导出失败:', error)
  }
}

onMounted(() => {
  const eventId = route.params.id
  if (eventId)
    statStore.setActiveEvent(Number(eventId), {
      productCode: selectedProduct.value,
      startDate: startDate.value,
      endDate: endDate.value,
      intervalMinutes: intervalMinutes.value,
    })
  setTimeout(updateChartWidth, 100)
  window.addEventListener('resize', updateChartWidth)
})

onUnmounted(() => {
  window.removeEventListener('resize', updateChartWidth)
})

watch(
  () => route.params.id,
  (newEventId) => {
    if (newEventId)
      statStore.setActiveEvent(Number(newEventId), {
        productCode: selectedProduct.value,
        startDate: startDate.value,
        endDate: endDate.value,
        intervalMinutes: intervalMinutes.value,
      })
  }
)
</script>

<style scoped>
/* 页头改 embedded 后，原副标题与下载按钮挪到内容区顶部；保证文字与操作都不丢。 */
.page-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-lg);
  flex-wrap: wrap;
  margin-bottom: var(--space-lg);
}

.page-hint-row {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.page-hint {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--font-base);
}

.download-btn {
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
  padding: var(--space-md) var(--space-xl);
  transition: all 0.2s ease;
  flex-shrink: 0;
  white-space: nowrap;
}

.download-actions {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  flex-wrap: wrap;
  justify-content: flex-end;
}

.download-btn:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
}

/* 统一区块样式 */
.filter-section,
.summary-section,
.chart-section,
.table-section {
  margin-bottom: var(--space-2xl);
}

.summary-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: var(--space-lg);
}

.chart-info {
  margin-bottom: var(--space-lg);
}

.chart-subtitle {
  color: var(--text-muted);
  font-size: var(--font-base);
}

/* 响应式布局 */
@media (--phone) {
  .download-btn {
    align-self: stretch;
    width: 100%;
    justify-content: center;
    font-size: var(--font-sm);
    padding: var(--space-sm) var(--space-lg);
  }

  .download-actions {
    width: 100%;
    justify-content: flex-start;
    gap: var(--space-sm);
  }

  .summary-cards {
    grid-template-columns: 1fr;
    gap: var(--space-sm);
  }

  .chart-subtitle {
    font-size: var(--font-xs);
  }
}
</style>
