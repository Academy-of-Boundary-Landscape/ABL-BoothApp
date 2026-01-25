<template>
  <div>
    <header class="page-header">
      <h1>商品管理</h1>
      <p>为当前展会添加、修改和移除上架商品。</p>
    </header>

    <main>
      <!-- 上架新商品区块 -->
      <section class="form-section">
        <div class="section-header" @click="isFormExpanded = !isFormExpanded">
          <h2>上架新商品</h2>
          <n-button text class="toggle-btn">
            {{ isFormExpanded ? '折叠' : '展开' }}
          </n-button>
        </div>
        <transition name="expand">
          <div v-show="isFormExpanded" class="section-container">
        <div class="search-filters">
          <n-input
            v-model:value="searchQuery"
            placeholder="搜索名称或编号..."
            clearable
            size="large"
            round
            class="search-input"
          />
          <n-select
            v-model:value="selectedCategory"
            :options="categoryOptionsForSelect"
            class="category-select"
            clearable
            size="small"
            placeholder="全部分类"
          />
        </div>

        <div class="product-preview-container">
          <div v-if="productStore.isLoading" class="loading-message">加载制品列表中...</div>
          <div v-else-if="productStore.error" class="error-message">{{ productStore.error }}</div>
          <ul v-else-if="filteredProducts.length" class="product-preview-list">
            <li
              v-for="product in filteredProducts"
              :key="product.id"
              class="preview-item"
              :class="{ selected: addProductData.product_code === product.product_code }"
              @click="selectProduct(product)"
            >
              <div class="preview-item-img-container">
                <n-image 
                  v-if="product.image_url" 
                  :src="product.image_url" 
                  :alt="product.name" 
                  class="preview-item-img" 
                  preview-disabled 
                />
                <div v-else class="preview-item-img-placeholder">
                  <span>{{ getProductLabel(product.name) }}</span>
                </div>
              </div>
              <div class="preview-item-info">
                <span class="preview-item-name">{{ product.name }}</span>
                <span class="preview-item-code">{{ product.product_code }}</span>
                <span v-if="product.default_price != null" class="preview-item-price">¥{{ Number(product.default_price).toFixed(2) }}</span>
              </div>
            </li>
          </ul>
          <p v-else class="no-results-message">未找到匹配的制品。</p>
        </div>

        <form @submit.prevent="handleAddProduct" class="add-product-form">
          <n-input v-model:value="addProductData.product_code" placeholder="商品编号 (可点击上方预览填充)" clearable required />
          <n-input-number 
            v-model:value="addProductData.initial_stock" 
            placeholder="初始库存" 
            :min="0" 
            :precision="0" 
            :show-button="true"
            required 
            ref="stockInputRef" 
          />
          <n-input-number 
            v-model:value="addProductData.price" 
            placeholder="展会售价 (可选)"  
            :precision="2" 
            :step="0.01" 
          />
          <n-button type="primary" attr-type="submit" :disabled="isAdding">
            {{ isAdding ? '上架中...' : '上架' }}
          </n-button>
        </form>
        <p v-if="addError" class="error-message">{{ addError }}</p>
      </div>
      </transition>
      </section>

      <!-- 商品列表区块 -->
      <section class="list-section">
        <div class="section-header" @click="isListExpanded = !isListExpanded">
          <h2>已上架商品</h2>
          <n-button text class="toggle-btn">
            {{ isListExpanded ? '折叠' : '展开' }}
          </n-button>
        </div>
        <transition name="expand">
          <div v-show="isListExpanded" class="section-container">
            <div v-if="eventDetailStore.isLoading" class="loading-message">正在加载商品列表...</div>
            <div v-else-if="eventDetailStore.products.length" class="table-wrapper">
        <table class="product-table">
          <thead>
            <tr>
              <th class="column-preview">预览</th>
              <th>编号</th>
              <th>名称</th>
              <th>展会售价</th>
              <th>初始库存</th>
              <th>当前库存</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="product in eventDetailStore.products" :key="product.id">
              <td>
                <div v-if="product.image_url" class="preview-img-container">
                  <n-image :src="product.image_url" :alt="product.name" class="preview-img" preview-disabled />
                </div>
                <div v-else class="preview-img-placeholder">
                  <span>{{ getProductLabel(product.name) }}</span>
                </div>
              </td>
              <td>{{ product.product_code }}</td>
              <td>{{ product.name }}</td>
              <td>¥{{ product.price.toFixed(2) }}</td>
              <td>{{ product.initial_stock }}</td>
              <td>{{ product.current_stock }}</td>
              <td>
                <n-space size="small" justify="end">
                  <n-button size="small" @click="openEditModal(product)">编辑</n-button>
                  <n-button size="small" type="error" quaternary @click="handleDelete(product)">删除</n-button>
                </n-space>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-else>该展会还未上架任何商品。</p>
          </div>
        </transition>
      </section>
    </main>

    <AppModal :show="isEditModalVisible" @close="closeEditModal">
      <template #header><h3>编辑上架商品</h3></template>
      <template #body>
        <form v-if="editableProduct" class="edit-form" @submit.prevent="handleUpdate">
          <div class="form-group">
            <label>展会售价 (¥):</label>
            <n-input-number 
              v-model:value="editableProduct.price" 
              :precision="2" 
              :step="0.01" 
              :show-button="true"
              required 
            />
          </div>
          <div class="form-group">
            <label>初始库存:</label>
            <n-input-number 
              v-model:value="editableProduct.initial_stock" 
              :min="0" 
              :precision="0" 
              :show-button="true"
              required 
            />
          </div>
          <p v-if="editError" class="error-message">{{ editError }}</p>
        </form>
      </template>
      <template #footer>
        <n-space>
          <n-button @click="closeEditModal">取消</n-button>
          <n-button type="primary" @click="handleUpdate" :disabled="isUpdating">
            {{ isUpdating ? '保存中...' : '保存更改' }}
          </n-button>
        </n-space>
      </template>
    </AppModal>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { RouterLink } from 'vue-router';
