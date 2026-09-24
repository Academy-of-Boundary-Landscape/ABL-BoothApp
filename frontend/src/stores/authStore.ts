import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api, unwrap, errorMessage, type Schemas } from '@/api/client'
// 【重要】确保导入了 router 实例
import router from '@/router'

/** 登录成功后前端保存的用户状态（后端 LoginResponse 的子集）。 */
interface AuthUser {
  role: string
  access: string
  authorizedEventId?: number | null
}

export const useAuthStore = defineStore('auth', () => {
  const user = ref<AuthUser | null>(JSON.parse(sessionStorage.getItem('user') ?? 'null') || null)

  const isAdmin = computed(() => user.value?.role === 'admin')

  const canAccessVendorPage = (eventId?: string | number | null): boolean => {
    // 确保 eventId 是数字类型以便比较
    const numericEventId = eventId ? parseInt(String(eventId), 10) : null

    if (!user.value || user.value.role !== 'vendor') return false

    // 管理员密码或全局密码登录时，可以访问所有展会
    if (user.value.access === 'all') return true

    // 展会专属密码登录时，检查 ID 是否匹配
    return user.value.authorizedEventId === numericEventId
  }

  async function login(
    password: string,
    role: string,
    eventId?: string | number | null,
    redirectPath?: string
  ): Promise<boolean> {
    try {
      // 1. 【核心改动】将 eventId 包含在发送给后端的数据中
      // 【修复】确保不发送 undefined，而是发送 null 或不发送该字段
      const payload: Schemas['LoginRequest'] = { password, role }
      if (eventId !== undefined && eventId !== null && eventId !== '') {
        // 后端 `event_id` 的自定义反序列化同时接受字符串与数字，但生成的契约只声明 number；
        // 这里把路由 query 来的字符串统一转成数字，具体见 .3b/REPORT.md。
        payload.eventId = typeof eventId === 'string' ? Number(eventId) : eventId
      }

      // 调试日志（不打印密码）
      console.log('[authStore] Login attempt:', { role: payload.role, eventId: payload.eventId })
      // 发送原始对象，让 client 统一序列化为 JSON
      // 显式给出响应类型：`unwrap` 从 `FetchResponse` 联合里推断时会把
      // 错误分支的 `data?: never` 一起并进来，导致返回值被推断成 `T | undefined`。
      const responseData = await unwrap(api.POST('/auth/login', { body: payload }))

      // 2. 根据后端返回的数据，构建并更新前端的用户状态对象
      const userData: AuthUser = {
        role: responseData.role,
        access: responseData.access || 'all',
      }

      // 2.1 保存后端返回的访问令牌到 sessionStorage，供 api client 使用
      if (responseData.token) {
        sessionStorage.setItem('access_token', responseData.token)
      }

      if (responseData.role === 'vendor' && responseData.access === 'event') {
        userData.authorizedEventId = responseData.eventId ? responseData.eventId : null
      }
      if (responseData.role === 'vendor' && responseData.access === 'all') {
        userData.authorizedEventId = null
      }

      user.value = userData
      sessionStorage.setItem('user', JSON.stringify(userData))

      // 标记此设备上曾有管理员成功登录过 —— 用于首页判断是否"全新未配置"
      if (userData.role === 'admin') {
        localStorage.setItem('admin_first_login_done', 'true')
      }

      // 3. 执行跳转
      const finalRedirectPath = redirectPath || (userData.role === 'admin' ? '/admin' : '/')
      await router.push(finalRedirectPath)

      return true
    } catch (error) {
      console.error('Login failed:', error)
      user.value = null
      sessionStorage.removeItem('user')
      throw new Error(errorMessage(error, '登录失败，请检查密码。'))
    }
  }
  async function logout(): Promise<void> {
    try {
      await unwrap(api.POST('/auth/logout'))
    } catch (err) {
      console.warn('Logout request failed, clearing client state anyway', err)
    }
    user.value = null
    sessionStorage.removeItem('user')
    sessionStorage.removeItem('access_token')
    router.push('/')
  }

  // 确保返回了所有需要的方法和状态
  return { user, isAdmin, login, logout, canAccessVendorPage }
})
