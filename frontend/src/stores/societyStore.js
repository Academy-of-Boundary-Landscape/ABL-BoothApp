import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/services/api'

/**
 * 社团（货主单位）管理。
 *
 * 「本社团」有且只有一个，由后端偏唯一索引保证——所以「设为本社团」不能自己
 * 先把旧的改掉，直接 PUT `{is_home: true}` 后端会先把旧的降级，顺序由后端负责。
 */
export const useSocietyStore = defineStore('society', () => {
  const societies = ref([])
  const isLoading = ref(false)
  const error = ref(null)

  async function fetchSocieties() {
    isLoading.value = true
    error.value = null
    try {
      const response = await api.get('/societies')
      societies.value = Array.isArray(response.data) ? response.data : []
    } catch (err) {
      error.value = '无法加载社团列表。'
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }

  async function createSociety(name) {
    try {
      const response = await api.post('/societies', { name })
      societies.value.push(response.data)
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '新建社团失败。')
    }
  }

  async function updateSociety(id, payload) {
    try {
      const response = await api.put(`/societies/${id}`, payload)
      const index = societies.value.findIndex((s) => s.id === id)
      if (index !== -1) {
        societies.value[index] = response.data
      }
      return response.data
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '更新社团失败。')
    }
  }

  async function deleteSociety(id) {
    try {
      await api.delete(`/societies/${id}`)
      societies.value = societies.value.filter((s) => s.id !== id)
    } catch (err) {
      console.error(err)
      throw new Error(err.response?.data?.error || '删除社团失败。')
    }
  }

  // 设为「本社团」后重新拉取：后端会把旧的本社团降级，本地只改一条会不同步。
  async function setHomeSociety(id) {
    await updateSociety(id, { is_home: true })
    await fetchSocieties()
  }

  return {
    societies,
    isLoading,
    error,
    fetchSocieties,
    createSociety,
    updateSociety,
    deleteSociety,
    setHomeSociety,
  }
})
