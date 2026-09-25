<template>
  <PageShell
    title="设置"
    subtitle="网络连接、安全管理、AI 视觉识别、外观与数据。"
    help="settings"
    width="content"
  >
    <n-space vertical size="large">
      <LanSettings />
      <SecuritySettings />
      <VisionSettings />
      <AppearanceSettings />
      <LegacyDataSettings />
      <AboutSettings />
    </n-space>
  </PageShell>
</template>

<script setup lang="ts">
import { nextTick, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { NSpace } from 'naive-ui'
import { PageShell } from '@/components/ui'
import LanSettings from '@/components/settings/LanSettings.vue'
import SecuritySettings from '@/components/settings/SecuritySettings.vue'
import VisionSettings from '@/components/settings/VisionSettings.vue'
import AppearanceSettings from '@/components/settings/AppearanceSettings.vue'
import LegacyDataSettings from '@/components/settings/LegacyDataSettings.vue'
import AboutSettings from '@/components/settings/AboutSettings.vue'

const route = useRoute()

// 进入设置页时按 hash 滚到对应区块（如 `/admin/about` → `#about`、
// 引导卡的 `/admin/settings#lan`）。没有对应元素就停在顶部。
async function scrollToHash() {
  const id = route.hash.replace(/^#/, '')
  if (!id) return
  await nextTick()
  const el = document.getElementById(id)
  if (el && typeof el.scrollIntoView === 'function') {
    el.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

onMounted(scrollToHash)
watch(() => route.hash, scrollToHash)
</script>
