<template>
  <PageShell
    title="全局商品库"
    subtitle="管理可复用的商品模板：创建 / 导入导出 / 搜索编辑。"
    help="master-products"
    width="content"
  >
    <n-space vertical size="large">
      <CreateMasterProductForm @created="refreshProducts('created')" />

      <BoothpackSyncPanel @imported="refreshProducts('imported')" />

      <MasterProductList @edit="openEditModal" @toggleStatus="handleToggleStatus" />
    </n-space>

    <EditMasterProductModal
      :show="isEditModalVisible"
      :product="editableProduct"
      :initial-tab="editInitialTab"
      @close="closeEditModal"
      @updated="onProductUpdated"
    />
  </PageShell>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { NSpace } from 'naive-ui'
import { PageShell } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import { useProductStore } from '@/stores/productStore'
import type { Schemas } from '@/api/client'

import CreateMasterProductForm from '@/components/product/CreateMasterProductForm.vue'
import BoothpackSyncPanel from '@/components/product/BoothpackSyncPanel.vue'
import MasterProductList from '@/components/product/MasterProductList.vue'
import EditMasterProductModal from '@/components/product/EditMasterProductModal.vue'

const store = useProductStore()
const fb = useFeedback()

const isEditModalVisible = ref(false)
const editableProduct = ref<Schemas['MasterProduct'] | null>(null)
const editInitialTab = ref('info')

function openEditModal(product: Schemas['MasterProduct'], initialTab = 'info') {
  editableProduct.value = product
  editInitialTab.value = initialTab
  isEditModalVisible.value = true
}

function closeEditModal() {
  isEditModalVisible.value = false
  editableProduct.value = null
  editInitialTab.value = 'info'
}

async function onProductUpdated() {
  await store.fetchMasterProducts()
}

async function handleToggleStatus(product: Schemas['MasterProduct']) {
  try {
    await store.toggleProductStatus(product)
    fb.success(`已${product.is_active ? '停用' : '启用'}：${product.name}`)
  } catch (err) {
    fb.error(err, '操作失败')
  }
}

async function refreshProducts(reason = '') {
  await store.fetchMasterProducts()

  if (reason === 'created') {
    fb.success('已添加商品，列表已刷新', { duration: 2500, closable: true })
  } else if (reason === 'imported') {
    fb.success('已导入数据包，列表已刷新', { duration: 2500, closable: true })
  }
}

onMounted(async () => {
  await store.fetchMasterProducts()
})
</script>
