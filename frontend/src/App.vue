<template>
  <n-config-provider class="app-container" :theme="naiveTheme" :theme-overrides="themeOverrides">
    <n-message-provider>
      <n-dialog-provider>
        <main class="app-content">
          <RouterView />
        </main>
        <GlobalAlert />
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup>
import { computed } from 'vue';
import { RouterView } from 'vue-router';
import GlobalAlert from '@/components/GlobalAlert.vue';
import { NConfigProvider, NMessageProvider, NDialogProvider, darkTheme } from 'naive-ui';
import { useThemeStore } from '@/stores/themeStore';

const themeStore = useThemeStore();

// 让 Naive UI 随 store 的深色/浅色状态切换
const naiveTheme = computed(() => (themeStore.isDark ? darkTheme : null));

// 覆盖色板随用户自定义主色实时更新
// 注意：在 Pinia setup store 中，naiveThemeOverrides 已经是解包后的值，
// 这里不需要再取 .value，直接包一层 computed 保持响应式即可。
const themeOverrides = computed(() => themeStore.naiveThemeOverrides);
</script>
<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  box-sizing: border-box;
  overflow: hidden;
  background: var(--bg-color);
}

.app-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-left: env(safe-area-inset-left, 0);
  padding-right: env(safe-area-inset-right, 0);
  padding-bottom: env(safe-area-inset-bottom, 0);
  box-sizing: border-box;
}

:global(html),
:global(body),
:global(#app) {
  height: 100%;
  overflow: hidden;
}

:global(body) {
  margin: 0;
  background: var(--bg-color);
}
</style>
