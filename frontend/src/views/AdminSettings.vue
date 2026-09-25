<template>
  <PageShell title="设置" subtitle="网络连接、安全管理、外观与数据。" width="content">
    <n-space vertical size="large">
      <LanSettings />
      <SecuritySettings />
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
import AppearanceSettings from '@/components/settings/AppearanceSettings.vue'
import LegacyDataSettings from '@/components/settings/LegacyDataSettings.vue'
import AboutSettings from '@/components/settings/AboutSettings.vue'

const route = useRoute()

// `/admin/about` 会重定向到这里并带 hash `#about`：滚到「关于与更新」区块。
async function scrollToHash() {
  if (route.hash !== '#about') return
  await nextTick()
  const el = document.getElementById('about')
  if (el && typeof el.scrollIntoView === 'function') {
    el.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

onMounted(scrollToHash)
watch(() => route.hash, scrollToHash)
</script>
