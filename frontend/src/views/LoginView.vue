<template>
  <div class="login-container">
    <n-card class="login-box" :bordered="false">
      <h2>{{ title }}</h2>
      <p>{{ subtitle }}</p>
      <n-alert
        v-if="props.role === 'admin' && isDefaultAdmin"
        type="warning"
        :bordered="false"
        style="margin-bottom: var(--space-lg)"
      >
        默认管理员密码为 admin123，请登录后尽快修改。
      </n-alert>

      <form @submit.prevent="handleLogin">
        <!-- 添加 :disabled 属性，防止请求期间修改密码 -->
        <n-input
          v-model:value="password"
          type="password"
          show-password-on="click"
          placeholder="请输入密码"
          size="large"
          :disabled="loading"
        />

        <div style="margin-top: var(--space-lg)">
          <!-- 添加 :loading 和 :disabled 属性 -->
          <n-button type="primary" attr-type="submit" block :loading="loading" :disabled="loading">
            <!-- 登录中显示的文字（可选，NaiveUI loading时通常只显示圈圈，这里保持原样即可） -->
            {{ loading ? '登录中...' : '进入' }}
          </n-button>
        </div>

        <div v-if="error" style="margin-top: var(--space-lg)">
          <n-alert type="error" :bordered="false">{{ error }}</n-alert>
        </div>
      </form>
    </n-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/authStore'
import { NCard, NInput, NButton, NAlert } from 'naive-ui'
import { api, unwrap } from '@/api/client'

const props = defineProps<{ role: string }>()

const store = useAuthStore()
const route = useRoute()

const password = ref('')
const error = ref('')
const isDefaultAdmin = ref(false)
const loading = ref(false) // 1. 新增 loading 状态

const title = computed(() => (props.role === 'admin' ? '管理员后台' : '摊主页面'))
const subtitle = computed(() => `请输入${title.value}密码以继续`)

async function handleLogin() {
  // 防止重复点击
  if (loading.value) return

  error.value = ''
  loading.value = true // 2. 开始加载

  // [调试] 打印登录信息
  console.log('[前端 DEBUG] Login Attempt:', {
    Role: props.role,
    EventId: route.query.eventId,
  })

  try {
    // query 的类型允许 string[]；正常路由下是单值，收窄成 store.login 接受的类型。
    const eventId = Array.isArray(route.query.eventId)
      ? route.query.eventId[0]
      : route.query.eventId
    const redirectQuery = route.query.redirect
    const redirectPath =
      (typeof redirectQuery === 'string' && redirectQuery) ||
      (props.role === 'admin' ? '/admin' : '/')

    // 执行登录逻辑
    await store.login(password.value, props.role, eventId, redirectPath)
  } catch (err) {
    console.error('[前端 DEBUG] Login Error:', err)
    error.value = (err instanceof Error && err.message) || '登录失败，请检查网络或密码'
  } finally {
    // 3. 无论成功失败，结束加载状态 (如果跳转很快，用户可能看不到这里，但为了稳健性必须加)
    loading.value = false
  }
}

onMounted(async () => {
  if (props.role !== 'admin') return
  try {
    const data = await unwrap(api.GET('/auth/is-default-admin-password'))
    isDefaultAdmin.value = !!data?.is_default
  } catch {
    isDefaultAdmin.value = false
  }
})
</script>

<style scoped>
/* 登录页面的居中样式 */
.login-container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100vh;
}
.login-box {
  width: 420px;
  max-width: 90vw;
  padding: var(--space-2xl);
  background-color: var(--card-bg-color);
  border-radius: var(--radius-md);
  text-align: center;
}
</style>
