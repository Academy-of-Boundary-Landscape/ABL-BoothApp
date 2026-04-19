import { ref, onMounted, onUnmounted } from 'vue'
import api from '@/services/api'

/**
 * 定期检查与后端的连接状态
 * @param {number} intervalMs - 检查间隔（毫秒），默认 8 秒
 * @param {number} timeoutMs - 单次请求超时（毫秒），默认 3 秒
 */
export function useConnectionCheck(intervalMs = 8000, timeoutMs = 3000) {
  const isConnected = ref(true)
  const lastCheckAt = ref(null)
  let timer = null
  let controller = null

  async function check() {
    try {
      controller = new AbortController()
      const timeout = setTimeout(() => controller.abort(), timeoutMs)
      await api.get('/server-info', { signal: controller.signal })
      clearTimeout(timeout)
      isConnected.value = true
    } catch {
      isConnected.value = false
    }
    lastCheckAt.value = Date.now()
  }

  function start() {
    check()
    timer = setInterval(check, intervalMs)
  }

  function stop() {
    clearInterval(timer)
    timer = null
    if (controller) controller.abort()
  }

  onMounted(start)
  onUnmounted(stop)

  return { isConnected, lastCheckAt }
}
