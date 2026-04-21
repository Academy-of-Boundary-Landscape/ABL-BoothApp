<template>
  <div class="list-container">
    <div class="section-header">
      <h2>展会列表</h2>
    </div>

    <!-- 搜索和过滤区域（Naive UI） -->
    <div class="search-container">
      <n-space wrap align="end" :size="16">
        <div class="search-group">
          <label for="search-name">按名称搜索:</label>
          <n-input
            id="search-name"
            v-model:value="searchName"
            clearable
            placeholder="输入展会名称关键字..."
          />
        </div>
        <div class="search-group">
          <label for="search-date-start">日期范围:</label>
          <div class="date-range-inputs">
            <n-date-picker
              id="search-date-start"
              v-model:value="dateRangeStart"
              type="date"
              clearable
              value-format="yyyy-MM-dd"
            />
            <span>至</span>
            <n-date-picker
              id="search-date-end"
              v-model:value="dateRangeEnd"
              type="date"
              clearable
              value-format="yyyy-MM-dd"
            />
          </div>
        </div>
        <n-button tertiary @click="clearFilters">清空筛选</n-button>
      </n-space>
    </div>

    <div v-if="store.isLoading" class="loading-message">正在加载展会数据...</div>
    <div v-else-if="store.error" class="error-message">{{ store.error }}</div>
    
    <!-- 【修改】v-for 循环现在使用 filteredEvents 计算属性 -->
    <ul v-else-if="filteredEvents.length" class="event-list">
      <RouterLink
      v-for="event in filteredEvents"
      :key="event.id"
      :to="`/admin/events/${event.id}/products`"
      custom
      v-slot="{ navigate }"
      >
      <li @click="navigate" class="event-card clickable" role="link">
        <n-card :title="event.name" :hoverable="true" embedded>
          <div class="event-info">
            <p>日期: {{ event.date }}</p>
            <p>地点: {{ event.location || '未指定' }}</p>
          </div>
          <template #header-extra>
            <n-tag :type="statusType(event.status)" size="small">{{ event.status }}</n-tag>
          </template>
          <template #footer>
            <div class="status-actions">
              <n-button v-if="event.status === '未进行'" size="small" @click.stop="changeStatus(event.id, '进行中')">► 开始</n-button>
              <n-button v-if="event.status === '进行中'" size="small" @click.stop="changeStatus(event.id, '已结束')">■ 结束</n-button>
              <n-button v-if="event.status === '已结束'" size="small" @click.stop="changeStatus(event.id,'未进行')">► 重新开始</n-button>
              <n-button size="small" type="primary" @click.stop="openEditModal(event)">编辑</n-button>
              <n-button size="small" type="error" @click.stop="confirmDelete(event.id)">删除</n-button>
            </div>
          </template>
        </n-card>
      </li>
      </RouterLink>
    </ul>

    <!-- 【修改】处理“无搜索结果”和“无任何展会”两种情况 -->
    <p v-else-if="store.events.length && !filteredEvents.length" class="no-results-message">
      没有找到符合筛选条件的展会。
    </p>
    <EmptyGuide
      v-else
      icon="📋"
      title="还没有创建展会"
      desc="展会是管理摊位的核心单位。每场漫展创建一个展会，然后在其中管理商品库存和订单。"
      hint="在上方「创建新展会」表单中填写信息开始吧"
    />


    <!-- 编辑模态框 (保持不变) -->
    <AppModal :show="isEditModalVisible" @close="closeEditModal">
      <template #header><h3>编辑展会</h3></template>
      <template #body><EditEventForm v-if="selectedEvent" ref="editForm" :event="selectedEvent" /></template>
      <template #footer>
        <button type="button" class="btn" @click="closeEditModal">取消</button>
        <button type="button" class="btn btn-primary" @click="handleUpdateEvent">保存更改</button>
      </template>
    </AppModal>
  </div>
</template>

<script setup>
import { onMounted, ref, computed } from 'vue';
import { useEventStore } from '@/stores/eventStore';
import AppModal from '@/components/shared/AppModal.vue';
import EditEventForm from '@/components/event/EditEventForm.vue';
import EmptyGuide from '@/components/shared/EmptyGuide.vue';
import { RouterLink } from 'vue-router';
import { NInput, NDatePicker, NButton, NCard, NSpace, NTag } from 'naive-ui';

const store = useEventStore();
const updatingStatusId = ref(null);

