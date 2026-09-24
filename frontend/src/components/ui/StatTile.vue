<template>
  <div class="stat-tile">
    <div class="stat-tile__label">{{ label }}</div>
    <div class="stat-tile__value" :class="`stat-tile__value--${tone}`">
      <slot name="value">{{ value }}</slot>
    </div>
    <div v-if="hint || $slots.hint" class="stat-tile__hint">
      <slot name="hint">{{ hint }}</slot>
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string
    value?: string | number
    hint?: string
    tone?: 'default' | 'success' | 'warning' | 'error'
  }>(),
  {
    value: '',
    hint: '',
    tone: 'default',
  }
)
</script>

<style scoped>
.stat-tile {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  padding: var(--space-lg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--card-bg-color);
}

.stat-tile__label {
  font-size: var(--font-sm);
  color: var(--text-muted);
}

.stat-tile__value {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  line-height: var(--leading-tight);
  color: var(--primary-text-color);
}

.stat-tile__value--success {
  color: var(--success-color);
}

.stat-tile__value--warning {
  color: var(--warning-color);
}

.stat-tile__value--error {
  color: var(--error-color);
}

.stat-tile__hint {
  font-size: var(--font-sm);
  color: var(--text-muted);
}
</style>
