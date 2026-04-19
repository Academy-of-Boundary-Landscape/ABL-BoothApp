<template>
  <div class="page">
    <header class="page-header">
      <div class="header-content">
        <div class="header-title-row">
          <h1>订单管理</h1>
          <HelpBubble page="event-orders" />
        </div>
        <p>查看并管理当前展会的所有订单记录。</p>
      </div>
    </header>

    <main class="page-body">
      <!-- 筛选器区块 -->
      <section class="filter-section">
        <div class="section-header" @click="isFilterExpanded = !isFilterExpanded">
          <h2>订单筛选</h2>
          <n-button text class="toggle-btn">
            {{ isFilterExpanded ? '折叠' : '展开' }}
          </n-button>
        </div>
        <transition name="expand">
          <div v-show="isFilterExpanded" class="section-container">
            <div class="filter-content">
              <div class="filter-row">
                <label for="status-filter">状态:</label>
                <n-select
                  id="status-filter"
                  v-model:value="statusFilter"
                  :options="statusOptions"
                  placeholder="选择筛选状态"
                  class="status-select"
                />
              </div>
              
              <div class="filter-row">
                <label>金额范围:</label>
                <div class="amount-range">
                  <n-input-number
                    v-model:value="minAmount"
                    :min="0"
                    :precision="2"
                    placeholder="最小金额"
                    clearable
                    class="amount-input"
                  >
                    <template #prefix>¥</template>
                  </n-input-number>
                  <span class="range-separator">-</span>
                  <n-input-number
                    v-model:value="maxAmount"
                    :min="0"
                    :precision="2"
                    placeholder="最大金额"
                    clearable
                    class="amount-input"
                  >
                    <template #prefix>¥</template>
                  </n-input-number>
                </div>
              </div>
              
              <div class="filter-row">
                <label for="product-filter">商品名称:</label>
                <n-input
                  id="product-filter"
                  v-model:value="productNameFilter"
                  placeholder="输入商品名称搜索"
                  clearable
                  class="product-input"
                />
              </div>
              
              <n-button
                v-if="statusFilter !== 'all' || minAmount !== null || maxAmount !== null || productNameFilter"
                @click="clearFilters"
                class="clear-btn"
                secondary
              >
                清空筛选
              </n-button>
            </div>
          </div>
        </transition>
      </section>

      <!-- 订单列表区块 -->
      <section class="list-section">
        <div class="section-header" @click="isListExpanded = !isListExpanded">
          <h2>订单列表</h2>
          <n-button text class="toggle-btn">
            {{ isListExpanded ? '折叠' : '展开' }}
          </n-button>
        </div>
        <transition name="expand">
          <div v-show="isListExpanded" class="section-container">

      <div v-if="store.isLoading" class="loading-message">
        <n-spin size="large">
          <template #description>正在加载订单...</template>
        </n-spin>
      </div>
      <div v-else-if="store.error" class="error-message">
        <n-alert type="error" :bordered="false">{{ store.error }}</n-alert>
      </div>
      
      <div v-else-if="filteredOrders.length" class="table-wrapper">
        <n-table class="order-table" size="small">
          <thead>
            <tr>
              <th>订单ID</th>
              <th>下单时间</th>
              <th>商品详情</th>
              <th>总金额</th>
              <th class="column-status">状态</th>
              <th class="column-actions">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="order in filteredOrders" :key="order.id">
              <td><strong>#{{ order.id }}</strong></td>
              <td>{{ formatTimestamp(order.timestamp) }}</td>
              <td>
                <ul class="item-list">
                  <li v-for="item in order.items" :key="item.id">
                    {{ item.product_name }} x {{ item.quantity }}
                  </li>
                </ul>
              </td>
              <td><strong>¥{{ order.total_amount.toFixed(2) }}</strong></td>
              <td>
                <n-tag :type="tagType(order.status)" size="large" round>{{ statusText(order.status) }}</n-tag>
              </td>
              <td>
                <n-dropdown :options="actionOptions(order.status)" @select="key => changeStatus(order.id, key)">
                  <n-button size="large">操作</n-button>
                </n-dropdown>
              </td>
            </tr>
          </tbody>
        </n-table>
      </div>
      <div v-else class="empty-guide">
        <div class="empty-guide-icon">📝</div>
        <div class="empty-guide-title">暂无订单</div>
        <div class="empty-guide-desc">当顾客通过点单页面下单后，订单会自动出现在这里。你可以在这里查看、完成或取消订单。</div>
        <div class="empty-guide-hint">将展会设为「进行中」，然后分享点单链接给顾客</div>
      </div>
          </div>
        </transition>
      </section>
    </main>
  </div>
</template>


