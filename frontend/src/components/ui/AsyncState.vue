<template>
  <div class="async-state">
    <template v-if="state === 'loading'">
      <div class="async-state__loading">
        <n-spin size="medium" />
        <span class="async-state__loading-text">{{ loadingText }}</span>
      </div>
      <slot name="loading" />
    </template>

    <n-spin v-else-if="overlay" :show="loading">
      <template v-if="state === 'error'">
        <n-alert class="async-state__error" type="error" :title="error ?? undefined">
          <div v-if="hasRetry" class="async-state__retry">
            <n-button size="small" @click="emit('retry')">重试</n-button>
          </div>
        </n-alert>
      </template>

      <template v-else-if="state === 'empty'">
        <slot name="empty">
          <EmptyState compact title="暂无数据" />
        </slot>
      </template>

      <template v-else>
        <slot />
      </template>
    </n-spin>

    <template v-else-if="state === 'error'">
      <n-alert class="async-state__error" type="error" :title="error ?? undefined">
        <div v-if="hasRetry" class="async-state__retry">
          <n-button size="small" @click="emit('retry')">重试</n-button>
        </div>
      </n-alert>
    </template>

    <template v-else-if="state === 'empty'">
      <slot name="empty">
        <EmptyState compact title="暂无数据" />
      </slot>
    </template>

    <template v-else>
      <slot />
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, getCurrentInstance } from 'vue'
import { NAlert, NButton, NSpin } from 'naive-ui'
import EmptyState from './EmptyState.vue'

const props = withDefaults(
  defineProps<{
    loading?: boolean
    error?: string | null
    empty?: boolean
    loadingText?: string
    /**
     * 覆盖式加载：true 时 `loading` 不替换内容，而是用 n-spin 盖在
     * error / empty / default 态上（旧数据仍可见）。默认 false 保持原行为。
     */
    overlay?: boolean
  }>(),
  {
    loading: false,
    error: null,
    empty: false,
    loadingText: '加载中…',
    overlay: false,
  }
)

const emit = defineEmits<{
  (e: 'retry'): void
}>()

// 只有父组件真的监听了 retry 才显示按钮，否则点了也没人接。
const instance = getCurrentInstance()
const hasRetry = computed(() => Boolean(instance?.vnode.props?.onRetry))

// overlay 模式下 loading 交给 n-spin 呈现，状态机只决定底层展示 error/empty/default；
// 优先级 error > empty 不变。
const state = computed(() => {
  if (props.loading && !props.overlay) return 'loading'
  if (props.error) return 'error'
  if (props.empty) return 'empty'
  return 'default'
})
</script>

<style scoped>
.async-state__loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-md);
  padding: var(--space-2xl) var(--space-xl);
  color: var(--text-muted);
}

.async-state__loading-text {
  font-size: var(--font-base);
}

.async-state__error {
  margin-bottom: var(--space-lg);
}

.async-state__retry {
  margin-top: var(--space-md);
}
</style>
