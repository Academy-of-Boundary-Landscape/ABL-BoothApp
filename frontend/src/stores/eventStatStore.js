import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import api from '@/services/api';

export const useEventStatStore = defineStore('eventStat', () => {
  // --- State ---
  const stats = ref(null);
  const isLoading = ref(false);
  const error = ref(null);
  const activeEventId = ref(null);

  // --- Getters (Computed Properties) ---
  const downloadUrl = computed(() => {
    if (activeEventId.value) {
      return `/api/events/${activeEventId.value}/sales_summary/download`;
    }
    return '#';
  });

  // --- Actions ---
  async function setActiveEvent(eventId, filters = {}) {
    const isSameEvent = activeEventId.value === eventId;
    activeEventId.value = eventId;
    if (!isSameEvent) stats.value = null;
    if (eventId) {
      await fetchStats(filters);
    }
  }

  async function fetchStats({ productCode, startDate, endDate, intervalMinutes } = {}) {
    if (!activeEventId.value) {
      error.value = "没有提供展会ID。";
      return;
    }
    isLoading.value = true;
    error.value = null;
    try {
      const response = await api.get(`/events/${activeEventId.value}/sales_summary`, {
        params: {
          product_code: productCode || undefined,
          start_date: startDate || undefined,
          end_date: endDate || undefined,
          interval_minutes: intervalMinutes || undefined,
        },
      });
      stats.value = response.data;
    } catch (err) {
      if (err.response && err.response.status === 404) {
        error.value = "无法找到该展会或该展会暂无销售数据。";
      } else {
        error.value = "加载销售统计时发生网络错误。";
      }
      stats.value = null;
    } finally {
      isLoading.value = false;
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
  };
});