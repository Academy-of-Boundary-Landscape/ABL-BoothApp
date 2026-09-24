import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, unwrap, type Schemas } from '@/api/client'

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
  const gifts = ref<Schemas['InventoryLogEntry'][]>([])
  const scraps = ref<Schemas['InventoryLogEntry'][]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  async function fetchGifts(eventId: number) {
    isLoading.value = true
    error.value = null
    try {
      const data = await unwrap(
        api.GET('/events/{event_id}/gifts', { params: { path: { event_id: eventId } } })
      )
      gifts.value = Array.isArray(data) ? data : []
    } catch (e) {
      error.value = '无法加载赠送记录。'
      console.error(e)
    } finally {
      isLoading.value = false
    }
  }

  async function fetchScraps(eventId: number) {
    isLoading.value = true
    error.value = null
    try {
      const data = await unwrap(
        api.GET('/events/{event_id}/scraps', { params: { path: { event_id: eventId } } })
      )
      scraps.value = Array.isArray(data) ? data : []
    } catch (e) {
      error.value = '无法加载报废记录。'
      console.error(e)
    } finally {
      isLoading.value = false
    }
  }

  async function logGift(
    eventId: number,
    payload: Schemas['InventoryLogRequest']
  ): Promise<Schemas['InventoryLogResponse']> {
    try {
      const entry = await unwrap(
        api.POST('/events/{event_id}/gifts', {
          params: { path: { event_id: eventId } },
          body: payload,
        })
      )
      await fetchGifts(eventId)
      return entry
    } catch (e) {
      console.error(e)
      // 原样抛给组件：组件用 errorMessage(e, …) 读后端原文。
      throw e
    }
  }

  async function logScrap(
    eventId: number,
    payload: Schemas['InventoryLogRequest']
  ): Promise<Schemas['InventoryLogResponse']> {
    try {
      const entry = await unwrap(
        api.POST('/events/{event_id}/scraps', {
          params: { path: { event_id: eventId } },
          body: payload,
        })
      )
      await fetchScraps(eventId)
      return entry
    } catch (e) {
      console.error(e)
      throw e
    }
  }

  /** 撤销一条登记。后端只接受赠送 / 报废两种 kind，错误原文照抛。 */
  async function reverse(eventId: number, journalId: number): Promise<void> {
    try {
      await unwrap(
        api.POST('/events/{event_id}/journals/{journal_id}/reverse', {
          params: { path: { event_id: eventId, journal_id: journalId } },
        })
      )
      await Promise.all([fetchGifts(eventId), fetchScraps(eventId)])
    } catch (e) {
      console.error(e)
      throw e
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
