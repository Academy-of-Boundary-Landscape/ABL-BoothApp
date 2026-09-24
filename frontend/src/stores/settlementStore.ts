import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, errorMessage, unwrap, type Schemas } from '@/api/client'

/**
 * 管理端结算：垫付、结算调整、收摊清点，以及只读的结算单。
 *
 * 金额一律是**分**：后端收分、返回分，页面上的「元」只在输入框和显示处换算
 * （`@/utils/money`）。这里不做任何换算，免得两边各转一次。
 *
 * 业务规则全部在后端（垫付与调整在冻结之后仍然允许增删，是 spec 偏离 3 的
 * 有意例外；清点要收全量、同一个渠道只能报一次）。前端不重复判断，把
 * 后端的错误原文（`errorMessage(e, …)`）原样抛给页面显示——判据写在两处就会有一处先腐烂。
 *
 * 每次写成功都重新拉一遍：垫付/调整会改账本，清点会写对账差异，本地改一条
 * 列表会和结算单漂移。
 */
export const useSettlementStore = defineStore('settlement', () => {
  const report = ref<Schemas['SettlementReport'] | null>(null)
  const advances = ref<Schemas['LedgerEntryRow'][]>([])
  const adjustments = ref<Schemas['LedgerEntryRow'][]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  async function fetchReport(eventId: number) {
    isLoading.value = true
    try {
      report.value = await unwrap(
        api.GET('/events/{event_id}/settlement', { params: { path: { event_id: eventId } } })
      )
    } catch (e) {
      error.value = errorMessage(e, '无法加载结算单。')
      console.error(e)
    } finally {
      isLoading.value = false
    }
  }

  async function fetchAdvances(eventId: number) {
    try {
      const data = await unwrap(
        api.GET('/events/{event_id}/advances', { params: { path: { event_id: eventId } } })
      )
      advances.value = Array.isArray(data) ? data : []
    } catch (e) {
      error.value = errorMessage(e, '无法加载垫付列表。')
      console.error(e)
    }
  }

  async function fetchAdjustments(eventId: number) {
    try {
      const data = await unwrap(
        api.GET('/events/{event_id}/adjustments', { params: { path: { event_id: eventId } } })
      )
      adjustments.value = Array.isArray(data) ? data : []
    } catch (e) {
      error.value = errorMessage(e, '无法加载结算调整列表。')
      console.error(e)
    }
  }

  async function refresh(eventId: number): Promise<void> {
    // 清 error 只在这里做一次：三个 fetch 并发跑，如果放在 fetchReport 开头
    // （或任一 fetch 里），先落地的那条 500 会被后启动的 fetch 静默抹掉，
    // 页面上什么都不显示、对应的表还空着。
    error.value = null
    await Promise.all([fetchReport(eventId), fetchAdvances(eventId), fetchAdjustments(eventId)])
  }

  async function createAdvance(
    eventId: number,
    payload: Schemas['AdvanceRequest']
  ): Promise<Schemas['CreatedEntry']> {
    try {
      const entry = await unwrap(
        api.POST('/events/{event_id}/advances', {
          params: { path: { event_id: eventId } },
          body: payload,
        })
      )
      await refresh(eventId)
      return entry
    } catch (e) {
      console.error(e)
      throw new Error(errorMessage(e, '新增垫付失败。'))
    }
  }

  async function deleteAdvance(eventId: number, id: number): Promise<void> {
    try {
      await unwrap(
        api.DELETE('/events/{event_id}/advances/{id}', {
          params: { path: { event_id: eventId, id } },
        })
      )
      await refresh(eventId)
    } catch (e) {
      console.error(e)
      throw new Error(errorMessage(e, '删除垫付失败。'))
    }
  }

  async function createAdjustment(
    eventId: number,
    payload: Schemas['AdjustmentRequest']
  ): Promise<Schemas['CreatedEntry']> {
    try {
      const entry = await unwrap(
        api.POST('/events/{event_id}/adjustments', {
          params: { path: { event_id: eventId } },
          body: payload,
        })
      )
      await refresh(eventId)
      return entry
    } catch (e) {
      console.error(e)
      throw new Error(errorMessage(e, '新增结算调整失败。'))
    }
  }

  async function deleteAdjustment(eventId: number, id: number): Promise<void> {
    try {
      await unwrap(
        api.DELETE('/events/{event_id}/adjustments/{id}', {
          params: { path: { event_id: eventId, id } },
        })
      )
      await refresh(eventId)
    } catch (e) {
      console.error(e)
      throw new Error(errorMessage(e, '删除结算调整失败。'))
    }
  }

  /** `counts` 已经是 `[{ channel, actual }]`（分），清点必须收本场全量渠道。 */
  async function reconcile(
    eventId: number,
    counts: Schemas['ReconcileRequest']['counts']
  ): Promise<Schemas['ReconcileResponse']> {
    try {
      const res = await unwrap(
        api.POST('/events/{event_id}/settlement/reconcile', {
          params: { path: { event_id: eventId } },
          body: { counts },
        })
      )
      await fetchReport(eventId)
      return res
    } catch (e) {
      console.error(e)
      throw new Error(errorMessage(e, '提交清点失败。'))
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
