import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/services/api'

/**
 * 管理端结算：垫付、结算调整、收摊清点，以及只读的结算单。
 *
 * 金额一律是**分**：后端收分、返回分，页面上的「元」只在输入框和显示处换算
 * （`@/utils/money`）。这里不做任何换算，免得两边各转一次。
 *
 * 业务规则全部在后端（垫付与调整在冻结之后仍然允许增删，是 spec 偏离 3 的
 * 有意例外；清点要收全量、同一个渠道只能报一次）。前端不重复判断，把
 * `err.response?.data?.error` 原样抛给页面显示——判据写在两处就会有一处先腐烂。
 *
 * 每次写成功都重新拉一遍：垫付/调整会改账本，清点会写对账差异，本地改一条
 * 列表会和结算单漂移。
 */
export const useSettlementStore = defineStore('settlement', () => {
  const report = ref(null) // SettlementReport
  const advances = ref([])
  const adjustments = ref([])
  const isLoading = ref(false)
  const error = ref(null)

  /** 后端错误原文优先，不要换成通用文案。 */
  function backendMessage(err, fallback) {
    return err.response?.data?.error || fallback
  }

  async function fetchReport(eventId) {
    isLoading.value = true
    try {
      const response = await api.get(`/events/${eventId}/settlement`)
      report.value = response.data
    } catch (err) {
      error.value = backendMessage(err, '无法加载结算单。')
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }

  async function fetchAdvances(eventId) {
    try {
      const response = await api.get(`/events/${eventId}/advances`)
      advances.value = Array.isArray(response.data) ? response.data : []
    } catch (err) {
      error.value = backendMessage(err, '无法加载垫付列表。')
      console.error(err)
    }
  }

  async function fetchAdjustments(eventId) {
    try {
      const response = await api.get(`/events/${eventId}/adjustments`)
      adjustments.value = Array.isArray(response.data) ? response.data : []
    } catch (err) {
      error.value = backendMessage(err, '无法加载结算调整列表。')
      console.error(err)
    }
  }

  async function refresh(eventId) {
    // 清 error 只在这里做一次：三个 fetch 并发跑，如果放在 fetchReport 开头
    // （或任一 fetch 里），先落地的那条 500 会被后启动的 fetch 静默抹掉，
    // 页面上什么都不显示、对应的表还空着。
    error.value = null
    await Promise.all([fetchReport(eventId), fetchAdvances(eventId), fetchAdjustments(eventId)])
  }

  async function createAdvance(eventId, payload) {
    try {
      const response = await api.post(`/events/${eventId}/advances`, payload)
      await refresh(eventId)
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(backendMessage(err, '新增垫付失败。'))
    }
  }

  async function deleteAdvance(eventId, id) {
    try {
      await api.delete(`/events/${eventId}/advances/${id}`)
      await refresh(eventId)
    } catch (err) {
      console.error(err)
      throw new Error(backendMessage(err, '删除垫付失败。'))
    }
  }

  async function createAdjustment(eventId, payload) {
    try {
      const response = await api.post(`/events/${eventId}/adjustments`, payload)
      await refresh(eventId)
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(backendMessage(err, '新增结算调整失败。'))
    }
  }

  async function deleteAdjustment(eventId, id) {
    try {
      await api.delete(`/events/${eventId}/adjustments/${id}`)
      await refresh(eventId)
    } catch (err) {
      console.error(err)
      throw new Error(backendMessage(err, '删除结算调整失败。'))
    }
  }

  /** `counts` 已经是 `[{ channel, actual }]`（分），清点必须收本场全量渠道。 */
  async function reconcile(eventId, counts) {
    try {
      const response = await api.post(`/events/${eventId}/settlement/reconcile`, { counts })
      await fetchReport(eventId)
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(backendMessage(err, '提交清点失败。'))
    }
  }

  function resetStore() {
    report.value = null
    advances.value = []
    adjustments.value = []
    isLoading.value = false
    error.value = null
  }

  return {
    report,
    advances,
    adjustments,
    isLoading,
    error,
    fetchReport,
    fetchAdvances,
    fetchAdjustments,
    refresh,
    createAdvance,
    deleteAdvance,
    createAdjustment,
    deleteAdjustment,
    reconcile,
    resetStore,
  }
})
