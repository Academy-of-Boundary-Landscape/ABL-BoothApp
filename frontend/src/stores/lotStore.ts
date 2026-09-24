import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, unwrap, type Schemas, errorMessage } from '@/api/client'

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
  const lots = ref<Schemas['LotResponse'][]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  async function fetchLots(eventId: string | number) {
    isLoading.value = true
    error.value = null
    try {
      const list = await unwrap(
        api.GET('/events/{event_id}/lots', { params: { path: { event_id: Number(eventId) } } })
      )
      lots.value = Array.isArray(list) ? list : []
    } catch (e) {
      error.value = '无法加载套装列表。'
      console.error(e)
    } finally {
      isLoading.value = false
    }
  }

  async function createLot(eventId: string | number, payload: Schemas['LotPayload']) {
    try {
      const created = await unwrap(
        api.POST('/events/{event_id}/lots', {
          params: { path: { event_id: Number(eventId) } },
          body: payload,
        })
      )
      lots.value.push(created)
      return created
    } catch (e) {
      console.error(e)
      throw new Error(errorMessage(e, '新建套装失败。'))
    }
  }

  async function updateLot(
    eventId: string | number,
    lotId: number,
    payload: Schemas['LotPayload']
  ) {
    try {
      const updated = await unwrap(
        api.PUT('/events/{event_id}/lots/{lot_id}', {
          params: { path: { event_id: Number(eventId), lot_id: lotId } },
          body: payload,
        })
      )
      const index = lots.value.findIndex((l) => l.id === lotId)
      if (index !== -1) lots.value[index] = updated
      return updated
    } catch (e) {
      console.error(e)
      throw new Error(errorMessage(e, '更新套装失败。'))
    }
  }

  async function deleteLot(eventId: string | number, lotId: number) {
    try {
      await unwrap(
        api.DELETE('/events/{event_id}/lots/{lot_id}', {
          params: { path: { event_id: Number(eventId), lot_id: lotId } },
        })
      )
      lots.value = lots.value.filter((l) => l.id !== lotId)
    } catch (e) {
      console.error(e)
      throw new Error(errorMessage(e, '删除套装失败。'))
    }
  }

  /**
   * 试算一份**还没保存**的配置：它会被顾客怎么用、你最多让多少、哪里可能配错了。
   *
   * 只读，不动 `lots`。后端走的是和创建完全相同的校验，所以这里的错误原文
   * 就是摊主按「新建」会看到的那一句——**原样抛出去，不要换成通用文案**，
   * 「试算说行、保存说不行」比没有试算更糟。
   */
  async function previewLot(eventId: string | number, payload: Schemas['LotPreviewRequest']) {
    try {
      return await unwrap(
        api.POST('/events/{event_id}/lots/preview', {
          params: { path: { event_id: Number(eventId) } },
          body: payload,
        })
      )
    } catch (e) {
      throw new Error(errorMessage(e, '试算失败。'))
    }
  }

  function resetStore() {
    lots.value = []
    error.value = null
  }

  return {
    lots,
    isLoading,
    error,
    fetchLots,
    createLot,
    updateLot,
    deleteLot,
    previewLot,
    resetStore,
  }
})
