import { ref, onMounted, onUnmounted } from 'vue'
import type { Ref } from 'vue'
import api from '@/services/api'

/** `useConnectionCheck` 的返回值。 */
export interface ConnectionCheck {
  /** 最近一次探测是否成功；初始为 true，首次探测前不误报断开。 */
  isConnected: Ref<boolean>
  /** 最近一次探测的时间戳（毫秒）；尚未探测过时为 null。 */
  lastCheckAt: Ref<number | null>
}

/**
 * 定期检查与后端的连接状态
 * @param intervalMs - 检查间隔（毫秒），默认 8 秒
 * @param timeoutMs - 单次请求超时（毫秒），默认 3 秒
 */
export function useConnectionCheck(intervalMs = 8000, timeoutMs = 3000): ConnectionCheck {
  const isConnected = ref(true)
  const lastCheckAt = ref<number | null>(null)
  let timer: ReturnType<typeof setInterval> | null = null
  let controller: AbortController | null = null

  async function check(): Promise<void> {
    try {
      const current = new AbortController()
      controller = current
      const timeout = setTimeout(() => current.abort(), timeoutMs)
      await api.get('/server-info', { signal: current.signal })
      clearTimeout(timeout)
      isConnected.value = true
    } catch {
      isConnected.value = false
    }
    lastCheckAt.value = Date.now()
  }

  function start(): void {
    check()
    timer = setInterval(check, intervalMs)
  }

  function stop(): void {
    clearInterval(timer ?? undefined)
    timer = null
    if (controller) controller.abort()
  }

  onMounted(start)
  onUnmounted(stop)

  return { isConnected, lastCheckAt }
}
