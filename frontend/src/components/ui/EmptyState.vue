<template>
  <div class="empty-state" :class="{ 'empty-state--compact': compact }">
    <div v-if="!compact" class="empty-state__icon">{{ icon }}</div>
    <div class="empty-state__title">{{ title }}</div>
    <div v-if="desc || $slots.desc" class="empty-state__desc">
      <slot name="desc">{{ desc }}</slot>
    </div>
    <div v-if="hint || $slots.hint" class="empty-state__hint">
      <slot name="hint">{{ hint }}</slot>
    </div>
    <div v-if="$slots.action" class="empty-state__action">
      <slot name="action" />
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    icon?: string
    title: string
    desc?: string
    hint?: string
    compact?: boolean
  }>(),
  {
    icon: '📭',
    desc: '',
    hint: '',
    compact: false,
  }
)
</script>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--space-2xl) var(--space-xl);
  text-align: center;
}

.empty-state__icon {
  font-size: var(--font-2xl);
  line-height: var(--leading-tight);
  margin-bottom: var(--space-md);
  opacity: 0.3;
}

.empty-state__title {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
  margin-bottom: var(--space-sm);
}

.empty-state__desc {
  font-size: var(--font-base);
  color: var(--text-muted);
  line-height: var(--leading-base);
  margin-bottom: var(--space-lg);
  max-width: var(--page-narrow);
}

.empty-state__hint {
  font-size: var(--font-sm);
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}

.empty-state__action {
  margin-top: var(--space-lg);
}

.empty-state--compact {
  flex-direction: row;
  flex-wrap: wrap;
  justify-content: center;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-md);
  color: var(--text-muted);
}

.empty-state--compact .empty-state__title {
  font-size: var(--font-sm);
  font-weight: var(--weight-regular);
  color: var(--text-muted);
  margin: 0;
}

.empty-state--compact .empty-state__desc {
  font-size: var(--font-sm);
  margin: 0;
}

.empty-state--compact .empty-state__action {
  margin: 0;
}

@media (--phone) {
  .empty-state {
    padding: var(--space-xl) var(--space-lg);
  }
}
</style>
