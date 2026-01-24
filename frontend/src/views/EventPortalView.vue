<template>
  <div class="portal-container">
    <n-card class="portal-box" :bordered="false">
      <header>
        <h1>欢迎光临</h1>
        <p>请选择您所在的展会进入点单页面</p>
      </header>
      
      <n-alert v-if="showAlert" type="warning" :bordered="false" class="version-alert">
        <div class="alert-content">
          <span>该APP目前处于较早期版本，请时刻关注检查更新，避免被可能的恶性Bug困扰！</span>
          <span>初次运行请点击上方的“管理员页面”进入后台。</span>
          <n-button text type="primary" @click="showAlert = false" class="close-btn">关闭</n-button>
        </div>
      </n-alert>
      
      <div v-if="eventStore.isLoading" class="loading">
        <n-spin>
          <template #description>正在加载展会列表...</template>
        </n-spin>
      </div>
      <div v-else-if="eventStore.error" class="error">
        <n-alert type="error" :bordered="false">{{ eventStore.error }}</n-alert>
      </div>
      
      <div v-else-if="ongoingEvents.length" class="event-list">
        <n-space vertical size="large">
          <RouterLink 
            v-for="event in ongoingEvents" 
            :key="event.id"
            :to="`/events/${event.id}/order`"
            class="event-link-card"
          >
            <n-card hoverable :bordered="true">
              <h2>{{ event.name }}</h2>
              <span>{{ event.date }} @ {{ event.location || '会场' }}</span>
            </n-card>
          </RouterLink>
        </n-space>
      </div>
      
      <div v-else class="no-events">
        <p>当前没有正在进行的贩售活动 (´·ω·`)</p>
        <p>请联系摊主确认展会状态。</p>
      </div>
    </n-card>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue';
import { RouterLink } from 'vue-router';
import { useEventStore } from '@/stores/eventStore'; // 复用我们已有的 eventStore
import { NCard, NSpin, NAlert, NSpace, NButton } from 'naive-ui';

const eventStore = useEventStore();
const showAlert = ref(true);

// 计算属性，筛选出“进行中”的展会
const ongoingEvents = computed(() => {
  // 防御性检查：确保 eventStore.events 是数组
  const events = Array.isArray(eventStore.events) ? eventStore.events : [];
  if (!Array.isArray(eventStore.events) && eventStore.events) {
    console.error('❌ eventStore.events 不是数组:', eventStore.events);
  }
  return events.filter(event => event.status === '进行中');
});

// 组件加载时，获取所有展会数据
onMounted(() => {
  eventStore.fetchEvents();
});
</script>

<style scoped>
.portal-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100vh;
  padding: 2rem;
  box-sizing: border-box;
}
.portal-box {
  width: 100%;
  max-width: 600px;
  background-color: var(--card-bg-color);
  border-radius: 8px;
  padding: 2rem 3rem;
  border: 1px solid var(--border-color);
  text-align: center;
}
header h1 {
  color: var(--accent-color);
  margin-top: 0;
}
header p {
  color: var(--text-muted);
  margin-bottom: 2rem;
}
.version-alert {
  margin-bottom: 2rem;
  background-color: #fffbe6;
  border-color: #ffe58f;
}
.alert-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
}
.close-btn {
  flex-shrink: 0;
}
.event-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}
.event-link-card {
  display: block;
  text-decoration: none;
  color: var(--primary-text-color);
}
.event-link-card h2 {
  margin: 0 0 0.5rem 0;
}
.event-link-card span {
  color: var(--text-muted);
}
.no-events p {
  line-height: 1.6;
}
</style>