import { useEventDetailStore } from '@/stores/eventDetailStore';
import { useProductStore } from '@/stores/productStore';
import AppModal from '@/components/shared/AppModal.vue';
import { NCard, NInput, NSelect, NImage, NInputNumber, NButton, NSpace, useDialog } from 'naive-ui';

const props = defineProps({ id: { type: String, required: true } });

const eventDetailStore = useEventDetailStore();
const productStore = useProductStore();
const dialog = useDialog();

const searchQuery = ref('');
const stockInputRef = ref(null);
const selectedCategory = ref(null);
const isFormExpanded = ref(true);
const isListExpanded = ref(true);

const categoryOptions = computed(() => {
  const cats = (productStore.masterProducts || [])
    .map(p => p.category)
    .filter(cat => !!cat && cat.trim() !== '');
  return [...new Set(cats)];
});

const categoryOptionsForSelect = computed(() =>
  categoryOptions.value.map(cat => ({ label: cat, value: cat }))
);

const filteredProducts = computed(() => {
  const existingProductCodes = new Set(eventDetailStore.products.map(p => p.product_code));

  let availableProducts = productStore.masterProducts
    .filter(masterProduct => !existingProductCodes.has(masterProduct.product_code));

  if (selectedCategory.value) {
    availableProducts = availableProducts.filter(p => p.category === selectedCategory.value);
  }

  if (searchQuery.value.trim()) {
    const lowerCaseQuery = searchQuery.value.toLowerCase();
    availableProducts = availableProducts.filter(product =>
      product.name.toLowerCase().includes(lowerCaseQuery) ||
      product.product_code.toLowerCase().includes(lowerCaseQuery)
    );
  }

  return availableProducts;
});

const isAdding = ref(false);
const addError = ref('');
const addProductData = ref({ product_code: '', initial_stock: null, price: null });

function selectProduct(product) {
  addProductData.value.product_code = product.product_code;
  if (product.default_price) {
    addProductData.value.price = product.default_price;
  }
  stockInputRef.value?.focus();
}

