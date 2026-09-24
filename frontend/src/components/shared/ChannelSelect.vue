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
    @update:value="onUpdate"
  />
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { NSelect } from 'naive-ui'
import api from '@/services/api'

const props = defineProps({ modelValue: { type: String, default: '' } })
const emit = defineEmits(['update:modelValue'])

// 模块级缓存：n-modal 隐藏时会卸载内容，每开一次收款弹窗就重挂载一次、
// 多拉一个 /channels。收摊前会堆一屋子这个请求，而列表几乎不变。不为它开 store，
// 一个模块作用域的 ref 就够——所有实例共用，第一次成功后就不再请求。
// 失效：新填的渠道在选中时并进缓存（见 onUpdate），下一次打开就能直接挑；
// 后端 /channels 是「预置三个 + 历史用过的」，提交成功后本来就会带上它。
// 拉失败**不写缓存**：一次网络抖动不该让摊主整个场次都看不到历史渠道，
// 否则他会把「微信支付」当新渠道再建一个账户，正是这个组件要防的分裂。
const channelCache = ref(null)
let inflight = null

function loadChannels() {
  if (channelCache.value) return Promise.resolve(channelCache.value)
  if (!inflight) {
    inflight = api
      .get('/channels')
      .then(({ data }) => {
        channelCache.value = Array.isArray(data) ? data : []
        return channelCache.value
      })
      .finally(() => {
        inflight = null
      })
  }
  return inflight
}

// 每个实例自己的兜底，不进缓存：请求失败时本次仍能收款
const fallback = ref(null)
const loading = ref(false)

// tag 模式下用户新填的值不在 options 里，补进去才不会显示成空白
const options = computed(() => {
  const all = [...(channelCache.value || fallback.value || [])]
  if (props.modelValue && !all.includes(props.modelValue)) all.push(props.modelValue)
  return all.map((c) => ({ label: c, value: c }))
})

/** 新填的渠道并进缓存，不然共享缓存会让下一个弹窗看不到它。 */
function onUpdate(value) {
  if (value && channelCache.value && !channelCache.value.includes(value)) {
    channelCache.value = [...channelCache.value, value]
  }
  emit('update:modelValue', value)
}

onMounted(async () => {
  loading.value = true
  try {
    await loadChannels()
  } catch {
    // 拿不到历史列表不该让摊主收不了款——退回三个预置的
    fallback.value = ['现金', '微信', '支付宝']
  } finally {
    loading.value = false
  }
})
</script>