<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useEventDetailStore } from '@/stores/eventDetailStore';
import { NSelect, NSpin, NAlert, NTable, NCard, NTag, NDropdown, NButton, NInput, NInputNumber, useDialog, useMessage } from 'naive-ui';
import HelpBubble from '@/components/shared/HelpBubble.vue';
import { formatTimestamp } from '@/utils/dateFormatter';
const props = defineProps({
  id: { type: String, required: true }
});

const store = useEventDetailStore();
const statusFilter = ref('all'); // 筛选器的状态
const minAmount = ref(null); // 最小金额
const maxAmount = ref(null); // 最大金额
const productNameFilter = ref(''); // 商品名称筛选
const dialog = useDialog();
const message = useMessage();
const isFilterExpanded = ref(true);
const isListExpanded = ref(true);
const statusOptions = [
  { label: '所有订单', value: 'all' },
  { label: '待处理', value: 'pending' },
  { label: '已完成', value: 'completed' },
  { label: '已取消', value: 'cancelled' }
];

// 计算属性，根据筛选器动态过滤订单
const filteredOrders = computed(() => {
  let orders = store.allOrders;
  
  // 状态筛选
  if (statusFilter.value !== 'all') {
    orders = orders.filter(order => order.status === statusFilter.value);
  }
  
  // 金额范围筛选
  if (minAmount.value !== null) {
    orders = orders.filter(order => order.total_amount >= minAmount.value);
  }
  if (maxAmount.value !== null) {
    orders = orders.filter(order => order.total_amount <= maxAmount.value);
  }
  
  // 商品名称筛选
  if (productNameFilter.value.trim()) {
    const keyword = productNameFilter.value.trim().toLowerCase();
    orders = orders.filter(order => 
      order.items.some(item => 
        item.product_name.toLowerCase().includes(keyword)
      )
    );
  }
  
  return orders;
});

function changeStatus(orderId, newStatus) {
  if (!newStatus) return;
  dialog.warning({
    title: '确认操作',
    content: `确定要将订单 #${orderId} 的状态修改为 "${statusText(newStatus)}" 吗？`,
    positiveText: '确认',
    negativeText: '取消',
    async onPositiveClick() {
      try {
        await store.adminUpdateOrderStatus(props.id, orderId, newStatus);
        message.success('状态已更新');
      } catch (error) {
        message.error(error.message || '更新失败');
      }
    }
  });
}

// --- 辅助函数 ---
function statusText(status) {
  const map = { pending: '待处理', completed: '已完成', cancelled: '已取消' };
  return map[status] || status;
}
function tagType(status) {
  if (status === 'pending') return 'warning';
  if (status === 'completed') return 'success';
  if (status === 'cancelled') return 'default';
  return 'default';
}

function actionOptions(status) {
  const opts = [];
  if (status !== 'pending') opts.push({ label: '设为待处理', key: 'pending' });
  if (status !== 'completed') opts.push({ label: '设为已完成', key: 'completed' });
  if (status !== 'cancelled') opts.push({ label: '设为已取消', key: 'cancelled' });
  return opts;
}

function clearFilters() {
  statusFilter.value = 'all';
  minAmount.value = null;
  maxAmount.value = null;
  productNameFilter.value = '';
}

// --- 生命周期 ---
onMounted(() => {
  store.fetchAllOrdersForEvent(props.id);
});
onUnmounted(() => {
  store.resetStore(); // 离开时重置store
});
</script>

<style scoped>
.page { max-width: 960px; }
.page-header { margin-bottom: 1.5rem; }
.page-header h1 { margin: 0 0 0.25rem; font-size: var(--font-xl); color: var(--accent-color); }
.page-header p { margin: 0; color: var(--text-muted); font-size: var(--font-base); }
.header-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.btn-back {
  position: absolute;
  top: 0;
  right: 0;
}

/* 通用区块样式 */
.filter-section,
.list-section {
  margin-bottom: 2rem;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  user-select: none;
  padding: 0.75rem 1rem;
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  transition: all 0.2s ease;
  margin-bottom: 0.5rem;
}

.section-header:hover {
  background: var(--hover-bg-color, var(--card-bg-color));
  border-color: var(--accent-color);
}

.section-header h2 {
  margin: 0;
  font-size: var(--font-lg);
  color: var(--accent-color);
  font-weight: 600;
}

.toggle-btn {
  font-size: var(--font-base);
  padding: 0.25rem 0.75rem;
  min-width: auto;
  color: var(--accent-color);
}

/* 展开/折叠动画 */
.expand-enter-active,
.expand-leave-active {
  transition: all 0.3s ease;
  overflow: hidden;
}

.expand-enter-from,
.expand-leave-to {
  opacity: 0;
  max-height: 0;
}

