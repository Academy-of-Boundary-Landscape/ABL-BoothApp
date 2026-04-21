<template>
  <div class="page">
    <header class="page-header">
      <div class="header-content">
        <div class="header-title-row">
          <h1>{{ pageTitle }}</h1>
          <HelpBubble page="event-stats" />
        </div>
        <p>查看当前展会的销售数据和统计分析。</p>
      </div>
      <div
        v-if="statStore.stats && statStore.stats.summary.length > 0"
        class="download-actions"
      >
        <n-button
          class="download-btn"
          type="default"
          ghost
          size="large"
          @click="downloadCsv"
        >
          下载 CSV
        </n-button>

        <n-button
          class="download-btn"
          type="primary"
          ghost
          size="large"
          @click="downloadReport"
        >
          <template #icon>
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>
          </template>
          下载 Excel 报告
        </n-button>
      </div>
    </header>

    <div v-if="statStore.isLoading" class="loading-indicator">
      <n-spin size="large">
        <template #description>正在从数据库中提取统计信息...</template>
      </n-spin>
    </div>

    <div v-else-if="statStore.error" class="error-message">
      <n-alert type="error" title="后端数据库寄了！" :bordered="false">
        {{ statStore.error }}
      </n-alert>
      <n-button @click="applyFilters" tertiary class="btn-secondary">重新建立连接</n-button>
    </div>

    <div v-else-if="statStore.stats" class="stats-content">
      <CollapsibleSection
        title="数据筛选"
        v-model:collapsed="isFilterCollapsed"
        class="filter-section"
      >
        <StatFilters
          :product-options="productOptions"
          :selected-product="selectedProduct"
          :start-date="startDate"
          :end-date="endDate"
          :interval-minutes="intervalMinutes"
          @update:selectedProduct="val => (selectedProduct = val)"
          @update:startDate="val => (startDate = val)"
          @update:endDate="val => (endDate = val)"
          @update:intervalMinutes="val => (intervalMinutes = val)"
          @change="applyFilters"
        />
      </CollapsibleSection>

      <!-- 关键数据总览 -->
      <CollapsibleSection
        title="关键数据总览"
        v-model:collapsed="isSummaryCollapsed"
        class="summary-section"
      >
        <div class="summary-cards">
          <div class="summary-card">
            <span class="label">总销售额</span>
            <span class="value">{{ formatCurrency(statStore.stats.total_revenue) }}</span>
          </div>
          <div class="summary-card">
            <span class="label">总销售件数</span>
            <span class="value">{{ totalItemsSold }}</span>
          </div>
          <div class="summary-card">
            <span class="label">销售品类数</span>
            <span class="value">{{ productVarietyCount }}</span>
          </div>
        </div>
      </CollapsibleSection>

      <!-- 销售趋势图 -->
      <CollapsibleSection
        title="销售额趋势"
        v-model:collapsed="isChartCollapsed"
        class="chart-section"
      >
        <div class="chart-info">
          <span v-if="statStore.stats.timeseries?.length" class="chart-subtitle">{{ chartSubtitle }}</span>
        </div>
        <SalesLineChart
          v-if="statStore.stats.timeseries?.length"
          :series="statStore.stats.timeseries"
          :width="chartWidth"
          :height="chartHeight"
          :padding="padding"
        />
        <p v-else class="no-data">// 暂无趋势数据</p>
      </CollapsibleSection>

      <!-- 销售详情表格 -->
      <CollapsibleSection
        title="销售数据表"
        v-model:collapsed="isTableCollapsed"
        class="table-section"
      >
            <p v-if="!statStore.stats.summary.length" class="no-data">
              // 无有效销售数据记录...
            </p>
            <div v-else class="table-wrapper">
              <table class="stats-table">
                <thead>
                  <tr>
                    <th>制品编号</th>
                    <th>制品名</th>
                    <th class="text-right">单价</th>
                    <th class="text-center">销售量</th>
                    <th class="text-right">销售额</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="item in statStore.stats.summary" :key="item.product_id">
                    <td class="id-cell">#{{ item.product_code }}</td>
                    <td>{{ item.product_name }}</td>
                    <td class="text-right currency-cell">{{ formatCurrency(item.unit_price) }}</td>
                    <td class="text-center quantity-cell">{{ item.total_quantity }}</td>
                    <td class="text-right currency-cell">{{ formatCurrency(item.total_revenue_per_item) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
      </CollapsibleSection>
    </div>
  </div>
</template>

<script setup>
import { onMounted, onUnmounted, watch, computed, ref } from 'vue';
import { useRoute } from 'vue-router';
import { useEventStatStore } from '@/stores/eventStatStore';
import SalesLineChart from '@/components/stats/SalesLineChart.vue';
import StatFilters from '@/components/stats/StatFilters.vue';
import { NButton, NSpin, NAlert, NCard, NTable } from 'naive-ui';
import HelpBubble from '@/components/shared/HelpBubble.vue';
import CollapsibleSection from '@/components/shared/CollapsibleSection.vue';

import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import { fetch as tauriFetch } from '@tauri-apps/plugin-http';

const route = useRoute();
const statStore = useEventStatStore();
const selectedProduct = ref('');
const startDate = ref('');
const endDate = ref('');
const intervalMinutes = ref(60);
const chartWidth = ref(800)
const chartHeight = 320;
const padding = 48;

function updateChartWidth() {
  const el = document.querySelector('.chart-section .cs-body')
  if (el) chartWidth.value = Math.min(el.clientWidth - padding * 2, 1200)
}
const isFilterCollapsed = ref(false);
const isSummaryCollapsed = ref(false);
const isChartCollapsed = ref(false);
const isTableCollapsed = ref(false);

const pageTitle = computed(() => statStore.stats?.event_name ? `${statStore.stats.event_name} - 数据统计` : '数据统计');
const totalItemsSold = computed(() => statStore.stats?.summary.reduce((sum, item) => sum + item.total_quantity, 0) || 0);
const productVarietyCount = computed(() => statStore.stats?.summary.length || 0);
const productOptions = computed(() => {
  const summary = statStore.stats?.summary || [];
  const unique = new Map();
  summary.forEach(item => {
    if (!unique.has(item.product_code)) {
      unique.set(item.product_code, { code: item.product_code, name: item.product_name });
    }
  });
  return Array.from(unique.values());
});


const chartSubtitle = computed(() => {
  const parts = [];
  if (selectedProduct.value) parts.push(`制品 ${selectedProduct.value}`);
  if (startDate.value) parts.push(`自 ${startDate.value}`);
  if (endDate.value) parts.push(`至 ${endDate.value}`);
  parts.push(intervalMinutes.value === 30 ? '每 30 分钟' : '每小时');
  return parts.join(' · ');
});


function formatCurrency(value) {
  if (typeof value !== 'number') return '¥ 0.00';
  return `¥ ${value.toFixed(2)}`;
}

// Chart implementation moved to SalesLineChart component

async function applyFilters() {
  await statStore.fetchStats({
    productCode: selectedProduct.value,
    startDate: startDate.value,
    endDate: endDate.value,
    intervalMinutes: intervalMinutes.value,
  });
}
const API_ORIGIN = 'http://127.0.0.1:5140';
function toAbsoluteApiUrl(url) {
  if (!url) return url;
  if (url.startsWith('http://') || url.startsWith('https://')) return url;
  if (url.startsWith('/')) return `${API_ORIGIN}${url}`;
  return `${API_ORIGIN}/${url}`;
}

async function downloadReport() {
  if (!statStore.stats || !statStore.stats.summary?.length) return;

  const isTauri = window.__TAURI_INTERNALS__ !== undefined;
  const token = sessionStorage.getItem('access_token');

  const safeName = (statStore.stats.event_name || 'sales_report').replace(/[\\/:*?"<>|]/g, '_');
  const fileName = `sales_report_${safeName}.xlsx`;

  const url = toAbsoluteApiUrl(statStore.downloadUrl);

  try {
    console.log('开始请求 Excel 报告:', url, 'isTauri:', isTauri);

    if (isTauri) {
      const headers = {
        Accept: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
      };
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const resp = await tauriFetch(url, { method: 'GET', headers });

      if (!resp.ok) {
        const text = await resp.text().catch(() => '');
        throw new Error(`下载失败: ${resp.status} ${resp.statusText} ${text.slice(0, 200)}`);
      }

      const ab = await resp.arrayBuffer();
      const bytes = new Uint8Array(ab);

      const filePath = await save({
        defaultPath: fileName,
        filters: [{ name: 'Excel Files', extensions: ['xlsx'] }]
      });
      if (!filePath) return;

      await writeFile(filePath, bytes);
      alert('导出成功');
      return;
    }

    // 浏览器环境
    const headers = {};
    if (token) headers['Authorization'] = `Bearer ${token}`;

    const response = await fetch(url, {
      method: 'GET',
      credentials: 'include',
      headers
    });

    if (!response.ok) {
      const text = await response.text().catch(() => '');
      throw new Error(`下载失败: ${response.status} ${text.slice(0, 200)}`);
    }

    const blob = await response.blob();
    const dl = window.URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.style.display = 'none';
    a.href = dl;
    a.download = fileName;
    document.body.appendChild(a);
    a.click();
    setTimeout(() => {
      document.body.removeChild(a);
      window.URL.revokeObjectURL(dl);
    }, 100);
  } catch (e) {
    console.error('下载 Excel 报告失败:', e);
    alert(e?.message || '下载失败');
  }
}

function escapeCsvCell(value) {
  const text = String(value ?? '');
  if (/[,"\n\r]/.test(text)) {
    return `"${text.replace(/"/g, '""')}"`;
  }
  return text;
}

async function downloadCsv() {
  const summary = statStore.stats?.summary || [];
  if (!summary.length) return;

  try {
    const isTauri = window.__TAURI_INTERNALS__ !== undefined;
    const safeName = (statStore.stats.event_name || 'sales_report').replace(/[\\/:*?"<>|]/g, '_');
    const fileName = `sales_report_${safeName}.csv`;

    const header = ['制品编号', '制品名', '单价', '销售量', '销售额'];
    const rows = summary.map(item => [
      item.product_code ?? '',
      item.product_name ?? '',
      typeof item.unit_price === 'number' ? item.unit_price.toFixed(2) : '0.00',
      item.total_quantity ?? 0,
      typeof item.total_revenue_per_item === 'number' ? item.total_revenue_per_item.toFixed(2) : '0.00',
    ]);

    const csvContent = [header, ...rows]
      .map(cols => cols.map(escapeCsvCell).join(','))
      .join('\r\n');

    if (isTauri) {
      const bytes = new TextEncoder().encode('\uFEFF' + csvContent);
      const filePath = await save({
        defaultPath: fileName,
        filters: [{ name: 'CSV Files', extensions: ['csv'] }]
      });

      if (!filePath) return;

      await writeFile(filePath, bytes);
      alert('CSV 导出成功');
      return;
    }

    const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' });
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.style.display = 'none';
    a.href = url;
    a.download = fileName;
    document.body.appendChild(a);
    a.click();
    setTimeout(() => {
      document.body.removeChild(a);
      window.URL.revokeObjectURL(url);
    }, 100);
  } catch (error) {
    console.error('[CSV] 导出失败:', error);
  }
}

onMounted(() => {
  const eventId = route.params.id;
  if (eventId) statStore.setActiveEvent(eventId, { productCode: selectedProduct.value, startDate: startDate.value, endDate: endDate.value, intervalMinutes: intervalMinutes.value });
  setTimeout(updateChartWidth, 100)
  window.addEventListener('resize', updateChartWidth)
});

onUnmounted(() => {
  window.removeEventListener('resize', updateChartWidth)
});

watch(() => route.params.id, (newEventId) => {
  if (newEventId) statStore.setActiveEvent(newEventId, { productCode: selectedProduct.value, startDate: startDate.value, endDate: endDate.value, intervalMinutes: intervalMinutes.value });
});
</script>

<style scoped>
/* 主题色通过 App.vue 动态注入 */

.page { max-width: 960px; }

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  margin-bottom: 1.5rem;
  min-width: 0;
}

.header-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.header-content {
  flex: 1;
  min-width: 0;
}

.page-header h1 {
  margin: 0 0 0.25rem;
  font-size: var(--font-xl);
  color: var(--accent-color);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.page-header p {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--font-base);
}

.download-btn {
  font-size: var(--font-md);
  font-weight: 600;
  padding: 0.75rem 1.5rem;
  transition: all 0.2s ease;
  flex-shrink: 0;
  white-space: nowrap;
}

.download-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
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
  margin-bottom: 2rem;
}

/* section 外壳样式由 CollapsibleSection 统一提供 */

.loading-indicator, .error-message {
  text-align: center;
  padding: 5rem 2rem;
  color: var(--secondary-text-color);
  border: 1px dashed var(--border-color);
  border-radius: var(--radius-md);
  background-color: var(--overlay-light);
}
.error-message p { margin: 0.5rem 0; }
.error-message strong { color: var(--error-color); }
.btn-secondary { background-color: var(--card-bg-color); color: var(--primary-text-color); margin-top: 1rem;}
.btn-secondary:hover { border-color: var(--primary-text-color); }

.spinner {
  width: 40px;
  height: 40px;
  border: 4px solid var(--border-color);
  border-top-color: var(--accent-color);
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin: 0 auto 1rem;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}

.summary-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1rem;
}

.summary-card {
  background: linear-gradient(135deg, var(--card-bg-color) 0%, var(--bg-color) 100%);
  padding: 1.5rem;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  transition: all 0.2s ease;
}

.summary-card:hover {
  transform: translateY(-3px);
  box-shadow: var(--shadow-md);
  border-color: var(--accent-color);
}

.summary-card .label {
  font-size: var(--font-base);
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.summary-card .value {
  font-size: var(--font-2xl);
  font-weight: 600;
  color: var(--accent-color);
  line-height: 1;
}

.filters {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1rem;
  margin-bottom: 1.5rem;
}

.filter-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.filter-group label {
  color: var(--secondary-text-color);
  font-size: var(--font-base);
}

.filter-group select,
.filter-group input[type="date"] {
  background: var(--card-bg-color);
  color: var(--primary-text-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  padding: 0.65rem 0.75rem;
}

.chart-info {
  margin-bottom: 1rem;
}

.chart-subtitle {
  color: var(--text-muted);
  font-size: var(--font-base);
}

.chart-wrapper {
  width: 100%;
  overflow: hidden;
  position: relative;
}

svg {
  width: 100%;
  height: auto;
}

.table-wrapper {
  width: 100%;
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
}

.stats-table {
  width: 100%;
  margin-top: 0;
  border-collapse: collapse;
  border-spacing: 0;
  text-align: left;
  font-size: var(--font-base);
  min-width: 700px;
}

/* 表头样式 */
.stats-table th {
  padding: 12px 16px;
  background-color: var(--card-bg-color);
  color: var(--primary-text-color);
  font-weight: 600;
  border-bottom: 2px solid var(--accent-color);
  white-space: nowrap;
}

/* 数据单元格样式 */
.stats-table td {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
  color: var(--secondary-text-color);
  vertical-align: middle;
}

/* 表格行的交互效果 */
.stats-table tbody tr {
  transition: background-color 0.2s ease-in-out;
}

.stats-table tbody tr:hover {
  background-color: var(--accent-color-light);
}

/* 特定列的微调 */
.stats-table th:first-child,
.stats-table td:first-child {
  padding-left: 0;
}

.stats-table th:last-child,
.stats-table td:last-child {
  text-align: right;
  padding-right: 0;
}

.line {
  fill: none;
  stroke: var(--accent-color);
  stroke-width: 2.5;
}

.area {
  fill: url(#revenueGradient);
  stroke: none;
}

.points circle {
  fill: var(--accent-color);
  stroke: var(--card-bg-color);
  stroke-width: 2;
}

.points text {
  fill: var(--primary-text-color);
  font-size: var(--font-xs);
}

.grid-lines line {
  stroke: var(--border-color-light);
  stroke-dasharray: 4 4;
  stroke-width: 1;
}

.y-ticks text {
  fill: var(--primary-text-color);
  font-size: var(--font-xs);
}

.chart-tooltip {
  position: absolute;
  transform: translate(-50%, -120%);
  background: var(--tooltip-bg-color);
  color: var(--text-on-dark);
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color-light);
  pointer-events: none;
  white-space: nowrap;
  box-shadow: var(--shadow-lg);
}

.tooltip-date {
  font-size: var(--font-sm);
  margin-bottom: 0.2rem;
  color: var(--secondary-text-color);
}

.tooltip-value {
  font-size: var(--font-base);
  font-weight: 600;
  color: var(--primary-text-color);
}



table {
  width: 100%;
  border-collapse: collapse;
}

th, td {
  padding: 1rem;
  text-align: left;
  border-bottom: 1px solid var(--border-color);
}

thead th {
  color: var(--secondary-text-color);
  font-weight: bold;
  text-transform: uppercase;
  font-size: var(--font-sm);
  letter-spacing: 1px;
}

tbody tr {
  transition: background-color 0.2s;
}
tbody tr:hover {
  background-color: var(--accent-color-light);
}
tbody td {
  color: var(--primary-text-color);
}
.id-cell {
  color: var(--secondary-text-color);
  font-family: 'Courier New', Courier, monospace;
}
.quantity-cell {
  font-weight: bold;
  font-size: var(--font-lg);
}
.currency-cell {
  color: var(--accent-color);
  font-weight: 500;
}
.text-right { text-align: right; }
.text-center { text-align: center; }

.no-data {
  color: var(--secondary-text-color);
  padding: 3rem;
  text-align: center;
  font-family: 'Courier New', Courier, monospace;
}

/* 响应式布局 */
@media (max-width: 768px) {
  .page {
    padding: 1rem;
  }

  .page-header {
    flex-direction: column;
    gap: 0.75rem;
  }

  .page-header h1 {
    white-space: normal;
    overflow: visible;
    text-overflow: unset;
  }

  .download-btn {
    align-self: flex-start;
    font-size: 0.9rem;
    padding: 0.6rem 1.2rem;
  }

  .download-actions {
    width: 100%;
    justify-content: flex-start;
  }

  .summary-cards {
    gap: 0.75rem;
  }

  .summary-card {
    padding: 1rem;
  }

  .summary-card .label {
    font-size: 0.85rem;
  }

  .summary-card .value {
    font-size: 1.5rem;
  }

  .stats-table {
    font-size: 0.85rem;
    min-width: 650px;
  }

  .stats-table th,
  .stats-table td {
    padding: 10px 12px;
  }

  .no-data {
    padding: 2rem 1rem;
  }
}

@media (max-width: 480px) {
  .page {
    padding: 0.75rem;
  }

  .stat-header {
    margin-bottom: 1rem;
    padding-bottom: 0.5rem;
  }

  .stat-header h1 {
    font-size: 1.25rem;
    white-space: normal;
    word-break: break-word;
  }

  .stat-header p {
    font-size: 0.85rem;
  }

  .summary-cards {
    grid-template-columns: 1fr;
    gap: 0.5rem;
  }

  .summary-card {
    padding: 0.75rem;
  }

  .summary-card .label {
    font-size: 0.8rem;
  }

  .summary-card .value {
    font-size: 1.25rem;
  }

  .download-btn {
    width: 100%;
    justify-content: center;
    font-size: 0.85rem;
    padding: 0.6rem 1rem;
  }

  .download-actions {
    gap: 0.5rem;
  }

  .stats-table {
    font-size: 0.75rem;
    min-width: 600px;
  }

  .stats-table th,
  .stats-table td {
    padding: 8px 10px;
  }

  .stats-table th {
    font-size: 0.7rem;
  }

  .id-cell {
    font-size: 0.7rem;
  }

  .quantity-cell {
    font-size: 0.9rem;
  }

  .currency-cell {
    font-size: 0.75rem;
  }

  .no-data {
    padding: 1.5rem 0.75rem;
    font-size: 0.85rem;
  }

  .chart-subtitle {
    font-size: 0.8rem;
  }
}
</style>
