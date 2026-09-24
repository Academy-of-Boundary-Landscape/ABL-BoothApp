<template>
  <div class="page-shell" :style="{ maxWidth }">
    <header class="page-shell__header">
      <div class="page-shell__heading">
        <h1 class="page-shell__title">
          <slot name="title">{{ title }}</slot>
        </h1>
        <p v-if="subtitle" class="page-shell__subtitle">{{ subtitle }}</p>
      </div>
      <div v-if="help || $slots.actions" class="page-shell__meta">
        <HelpBubble v-if="help" :page="help" />
        <div v-if="$slots.actions" class="page-shell__actions">
          <slot name="actions" />
        </div>
      </div>
    </header>
    <div class="page-shell__body">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import HelpBubble from '@/components/shared/HelpBubble.vue'

const props = withDefaults(
  defineProps<{
    title: string
    subtitle?: string
    help?: string
    width?: 'narrow' | 'content' | 'wide' | 'full'
  }>(),
  {
    subtitle: '',
    help: '',
    width: 'content',
  }
)

const maxWidth = computed(() => {
  switch (props.width) {
    case 'narrow':
      return 'var(--page-narrow)'
    case 'wide':
      return 'var(--page-wide)'
    case 'full':
      return 'none'
    default:
      return 'var(--page-content)'
  }
})
</script>

<style scoped>
.page-shell {
  width: 100%;
  margin-inline: auto;
  padding: var(--space-xl);
  box-sizing: border-box;
}

.page-shell__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-lg);
  margin-bottom: var(--space-xl);
}

.page-shell__title {
  margin: 0;
  font-size: var(--font-xl);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
  line-height: var(--leading-tight);
}

.page-shell__subtitle {
  margin: var(--space-xs) 0 0;
  font-size: var(--font-base);
  color: var(--text-muted);
}

.page-shell__meta {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  flex-shrink: 0;
}

.page-shell__actions {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

@media (--phone) {
  .page-shell {
    padding: var(--space-lg);
  }
  .page-shell__header {
    flex-direction: column;
  }
}
</style>
