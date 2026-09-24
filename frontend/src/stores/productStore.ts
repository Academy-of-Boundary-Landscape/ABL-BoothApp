import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api, unwrap, type Schemas } from '@/api/client'
import { getImageUrl } from '@/services/url'

export const useProductStore = defineStore('masterProduct', () => {
  // --- State ---
  const masterProducts = ref<Schemas['MasterProduct'][]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const searchTerm = ref('')
  const showInactive = ref(false)

  // --- Getters (Computed) ---
  const filteredProducts = computed(() => {
    if (!searchTerm.value.trim()) {
      return masterProducts.value
    }
    const lowerCaseSearchTerm = searchTerm.value.toLowerCase()
    return masterProducts.value.filter((product) => {
      const nameMatch = product.name.toLowerCase().includes(lowerCaseSearchTerm)
      const codeMatch = product.product_code.toLowerCase().includes(lowerCaseSearchTerm)
      const tagMatch = (product.tags || '').toLowerCase().includes(lowerCaseSearchTerm)
      return nameMatch || codeMatch || tagMatch
    })
  })

  const categoryOptions = computed(() => {
    const counts = new Map<string, number>()

    masterProducts.value.forEach((product) => {
      const category = String(product.category || '').trim()
      if (!category) {
        return
      }
      counts.set(category, (counts.get(category) || 0) + 1)
    })

    return Array.from(counts.entries())
      .sort((a, b) => {
        if (b[1] !== a[1]) {
          return b[1] - a[1]
        }
        return a[0].localeCompare(b[0], 'zh-CN')
      })
      .map(([category]) => ({
        label: category,
        value: category,
      }))
  })

  const tagOptions = computed(() => {
    const counts = new Map<string, number>()

    masterProducts.value.forEach((product) => {
      const raw = String(product.tags || '').trim()
      if (!raw) return
      raw.split(',').forEach((t) => {
        const tag = t.trim()
        if (!tag) return
        counts.set(tag, (counts.get(tag) || 0) + 1)
      })
    })

    return Array.from(counts.entries())
      .sort((a, b) => {
        if (b[1] !== a[1]) {
          return b[1] - a[1]
        }
        return a[0].localeCompare(b[0], 'zh-CN')
      })
      .map(([tag]) => ({
        label: tag,
        value: tag,
      }))
  })

  // --- Actions ---

  // fetchMasterProducts 无需修改
  async function fetchMasterProducts() {
    isLoading.value = true
    error.value = null
    try {
      const list = await unwrap(
        api.GET('/master-products', {
          params: { query: showInactive.value ? { all: true } : {} },
        })
      )
      masterProducts.value = list.map((product) => ({
        ...product,
        image_url: getImageUrl(product.image_url ?? ''),
      }))
    } catch (e) {
      error.value = '无法加载商品库列表。'
      console.error(e)
    } finally {
      isLoading.value = false
    }
  }

  // 【已修改】createMasterProduct 现在直接接收一个 FormData 对象
  async function createMasterProduct(formData: FormData) {
    try {
      // 组件已经准备好了 FormData，我们直接发送即可
      // multipart：FormData 原样作为 body 传，类型上用 never 绕过表单字段检查
      const created = await unwrap(api.POST('/master-products', { body: formData as never }))
      const product = { ...created, image_url: getImageUrl(created.image_url ?? '') }
      masterProducts.value.unshift(product)
      return product
    } catch (e) {
      console.error(e)
      throw e
    }
  }

  // 【已修改】updateMasterProduct 现在接收 productId 和 FormData 两个参数
  async function updateMasterProduct(productId: number, formData: FormData) {
    try {
      // 使用 POST 发送 FormData 来更新，以获得更好的兼容性
      // multipart：FormData 原样作为 body 传，类型上用 never 绕过表单字段检查
      const updated = await unwrap(
        api.POST('/master-products/{id}', {
          params: { path: { id: productId } },
          body: formData as never,
        })
      )
      const product = { ...updated, image_url: getImageUrl(updated.image_url ?? '') }
      const index = masterProducts.value.findIndex((p) => p.id === productId)
      if (index !== -1) {
        // 使用新数据替换旧数据，以确保响应性
        masterProducts.value[index] = product
      }
      return product
    } catch (e) {
      console.error(e)
      throw e
    }
  }

  // toggleProductStatus 无需修改
  async function toggleProductStatus(product: Schemas['MasterProduct']) {
    try {
      const newStatus = !product.is_active
      const updated = await unwrap(
        api.PUT('/master-products/{id}/status', {
          params: { path: { id: product.id } },
          body: { is_active: newStatus },
        })
      )
      const updatedProduct = { ...updated, image_url: getImageUrl(updated.image_url ?? '') }
      const index = masterProducts.value.findIndex((p) => p.id === product.id)
      if (index !== -1) {
        masterProducts.value[index] = updatedProduct
      }
    } catch (e) {
      console.error(e)
      throw e
    }
  }

  // --- Return ---
  return {
    masterProducts,
    isLoading,
    error,
    searchTerm,
    filteredProducts,
    categoryOptions,
    tagOptions,
    fetchMasterProducts,
    createMasterProduct,
    updateMasterProduct,
    toggleProductStatus,
    showInactive,
  }
})
