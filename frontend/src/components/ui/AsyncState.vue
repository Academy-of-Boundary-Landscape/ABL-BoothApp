<template>
  <div class="async-state">
    <template v-if="state === 'loading'">
      <div class="async-state__loading">
        <n-spin size="medium" />
        <span class="async-state__loading-text">{{ loadingText }}</span>
      </div>
      <slot name="loading" />
    </template>

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
  }>(),
  {
    loading: false,
    error: null,
    empty: false,
    loadingText: '加载中…',
  }
)

const emit = defineEmits<{
  (e: 'retry'): void
}>()

// 只有父组件真的监听了 retry 才显示按钮，否则点了也没人接。
const instance = getCurrentInstance()
const hasRetry = computed(() => Boolean(instance?.vnode.props?.onRetry))

const state = computed(() => {
  if (props.loading) return 'loading'
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