async function handleAddProduct() {
  isAdding.value = true;
  addError.value = '';
  try {
    // 验证输入
    if (!addProductData.value.product_code || !addProductData.value.product_code.trim()) {
      addError.value = '请输入商品编号';
      isAdding.value = false;
      return;
    }
    
    if (addProductData.value.initial_stock === null || addProductData.value.initial_stock === undefined) {
      addError.value = '请输入初始库存';
      isAdding.value = false;
      return;
    }
    
    // 确保库存是整数
    if (!Number.isInteger(addProductData.value.initial_stock) || addProductData.value.initial_stock < 0) {
      addError.value = '初始库存必须是非负整数';
      isAdding.value = false;
      return;
    }
    
    // 价格可以为负数（对应折扣），这里无需验证
    
    const dataToSend = { ...addProductData.value };
    if (dataToSend.price === null || dataToSend.price === '') {
      delete dataToSend.price;
    }
    await eventDetailStore.addProductToEvent(props.id, dataToSend);
    await eventDetailStore.fetchProductsForEvent(props.id);
    addProductData.value = { product_code: '', initial_stock: null, price: null };
    searchQuery.value = '';
  } catch (error) {
    addError.value = error.message;
  } finally {
    isAdding.value = false;
  }
}

const isEditModalVisible = ref(false);
const isUpdating = ref(false);
const editError = ref('');
const editableProduct = ref(null);

function openEditModal(product) {
  editableProduct.value = { ...product };
  isEditModalVisible.value = true;
}

function closeEditModal() {
  isEditModalVisible.value = false;
  editError.value = '';
}

async function handleUpdate() {
  if (!editableProduct.value) return;
  isUpdating.value = true;
  editError.value = '';
  try {
    const { id, price, initial_stock } = editableProduct.value;
    
    // 验证价格（允许负数用于折扣）
    if (price === null || price === undefined) {
      editError.value = '请输入有效的售价';
      isUpdating.value = false;
      return;
    }
    
    // 验证库存必须是整数
    if (!Number.isInteger(initial_stock) || initial_stock < 0) {
      editError.value = '初始库存必须是非负整数';
      isUpdating.value = false;
      return;
    }
    
    await eventDetailStore.updateEventProduct(id, { price, initial_stock });
    closeEditModal();
  } catch (error) {
    editError.value = error.message;
  } finally {
    isUpdating.value = false;
  }
}

async function handleDelete(product) {
  dialog.warning({
    title: '确认下架',
    content: `确定要从该展会下架 "${product.name}" 吗？此操作不可恢复。`,
    positiveText: '确认下架',
    negativeText: '取消',
    async onPositiveClick() {
      try {
        await eventDetailStore.deleteEventProduct(product.id);
        await eventDetailStore.fetchProductsForEvent(props.id);
      } catch (error) {
        dialog.error({
          title: '删除失败',
          content: error.message || '无法下架商品，请稍后重试',
          positiveText: '知道了'
        });
      }
    }
  });
}

onMounted(() => {
  eventDetailStore.fetchProductsForEvent(props.id);
  productStore.fetchMasterProducts();
});

onUnmounted(() => {
  eventDetailStore.resetStore();
});

// 获取商品名称的前几个字作为占位符
function getProductLabel(name) {
  if (!name) return '无图';
  // 中文字符通常一个字占一个字符宽度，英文需要2-3个，这里简单取前3个字符
  return name.substring(0, 3);
}
</script>

<style scoped>
.page-header {
  position: relative;
  margin-bottom: 2rem;
  padding-bottom: 1rem;
  border-bottom: 1px solid var(--border-color);
}
.page-header h1 { color: var(--accent-color); margin: 0; }
.page-header p { color: var(--text-muted); margin-top: 0.5rem; }
.btn-back {
  position: absolute;
  top: 0;
  right: 0;
}

/* 通用区块样式 */
.form-section,
.list-section {
  margin-bottom: 2rem;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  user-select: none;
  padding: 0.75rem 1rem;
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: 8px;
  transition: all 0.2s ease;
  margin-bottom: 0.5rem;
}

.section-header:hover {
  background: var(--hover-bg-color, var(--card-bg-color));
  border-color: var(--accent-color);
}

.section-header h2 {
  margin: 0;
  font-size: 1.25rem;
  color: var(--accent-color);
  font-weight: 600;
}

.toggle-btn {
  font-size: 0.9rem;
  padding: 0.25rem 0.75rem;
  min-width: auto;
  color: var(--accent-color);
}

/* 展开/折叠动画 */
.expand-enter-active,
.expand-leave-active {
  transition: all 0.3s ease;
  overflow: hidden;
}

