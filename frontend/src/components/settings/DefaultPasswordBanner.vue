<!--
  出厂默认密码提醒。默认密码写在公开的使用教程里，而局域网（尤其是场馆 WiFi）上的
  任何设备都能打开登录页——所以常驻提醒，但不拦登录（现场借设备登录是正常用法）。
  关掉只在本次会话内有效，下次打开还会提醒，直到改掉为止。
-->
<template>
  <n-alert
    v-if="items.length && !dismissed"
    type="warning"
    closable
    title="还在用出厂默认密码"
    class="default-password-banner"
    @close="dismiss"
  >
    <p class="banner-line">
      {{ items.join('、') }}还是出厂默认值。它们写在公开的使用教程里，同一局域网（比如场馆
      WiFi）里的任何设备都能用它登录。
    </p>
    <n-button v-if="!onSettings" size="small" type="warning" secondary @click="goFix">
      去修改
    </n-button>
  </n-alert>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { NAlert, NButton } from 'naive-ui'
import { useDefaultPasswords } from '@/composables/useDefaultPasswords'

const DISMISS_KEY = 'default-password-banner-dismissed'

const route = useRoute()
const router = useRouter()
const { state, refresh } = useDefaultPasswords()

const items = computed(() => {
  const s = state.value
  if (!s) return []
  const out: string[] = []
  if (s.admin) out.push('管理员密码（admin123）')
  if (s.vendor) out.push('摊主密码（vendor123）')
  return out
})

const onSettings = computed(() => route.name === 'admin-settings')

function readDismissed() {
  try {
    return sessionStorage.getItem(DISMISS_KEY) === '1'
  } catch {
    return false
  }
}
const dismissed = ref(readDismissed())

function dismiss() {
  dismissed.value = true
  try {
    sessionStorage.setItem(DISMISS_KEY, '1')
  } catch {
    /* 关不住也只是下次还提醒 */
  }
}

function goFix() {
  router.push({ name: 'admin-settings', hash: '#security' })
}

onMounted(refresh)
</script>

<style scoped>
.default-password-banner {
  margin-bottom: var(--space-lg);
}
.banner-line {
  margin: 0 0 var(--space-sm);
  line-height: 1.6;
}
</style>
