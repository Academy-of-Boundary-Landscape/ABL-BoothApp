<template>
  <SectionCard
    title="商品列表"
    collapsible
    v-model:collapsed="isListCollapsed"
    class="list-container"
  >
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
      <n-checkbox v-model:checked="onlyMissingVisionImages" class="show-inactive-checkbox">
        <span class="checkbox-label">只看缺识别图的商品</span>
      </n-checkbox>
    </div>

    <AsyncState
      :loading="store.isLoading"
      :error="store.error"
      :empty="!filteredProducts.length"
      overlay
    >
      <div class="table-scroll">
        <table class="product-table">
          <thead>
            <tr>
              <th>图像</th>
              <th>编号</th>
              <th>名称</th>
              <th>默认价格</th>
              <th>商品分类</th>
              <th>标签</th>
              <th>识别图</th>
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
                  style="width: 80px; height: 80px"
                  :img-props="{
                    style: 'width: 100%; height: 100%; object-fit: contain; display: block;',
                  }"
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
                    style="margin: var(--space-xs)"
                  >
                    {{ tag.trim() }}
                  </n-tag>
                </template>
              </td>

              <td class="vision-cell">
                <n-tag size="small" :type="visionTagType(product.image_count)" :bordered="false">
                  {{ visionTagLabel(product.image_count) }}
                </n-tag>
              </td>

              <td class="action-cell">
                <n-button size="small" tertiary @click="$emit('edit', product)">编辑</n-button>
                <n-button
                  size="small"
                  type="info"
                  tertiary
                  @click="$emit('edit', product, 'gallery')"
                  style="margin-left: var(--space-sm)"
                  :title="'直接打开识别图 Tab'"
                >
                  识别图
                </n-button>
                <n-button
                  size="small"
                  :type="product.is_active ? 'error' : 'success'"
                  tertiary
                  @click="$emit('toggleStatus', product)"
                  style="margin-left: var(--space-sm)"
                >
                  {{ product.is_active ? '停用' : '启用' }}
                </n-button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <template #empty>
        <p v-if="hasActiveFilters">当前筛选条件下没有找到匹配的商品。</p>
        <EmptyState
          v-else
          icon="🛍️"
          title="全局商品库为空"
          desc="先在这里添加你的制品信息（名称、价格、图片），之后就能在每场展会中快速上架。"
          hint="在上方表单中创建你的第一个商品"
        />
      </template>
    </AsyncState>
  </SectionCard>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useProductStore } from '@/stores/productStore'
import { NButton, NCheckbox, NImage, NInput, NInputNumber, NSelect, NTag } from 'naive-ui'
import { AsyncState, EmptyState, SectionCard } from '@/components/ui'
import type { Schemas } from '@/api/client'

const store = useProductStore()
const route = useRoute()

defineEmits<{
  (e: 'edit', product: Schemas['MasterProduct'], initialTab?: string): void
  (e: 'toggleStatus', product: Schemas['MasterProduct']): void
}>()

const isListCollapsed = ref(false)
const selectedCategory = ref('')
const maxPrice = ref<number | null>(null)
const onlyMissingVisionImages = ref(false)

const hasActiveFilters = computed(() => {
  return Boolean(
    store.searchTerm ||
      selectedCategory.value ||
      maxPrice.value != null ||
      onlyMissingVisionImages.value
  )
})

const filteredProducts = computed(() => {
  let list = store.filteredProducts || []

  if (selectedCategory.value) {
    list = list.filter((product) => product.category === selectedCategory.value)
  }

  if (maxPrice.value != null) {
    list = list.filter((product) => Number(product.default_price ?? 0) <= Number(maxPrice.value))
  }

  if (onlyMissingVisionImages.value) {
    list = list.filter((product) => !Number(product.image_count || 0))
  }

  return list
})

function handleClearFilters() {
  store.searchTerm = ''
  selectedCategory.value = ''
  maxPrice.value = null
  onlyMissingVisionImages.value = false
}

function visionTagLabel(count: number | null | undefined) {
  const n = Number(count || 0)
  if (n === 0) return '未上传'
  return `${n} 张`
}

