<!--
  摊主端外壳（spec §3.6）：页头（展会名 + 去点单 + 切换展会）+ 三个 tab 子路由。
  3 秒轮询、新单提示音与 `<audio>` 元素都挂在这里，切到库存 / 收摊 tab 也不中断。

  手机（`useViewport().isPhone`）把 tab 行换成底部固定 tab 栏，并在根元素上写
  `--vendor-tabbar-height`（tab 栏实际高度 + 安全区）；更宽时为 0px。后续任务靠它
  把收摊页的主按钮定位在 tab 栏之上，不要删。
-->
<template>
  <div class="vendor-shell" :class="{ 'vendor-shell--phone': isPhone }">
    <PageShell width="wide">
      <template #title>{{ eventName }}</template>
      <template #actions>
        <RouterLink
          class="vendor-shell__action"
          :to="{ name: 'customer-order', params: { id: props.id } }"
        >
          去点单
        </RouterLink>
        <RouterLink class="vendor-shell__action" :to="{ name: 'vendor-select' }">
          切换展会
        </RouterLink>
      </template>

      <!-- 宽屏：页头下的 tab 行 -->
      <nav v-if="!isPhone" class="vendor-tabs">
        <RouterLink
          v-for="tab in TABS"
          :key="tab.name"
          class="vendor-tab"
          :to="{ name: tab.name, params: { id: props.id } }"
        >
          {{ tab.label }}
          <span v-if="tab.name === 'vendor-orders' && pendingCount" class="vendor-tab__badge">
            {{ pendingCount }}
          </span>
        </RouterLink>
      </nav>

      <router-view />
    </PageShell>

    <!-- 手机：底部固定三格 tab 栏 -->
    <nav v-if="isPhone" class="vendor-tabbar">
      <RouterLink
        v-for="tab in TABS"
        :key="tab.name"
        class="vendor-tabbar__item"
        :to="{ name: tab.name, params: { id: props.id } }"
      >
        {{ tab.label }}
        <span v-if="tab.name === 'vendor-orders' && pendingCount" class="vendor-tab__badge">
          {{ pendingCount }}
        </span>
      </RouterLink>
    </nav>

    <!-- 隐藏的音频播放器，引用 public 目录下的 notify.mp3 -->
    <audio ref="audioRef" src="/notify.mp3" preload="auto"></audio>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, toRef } from 'vue'
import { RouterLink } from 'vue-router'
import { PageShell } from '@/components/ui'
import { useEventStore } from '@/stores/eventStore'
import { useViewport } from '@/composables/useViewport'
import { provideVendorPolling, useVendorPolling } from '@/composables/useVendorPolling'

const props = defineProps<{ id: string }>()

const TABS = [
  { name: 'vendor-orders', label: '订单' },
  { name: 'vendor-inventory', label: '库存' },
  { name: 'vendor-closing', label: '收摊' },
] as const

const eventStore = useEventStore()
const { isPhone } = useViewport()
const audioRef = ref<HTMLAudioElement | null>(null)

const eventName = computed(() => {
  const event = eventStore.events.find((e) => e.id === parseInt(props.id, 10))
  return event ? event.name : `展会 #${props.id}`
})

// 轮询只在这里启动一次；子页通过 inject 拿 refresh。
const polling = useVendorPolling(toRef(props, 'id'), audioRef)
provideVendorPolling(polling)
const { pendingCount } = polling
</script>

<style scoped>
.vendor-shell {
  /* 默认 0：没有底部 tab 栏时子页的主按钮不需要上抬。 */
  --vendor-tabbar-height: 0px;
  min-height: 100%;
}

/* 手机才有的底部 tab 栏高度；含 iPhone 底部安全区。 */
.vendor-shell--phone {
  --vendor-tabbar-height: calc(
    var(--space-2xl) + var(--space-lg) + env(safe-area-inset-bottom, 0px)
  );
}

/* 底部 tab 栏是 fixed 的，给内容留出等高的下边距，别让最后一屏被它盖住。 */
.vendor-shell--phone :deep(.page-shell) {
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- --vendor-tabbar-height 不是 space token，无法用 spaceList 表达 */
  padding-bottom: calc(var(--vendor-tabbar-height) + var(--space-lg));
}

.vendor-shell__action {
  display: inline-flex;
  align-items: center;
  min-height: 44px;
  padding: var(--space-xs) var(--space-md);
  border-radius: var(--radius-pill);
  border: 1px solid var(--border-color);
  color: var(--secondary-text-color);
  font-size: var(--font-sm);
  text-decoration: none;
  white-space: nowrap;
}

.vendor-shell__action:hover {
  background: var(--accent-color);
  color: var(--text-white);
  border-color: var(--accent-color);
}

.vendor-tabs {
  display: flex;
  gap: var(--space-sm);
  padding-bottom: var(--space-sm);
  border-bottom: 1px solid var(--border-color);
  margin-bottom: var(--space-lg);
  overflow-x: auto;
}

.vendor-tab {
  display: inline-flex;
  align-items: center;
  gap: var(--space-xs);
  min-height: 44px;
  padding: var(--space-xs) var(--space-md);
  border-radius: var(--radius-pill);
  color: var(--secondary-text-color);
  font-size: var(--font-base);
  text-decoration: none;
  white-space: nowrap;
}

.vendor-tab:hover {
  color: var(--accent-color);
}

.vendor-tab.router-link-active {
  background: var(--accent-color-light);
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}

.vendor-tabbar {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 100;
  display: flex;
  background-color: var(--card-bg-color);
  border-top: 1px solid var(--border-color);
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- iPhone 底部安全区适配，env() 无法用 space token 表达 */
  padding-bottom: env(safe-area-inset-bottom, 0px);
}

.vendor-tabbar__item {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-xs);
  min-height: calc(var(--space-2xl) + var(--space-lg));
  color: var(--secondary-text-color);
  font-size: var(--font-base);
  text-decoration: none;
}

.vendor-tabbar__item.router-link-active {
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}

.vendor-tab__badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  height: 20px;
  padding: 0 var(--space-xs);
  border-radius: var(--radius-pill);
  background: var(--error-color);
  color: var(--text-white);
  font-size: var(--font-xs);
  font-weight: var(--weight-bold);
  line-height: 1;
}
</style>
