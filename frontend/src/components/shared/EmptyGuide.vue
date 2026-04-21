<template>
  <div class="empty-guide">
    <div class="empty-guide-icon">{{ icon }}</div>
    <div class="empty-guide-title">{{ title }}</div>
    <div v-if="desc || $slots.desc" class="empty-guide-desc">
      <slot name="desc">{{ desc }}</slot>
    </div>
    <div v-if="hint || $slots.hint" class="empty-guide-hint">
      <slot name="hint">{{ hint }}</slot>
    </div>
  </div>
</template>

<script setup>
defineProps({
  icon: { type: String, default: '📭' },
  title: { type: String, required: true },
  desc: { type: String, default: '' },
  hint: { type: String, default: '' },
})
</script>

<style scoped>
.empty-guide {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 3rem 2rem;
  text-align: center;
}
.empty-guide-icon {
  font-size: 3rem;
  margin-bottom: 12px;
  opacity: 0.3;
  line-height: 1;
}
.empty-guide-title {
  font-size: var(--font-lg);
  font-weight: 700;
  color: var(--primary-text-color);
  margin-bottom: 8px;
}
.empty-guide-desc {
  font-size: var(--font-base);
  color: var(--text-muted);
  max-width: 440px;
  line-height: 1.6;
  margin-bottom: 16px;
}
.empty-guide-hint {
  font-size: var(--font-sm);
  color: var(--accent-color);
  font-weight: 600;
}

@media (max-width: 480px) {
  .empty-guide { padding: 2rem 1rem; }
  .empty-guide-icon { font-size: 2.5rem; }
}
</style>
