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
      <CreateMasterProductForm @created="refreshProducts('created')" />

      <BoothpackSyncPanel @imported="refreshProducts('imported')" />

      <MasterProductList
        @edit="openEditModal"
        @toggleStatus="handleToggleStatus"
      />

      <EditMasterProductModal
        :show="isEditModalVisible"
        :product="editableProduct"
        @close="closeEditModal"
        @updated="onProductUpdated"
      />
    </main>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { useProductStore } from '@/stores/productStore'

import CreateMasterProductForm from '@/components/product/CreateMasterProductForm.vue'
import BoothpackSyncPanel from '@/components/product/BoothpackSyncPanel.vue'
import MasterProductList from '@/components/product/MasterProductList.vue'
import EditMasterProductModal from '@/components/product/EditMasterProductModal.vue'
import HelpBubble from '@/components/shared/HelpBubble.vue'

const store = useProductStore()

const isEditModalVisible = ref(false)
const editableProduct = ref(null)

function openEditModal(product) {
  editableProduct.value = product
  isEditModalVisible.value = true
}

function closeEditModal() {
  isEditModalVisible.value = false
  editableProduct.value = null
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
  width: 100%;
}

.page-header {
  padding: 1rem 1.25rem;
  margin-bottom: 1rem;
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-lg);
}

.header-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.header-content h1 {
  margin: 0;
  font-size: var(--font-xl);
  color: var(--accent-color);
  font-weight: 700;
}

.header-content p {
  margin: 0.35rem 0 0 0;
  color: var(--text-muted);
  font-size: var(--font-base);
}

.page-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
</style>
