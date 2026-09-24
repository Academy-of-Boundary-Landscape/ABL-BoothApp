<!--
  可输入的渠道下拉。**这才是真正防「微信」和「微信支付」分裂成两个账户的那一条**
  （②-1/②-2 交接段第 3 条）——列表来自后端 `/channels`（预置三个 + 历史用过的），
  摊主优先从已有的里挑，确实要新渠道才现填。

  长度上限和规范化在后端（domain/channel.rs），这里不重复判，
  报错原样显示后端那句话。
-->
<template>
  <n-select
    :value="modelValue"
    :options="options"
    filterable
    tag
    placeholder="选择或输入收款渠道"
    :loading="loading"
    @update:value="(v) => emit('update:modelValue', v)"
  />
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { NSelect } from 'naive-ui'
import api from '@/services/api'

const props = defineProps({ modelValue: { type: String, default: '' } })
const emit = defineEmits(['update:modelValue'])

const known = ref([])
const loading = ref(false)

// tag 模式下用户新填的值不在 options 里，补进去才不会显示成空白
const options = computed(() => {
  const all = [...known.value]
  if (props.modelValue && !all.includes(props.modelValue)) all.push(props.modelValue)
  return all.map((c) => ({ label: c, value: c }))
})

onMounted(async () => {
  loading.value = true
  try {
    const { data } = await api.get('/channels')
    known.value = Array.isArray(data) ? data : []
  } catch {
    // 拿不到历史列表不该让摊主收不了款——退回三个预置的
    known.value = ['现金', '微信', '支付宝']
  } finally {
    loading.value = false
  }
})
</script>
