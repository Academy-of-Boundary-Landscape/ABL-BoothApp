<!--
  摊主 · 库存（spec §3.6 / §5.6）：实时销售统计 +「登记赠送 / 报废」。
  现场仓余额变了就刷一次 `eventDetailStore`，LiveStats 的色块 / 进度条跟着更新。

  外壳已有页头，这里用 `embedded` 的 PageShell，只留内容区。
-->
<template>
  <PageShell embedded width="wide">
    <div class="inventory-toolbar">
      <p class="inventory-hint">现场仓余额实时更新；赠送 / 报废登记后库存立刻变化。</p>
      <n-button class="inventory-action" @click="showInventoryModal = true">
        登记赠送/报废
      </n-button>
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
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  flex-wrap: wrap;
  margin-bottom: var(--space-md);
}
.inventory-hint {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--font-sm);
  line-height: 1.6;
}
/* 手机上这是主操作，给足 44px 点按高度。 */
.inventory-action {
  min-height: 44px;
}

@media (--phone) {
  .inventory-toolbar {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
