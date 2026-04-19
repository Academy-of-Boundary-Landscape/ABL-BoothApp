<template>
  <div class="list-container">
    <div class="section-header" @click="isListCollapsed = !isListCollapsed">
      <h2>商品列表</h2>
      <n-button text class="toggle-btn">
        {{ isListCollapsed ? '展开' : '折叠' }}
      </n-button>
    </div>

    <transition name="expand">
      <div v-show="!isListCollapsed" class="section-container">
        <div class="search-section">
          <div class="search-header">
            <h3>搜索和过滤</h3>
            <p class="search-hint">按关键词、分类或默认价格快速筛选商品</p>
          </div>

          <div class="search-box">
            <n-input
              v-model:value="store.searchTerm"
              placeholder="搜索商品名称或编号..."
              clearable
              class="search-input"
            />
            <n-select
              v-model:value="selectedCategory"
              :options="store.categoryOptions"
              clearable
              placeholder="选择分类"
              class="category-select"
            />
            <n-input-number
              v-model:value="maxPrice"
              :show-button="false"
              :precision="2"
              :min="0"
              placeholder="最高价格"
              class="price-filter"
            />
            <n-button
              tertiary
              class="clear-btn"
              @click="handleClearFilters"
              v-if="store.searchTerm || selectedCategory || maxPrice != null"
            >
              清空
            </n-button>
          </div>
        </div>

        <div class="filter-options">
          <n-checkbox
            v-model:checked="store.showInactive"
            @update:checked="store.fetchMasterProducts()"
            class="show-inactive-checkbox"
          >
            <span class="checkbox-label">显示已停用的商品</span>
          </n-checkbox>
        </div>

        <n-spin :show="store.isLoading">
          <div v-if="store.error" class="error-message">{{ store.error }}</div>

          <div v-else-if="filteredProducts.length" class="table-wrapper">
            <table class="product-table">
              <thead>
                <tr>
                  <th>图像</th>
                  <th>编号</th>
                  <th>名称</th>
                  <th>默认价格</th>
                  <th>商品分类</th>
                  <th>标签</th>
                  <th>操作</th>
                </tr>
              </thead>

              <tbody>
                <tr
                  v-for="product in filteredProducts"
                  :key="product.id"
                  :class="{ inactive: !product.is_active }"
                >
                  <td>
                    <n-image
                      v-if="product.image_url"
                      :src="product.image_url"
                      :alt="product.name"
                      class="preview-img"
                      preview-disabled
                      style="width: 80px; height: 80px;"
                      :img-props="{ style: 'width: 100%; height: 100%; object-fit: contain; display: block;' }"
                    />
                    <span v-else class="no-img">无图</span>
                  </td>

                  <td>{{ product.product_code }}</td>
                  <td>{{ product.name }}</td>
                  <td>¥{{ Number(product.default_price ?? 0).toFixed(2) }}</td>
                  <td>{{ product.category || '未分类' }}</td>
                  <td class="tags-cell">
                    <template v-if="product.tags">
                      <n-tag
                        v-for="tag in product.tags.split(',').filter(Boolean)"
                        :key="tag"
                        size="small"
                        :bordered="false"
                        type="info"
                        style="margin: 2px;"
                      >
                        {{ tag.trim() }}
                      </n-tag>
                    </template>
                  </td>

                  <td class="action-cell">
                    <n-button size="small" tertiary @click="$emit('edit', product)">编辑</n-button>
                    <n-button
                      size="small"
                      :type="product.is_active ? 'error' : 'success'"
                      tertiary
                      @click="$emit('toggleStatus', product)"
                      style="margin-left: 8px;"
                    >
                      {{ product.is_active ? '停用' : '启用' }}
                    </n-button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <p v-else-if="hasActiveFilters">
            当前筛选条件下没有找到匹配的商品。
          </p>
          <div v-else class="empty-guide">
            <div class="empty-guide-icon">🛍️</div>
            <div class="empty-guide-title">全局商品库为空</div>
            <div class="empty-guide-desc">先在这里添加你的制品信息（名称、价格、图片），之后就能在每场展会中快速上架。</div>
            <div class="empty-guide-hint">在上方表单中创建你的第一个商品</div>
          </div>
        </n-spin>
      </div>
    </transition>
  </div>
</template>

<script setup>
import { computed, ref } from 'vue'
import { useProductStore } from '@/stores/productStore'
import { NButton, NCheckbox, NImage, NInput, NInputNumber, NSelect, NSpin, NTag } from 'naive-ui'

const store = useProductStore()

defineEmits(['edit', 'toggleStatus'])

const isListCollapsed = ref(false)
const selectedCategory = ref('')
const maxPrice = ref(null)

const hasActiveFilters = computed(() => {
  return Boolean(store.searchTerm || selectedCategory.value || maxPrice.value != null)
})

const filteredProducts = computed(() => {
  let list = store.filteredProducts || []

  if (selectedCategory.value) {
    list = list.filter((product) => product.category === selectedCategory.value)
  }

  if (maxPrice.value != null) {
    list = list.filter((product) => Number(product.default_price ?? 0) <= Number(maxPrice.value))
  }

  return list
})

function handleClearFilters() {
  store.searchTerm = ''
  selectedCategory.value = ''
  maxPrice.value = null
}
</script>

