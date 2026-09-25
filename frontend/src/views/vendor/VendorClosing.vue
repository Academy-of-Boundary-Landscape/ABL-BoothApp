<!--
  摊主 · 收摊（spec §3.6 / §5.6）。
  未结算 → `ClosingWizard`（手机优先页面，步骤由后端状态推出，不存本地 step）；
  已结算（或向导里刚 `settled`）→ 原地换成只读结算单 `SettlementReportView`，
  不再停在「账本已冻结」屏。
-->
<template>
  <PageShell embedded width="wide">
    <ClosingWizard v-if="!isSettled" :event-id="props.id" @settled="onSettled" />
    <SettlementReportView v-else :event-id="eventId" />
  </PageShell>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { PageShell } from '@/components/ui'
import { useEventStore } from '@/stores/eventStore'
import ClosingWizard from '@/components/vendor/ClosingWizard.vue'
import SettlementReportView from '@/components/settlement/SettlementReportView.vue'

const props = defineProps<{ id: string | number }>()

const eventId = computed(() => Number(props.id))
const eventStore = useEventStore()

// 向导里刚结算完不等重新拉展会列表：本地先翻成「已结算」，立刻渲染结算单。
const settledLocally = ref(false)

const isSettled = computed(() => {
  if (settledLocally.value) return true
  const event = eventStore.events.find((e) => e.id === eventId.value)
  return event?.status === '已结算'
})

function onSettled() {
  settledLocally.value = true
  const event = eventStore.events.find((e) => e.id === eventId.value)
  if (event) event.status = '已结算'
}
</script>