.expand-enter-to,
.expand-leave-from {
  opacity: 1;
  max-height: 2000px;
}

.section-container {
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: 1.5rem;
  overflow: hidden;
}

.table-wrapper {
  width: 100%;
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
}

.filter-content {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.filter-row {
  display: flex;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}

.filter-content label {
  font-size: var(--font-base);
  font-weight: 500;
  color: var(--primary-text-color);
  white-space: nowrap;
  min-width: 90px;
}

.status-select {
  min-width: 200px;
  flex: 1;
  max-width: 300px;
}

.amount-range {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex: 1;
}

.amount-input {
  flex: 1;
  min-width: 120px;
  max-width: 200px;
}

.range-separator {
  color: var(--text-muted);
  font-weight: 500;
}

.product-input {
  flex: 1;
  max-width: 400px;
}

.clear-btn {
  align-self: flex-start;
  margin-left: 90px;
}

.form-container {
  background-color: var(--card-bg-color);
  border: 1px solid var(--border-color);
  padding: 1.5rem;
  border-radius: var(--radius-md);
  margin-bottom: 2rem;
}

.add-product-form input[type="text"],
.add-product-form input[type="number"] {
  background-color: var(--bg-color);
  border: 1px solid var(--border-color);
  color: var(--primary-text-color);
  padding: 10px;
  border-radius: var(--radius-sm);
  box-sizing: border-box;
  height: 42px;
}

/* --- 表格样式 --- */
.order-table {
  width: 100%;
  margin-top: 0;
  border-collapse: collapse;
  border-spacing: 0;
  text-align: left;
  font-size: var(--font-base);
}
.order-table th {
  padding: 12px 16px;
  background-color: var(--card-bg-color);
  color: var(--primary-text-color);
  font-weight: 600;
  border-bottom: 2px solid var(--accent-color);
  white-space: nowrap;
}
.order-table tbody tr:hover {
  background-color: var(--accent-color-light);
}
.order-table th:first-child, .order-table td:first-child { padding-left: 0; }
.order-table th:last-child, .order-table td:last-child { text-align: right; padding-right: 0; }

.column-preview { width: 80px; }

.preview-img {
  width: 50px;
  height: 50px;
  object-fit: cover;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-color);
  vertical-align: middle;
}
.no-img {
  display: inline-block;
  width: 50px;
  height: 50px;
  line-height: 50px;
  text-align: center;
  font-size: var(--font-sm);
  color: var(--text-disabled);
  background-color: var(--bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  vertical-align: middle;
}

.loading-message, .error-message {
  padding: 1rem;
  text-align: center;
}
.error-message { color: var(--error-color); }

.edit-form .form-group { margin-bottom: 1rem; }
.edit-form label { display: block; margin-bottom: 0.5rem; }
.edit-form input {
  width: 100%;
  background-color: var(--bg-color);
  border: 1px solid var(--border-color);
  color: var(--primary-text-color);
  padding: 10px;
  border-radius: var(--radius-sm);
  box-sizing: border-box;
}

.btn-primary {
  background-color: var(--accent-color);
  color: var(--bg-color);
}
button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.action-btn {
  background: none;
  border: 1px solid transparent; /* 默认透明边框 */
  color: var(--primary-text-color);
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--font-base);
  transition: background-color 0.2s, color 0.2s, border-color 0.2s;
  display: inline-flex; /* 让图标和文字对齐 */
  align-items: center;
  gap: 0.4rem; /* 图标和文字的间距 */
  white-space: nowrap; /* 防止文字换行 */
}

.action-btn:hover {
  background-color: var(--card-bg-color);
  border-color: var(--border-color);
}

/* 危险操作按钮的特定样式 */
.action-btn.btn-danger {
  color: var(--error-color);
}

.action-btn.btn-danger:hover {
  background-color: var(--accent-color-light);
  border-color: var(--error-color);
}
.filters {
  margin-bottom: 1.5rem;
}

/* --- 筛选器样式 (复用 VendorView 的样式) --- */
.filters {
  margin-bottom: 1.5rem;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 1rem;
}
/* 文字大小要保证能排开一行 */
.filters label {
  font-size: var(--font-base);
  font-weight: 600;
  white-space: nowrap; /* 防止中文自动换行 */
  flex-shrink: 0;      /* 在 flex 布局中不缩小导致换行 */
}
.custom-select-wrapper {
  position: relative;
  display: inline-block;
  min-width: 200px;
}
.custom-select-wrapper::after {
  content: '▼';
  font-size: var(--font-sm);
  color: var(--accent-color);
  position: absolute;
  right: 15px;
  top: 50%;
  transform: translateY(-50%);
  pointer-events: none;
}
.custom-select-wrapper select {
  -webkit-appearance: none;
  appearance: none;
  width: 100%;
  padding: 8px 30px 8px 12px;
  background-color: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--primary-text-color);
  cursor: pointer;
}

