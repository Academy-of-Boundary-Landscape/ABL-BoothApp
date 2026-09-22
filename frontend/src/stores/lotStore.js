import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/services/api'

/**
 * 套装（Lot）= 候选商品集合 + 要选几件 + 总价。
 *
 * 金额一律是**分**：后端收分、返回分，页面上的「元」只在输入框和显示处换算
 * （`@/utils/money`）。这里不做任何换算，免得两边各转一次。
 *
 * 候选集必须同一货主是后端的硬校验（spec 4.1），前端只负责把错误原文显示出来——
 * 不在前端重复这条规则，否则两边的判断迟早不一致。
 */
export const useLotStore = defineStore('lot', () => {
  const lots = ref([])
  const isLoading = ref(false)
  const error = ref(null)

  async function fetchLots(eventId) {
    isLoading.value = true
    error.value = null
    try {
      const response = await api.get(`/events/${eventId}/lots`)
      lots.value = Array.isArray(response.data) ? response.data : []
    } catch (err) {
      error.value = '无法加载套装列表。'
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }

  async function createLot(eventId, payload) {
    try {
      const response = await api.post(`/events/${eventId}/lots`, payload)
      lots.value.push(response.data)
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '新建套装失败。')
    }
  }

  async function updateLot(eventId, lotId, payload) {
    try {
      const response = await api.put(`/events/${eventId}/lots/${lotId}`, payload)
      const index = lots.value.findIndex((l) => l.id === lotId)
      if (index !== -1) lots.value[index] = response.data
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '更新套装失败。')
    }
  }

  async function deleteLot(eventId, lotId) {
    try {
      await api.delete(`/events/${eventId}/lots/${lotId}`)
      lots.value = lots.value.filter((l) => l.id !== lotId)
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '删除套装失败。')
    }
  }

  function resetStore() {
    lots.value = []
    error.value = null
  }

  return { lots, isLoading, error, fetchLots, createLot, updateLot, deleteLot, resetStore }
})
