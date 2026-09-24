<template>
  <n-select
    :value="modelValue"
    :options="options"
    :loading="loading"
    :placeholder="placeholder"
    @update:value="(v: number) => emit('update:modelValue', v)"
  />
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { NSelect } from 'naive-ui'
import { useSocietyStore } from '@/stores/societyStore'
import { societyOptions } from '@/utils/societyOptions'

/**
 * 选择商品的所属社团（货主）。决定这件货卖出的钱归谁、结算单上记在谁名下——
 * 代卖社团的货必须选对，否则结算单会把钱全算成本社团的。
 */
withDefaults(defineProps<{ modelValue: number | null | undefined; placeholder?: string }>(), {
  placeholder: '选择所属社团',
})
const emit = defineEmits<{ (e: 'update:modelValue', v: number): void }>()

const store = useSocietyStore()
const loading = ref(false)
const options = computed(() => societyOptions(store.societies))

onMounted(async () => {
  if (store.societies.length > 0) return
  loading.value = true
  try {
    await store.fetchSocieties()
  } finally {
    loading.value = false
  }
})
</script>