// =======================================================
// 【新增】搜索和过滤相关的状态
// =======================================================
const searchName = ref('');
const dateRangeStart = ref(null);
const dateRangeEnd = ref(null);
const filteredEvents = computed(() => {
  // 从原始列表开始
  let events = store.events;

  // 1. 按名称过滤
  if (searchName.value.trim()) {
    const lowerCaseQuery = searchName.value.toLowerCase();
    events = events.filter(event =>
      event.name.toLowerCase().includes(lowerCaseQuery)
    );
  }

  // 2. 按开始日期过滤
  if (dateRangeStart.value) {
    events = events.filter(event => new Date(event.date) >= new Date(dateRangeStart.value));
  }

  // 3. 按结束日期过滤
  if (dateRangeEnd.value) {
    // 创建一个 Date 对象并设置到当天的最后一刻，以确保包含选定的结束日期
    const endDate = new Date(dateRangeEnd.value);
    endDate.setHours(23, 59, 59, 999);
    events = events.filter(event => new Date(event.date) <= endDate);
  }

  return events;
});

// 【新增】清空所有筛选条件的函数
function clearFilters() {
  searchName.value = '';
  dateRangeStart.value = null;
  dateRangeEnd.value = null;
}

// 【新增】编辑模态框相关的状态
const isEditModalVisible = ref(false);
const selectedEvent = ref(null);
const editForm = ref(null); // 用于获取 EditEventForm 组件的实例

onMounted(() => {
  store.fetchEvents();
});

const statusClass = (status) => {
  return {
    'status-ongoing': status === '进行中',
    'status-finished': status === '已结束',
    'status-upcoming': status === '未进行',
  };
};
const statusType = (status) => {
  if (status === '进行中') return 'warning';
  if (status === '已结束') return 'default';
  return 'success'; // 未进行
};
async function confirmDelete(eventId) {
  // 弹出浏览器原生确认框
  if (window.confirm('您确定要删除这个展会吗？此操作无法撤销。')) {
    try {
      // 调用 store 中的 deleteEvent 方法执行删除操作
      // 您需要在 eventStore.js 中实现 deleteEvent 方法，
      // 该方法会向后端发送 DELETE 请求。
      await store.deleteEvent(eventId);
      // 可选：删除成功后显示提示
      // alert('展会已删除');
    } catch (error) {
      // 显示错误信息
      alert(error.message || '删除失败，请稍后再试。');
    }
  }
}
// 【新增】处理状态变更的函数
async function changeStatus(eventId, newStatus) {
  // 防止重复点击
  if (updatingStatusId.value) return;

  updatingStatusId.value = eventId;
  try {
    await store.updateEventStatus(eventId, newStatus);
  } catch (error) {
    // 如果 store 抛出错误，在这里通知用户
    alert(error.message);
  } finally {
    // 无论成功或失败，最后都清除更新中的状态
    updatingStatusId.value = null;
  }
}
function openEditModal(event) {
  selectedEvent.value = event;
  isEditModalVisible.value = true;
}

// 【新增】关闭编辑模态框的函数
function closeEditModal() {
  isEditModalVisible.value = false;
  selectedEvent.value = null;
}

// 【新增】处理更新提交的函数
async function handleUpdateEvent() {
  // 增加对 selectedEvent 的检查，更安全
  if (editForm.value && selectedEvent.value) { 
    const formData = editForm.value.submit();
    if (formData) {
      try {
        // 【核心修正】
        // 第一个参数传入 event ID
        // 第二个参数传入 FormData
        console.log('尝试进行更新')
        await store.updateEvent(selectedEvent.value.id, formData);
        console.log('更新成功');
        closeEditModal(); // 成功后关闭模态框
      } catch (error) {
        alert(error.message); // 显示错误
      }
    }
  }
}
</script>

<style scoped>
.list-container {
  background-color: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  margin-bottom: 1.5rem;
  overflow: hidden;
}

