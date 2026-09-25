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

          <n-button v-if="!isMobile" circle size="small" @click="toggleSidebar" class="toggle-btn">
            <template #icon>
              <span v-if="isSidebarCollapsed">»</span>
              <span v-else>«</span>
            </template>
          </n-button>
        </div>

        <!-- ✅ 中间可滚动区域：菜单 -->
        <div class="sidebar-body">
          <n-menu
            :collapsed="isSidebarCollapsed"
            :collapsed-width="80"
            :collapsed-icon-size="22"
            :options="menuOptions"
            :value="activeKey"
          />
        </div>

        <!-- ✅ 底部固定区域：快捷入口 -->
        <div v-if="!isSidebarCollapsed" class="sidebar-footer">
          <n-divider />
          <p class="section-title">快捷入口</p>
          <n-space vertical>
            <n-button block secondary type="primary" @click="$router.push('/vendor')">
              <template #icon>
                <n-icon><ExternalIcon /></n-icon>
              </template>
              摊主端
            </n-button>
            <n-button block secondary @click="$router.push('/')">
              <template #icon>
                <n-icon><ExternalIcon /></n-icon>
              </template>
              顾客端
            </n-button>
          </n-space>

          <!-- ✅ 给底部留安全区，防止低高度/移动端被遮 -->
          <div class="footer-safe-space" />
        </div>
      </div>
    </n-layout-sider>

    <n-layout-content class="main-content" content-style="padding: 24px;" :native-scrollbar="false">
      <n-button v-if="isMobile" circle type="primary" class="mobile-fab" @click="toggleSidebar">
        <template #icon>{{ isSidebarCollapsed ? '☰' : '✕' }}</template>
      </n-button>

      <DefaultPasswordBanner />
      <router-view />

      <div
        v-if="!isSidebarCollapsed && isMobile"
        class="mobile-overlay"
        @click="closeSidebar"
      ></div>
    </n-layout-content>
  </n-layout>
</template>

<script setup lang="ts">
import { computed, h, ref, watch } from 'vue'
import { RouterLink, useRoute } from 'vue-router'
import {
  NLayout,
  NLayoutSider,
  NLayoutContent,
  NButton,
  NMenu,
  NDivider,
  NSpace,
  NIcon,
  type MenuOption,
} from 'naive-ui'
import { useViewport } from '@/composables/useViewport'
import DefaultPasswordBanner from '@/components/settings/DefaultPasswordBanner.vue'
import { resolveActiveKey } from './adminNav'

const route = useRoute()

const isSidebarCollapsed = ref(false)
const { isTablet: isMobile } = useViewport()

watch(
  isMobile,
  (v) => {
    isSidebarCollapsed.value = v
  },
  { immediate: true }
)

// 侧栏不再随展会跳变：固定五项 + 快捷入口（spec §3.3）。
const activeKey = computed(() => resolveActiveKey(route.path))

const ExternalIcon = () =>
  h(
    'svg',
    {
      xmlns: 'http://www.w3.org/2000/svg',
      viewBox: '0 0 24 24',
      fill: 'none',
      stroke: 'currentColor',
      strokeWidth: '2',
    },
    [
      h('path', { d: 'M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6' }),
      h('polyline', { points: '15 3 21 3 21 9' }),
      h('line', { x1: '10', y1: '14', x2: '21', y2: '3' }),
    ]
  )

const menuOptions: MenuOption[] = [
  {
    label: () => h(RouterLink, { to: '/admin/events' }, { default: () => '展会' }),
    key: '/admin/events',
  },
  {
    label: () => h(RouterLink, { to: '/admin/master-products' }, { default: () => '商品库' }),
    key: '/admin/master-products',
  },
  {
    label: () => h(RouterLink, { to: '/admin/societies' }, { default: () => '社团' }),
    key: '/admin/societies',
  },
  { type: 'divider', key: 'd1' },
  {
    label: () => h(RouterLink, { to: '/admin/settings' }, { default: () => '设置' }),
    key: '/admin/settings',
  },
  {
    label: () => h(RouterLink, { to: '/admin/help' }, { default: () => '使用教程' }),
    key: '/admin/help',
  },
]

function toggleSidebar() {
  isSidebarCollapsed.value = !isSidebarCollapsed.value
}
function closeSidebar() {
  if (isMobile.value) isSidebarCollapsed.value = true
}
</script>

<style scoped>
/* sider 本体 */
.sidebar-container {
  height: 100vh;
  height: 100dvh;
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
  font-size: var(--font-lg);
  color: var(--primary-text-color);
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
  padding-bottom: var(--space-sm); /* 给滚动底部一点呼吸 */
}

.sidebar-footer {
  flex: 0 0 auto;
  padding: 0 var(--space-lg) var(--space-md) var(--space-lg);
}

/* ✅ 安全区/低高度兜底 */
.footer-safe-space {
  height: 12px;
}

/* 标题 */
.section-title {
  font-size: var(--font-xs);
  color: var(--text-muted);
  margin-bottom: var(--space-md);
  padding-left: var(--space-xs);
}

/* 移动端汉堡按钮 */
.mobile-fab {
  position: fixed;
  bottom: calc(var(--space-xl) + env(safe-area-inset-bottom, 0));
  right: var(--space-xl);
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
  min-height: 100dvh;
  box-sizing: border-box;
  overflow: auto;
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- env() 安全区适配，无对应空间 token */
  padding-bottom: calc(var(--space-2xl) + env(safe-area-inset-bottom, 0));
}

/* 移动端 sider 悬浮 */
@media (--tablet) {
  :deep(.n-layout-sider) {
    position: fixed !important;
    height: 100vh !important;
    height: 100dvh !important;
    z-index: 1000;
  }

  /* 移动端侧边栏底部按钮区域：额外留 50px，避免被系统操作栏遮挡 */
  .sidebar-footer {
    /* stylelint-disable-next-line declaration-property-value-allowed-list -- env() 安全区适配，无对应空间 token */
    padding-bottom: calc(var(--space-md) + var(--space-2xl) + env(safe-area-inset-bottom, 0));
  }
}
</style>
