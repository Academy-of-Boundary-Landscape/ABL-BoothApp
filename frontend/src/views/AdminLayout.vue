<template>
  <n-layout has-sider position="absolute">
    <n-layout-sider
      bordered
      collapse-mode="width"
      :collapsed-width="isMobile ? 0 : 80"
      :width="240"
      :collapsed="isSidebarCollapsed"
      :show-trigger="false"
      @collapse="isSidebarCollapsed = true"
      @expand="isSidebarCollapsed = false"
      class="sidebar-container"
    >
      <!-- ✅ 用一个壳把 header / body / footer 分开 -->
      <div class="sidebar-shell">
        <!-- 侧边栏头部（固定） -->
        <div class="sidebar-header" :style="{ padding: isSidebarCollapsed ? '1rem 0' : '1.5rem' }">
          <h2 v-if="!isSidebarCollapsed" class="logo-text">管理后台</h2>

          <n-button
            v-if="!isMobile"
            circle
            size="small"
            @click="toggleSidebar"
            class="toggle-btn"
          >
            <template #icon>
              <span v-if="isSidebarCollapsed">»</span>
              <span v-else>«</span>
            </template>
          </n-button>
        </div>

        <!-- ✅ 中间可滚动区域：菜单 + 正在进行的展会 -->
        <div class="sidebar-body">
          <n-menu
            :collapsed="isSidebarCollapsed"
            :collapsed-width="80"
            :collapsed-icon-size="22"
            :options="menuOptions"
            :value="activeKey"
          />

          <div
            v-if="!event && ongoingEvents.length > 0 && !isSidebarCollapsed"
            class="ongoing-events-section"
          >
            <n-divider />
            <p class="section-title">正在进行的展会</p>
            <n-space vertical :size="8">
              <n-button
                v-for="evt in ongoingEvents"
                :key="evt.id"
                block
                secondary
                size="small"
                @click="$router.push(`/admin/events/${evt.id}/products`)"
                class="ongoing-event-btn"
              >
                <template #icon>
                  <span class="event-status-dot">●</span>
                </template>
                <span class="event-name-text">{{ evt.name }}</span>
              </n-button>
            </n-space>
          </div>
        </div>

        <!-- ✅ 底部固定区域：永远不被遮住 -->
        <div v-if="!isSidebarCollapsed" class="sidebar-footer">
          <n-divider />
          <p class="section-title">快捷视图</p>
          <n-space vertical>
            <n-button block secondary type="primary" @click="$router.push('/vendor')">
              <template #icon>
                <n-icon><ExternalIcon /></n-icon>
              </template>
              摊主视图
            </n-button>
            <n-button block secondary @click="$router.push('/')">
              <template #icon>
                <n-icon><ExternalIcon /></n-icon>
              </template>
              顾客视图
            </n-button>
          </n-space>

          <n-divider />
          <p class="section-title">系统</p>
          <n-space vertical>
            <n-button block secondary @click="showUpdateModal = true">检查更新</n-button>
            <n-button block secondary @click="$router.push('/admin/about')">关于</n-button>
          </n-space>

          <!-- ✅ 给底部留安全区，防止低高度/移动端被遮 -->
          <div class="footer-safe-space" />
        </div>
      </div>
    </n-layout-sider>

    <n-layout-content
      class="main-content"
      content-style="padding: 24px;"
      :native-scrollbar="false"
    >
      <n-button
        v-if="isMobile"
        circle
        type="primary"
        class="mobile-fab"
        @click="toggleSidebar"
      >
        <template #icon>{{ isSidebarCollapsed ? '☰' : '✕' }}</template>
      </n-button>

      <router-view />

      <n-modal
        v-model:show="showThemeModal"
        preset="card"
        title="主题设置"
        :mask-closable="false"
        style="max-width: 960px"
        class="theme-modal"
      >
        <ThemeSetting />
      </n-modal>

      <UpdateModal :show="showUpdateModal" @update:show="showUpdateModal = $event" />

      <div
        v-if="!isSidebarCollapsed && isMobile"
        class="mobile-overlay"
        @click="closeSidebar"
      ></div>
    </n-layout-content>
  </n-layout>
</template>

<script setup>
import { computed, ref, onMounted, onUnmounted, h } from 'vue'
import { RouterLink, useRouter, useRoute } from 'vue-router'
import {
  NLayout, NLayoutSider, NLayoutContent, NButton,
  NMenu, NDivider, NSpace, NIcon, NModal
} from 'naive-ui'
import { useEventStore } from '@/stores/eventStore'
import ThemeSetting from '@/views/ThemeSetting.vue'
import UpdateModal from '@/components/shared/UpdateModal.vue'

const route = useRoute()
const router = useRouter()
const eventStore = useEventStore()

const isSidebarCollapsed = ref(false)
const isMobile = ref(window.innerWidth < 992)
const showThemeModal = ref(false)
const showUpdateModal = ref(false)

const activeKey = computed(() => route.path)

const ExternalIcon = () =>
  h('svg', { xmlns: 'http://www.w3.org/2000/svg', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', strokeWidth: '2' }, [
    h('path', { d: 'M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6' }),
    h('polyline', { points: '15 3 21 3 21 9' }),
    h('line', { x1: '10', y1: '14', x2: '21', y2: '3' })
  ])

