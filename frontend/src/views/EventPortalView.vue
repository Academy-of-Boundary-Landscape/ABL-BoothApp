<template>
  <div class="portal-container">
    <n-card class="portal-box" :bordered="false">
      <!-- ========== 首次使用欢迎页（无展会 + 从未 admin 登录） ========== -->
      <template v-if="isFirstTimeSetup">
        <header class="welcome-header">
          <div class="welcome-emoji">🎉</div>
          <h1>欢迎使用摊盒</h1>
          <p class="welcome-sub">Booth-Kernel · 本地化同人展出摊系统</p>
        </header>

        <div class="welcome-body">
          <p class="welcome-intro">
            看起来这是你第一次使用。让我们开始吧：
          </p>
          <router-link to="/admin" class="welcome-cta">
            <span class="welcome-cta-icon">🔧</span>
            <span class="welcome-cta-text">进入管理后台开始配置</span>
            <span class="welcome-cta-arrow">→</span>
          </router-link>
          <p class="welcome-foot">
            配置完成后，顾客扫码访问本页即可看到你的展会。
          </p>
        </div>

        <div class="portal-nav">
          <router-link to="/admin" class="portal-nav-link">我是摊主 / 管理员</router-link>
          <router-link to="/vendor" class="portal-nav-link">我是协同摊主</router-link>
        </div>
      </template>

      <!-- ========== 常规顾客入口 ========== -->
      <template v-else>
        <header>
          <h1>欢迎光临</h1>
          <p>请选择您所在的展会进入点单页面</p>
        </header>

        <n-alert v-if="showAlert" type="warning" :bordered="false" class="version-alert">
          <div class="alert-content">
            <span>该 App 仍处于早期版本，建议定期检查更新；初次使用请先进入"管理员页面"完成后台设置。</span>
            <n-button text type="primary" @click="dismissAlert" class="close-btn">不再提示</n-button>
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
          <p>摊主可能还在准备中，或今日活动尚未开始。</p>
        </div>

        <div class="portal-nav">
          <router-link to="/admin" class="portal-nav-link">管理后台</router-link>
          <router-link to="/vendor" class="portal-nav-link">摊主页面</router-link>
        </div>
      </template>
    </n-card>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue';
import { RouterLink } from 'vue-router';
import { useEventStore } from '@/stores/eventStore'; // 复用我们已有的 eventStore
import { NCard, NSpin, NAlert, NSpace, NButton } from 'naive-ui';

const VERSION_ALERT_KEY = 'portal_version_alert_dismissed_v1.1';
const ADMIN_FIRST_LOGIN_KEY = 'admin_first_login_done';

const eventStore = useEventStore();

// 版本告警：用户点过"不再提示"后持久隐藏
const showAlert = ref(!localStorage.getItem(VERSION_ALERT_KEY));
function dismissAlert() {
  showAlert.value = false;
  localStorage.setItem(VERSION_ALERT_KEY, '1');
}

// 筛选出"进行中"的展会
const ongoingEvents = computed(() => {
  // 防御性检查：确保 eventStore.events 是数组
  const events = Array.isArray(eventStore.events) ? eventStore.events : [];
  if (!Array.isArray(eventStore.events) && eventStore.events) {
    console.error('❌ eventStore.events 不是数组:', eventStore.events);
  }
  return events.filter(event => event.status === '进行中');
});

// 首次使用判定：加载完成 + 无任何展会 + 本设备从未有管理员成功登录过
// 一旦条件成立，就给摊主看"欢迎配置"页；顾客不可能命中此分支（他们的设备上 admin 从未登过）
const isFirstTimeSetup = computed(() => {
  if (eventStore.isLoading) return false;
  if (eventStore.error) return false;
  const events = Array.isArray(eventStore.events) ? eventStore.events : [];
  if (events.length > 0) return false;
  return !localStorage.getItem(ADMIN_FIRST_LOGIN_KEY);
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
  padding: 1rem;
  box-sizing: border-box;
}
.portal-box {
  width: 100%;
  max-width: 600px;
  background-color: var(--card-bg-color);
  border-radius: var(--radius-md);
  padding: 1.25rem 1rem;
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
  margin-bottom: 1.25rem;
  background-color: var(--highlight-color);
  border-color: var(--warning-color);
}
.alert-content {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: flex-start;
  gap: 1rem;
  width: 100%;
}
.close-btn {
  flex-shrink: 0;
  align-self: flex-end;
}

/* 手机端：进一步收紧外边距和内边距，避免内容过窄 */
@media (max-width: 767px) {
  .portal-container {
    padding: 0.5rem;
  }

  .portal-box {
    padding: 1rem 0.75rem;
  }

  header p {
    margin-bottom: 1.25rem;
  }
}

/* 平板及以上屏幕 */
@media (min-width: 768px) {
  .alert-content {
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
  }
  .close-btn {
    align-self: center;
  }
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
.portal-nav {
  display: flex;
  justify-content: center;
  gap: 12px;
  margin-top: 2rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--border-color);
}
.portal-nav-link {
  padding: 6px 16px;
  border-radius: var(--radius-pill);
  font-size: var(--font-sm);
  color: var(--text-muted);
  text-decoration: none;
  border: 1px solid var(--border-color);
  transition: all 0.15s;
}
.portal-nav-link:hover {
  background: var(--accent-color);
  color: white;
  border-color: var(--accent-color);
}

/* ========== 首次使用欢迎页 ========== */
.welcome-header {
  text-align: center;
  margin-bottom: 1.5rem;
}
.welcome-emoji {
  font-size: 3rem;
  line-height: 1;
  margin-bottom: 0.5rem;
}
.welcome-header h1 {
  color: var(--accent-color);
  margin: 0 0 0.25rem;
}
.welcome-sub {
  color: var(--text-muted);
  font-size: var(--font-sm);
  margin: 0;
}

.welcome-body {
  padding: 0.5rem 0 1rem;
}
.welcome-intro {
  text-align: center;
  color: var(--primary-text-color);
  font-size: var(--font-base);
  margin: 0 0 1.25rem;
}

.welcome-cta {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 1rem 1.25rem;
  background: var(--accent-color);
  color: white;
  text-decoration: none;
  border-radius: var(--radius-md);
  font-size: var(--font-md, 16px);
  font-weight: 600;
  transition: transform 0.12s ease, box-shadow 0.12s ease, filter 0.12s ease;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.08);
}
.welcome-cta:hover {
  transform: translateY(-1px);
  filter: brightness(1.06);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
}
.welcome-cta:active {
  transform: translateY(0);
}
.welcome-cta-icon {
  font-size: 1.25rem;
  line-height: 1;
}
.welcome-cta-arrow {
  font-size: 1.1rem;
  line-height: 1;
  opacity: 0.85;
}
.welcome-foot {
  text-align: center;
  color: var(--text-muted);
  font-size: var(--font-sm);
  margin: 1.25rem 0 0;
  line-height: 1.6;
}
</style>