<style scoped>
.list-container {
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
  border-radius: var(--radius-md);
  transition: all 0.2s ease;
  margin-bottom: 0.5rem;
}

.section-header:hover {
  background: var(--hover-bg-color, var(--card-bg-color));
  border-color: var(--accent-color);
}

.section-header h2 {
  margin: 0;
  font-size: var(--font-lg);
  color: var(--accent-color);
  font-weight: 600;
}

.toggle-btn {
  font-size: var(--font-base);
  padding: 0.25rem 0.75rem;
  min-width: auto;
  color: var(--accent-color);
}

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
  border-radius: var(--radius-md);
  padding: 1.5rem;
}

.search-section {
  margin-bottom: 1.5rem;
  padding-bottom: 1.5rem;
  border-bottom: 1px solid var(--border-color);
}

.search-header {
  margin-bottom: 1rem;
}

.search-header h3 {
  margin: 0;
  color: var(--accent-color);
  font-size: var(--font-md);
  font-weight: 600;
}

.search-hint {
  margin: 0.5rem 0 0;
  color: var(--text-muted);
  font-size: var(--font-sm);
}

.search-box {
  display: grid;
  grid-template-columns: minmax(220px, 1.8fr) minmax(150px, 1fr) minmax(120px, 0.8fr) auto;
  align-items: center;
  gap: 12px;
}

.search-input,
.category-select,
.price-filter {
  min-width: 0;
}

.price-filter :deep(.n-input-number) {
  width: 100%;
}

.clear-btn {
  justify-self: end;
}

.filter-options {
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid var(--border-color);
}

.show-inactive-checkbox {
  display: flex;
  align-items: center;
}

.checkbox-label {
  color: var(--primary-text-color);
  font-size: var(--font-base);
  margin-left: 0.5rem;
}

.table-wrapper {
  width: 100%;
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
}

.product-table {
  width: 100%;
  margin-top: 0;
  border-collapse: collapse;
  border-spacing: 0;
  text-align: left;
  font-size: var(--font-base);
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
  color: var(--secondary-text-color);
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

.action-cell {
  white-space: nowrap;
}

.preview-img {
  width: 64px;
  height: 64px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-color);
  vertical-align: middle;
}

:deep(.preview-img img) {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
  background: var(--bg-color);
}

.no-img {
  display: inline-block;
  width: 50px;
  height: 50px;
  line-height: 50px;
  text-align: center;
  font-size: var(--font-sm);
  color: var(--text-disabled);
  background-color: var(--bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  vertical-align: middle;
}

.inactive {
  opacity: 0.5;
  background-color: var(--bg-elevated);
}

.inactive td {
  text-decoration: line-through;
}

@media (max-width: 768px) {
  .section-container { padding: 1rem; }
  .search-section { margin-bottom: 1rem; padding-bottom: 1rem; }
  .search-header h3 { font-size: 0.95rem; }
  .search-hint { font-size: 0.8rem; }
  .search-box {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 8px;
  }
  .clear-btn {
    justify-self: stretch;
    grid-column: 1 / -1;
  }
  .product-table { font-size: 0.85rem; min-width: 650px; }
  .product-table th, .product-table td { padding: 10px 12px; }
  .preview-img { width: 60px; height: 60px; }
  .no-img { width: 60px; height: 60px; line-height: 60px; font-size: 0.75rem; }
}

@media (max-width: 480px) {
  .list-container { margin-bottom: 1.5rem; }
  .section-header { padding: 0.6rem 0.75rem; }
  .section-header h2 { font-size: 1.1rem; }
  .section-container { padding: 0.75rem; }
  .search-section { margin-bottom: 0.75rem; padding-bottom: 0.75rem; }
  .search-header h3 { font-size: 0.9rem; }
  .search-hint { font-size: 0.75rem; }
  .search-box {
    grid-template-columns: 1fr;
    gap: 6px;
  }
  .clear-btn {
    grid-column: auto;
  }
  .filter-options { margin-top: 0.75rem; padding-top: 0.75rem; }
  .checkbox-label { font-size: 0.85rem; }
  .product-table { font-size: 0.75rem; min-width: 600px; }
  .product-table th, .product-table td { padding: 8px 10px; }
  .product-table th { font-size: 0.7rem; }
  .preview-img { width: 50px; height: 50px; }
  .no-img { width: 50px; height: 50px; line-height: 50px; font-size: 0.7rem; }
  .action-cell :deep(.n-button) { font-size: 0.75rem; padding: 4px 8px; }
}

.empty-guide {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 3rem 2rem;
  text-align: center;
}
.empty-guide-icon {
  font-size: 3rem;
  margin-bottom: 12px;
  opacity: 0.3;
}
.empty-guide-title {
  font-size: var(--font-lg);
  font-weight: 700;
  color: var(--primary-text-color);
  margin-bottom: 8px;
}
.empty-guide-desc {
  font-size: var(--font-base);
  color: var(--text-muted);
  max-width: 400px;
  line-height: 1.6;
  margin-bottom: 16px;
}
.empty-guide-hint {
  font-size: var(--font-sm);
  color: var(--accent-color);
  font-weight: 600;
}
</style>