const event = computed(() => {
  const eventId = route.params.id
  if (!eventId) return null
  return eventStore.events.find(e => e.id === parseInt(eventId, 10)) || { name: '加载中...', id: eventId }
})

const ongoingEvents = computed(() => {
  const events = Array.isArray(eventStore.events) ? eventStore.events : []
  return events.filter(e => e.status === '进行中').slice(0, 3)
})

const menuOptions = computed(() => {
  const baseOptions = [
    { label: () => h(RouterLink, { to: '/admin' }, { default: () => '展会管理' }), key: '/admin' },
    { label: () => h(RouterLink, { to: '/admin/master-products' }, { default: () => '全局商品库' }), key: '/admin/master-products' },
    { label: () => h(RouterLink, { to: '/admin/help' }, { default: () => '使用教程' }), key: '/admin/help' },
    { label: () => h('div', { onClick: () => (showThemeModal.value = true), style: { cursor: 'pointer' } }, '主题设置'), key: '/admin/theme-setting' }
  ]

  if (event.value) {
    baseOptions.push(
      { type: 'divider', key: 'd1' },
      {
        label: () => h('div', { class: 'menu-event-name', title: event.value.name }, event.value.name),
        key: 'event-group',
        type: 'group',
        children: [
          { label: () => h(RouterLink, { to: `/admin/events/${event.value.id}/products` }, { default: () => '商品管理' }), key: `/admin/events/${event.value.id}/products` },
          { label: () => h(RouterLink, { to: `/admin/events/${event.value.id}/orders` }, { default: () => '订单管理' }), key: `/admin/events/${event.value.id}/orders` },
          { label: () => h(RouterLink, { to: `/admin/events/${event.value.id}/stats` }, { default: () => '销售统计' }), key: `/admin/events/${event.value.id}/stats` }
        ]
      }
    )
  }

  return baseOptions
})

function toggleSidebar() {
  isSidebarCollapsed.value = !isSidebarCollapsed.value
}
function closeSidebar() {
  if (isMobile.value) isSidebarCollapsed.value = true
}

const handleResize = () => {
  isMobile.value = window.innerWidth < 992
  isSidebarCollapsed.value = isMobile.value ? true : false
}

onMounted(() => {
  window.addEventListener('resize', handleResize)
  handleResize()

  if (!Array.isArray(eventStore.events) || eventStore.events.length === 0) {
    eventStore.fetchEvents?.()
  }
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
})
</script>

<style scoped>
/* sider 本体 */
.sidebar-container {
  height: 100vh;
}

/* ✅ 三段布局：header/body/footer */
.sidebar-shell {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0; /* 关键：让 body 可以滚动 */
}

.sidebar-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: padding 0.3s;
  min-width: 0;
  flex: 0 0 auto;
}

.logo-text {
  margin: 0;
  font-size: 1.25rem;
  color: var(--n-text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
  min-width: 0;
}

/* ✅ 中间区域：可滚动，底部按钮不会被挤走 */
.sidebar-body {
  flex: 1 1 auto;
  min-height: 0;
  overflow: auto;
  padding-bottom: 8px; /* 给滚动底部一点呼吸 */
}

.sidebar-footer {
  flex: 0 0 auto;
  padding: 0 1rem 0.75rem 1rem;
}

/* ✅ 安全区/低高度兜底 */
.footer-safe-space {
  height: 12px;
}

/* 标题 */
.section-title {
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-bottom: 0.75rem;
  padding-left: 4px;
}

/* ongoing */
.ongoing-events-section {
  padding: 0 1rem;
  margin-top: 0.75rem;
}

.ongoing-event-btn {
  text-align: left;
  justify-content: flex-start;
  height: auto;
  min-height: 36px;
  padding: 8px 12px;
  width: 100%;
  overflow: hidden;
}

.event-status-dot {
  color: var(--status-warning);
  font-size: 12px;
  margin-right: 4px;
  flex-shrink: 0;
}

.event-name-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
  min-width: 0;
}

.menu-event-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: block;
  max-width: 100%;
}

:deep(.n-menu-item-group-title) {
  overflow: hidden !important;
  text-overflow: ellipsis !important;
  white-space: nowrap !important;
  max-width: 100% !important;
  padding-right: 8px !important;
}

/* 移动端汉堡按钮 */
.mobile-fab {
  position: fixed;
  bottom: 100px;
  right: 24px;
  z-index: 1001;
  box-shadow: var(--shadow-md);
}

.mobile-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay-color);
  z-index: 999;
}

/* 主内容区滚动 */
.main-content {
  min-height: 100vh;
  box-sizing: border-box;
  overflow: auto;
  padding-bottom: 40px;
}

/* 移动端 sider 悬浮 */
@media (max-width: 992px) {
  :deep(.n-layout-sider) {
    position: fixed !important;
    height: 100% !important;
    z-index: 1000;
  }

  /* 移动端侧边栏底部按钮区域：额外留 50px，避免被系统操作栏遮挡 */
  .sidebar-footer {
    padding-bottom: calc(0.75rem + 50px + env(safe-area-inset-bottom, 0));
  }
}

.theme-modal :deep(.n-card__content) {
  padding-top: 0;
}
</style>