function visionTagType(count: number | null | undefined): 'error' | 'warning' | 'success' {
  const n = Number(count || 0)
  if (n === 0) return 'error'
  if (n < 3) return 'warning'
  return 'success'
}

// 从 /admin/master-products?filter=need_vision_images 进入时自动开启筛选
watch(
  () => route.query.filter,
  (filter) => {
    if (filter === 'need_vision_images') {
      onlyMissingVisionImages.value = true
      isListCollapsed.value = false
    }
  },
  { immediate: true }
)
</script>

<style scoped>
.list-container {
  margin-bottom: var(--space-2xl);
}

/* section 外壳样式已由 SectionCard 组件统一提供 */

.search-section {
  margin-bottom: var(--space-xl);
  padding-bottom: var(--space-xl);
  border-bottom: 1px solid var(--border-color);
}

.search-header {
  margin-bottom: var(--space-lg);
}

.search-header h3 {
  margin: 0;
  color: var(--accent-color);
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
}

.search-hint {
  margin: var(--space-sm) 0 0;
  color: var(--text-muted);
  font-size: var(--font-sm);
}

.search-box {
  display: grid;
  grid-template-columns: minmax(220px, 1.8fr) minmax(150px, 1fr) minmax(120px, 0.8fr) auto;
  align-items: center;
  gap: var(--space-md);
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
  margin-top: var(--space-lg);
  padding-top: var(--space-lg);
  border-top: 1px solid var(--border-color);
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-lg);
}

.show-inactive-checkbox {
  display: flex;
  align-items: center;
}

.checkbox-label {
  color: var(--primary-text-color);
  font-size: var(--font-base);
  margin-left: var(--space-sm);
}

.product-table {
  width: 100%;
  margin-top: 0;
  border-collapse: collapse;
  border-spacing: 0;
  text-align: left;
  font-size: var(--font-base);
  min-width: 820px;
}

.product-table th {
  padding: var(--space-md) var(--space-lg);
  background-color: var(--card-bg-color);
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
  border-bottom: 2px solid var(--accent-color);
  white-space: nowrap;
}

.product-table td {
  padding: var(--space-md) var(--space-lg);
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
.vision-cell {
  white-space: nowrap;
  text-align: center;
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

@media (--phone) {
  .search-section {
    margin-bottom: var(--space-lg);
    padding-bottom: var(--space-lg);
  }
  .search-header h3 {
    font-size: var(--font-base);
  }
  .search-hint {
    font-size: var(--font-sm);
  }
  .search-box {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--space-sm);
  }
  .clear-btn {
    justify-self: stretch;
    grid-column: 1 / -1;
  }
  .product-table {
    font-size: var(--font-sm);
    min-width: 760px;
  }
  .product-table th,
  .product-table td {
    padding: var(--space-sm) var(--space-md);
  }
  .preview-img {
    width: 60px;
    height: 60px;
  }
  .no-img {
    width: 60px;
    height: 60px;
    line-height: 60px;
    font-size: var(--font-xs);
  }
}

@media (--phone) {
  .list-container {
    margin-bottom: var(--space-xl);
  }
  .search-section {
    margin-bottom: var(--space-md);
    padding-bottom: var(--space-md);
  }
  .search-header h3 {
    font-size: var(--font-sm);
  }
  .search-hint {
    font-size: var(--font-xs);
  }
  .search-box {
    grid-template-columns: 1fr;
    gap: var(--space-sm);
  }
  .clear-btn {
    grid-column: auto;
  }
  .filter-options {
    margin-top: var(--space-md);
    padding-top: var(--space-md);
  }
  .checkbox-label {
    font-size: var(--font-sm);
  }
  .product-table {
    font-size: var(--font-xs);
    min-width: 600px;
  }
  .product-table th,
  .product-table td {
    padding: var(--space-sm);
  }
  .product-table th {
    font-size: var(--font-xs);
  }
  .preview-img {
    width: 50px;
    height: 50px;
  }
  .no-img {
    width: 50px;
    height: 50px;
    line-height: 50px;
    font-size: var(--font-xs);
  }
  .action-cell :deep(.n-button) {
    font-size: var(--font-xs);
    padding: var(--space-xs) var(--space-sm);
  }
}
</style>