/* --- 状态徽章的全新样式 --- */
.status-badge {
  display: inline-block;
  padding: 4px 12px;
  border-radius: var(--radius-xl);
  font-size: var(--font-sm);
  font-weight: 500;
  white-space: nowrap;
}
.status-badge.status-pending {
  background-color: var(--accent-color-light);
  color: var(--warning-color-alt);
}
.status-badge.status-completed {
  background-color: var(--accent-color-light);
  color: var(--accent-color);
}
.status-badge.status-cancelled {
  background-color: var(--hover-bg-color);
  color: var(--cancelled-color);
}

/* --- 操作菜单的全新样式 --- */
.action-menu {
  position: relative;
  display: inline-block;
  text-align: left;
}
.action-btn { /* 这是触发按钮 */
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 6px 12px;
  background-color: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  cursor: pointer;
  color: var(--primary-text-color);
}
.action-btn:hover {
  background-color: var(--bg-color);
  border-color: var(--accent-color);
}

.menu-items {
  position: absolute;
  right: 0;
  margin-top: 0.5rem;
  width: 150px;
  origin-top-right: 0; /* 动画基点 */
  background-color: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-md);
  z-index: 10;
  overflow: hidden; /* 保证内部元素的圆角 */
}

.menu-items button {
  display: block;
  width: 100%;
  padding: 0.75rem 1rem;
  text-align: left;
  background: none;
  border: none;
  color: var(--primary-text-color);
  cursor: pointer;
}
.menu-items button.active,
.menu-items button:hover {
  background-color: var(--accent-color);
  color: var(--bg-color);
}

/* 响应式布局 */
@media (max-width: 768px) {
  main {
    padding: 0;
  }

  .page-header {
    margin-bottom: 1.5rem;
    padding-bottom: 0.75rem;
  }

  .page-header h1 {
    font-size: 1.3rem;
  }

  .page-header p {
    font-size: 0.9rem;
  }

  .section-container {
    padding: 1rem;
  }

  .filter-content {
    gap: 1rem;
  }

  .filter-row {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
  }

  .filter-content label {
    font-size: 0.9rem;
    min-width: auto;
  }

  .status-select,
  .product-input {
    max-width: none;
    width: 100%;
  }

  .amount-range {
    width: 100%;
    flex-wrap: wrap;
  }

  .amount-input {
    max-width: none;
    min-width: 0;
  }

  .clear-btn {
    margin-left: 0;
    width: 100%;
  }

  .order-table {
    font-size: 0.85rem;
    min-width: 600px;
  }

  .order-table th,
  .order-table td {
    padding: 8px;
  }

  .item-list {
    font-size: 0.8rem;
  }
}

@media (max-width: 480px) {
  .page-header {
    margin-bottom: 1rem;
    padding-bottom: 0.5rem;
  }

  .page-header h1 {
    font-size: 1.1rem;
  }

  .page-header p {
    font-size: 0.8rem;
  }

  .section-header {
    padding: 0.6rem 0.75rem;
  }

  .section-header h2 {
    font-size: 1.1rem;
  }

  .section-container {
    padding: 0.75rem;
  }

  .filter-content {
    gap: 0.75rem;
  }

  .filter-row {
    gap: 0.4rem;
  }

  .filter-content label {
    font-size: 0.85rem;
  }

  .amount-input {
    flex: 1 1 calc(50% - 1rem);
  }

  .order-table {
    font-size: 0.75rem;
    min-width: 550px;
  }

  .order-table th,
  .order-table td {
    padding: 6px 4px;
  }

  .order-table th {
    font-size: 0.7rem;
  }

  .item-list {
    padding-left: 1rem;
    margin: 0;
    font-size: 0.7rem;
  }

  .item-list li {
    margin-bottom: 0.2rem;
  }
}

.empty-guide {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 3rem 2rem;
  text-align: center;
}
.empty-guide-icon {
  font-size: 3rem;
  margin-bottom: 12px;
  opacity: 0.3;
}
.empty-guide-title {
  font-size: var(--font-lg);
  font-weight: 700;
  color: var(--primary-text-color);
  margin-bottom: 8px;
}
.empty-guide-desc {
  font-size: var(--font-base);
  color: var(--text-muted);
  max-width: 400px;
  line-height: 1.6;
  margin-bottom: 16px;
}
.empty-guide-hint {
  font-size: var(--font-sm);
  color: var(--accent-color);
  font-weight: 600;
}
</style>