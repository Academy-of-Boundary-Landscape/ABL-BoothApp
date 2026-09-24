import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/services/api'

/**
 * 收摊向导。**能不能进下一步由后端说了算**——`blockers` 非空就挡住。
 * 前端不自己判断，判据写在两处就会有一处先腐烂（②-2 把试算放后端是同一个理由）。
 *
 * 四步之间没有会话状态：每个 action 成功后重新拉一次 `fetchState`，不本地改 state。
 * 退出、换设备、重进，都从后端返回的真实状态继续（spec 1.6）。
 *
 * 冻结语义的例外：结算之后仍可补垫付、结算调整和收摊清点。所以这里没有
 * `require_event_open` 式的本地判断，后端放行就放行。
 */
export const useClosingStore = defineStore('closing', () => {
  const state = ref(null) // { status, pending_orders, onsite_remaining, stocktaken_at, blockers }
  const isLoading = ref(false)

  async function fetchState(eventId) {
    isLoading.value = true
    try {
      const response = await api.get(`/events/${eventId}/closing`)
      state.value = response.data
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '无法加载收摊状态。')
    } finally {
      isLoading.value = false
    }
  }

  async function stocktake(eventId, counts) {
    try {
      await api.post(`/events/${eventId}/closing/stocktake`, { counts })
      await fetchState(eventId)
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '提交盘点失败。')
    }
  }

  async function takeback(eventId) {
    try {
      await api.post(`/events/${eventId}/closing/takeback`)
      await fetchState(eventId)
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '确认带回失败。')
    }
  }

  async function settle(eventId) {
    try {
      await api.post(`/events/${eventId}/closing/settle`)
      await fetchState(eventId)
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '结束展会失败。')
    }
  }

  function resetStore() {
    state.value = null
  }

  return { state, isLoading, fetchState, stocktake, takeback, settle, resetStore }
})
