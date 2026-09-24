import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/services/api'

/**
 * 赠送 / 报废登记。
 *
 * 两件事在记账口径上都只是货从现场仓挪进一个虚拟仓，钱那一侧默认一个字不动；
 * 「这笔我自掏」是唯一的例外（后端 spec 偏离 2）。所以这里不做任何算术，
 * 只负责把后端的登记 / 撤销结果原样搬回来。
 *
 * 业务规则（能不能送、超没超余额、渠道规范化）全在后端，前端不重复判——
 * 两边各判一遍迟早不一致（`lotStore.js` 的注释写了这条）。
 */
export const useInventoryLogStore = defineStore('inventoryLog', () => {
  const gifts = ref([])
  const scraps = ref([])
  const isLoading = ref(false)
  const error = ref(null)

  async function fetchGifts(eventId) {
    isLoading.value = true
    error.value = null
    try {
      const response = await api.get(`/events/${eventId}/gifts`)
      gifts.value = Array.isArray(response.data) ? response.data : []
    } catch (err) {
      error.value = '无法加载赠送记录。'
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }

  async function fetchScraps(eventId) {
    isLoading.value = true
    error.value = null
    try {
      const response = await api.get(`/events/${eventId}/scraps`)
      scraps.value = Array.isArray(response.data) ? response.data : []
    } catch (err) {
      error.value = '无法加载报废记录。'
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }

  async function logGift(eventId, payload) {
    try {
      const response = await api.post(`/events/${eventId}/gifts`, payload)
      await fetchGifts(eventId)
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '登记赠送失败。')
    }
  }

  async function logScrap(eventId, payload) {
    try {
      const response = await api.post(`/events/${eventId}/scraps`, payload)
      await fetchScraps(eventId)
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '登记报废失败。')
    }
  }

  /** 撤销一条登记。后端只接受赠送 / 报废两种 kind，错误原文照抛。 */
  async function reverse(eventId, journalId) {
    try {
      await api.post(`/events/${eventId}/journals/${journalId}/reverse`)
      await Promise.all([fetchGifts(eventId), fetchScraps(eventId)])
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '撤销失败。')
    }
  }

  return {
    gifts,
    scraps,
    isLoading,
    error,
    fetchGifts,
    fetchScraps,
    logGift,
    logScrap,
    reverse,
  }
})