.expand-enter-from,
.expand-leave-to {
  opacity: 0;
  max-height: 0;
}

.expand-enter-to,
.expand-leave-from {
  opacity: 1;
  max-height: 2000px;
}

.section-container {
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: 8px;
  padding: 1.5rem;
  overflow: hidden;
}

.table-wrapper {
  width: 100%;
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
  border: 1px solid var(--border-color);
  border-radius: 4px;
}

.add-product-form {
  display: flex;
  gap: 1rem;
  align-items: center;
  flex-wrap: wrap;
  margin-top: 1rem;
}

/* --- 表格样式 --- */
.product-table {
  width: 100%;
  margin: 0;
  border-collapse: collapse;
  border-spacing: 0;
  text-align: left;
  font-size: 0.95rem;
  min-width: 700px;
}

.product-table th {
  padding: 12px 16px;
  background-color: var(--card-bg-color);
  color: var(--primary-text-color);
  font-weight: 600;
  border-bottom: 2px solid var(--accent-color);
  white-space: nowrap;
}

.product-table td {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
  color: var(--text-placeholder);
  vertical-align: middle;
}

.product-table tbody tr {
  transition: background-color 0.2s ease-in-out;
}

.product-table tbody tr:hover {
  background-color: var(--accent-color-light);
}

.product-table th:first-child,
.product-table td:first-child {
  padding-left: 0;
}

.product-table th:last-child,
.product-table td:last-child {
  text-align: right;
  padding-right: 0;
}

.column-preview {
  width: 60px;
}

.preview-img-container {
  display: flex;
  align-items: center;
  justify-content: center;
}

.preview-img {
  width: 50px;
  height: 50px;
  object-fit: cover;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  vertical-align: middle;
}

.preview-img :deep(img) {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.preview-img-placeholder {
  width: 50px;
  height: 50px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--accent-color), var(--accent-color-light, var(--accent-color)));
  border-radius: 4px;
  border: 1px solid var(--border-color);
  color: var(--text-white);
  font-size: 0.75rem;
  font-weight: 600;
  text-align: center;
  overflow: hidden;
}
.no-img {
  display: inline-block;
  width: 50px;
  height: 50px;
  line-height: 50px;
  text-align: center;
  font-size: 0.8rem;
  color: var(--text-disabled);
  background-color: var(--bg-color);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  vertical-align: middle;
}

.loading-message, .error-message {
  padding: 1rem;
  text-align: center;
}
.error-message { color: var(--error-color); }

.edit-form .form-group { margin-bottom: 1rem; }
.edit-form label { display: block; margin-bottom: 0.5rem; }

.action-btn {
  background: none;
  border: 1px solid transparent;
  color: var(--primary-text-color);
  padding: 6px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.9rem;
  transition: background-color 0.2s, color 0.2s, border-color 0.2s;
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  white-space: nowrap;
}

.product-search-container {
  margin-bottom: 0.5rem;
}

.search-filters {
  display: flex;
  gap: 1rem;
  margin-bottom: 1rem;
}

.search-input {
  flex-grow: 1;
}

.product-preview-container {
  max-height: 320px;
  overflow-y: auto;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  padding: 0.5rem;
  background-color: var(--bg-color);
}
.no-results-message {
  color: var(--text-disabled);
  padding: 1rem;
  text-align: center;
}

.product-preview-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 0.5rem;
}

.preview-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 0.4rem;
  border: 1px solid transparent;
  border-radius: 4px;
  cursor: pointer;
 

.preview-item-img-container {
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 0.5rem;
}

.preview-item-img-placeholder {
  width: 64px;
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--accent-color), var(--accent-color-light, var(--accent-color)));
  border-radius: 4px;
  border: 1px solid var(--border-color);
  color: var(--text-white);
  font-size: 0.85rem;
  font-weight: 600;
  text-align: center;
} transition: background-color 0.2s, border-color 0.2s;
  background-color: var(--card-bg-color);
}
.preview-item:hover {
  border-color: var(--accent-color);
  background-color: var(--accent-color-light);
}
.preview-item.selected {
  border-color: var(--accent-color);
  background-color: var(--accent-color-light);
}

