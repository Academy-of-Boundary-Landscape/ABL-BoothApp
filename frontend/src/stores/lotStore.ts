import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, unwrap, type Schemas } from '@/api/client'

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
      const list = await unwrap<Schemas['LotResponse'][]>(
        // @ts-expect-error openapi-fetch 的 Readable<> 会把 branded Cents 展平成对象，与 schema 的 Cents 不兼容（第三方类型缺陷）
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
      const created = await unwrap<Schemas['LotResponse']>(
        // @ts-expect-error openapi-fetch 的 Readable<> 会把 branded Cents 展平成对象，与 schema 的 Cents 不兼容（第三方类型缺陷）
        api.POST('/events/{event_id}/lots', {
          params: { path: { event_id: Number(eventId) } },
          body: payload,
        })
      )
      lots.value.push(created)
      return created
    } catch (e) {
      console.error(e)
      throw e
    }
  }

  async function updateLot(
    eventId: string | number,
    lotId: number,
    payload: Schemas['LotPayload']
  ) {
    try {
      const updated = await unwrap<Schemas['LotResponse']>(
        // @ts-expect-error openapi-fetch 的 Readable<> 会把 branded Cents 展平成对象，与 schema 的 Cents 不兼容（第三方类型缺陷）
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
      throw e
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
      throw e
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
    return await unwrap<Schemas['LotPreviewResponse']>(
      // @ts-expect-error openapi-fetch 的 Readable<> 会把 branded Cents 展平成对象，与 schema 的 Cents 不兼容（第三方类型缺陷）
      api.POST('/events/{event_id}/lots/preview', {
        params: { path: { event_id: Number(eventId) } },
        body: payload,
      })
    )
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
