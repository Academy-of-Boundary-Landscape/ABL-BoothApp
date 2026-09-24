<template>
  <div class="selection-container">
    <n-card class="selection-box" :bordered="false">
      <template #header>
        <h2>请选择您所在的展会</h2>
      </template>
      <p>选择后将需要输入该展会的摊主密码。</p>

      <AsyncState
        :loading="eventStore.isLoading"
        :error="eventStore.error"
        :empty="!ongoingEvents.length"
        loading-text="正在加载展会列表..."
      >
        <div class="event-list">
          <n-space vertical size="large">
            <n-card
              v-for="event in ongoingEvents"
              :key="event.id"
              class="event-item"
              hoverable
              :bordered="true"
              @click="selectEvent(event)"
            >
              <h3>{{ event.name }}</h3>
              <span>{{ event.date }}</span>
            </n-card>
          </n-space>
        </div>

        <template #empty>
          <EmptyState icon="" title="当前没有正在进行的展会。" />
        </template>
      </AsyncState>

      <div class="admin-login-link">
        <RouterLink to="/login/admin">
          <n-button text>管理员入口</n-button>
        </RouterLink>
      </div>
    </n-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRouter, RouterLink } from 'vue-router'
import { useEventStore } from '@/stores/eventStore'
import { NCard, NSpace, NButton } from 'naive-ui'
import { AsyncState, EmptyState } from '@/components/ui'
import type { Schemas } from '@/api/client'

const eventStore = useEventStore()
const router = useRouter()

// 计算属性，只筛选出“进行中”的展会
const ongoingEvents = computed(() => {
  // 防御性检查：确保 eventStore.events 是数组
  const events = Array.isArray(eventStore.events) ? eventStore.events : []
  if (!Array.isArray(eventStore.events) && eventStore.events) {
    console.error('❌ eventStore.events 不是数组:', eventStore.events)
  }
  return events.filter((event) => event.status === '进行中')
})

function selectEvent(event: Schemas['EventResponse']) {
  // 当用户选择一个展会时，跳转到该展会的摊主登录页面
  router.push({
    name: 'login',
    params: { role: 'vendor' },
    query: {
      eventId: event.id,
      // 登录成功后，我们希望他跳转到这个展会的摊主页面
      redirect: `/vendor/${event.id}`,
    },
  })
}

onMounted(() => {
  // 页面加载时，获取所有展会列表
  eventStore.fetchEvents()
})
</script>

<style scoped>
.selection-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100vh;
}
.selection-box {
  width: 500px;
  max-width: 90%;
  padding: var(--space-2xl);
  background-color: var(--card-bg-color);
  border-radius: var(--radius-md);
  text-align: center;
}
.event-list {
  margin-top: var(--space-2xl);
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}
.event-item {
  cursor: pointer;
}
.admin-login-link {
  margin-top: var(--space-2xl);
  font-size: var(--font-base);
}
.admin-login-link a {
  text-decoration: none;
}
</style>