.preview-item-img {
  width: 64px;
  height: 64px;
  object-fit: cover;
  border-radius: 4px;
  margin-bottom: 0.5rem;
}
.preview-item-img :deep(img) {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.preview-item-info {
  display: flex;
  flex-direction: column;
}

.preview-item-name {
  font-size: 0.85rem;
  font-weight: 500;
  color: var(--primary-text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  width: 110px;
}

.preview-item-code {
  font-size: 0.75rem;
  color: var(--text-disabled);
}
.preview-item-price {
  font-size: 0.75rem;
  color: var(--accent-color);
}

.category-select {
  min-width: 140px;
}

/* 让搜索栏更显眼 */
.search-input :deep(.n-input) {
  border-width: 2px;
}
.search-input :deep(.n-input:not(.n-input--disabled):focus-within) {
  box-shadow: 0 0 0 2px var(--accent-color-light);
  border-color: var(--accent-color);
}

/* 表格缩略图的适配，避免超出 */
.preview-img {
  width: 50px;
  height: 50px;
}
.preview-img :deep(img) {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

/* 响应式布局 */
@media (max-width: 768px) {
  main {
    padding: 0;
  }

  .page-header {
    margin-bottom: 1.5rem;
    padding-bottom: 0.75rem;
  }

  .page-header h1 {
    font-size: 1.3rem;
  }

  .page-header p {
    font-size: 0.9rem;
  }

  .section-container {
    padding: 1rem;
  }

  .search-filters {
    flex-direction: column;
    gap: 0.75rem;
  }

  .search-input,
  .category-select {
    width: 100%;
    min-width: auto;
  }

  .product-preview-container {
    max-height: 240px;
  }

  .product-preview-list {
    grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
    gap: 0.4rem;
  }

  .preview-item-img {
    width: 50px;
    height: 50px;
  }

  .preview-item-img-placeholder {
    width: 50px;
    height: 50px;
    font-size: 0.75rem;
  }

  .preview-item {
    padding: 0.3rem;
  }

  .preview-item-name {
    font-size: 0.8rem;
    width: 90px;
  }

  .preview-item-code,
  .preview-item-price {
    font-size: 0.7rem;
  }

  .add-product-form {
    flex-direction: column;
    gap: 0.75rem;
  }

  .add-product-form > * {
    width: 100%;
  }

  .product-table {
    font-size: 0.85rem;
    min-width: 650px;
  }

  .product-table th,
  .product-table td {
    padding: 10px 12px;
  }

  .column-preview {
    width: 50px;
  }

  .preview-img,
  .preview-img-placeholder {
    width: 45px;
    height: 45px;
  }
}

@media (max-width: 480px) {
  .page-header {
    margin-bottom: 1rem;
    padding-bottom: 0.5rem;
  }

  .page-header h1 {
    font-size: 1.1rem;
  }

  .page-header p {
    font-size: 0.8rem;
  }

  .section-header {
    padding: 0.6rem 0.75rem;
  }

  .section-header h2 {
    font-size: 1.1rem;
  }

  .section-container {
    padding: 0.75rem;
  }

  .search-filters {
    gap: 0.5rem;
  }

  .product-preview-container {
    max-height: 200px;
    padding: 0.4rem;
  }

  .product-preview-list {
    grid-template-columns: repeat(auto-fill, minmax(80px, 1fr));
    gap: 0.3rem;
  }

  .preview-item {
    padding: 0.25rem;
  }

  .preview-item-img {
    width: 40px;
    height: 40px;
  }

  .preview-item-img-placeholder {
    width: 40px;
    height: 40px;
    font-size: 0.7rem;
  }

  .preview-item-name {
    font-size: 0.75rem;
    width: 70px;
  }

  .preview-item-code,
  .preview-item-price {
    font-size: 0.65rem;
  }

  .add-product-form {
    gap: 0.5rem;
    margin-top: 0.75rem;
  }

  .product-table {
    font-size: 0.75rem;
    min-width: 600px;
  }

  .product-table th,
  .product-table td {
    padding: 8px 10px;
  }

  .product-table th {
    font-size: 0.7rem;
  }

  .column-preview {
    width: 40px;
  }

  .preview-img,
  .preview-img-placeholder {
    width: 35px;
    height: 35px;
    font-size: 0.65rem;
  }

  .edit-form .form-group {
    margin-bottom: 0.75rem;
  }
}
</style>
