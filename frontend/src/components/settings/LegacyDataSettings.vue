<template>
  <SectionCard
    v-if="legacyHasBackup"
    title="历史数据（v1）"
    collapsible
    v-model:collapsed="collapsed"
  >
    <p class="legacy-desc">
      新模型没有迁移旧版的展会和订单，旧数据完整保留在
      <code>sale_system.db.v1-backup</code>，可以随时导出为 Excel。
    </p>
    <n-space align="center" :wrap="true">
      <n-button type="primary" :loading="exporting" @click="handleExport">
        {{ exporting ? '导出中…' : '导出旧数据为 Excel' }}
      </n-button>
      <span class="hint">
        备份中含 {{ status?.event_count }} 个展会 / {{ status?.order_count }} 张订单
      </span>
    </n-space>
  </SectionCard>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { NButton, NSpace } from 'naive-ui'
import { SectionCard } from '@/components/ui'
import { api, unwrap, type Schemas } from '@/api/client'
import { useFeedback } from '@/composables/useFeedback'
import { exportLegacyXlsx } from '@/utils/legacyExport'

const fb = useFeedback()
const collapsed = ref(false)
const exporting = ref(false)
const status = ref<Schemas['LegacyStatus'] | null>(null)

const legacyHasBackup = computed(() => status.value?.has_backup === true)

// 首启弹窗（MigrationNotice）只弹一次且立刻标记已读；若它不常驻，用户点完
// 「知道了」这台设备就再也导不出 v1 数据。这里是有备份时的常驻出口。
async function loadStatus() {
  try {
    status.value = await unwrap(api.GET('/legacy/status'))
  } catch (e) {
    // 读不到状态只是不显示这个 section，绝不能影响设置页主流程
    console.warn('[LegacyDataSettings] 读取历史数据状态失败', e)
  }
}

async function handleExport() {
  exporting.value = true
  try {
    const ok = await exportLegacyXlsx()
    if (ok) fb.success('导出成功')
  } catch (e) {
    console.error('下载旧数据失败:', e)
    fb.error(e, '下载失败')
  } finally {
    exporting.value = false
  }
}

onMounted(loadStatus)
</script>

<style scoped>
.legacy-desc {
  margin: 0 0 var(--space-lg);
  font-size: var(--font-base);
  line-height: 1.7;
  color: var(--secondary-text-color);
}
.legacy-desc code {
  background: var(--bg-elevated);
  padding: var(--space-xs);
  border-radius: var(--radius-sm);
}
.hint {
  font-size: var(--font-sm);
  color: var(--text-muted);
}
</style>
