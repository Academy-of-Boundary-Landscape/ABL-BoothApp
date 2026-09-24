import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api, unwrap, ApiRequestError, type Schemas } from '@/api/client'

type StatsFilters = {
  productCode?: string
  startDate?: string
  endDate?: string
  intervalMinutes?: number
}

export const useEventStatStore = defineStore('eventStat', () => {
  // --- State ---
  const stats = ref<Schemas['StatsSalesResponse'] | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const activeEventId = ref<number | null>(null)

  // --- Getters (Computed Properties) ---
  const downloadUrl = computed(() => {
    if (activeEventId.value) {
      return `/api/events/${activeEventId.value}/sales_summary/download`
    }
    return '#'
  })

  // --- Actions ---
  async function setActiveEvent(eventId: number, filters: StatsFilters = {}) {
    const isSameEvent = activeEventId.value === eventId
    activeEventId.value = eventId
    if (!isSameEvent) stats.value = null
    if (eventId) {
      await fetchStats(filters)
    }
  }

  async function fetchStats({
    productCode,
    startDate,
    endDate,
    intervalMinutes,
  }: StatsFilters = {}) {
    if (!activeEventId.value) {
      error.value = '没有提供展会ID。'
      return
    }
    const eventId = activeEventId.value
    isLoading.value = true
    error.value = null
    try {
      const response = await unwrap(
        api.GET('/events/{event_id}/sales_summary', {
          params: {
            path: { event_id: eventId },
            query: {
              product_code: productCode || undefined,
              start_date: startDate || undefined,
              end_date: endDate || undefined,
              interval_minutes: intervalMinutes || undefined,
            },
          },
        })
      )
      stats.value = response
    } catch (e) {
      if (e instanceof ApiRequestError && e.status === 404) {
        error.value = '无法找到该展会或该展会暂无销售数据。'
      } else {
        error.value = '加载销售统计时发生网络错误。'
      }
      stats.value = null
    } finally {
      isLoading.value = false
    }
  }

  return {
    stats,
    isLoading,
    error,
    activeEventId,
    downloadUrl,
    setActiveEvent,
    fetchStats,
  }
})
