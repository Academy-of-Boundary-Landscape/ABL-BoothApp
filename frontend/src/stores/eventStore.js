import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/services/api'
import { getImageUrl } from '@/services/url'

export const useEventStore = defineStore('event', () => {
  const events = ref([])
  const isLoading = ref(false)
  const error = ref(null)

  // 预处理展会数据,将付款码 URL 转换为完整路径
  const processEvent = (event) => {
    return {
      ...event,
      qrcode_url: getImageUrl(event.qrcode_url),
    }
  }

  const processEvents = (events) => {
    return events.map(processEvent)
  }

  async function fetchEvents() {
    // ... 此函数保持不变 ...
    isLoading.value = true
    error.value = null
    try {
      const response = await api.get('/events')
      // 防御性检查：确保响应数据是数组
      const rawEvents = Array.isArray(response.data) ? response.data : []
      events.value = processEvents(rawEvents)
      if (!Array.isArray(response.data)) {
        console.warn('⚠️ API 返回的数据不是数组，已转换为空数组', response.data)
        error.value = '数据格式错误，请稍后重试。'
      }
    } catch (err) {
      error.value = '无法加载展会列表，请检查后端服务是否开启。'
      events.value = [] // 错误时也要确保是数组
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }

  async function createEvent(eventData) {
    // ... 此函数保持不变 ...
    try {
      const response = await api.post('/events', eventData)
      const processedEvent = processEvent(response.data)
      events.value.unshift(processedEvent)
      return processedEvent
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '创建展会失败，请重试。')
    }
  }

  // 【新增】Action: 更新展会状态
  async function updateEventStatus(eventId, newStatus) {
    try {
      console.log('尝试更新展会状态', eventId, newStatus)
      const response = await api.put(`/events/${eventId}/status`, { status: newStatus })

      const index = events.value.findIndex((e) => e.id === eventId)
      if (index !== -1) {
        events.value[index].status = response.data.status
      }
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '更新状态失败，请重试。')
    }
  }

  async function updateEvent(eventId, formData) {
    try {
      // 使用 POST 方法发送 FormData，兼容性更好
      // Axios 会自动为 FormData 设置正确的 Content-Type
      // console.log('尝试更新展会信息', eventId, formData);
      const response = await api.post(`/events/${eventId}`, formData)

      const processedEvent = processEvent(response.data)
      // 更新成功后，同样在前端直接更新数据
      // 注意：这里的 id 是 eventId，需要确保类型一致
      const index = events.value.findIndex((e) => e.id === Number(eventId))
      if (index !== -1) {
        // 使用 Object.assign 来合并更新后的字段
        Object.assign(events.value[index], processedEvent)
      }
      return processedEvent
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '更新展会信息失败。')
    }
  }
  async function deleteEvent(eventId) {
    try {
      await api.delete(`/events/${eventId}`)
      // 删除成功后，从本地 events 数组中移除该展会
      events.value = events.value.filter((e) => e.id !== eventId)
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '删除展会失败，请重试。')
    }
  }

  return {
    events,
    isLoading,
    error,
    fetchEvents,
    createEvent,
    updateEventStatus,
    updateEvent,
    deleteEvent, // 【新增】导出删除函数
  }
})
