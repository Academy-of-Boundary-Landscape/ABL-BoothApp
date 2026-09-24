import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, unwrap, type Schemas, errorMessage } from '@/api/client'

/**
 * 社团（货主单位）管理。
 *
 * 「本社团」有且只有一个，由后端偏唯一索引保证——所以「设为本社团」不能自己
 * 先把旧的改掉，直接 PUT `{is_home: true}` 后端会先把旧的降级，顺序由后端负责。
 */
export const useSocietyStore = defineStore('society', () => {
  const societies = ref<Schemas['Society'][]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  async function fetchSocieties() {
    isLoading.value = true
    error.value = null
    try {
      const response = await unwrap(api.GET('/societies'))
      societies.value = Array.isArray(response) ? response : []
    } catch (err) {
      error.value = '无法加载社团列表。'
      console.error(err)
    } finally {
      isLoading.value = false
    }
  }

  async function createSociety(name: string) {
    try {
      const response = await unwrap(api.POST('/societies', { body: { name } }))
      societies.value.push(response)
      return response
    } catch (err) {
      console.error(err)
      throw new Error(errorMessage(err, '新建社团失败。'))
    }
  }

  async function updateSociety(id: number, payload: Schemas['UpdateSocietyRequest']) {
    try {
      const response = await unwrap(
        api.PUT('/societies/{id}', { params: { path: { id } }, body: payload })
      )
      const index = societies.value.findIndex((s) => s.id === id)
      if (index !== -1) {
        societies.value[index] = response
      }
      return response
    } catch (err) {
      console.error(err)
      throw new Error(errorMessage(err, '更新社团失败。'))
    }
  }

  async function deleteSociety(id: number) {
    try {
      await unwrap(api.DELETE('/societies/{id}', { params: { path: { id } } }))
      societies.value = societies.value.filter((s) => s.id !== id)
    } catch (err) {
      console.error(err)
      throw new Error(errorMessage(err, '删除社团失败。'))
    }
  }

  // 设为「本社团」后重新拉取：后端会把旧的本社团降级，本地只改一条会不同步。
  async function setHomeSociety(id: number) {
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
