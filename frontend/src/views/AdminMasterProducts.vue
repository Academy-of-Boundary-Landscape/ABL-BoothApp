<template>
  <div class="page">
    <header class="page-header">
      <div class="header-content">
        <div class="header-title-row">
          <h1>全局商品库</h1>
          <HelpBubble page="master-products" />
        </div>
        <p>管理可复用的商品模板：创建 / 导入导出 / 搜索编辑。</p>
      </div>
    </header>

    <main class="page-body">
      <n-space vertical size="large">
        <CreateMasterProductForm @created="refreshProducts('created')" />

        <BoothpackSyncPanel @imported="refreshProducts('imported')" />

        <MasterProductList
          @edit="openEditModal"
          @toggleStatus="handleToggleStatus"
        />
      </n-space>

      <EditMasterProductModal
        :show="isEditModalVisible"
        :product="editableProduct"
        :initial-tab="editInitialTab"
        @close="closeEditModal"
        @updated="onProductUpdated"
      />
    </main>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import { NSpace, useMessage } from 'naive-ui'
import { useProductStore } from '@/stores/productStore'

import CreateMasterProductForm from '@/components/product/CreateMasterProductForm.vue'
import BoothpackSyncPanel from '@/components/product/BoothpackSyncPanel.vue'
import MasterProductList from '@/components/product/MasterProductList.vue'
import EditMasterProductModal from '@/components/product/EditMasterProductModal.vue'
import HelpBubble from '@/components/shared/HelpBubble.vue'

const store = useProductStore()

const isEditModalVisible = ref(false)
const editableProduct = ref(null)
const editInitialTab = ref('info')

function openEditModal(product, initialTab = 'info') {
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

async function handleToggleStatus(product) {
  try {
    await store.toggleProductStatus(product)
    message.success(`已${product.is_active ? '停用' : '启用'}：${product.name}`)
  } catch (err) {
    message.error(err?.message || '操作失败')
  }
}
const message = useMessage()

async function refreshProducts(reason = '') {
  await store.fetchMasterProducts()

  if (reason === 'created') {
    message.success('已添加商品，列表已刷新', { duration: 2500, closable: true })
  } else if (reason === 'imported') {
    message.success('已导入数据包，列表已刷新', { duration: 2500, closable: true })
  }
}

onMounted(async () => {
  await store.fetchMasterProducts()
})

</script>

<style scoped>
.page {
  max-width: 960px;
}

.header-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.page-header {
  margin-bottom: 1.5rem;
}
.page-header h1 {
  margin: 0 0 0.25rem;
  font-size: var(--font-xl);
  color: var(--accent-color);
}
.page-header p {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--font-base);
}
</style>
