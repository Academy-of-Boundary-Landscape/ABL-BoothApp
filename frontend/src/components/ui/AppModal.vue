<template>
  <n-modal :show="show" :mask-closable="maskClosable" @update:show="onUpdateShow">
    <n-card
      class="app-modal__card"
      :class="[`app-modal__card--${size}`, { 'app-modal__card--phone': isPhone }]"
      :style="cardStyle"
      :bordered="false"
      size="medium"
    >
      <template #header>
        <div class="app-modal__header">
          <span class="app-modal__title">
            <slot name="header">{{ title }}</slot>
          </span>
          <n-button quaternary circle size="small" class="app-modal__close" @click="close">
            ×
          </n-button>
        </div>
      </template>

      <div class="app-modal__body">
        <slot />
      </div>

      <template v-if="$slots.footer" #footer>
        <div class="app-modal__footer">
          <slot name="footer" />
        </div>
      </template>
    </n-card>
  </n-modal>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton, NCard, NModal } from 'naive-ui'
import { useViewport } from '@/composables/useViewport'

const props = withDefaults(
  defineProps<{
    show: boolean
    title?: string
    size?: 'sm' | 'md' | 'lg'
    maskClosable?: boolean
  }>(),
  {
    title: '',
    size: 'md',
    maskClosable: true,
  }
)

const emit = defineEmits<{
  (e: 'update:show', v: boolean): void
}>()

const sizeWidth = {
  sm: '480px',
  md: '640px',
  lg: '960px',
} as const

const { isPhone } = useViewport()

const cardStyle = computed(() => {
  if (isPhone.value) {
    return {
      width: '100vw',
      maxWidth: '100vw',
      height: '100dvh',
      borderRadius: '0',
    }
  }
  return {
    width: sizeWidth[props.size],
    maxWidth: '92vw',
  }
})

function close() {
  emit('update:show', false)
}

function onUpdateShow(val: boolean) {
  if (!val) emit('update:show', false)
}
</script>

<style scoped>
.app-modal__card {
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
}

.app-modal__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-md);
  border-bottom: 1px solid var(--border-color);
}

.app-modal__title {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
}

.app-modal__body {
  padding: var(--space-md) 0;
}

.app-modal__footer {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: var(--space-sm);
  border-top: 1px solid var(--border-color);
}

.app-modal__card--phone {
  display: flex;
  flex-direction: column;
}

.app-modal__card--phone :deep(.n-card__content) {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

@media (--phone) {
  .app-modal__body {
    padding: var(--space-sm) 0;
  }
}
</style>
