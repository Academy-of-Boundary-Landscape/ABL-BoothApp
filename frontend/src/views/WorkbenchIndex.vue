<template>
  <div class="workbench-index">
    <AsyncState :loading="loading" :error="error" loading-text="正在加载展会…">
      <span class="workbench-index__redirect">正在进入工作台…</span>
    </AsyncState>

    <!-- 展会不存在 / 已删除：AsyncState 已经渲染错误文案，这里补一个出口，不做重定向。 -->
    <div v-if="!loading && error" class="workbench-index__back">
      <n-button @click="goEvents">返回展会列表</n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { watch } from 'vue'
import { useRouter } from 'vue-router'
import { NButton } from 'naive-ui'
import { AsyncState } from '@/components/ui'
import { useWorkbenchEvent } from '@/composables/useWorkbenchEvent'
import type { Schemas } from '@/api/client'

const router = useRouter()
const { event, loading, error } = useWorkbenchEvent()

// 状态 → 落地子页（spec §3.1）。
const REDIRECT_BY_STATUS: Record<Schemas['EventStatus'], string> = {
  筹备: 'admin-event-products',
  进行中: 'admin-event-orders',
  已结算: 'admin-event-settlement',
}

watch(
  [loading, event],
  () => {
    if (loading.value || !event.value) return
    void router.replace({
      name: REDIRECT_BY_STATUS[event.value.status],
      params: { id: event.value.id },
    })
  },
  { immediate: true }
)

function goEvents() {
  void router.push({ name: 'admin-events' })
}
</script>

<style scoped>
.workbench-index__redirect {
  display: block;
  padding: var(--space-xl);
  color: var(--text-muted);
  font-size: var(--font-base);
}

.workbench-index__back {
  margin-top: var(--space-lg);
}
</style>
