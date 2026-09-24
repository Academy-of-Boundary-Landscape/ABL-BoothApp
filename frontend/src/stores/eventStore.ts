import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, unwrap, type Schemas } from '@/api/client'
import { getImageUrl } from '@/services/url'

export const useEventStore = defineStore('event', () => {
  const events = ref<Schemas['EventResponse'][]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // 预处理展会数据,将付款码 URL 转换为完整路径
  const processEvent = (event: Schemas['EventResponse']): Schemas['EventResponse'] => {
    return {
      ...event,
      qrcode_url: getImageUrl(event.qrcode_url ?? ''),
    }
  }

  const processEvents = (events: Schemas['EventResponse'][]): Schemas['EventResponse'][] => {
    return events.map(processEvent)
  }

  async function fetchEvents() {
    // ... 此函数保持不变 ...
    isLoading.value = true
    error.value = null
    try {
      const data: unknown = await unwrap(api.GET('/events'))
      // 防御性检查：确保响应数据是数组
      const rawEvents = Array.isArray(data) ? data : []
      events.value = processEvents(rawEvents)
      if (!Array.isArray(data)) {
        console.warn('⚠️ API 返回的数据不是数组，已转换为空数组', data)
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

  async function createEvent(eventData: FormData) {
    // ... 此函数保持不变 ...
    try {
      const response = await unwrap<Schemas['EventResponse']>(
        // multipart：契约是 CreateEventForm，实际发 FormData
        api.POST('/events', { body: eventData as never })
      )
      const processedEvent = processEvent(response)
      events.value.unshift(processedEvent)
      return processedEvent
    } catch (err) {
      console.error(err)
      throw err
    }
  }

  // 【新增】Action: 更新展会状态
  async function updateEventStatus(eventId: number, newStatus: Schemas['EventStatus']) {
    try {
      console.log('尝试更新展会状态', eventId, newStatus)
      const response = await unwrap<Schemas['EventResponse']>(
        api.PUT('/events/{id}/status', {
          params: { path: { id: eventId } },
          body: { status: newStatus },
        })
      )

      const index = events.value.findIndex((e) => e.id === eventId)
      if (index !== -1) {
        events.value[index].status = response.status
      }
      return response
    } catch (err) {
      console.error(err)
      throw err
    }
  }

  async function updateEvent(eventId: number, formData: FormData) {
    try {
      // 使用 POST 方法发送 FormData，兼容性更好
      // Axios 会自动为 FormData 设置正确的 Content-Type
      // console.log('尝试更新展会信息', eventId, formData);
      const response = await unwrap<Schemas['EventResponse']>(
        // multipart：契约是 UpdateEventForm，实际发 FormData
        api.POST('/events/{id}', {
          params: { path: { id: eventId } },
          body: formData as never,
        })
      )

      const processedEvent = processEvent(response)
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
      throw err
    }
  }
  async function deleteEvent(eventId: number) {
    try {
      await unwrap(api.DELETE('/events/{id}', { params: { path: { id: eventId } } }))
      // 删除成功后，从本地 events 数组中移除该展会
      events.value = events.value.filter((e) => e.id !== eventId)
    } catch (err) {
      console.error(err)
      throw err
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
