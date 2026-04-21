<template>
  <div class="cs-wrapper">
    <div
      class="cs-header"
      role="button"
      :aria-expanded="!isCollapsed"
      @click="toggle"
    >
      <h2 class="cs-title">
        <slot name="title">{{ title }}</slot>
        <!-- header-extra 放置在标题侧（HelpBubble、状态 tag 等）；
             点击这里不应触发折叠，所以包一层 click.stop -->
        <span v-if="$slots['header-extra']" class="cs-extra" @click.stop>
          <slot name="header-extra" />
        </span>
      </h2>
      <n-button text class="cs-toggle-btn" tabindex="-1">
        {{ isCollapsed ? '展开' : '折叠' }}
      </n-button>
    </div>

    <transition name="cs-expand">
      <div v-show="!isCollapsed" class="cs-body">
        <slot />
      </div>
    </transition>
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'
import { NButton } from 'naive-ui'

const props = defineProps({
  title: { type: String, default: '' },
  // 初始折叠状态
  defaultCollapsed: { type: Boolean, default: false },
  // 受控模式（可选）：传入后优先使用此值
  collapsed: { type: Boolean, default: null },
})

const emit = defineEmits(['update:collapsed', 'toggle'])

const internalCollapsed = ref(props.defaultCollapsed)
const isCollapsed = ref(
  props.collapsed !== null ? props.collapsed : props.defaultCollapsed
)

// 外部 v-model:collapsed 变化时同步
watch(
  () => props.collapsed,
  (v) => {
    if (v !== null) isCollapsed.value = v
  },
)

function toggle() {
  const next = !isCollapsed.value
  isCollapsed.value = next
  internalCollapsed.value = next
  emit('update:collapsed', next)
  emit('toggle', next)
}

defineExpose({ isCollapsed, toggle })
</script>

<style scoped>
.cs-wrapper {
  margin-bottom: 0;
}

.cs-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  user-select: none;
  padding: 0.75rem 1rem;
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  transition: border-color 0.2s ease, background-color 0.2s ease;
  margin-bottom: 0.5rem;
}
.cs-header:hover {
  background: var(--hover-bg-color, var(--card-bg-color));
  border-color: var(--accent-color);
}

.cs-title {
  margin: 0;
  display: inline-flex;
  align-items: center;
  gap: 10px;
  font-size: var(--font-lg);
  color: var(--accent-color);
  font-weight: 600;
}

.cs-extra {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.cs-toggle-btn {
  font-size: var(--font-base);
  padding: 0.25rem 0.75rem;
  min-width: auto;
  color: var(--accent-color);
}

.cs-body {
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: 1.5rem;
}

/* 展开/折叠动画 —— 3000px 对大型 section（统计图表、长列表）足够兜底 */
.cs-expand-enter-active,
.cs-expand-leave-active {
  transition: opacity 0.25s ease, max-height 0.3s ease;
  overflow: hidden;
}
.cs-expand-enter-from,
.cs-expand-leave-to {
  opacity: 0;
  max-height: 0;
}
.cs-expand-enter-to,
.cs-expand-leave-from {
  opacity: 1;
  max-height: 3000px;
}

@media (max-width: 480px) {
  .cs-header { padding: 0.6rem 0.75rem; }
  .cs-title { font-size: 1.1rem; }
  .cs-body { padding: 1rem; }
}
</style>
