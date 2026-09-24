<template>
  <span class="money" :class="[size, { strike }]">{{ text }}</span>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { formatYuan, type Cents } from '@/utils/money'

const props = withDefaults(
  defineProps<{
    value: Cents | null | undefined
    signed?: boolean
    strike?: boolean
    size?: 'sm' | 'md' | 'lg'
  }>(),
  {
    signed: false,
    strike: false,
    size: 'md',
  }
)

const text = computed(() => {
  const base = formatYuan(props.value)
  if (props.signed && typeof props.value === 'number' && props.value > 0) {
    return `+${base}`
  }
  return base
})
</script>

<style scoped>
.money {
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.money.sm {
  font-size: var(--font-sm);
}

.money.md {
  font-size: var(--font-base);
}

.money.lg {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
}

.strike {
  text-decoration: line-through;
}
</style>
