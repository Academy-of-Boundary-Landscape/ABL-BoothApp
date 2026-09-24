<template>
  <n-card class="section-card" :bordered="true">
    <template v-if="title || $slots.extra || collapsible" #header>
      <div
        class="section-card__header"
        :class="{ 'section-card__header--collapsible': collapsible }"
        :role="collapsible ? 'button' : undefined"
        :aria-expanded="collapsible ? !isCollapsed : undefined"
        @click="toggle"
      >
        <span class="section-card__title">{{ title }}</span>
        <span v-if="$slots.extra" class="section-card__extra" @click.stop>
          <slot name="extra" />
        </span>
        <span
          v-if="collapsible"
          class="section-card__arrow"
          :class="{ 'section-card__arrow--collapsed': isCollapsed }"
          >▾</span
        >
      </div>
    </template>

    <Transition name="expand">
      <div v-show="!isCollapsed" class="section-card__body">
        <slot />
      </div>
    </Transition>

    <template v-if="$slots.footer" #footer>
      <div class="section-card__footer">
        <slot name="footer" />
      </div>
    </template>
  </n-card>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { NCard } from 'naive-ui'

const props = withDefaults(
  defineProps<{
    title?: string
    collapsible?: boolean
    collapsed?: boolean
  }>(),
  {
    title: '',
    collapsible: false,
    collapsed: false,
  }
)

const emit = defineEmits<{
  (e: 'update:collapsed', v: boolean): void
}>()

const isCollapsed = ref(props.collapsed)

watch(
  () => props.collapsed,
  (v) => {
    isCollapsed.value = v
  }
)

function toggle() {
  if (!props.collapsible) return
  isCollapsed.value = !isCollapsed.value
  emit('update:collapsed', isCollapsed.value)
}

defineExpose({ isCollapsed, toggle })
</script>

<style scoped>
.section-card {
  border-radius: var(--radius-lg);
}

.section-card__header {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.section-card__header--collapsible {
  cursor: pointer;
  user-select: none;
}

.section-card__title {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
}

.section-card__extra {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  margin-left: auto;
}

.section-card__arrow {
  margin-left: auto;
  transition: transform 0.2s ease;
}

.section-card__arrow--collapsed {
  transform: rotate(-90deg);
}

.section-card__footer {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: var(--space-sm);
}
</style>