.section-header {
  background: var(--card-bg-color);
  border-bottom: 2px solid var(--border-color);
  padding: 1rem 1.5rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.section-header h2 {
  margin: 0;
  color: var(--accent-color);
  font-size: var(--font-lg);
  font-weight: 600;
}

.search-container {
  padding: 1rem 1.5rem;
}

.event-list {
  list-style: none;
  padding: 1.5rem;
  margin: 0;
}

/* 每个卡片为独立长条 */
.event-card {
  box-sizing: border-box;
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  width: 100%;
  background-color: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  padding: 18px 20px;
  margin-bottom: 16px;     /* 卡片之间的间距 */
  box-shadow: var(--shadow-md);
  transition: transform 0.12s ease, box-shadow 0.12s ease;
  cursor: default;         /* clickable 类会改为 pointer */
  min-height: 72px;     /* 保持卡片高度一致 */
}

/* 保持原有 clickable 行为（整行可点击） */
.event-card.clickable {
  cursor: pointer;
}
.event-card.clickable:hover {
  transform: translateY(-6px);
  box-shadow: var(--shadow-xl);
}
/* 左侧信息区域占满剩余空间 */
.event-info {
  flex: 1 1 auto;
  min-width: 0;            /* 保证文本可以正确换行 */
}
.event-info h3 {
  margin: 0 0 6px 0;
  font-size: var(--font-lg);
  line-height: 1.25;
  font-weight: 700;
}
.event-info p {
  margin: 0;
  color: var(--secondary-text-color);
  font-size: var(--font-md);
  line-height: 1.3;
}

/* 右侧状态与操作区固定宽度，垂直居中 */
.event-status {
  flex: 0 0 240px;         /* 根据需要调整宽度 */
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 10px;
  text-align: right;
}

/* 状态徽章样式 */
.status-badge {
  display: inline-block;
  padding: 6px 10px;
  border-radius: var(--radius-pill);
  font-size: var(--font-base);
  font-weight: 600;
  color: var(--primary-text-color);
  border: 1px solid transparent;
}

/* 状态颜色类（保留现有类名） */
.status-ongoing { background: var(--accent-color-light); border-color: rgba(255, 223, 87, 0.25); }
.status-finished { background: var(--bg-elevated); border-color: var(--border-color-light); }
.status-upcoming { background: var(--accent-color-light); border-color: var(--accent-color); }

.status-actions {
  margin-top: 0.5rem;
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  justify-content: flex-end;
  width: 100%;
  min-width: 250px;
}
.action-btn {
  background: none;
  border: 1px solid var(--primary-text-color);
  color: var(--primary-text-color);
  padding: 8px 12px;
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: var(--font-base);
}
.action-btn:hover {
  background-color: var(--primary-text-color);
  color: var(--bg-color);
}
.event-card.clickable {
  cursor: pointer;
  transition: background-color 0.2s, border-color 0.2s;
}
.event-card.clickable:hover {
  background-color: var(--accent-color-light);
  border-color: var(--accent-color);
}
.delete-btn {
  border-color: var(--delete-color);
  color: var(--delete-color);
}

.delete-btn:hover {
  background-color: var(--delete-color);
  color: var(--text-white);
}
.search-container {
  padding: 1rem 1.5rem;
  display: flex;
  flex-wrap: wrap; /* 在小屏幕上换行 */
  gap: 1rem;
  align-items: flex-end; /* 让元素底部对齐 */
  border-bottom: 1px solid var(--border-color);
  margin-bottom: 1rem;
}

.search-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  flex-grow: 1; /* 让组占据可用空间 */
}

.search-group label {
  font-size: var(--font-base);
  color: var(--text-muted);
}

.search-group input[type="text"],
.search-group input[type="date"] {
  background-color: var(--bg-color);
  border: 1px solid var(--border-color);
  color: var(--primary-text-color);
  padding: 8px;
  border-radius: var(--radius-sm);
  font-size: var(--font-md);
}

.date-range-inputs {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.date-range-inputs span {
  color: var(--text-muted);
}

.btn-secondary {
  background-color: var(--btn-secondary);
  border-color: var(--btn-secondary);
  color: var(--text-white);
  padding: 8px 12px;
  height: fit-content; /* 让按钮高度与输入框匹配 */
}
.btn-secondary:hover {
  background-color: var(--btn-secondary-hover);
}
.no-results-message {
  text-align: center;
  padding: 2rem 1.5rem;
  color: var(--text-muted);
}

/* 响应式布局调整 */
@media (max-width: 768px) {
  .event-card {
    flex-direction: column;
    align-items: flex-start;
    min-height: auto;
    padding: 16px;
  }

  .event-info {
    width: 100%;
  }

  .status-actions {
    width: 100%;
    justify-content: flex-start;
  }

  .search-container {
    flex-direction: column;
    padding: 0.75rem;
  }

  .search-group {
    width: 100%;
  }

  .date-range-inputs {
    flex-direction: column;
    width: 100%;
  }

  .date-range-inputs span {
    display: none;
  }
}

@media (max-width: 480px) {
  .event-card {
    padding: 12px;
    gap: 8px;
  }

  .event-info h3 {
    font-size: 1rem;
  }

  .event-info p {
    font-size: 0.9rem;
  }

  .status-actions {
    gap: 6px;
  }

  .search-group label {
    font-size: 0.8rem;
  }
}

</style>
