<!--
  摊主 · 库存（spec §3.6）：实时销售统计 +「登记赠送 / 报废」。
  现场仓余额变了就刷一次 `eventDetailStore`，LiveStats 的色块 / 进度条跟着更新。
-->
<template>
  <PageShell embedded width="wide">
    <div class="inventory-toolbar">
      <n-button @click="showInventoryModal = true">登记赠送/报废</n-button>
    </div>

    <LiveStats :event-id="props.id" />

    <!-- 赠送/报废登记：商品候选复用收摊接口的现场仓余额，登记成功刷新库存统计 -->
    <InventoryLogModal
      :show="showInventoryModal"
      :event-id="props.id"
      @close="showInventoryModal = false"
      @logged="onInventoryLogged"
    />
  </PageShell>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NButton } from 'naive-ui'
import { PageShell } from '@/components/ui'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import LiveStats from '@/components/vendor/LiveStats.vue'
import InventoryLogModal from '@/components/vendor/InventoryLogModal.vue'

const props = defineProps<{ id: string | number }>()

const eventDetailStore = useEventDetailStore()
const showInventoryModal = ref(false)

async function onInventoryLogged() {
  await eventDetailStore.fetchProductsForEvent(Number(props.id))
}
</script>

<style scoped>
.inventory-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: var(--space-md);
}
</style>
