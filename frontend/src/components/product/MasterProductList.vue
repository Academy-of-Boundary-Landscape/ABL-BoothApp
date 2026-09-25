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
      <n-data-table
        :columns="columns"
        :data="filteredProducts"
        :row-key="(row) => row.id"
        :row-class-name="rowClassName"
        :scroll-x="1020"
        size="small"
      />

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
import { computed, ref, watch, h } from 'vue'
import { useRoute } from 'vue-router'
import { useProductStore } from '@/stores/productStore'
import {
  NButton,
  NCheckbox,
  NDataTable,
  NImage,
  NInput,
  NInputNumber,
  NSelect,
  NTag,
  type DataTableColumns,
} from 'naive-ui'
import { AsyncState, EmptyState, SectionCard } from '@/components/ui'
import type { Schemas } from '@/api/client'

const store = useProductStore()
const route = useRoute()

const emit = defineEmits<{
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

function rowClassName(product: Schemas['MasterProduct']) {
  return product.is_active ? '' : 'inactive'
}

const columns: DataTableColumns<Schemas['MasterProduct']> = [
  {
    title: '图像',
    key: 'image_url',
    width: 92,
    render: (product) =>
      product.image_url
        ? h(NImage, {
            src: product.image_url,
            alt: product.name,
            previewDisabled: true,
            class: 'preview-img',
            imgProps: {
              style: 'width: 100%; height: 100%; object-fit: contain; display: block;',
            },
          })
        : h('span', { class: 'no-img' }, '无图'),
  },
  { title: '编号', key: 'product_code', width: 120 },
  { title: '名称', key: 'name', minWidth: 160 },
  {
    title: '默认价格',
    key: 'default_price',
    width: 110,
    render: (product) => `¥${Number(product.default_price ?? 0).toFixed(2)}`,
  },
  {
    title: '商品分类',
    key: 'category',
    width: 120,
    render: (product) => product.category || '未分类',
  },
  {
    title: '标签',
    key: 'tags',
    minWidth: 160,
    render: (product) => {
      const tags = (product.tags || '')
        .split(',')
        .map((tag) => tag.trim())
        .filter(Boolean)
      if (!tags.length) return ''
      return h(
        'div',
        { class: 'tags-cell' },
        tags.map((tag) =>
          h(
            NTag,
            { key: tag, size: 'small', bordered: false, type: 'info' },
            { default: () => tag }
          )
        )
      )
    },
  },
  {
    title: '识别图',
    key: 'image_count',
    width: 100,
    align: 'center',
    render: (product) =>
      h(
        NTag,
        { size: 'small', type: visionTagType(product.image_count), bordered: false },
        { default: () => visionTagLabel(product.image_count) }
      ),
  },
  {
    title: '操作',
    key: 'actions',
    width: 230,
    align: 'right',
    render: (product) =>
      h('div', { class: 'action-cell' }, [
        h(
          NButton,
          { size: 'small', tertiary: true, onClick: () => emit('edit', product) },
          { default: () => '编辑' }
        ),
        h(
          NButton,
          {
            size: 'small',
            type: 'info',
            tertiary: true,
            title: '直接打开识别图 Tab',
            onClick: () => emit('edit', product, 'gallery'),
          },
          { default: () => '识别图' }
        ),
        h(
          NButton,
          {
            size: 'small',
            type: product.is_active ? 'error' : 'success',
            tertiary: true,
            onClick: () => emit('toggleStatus', product),
          },
          { default: () => (product.is_active ? '停用' : '启用') }
        ),
      ]),
  },
]

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

.tags-cell {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-xs);
}

.action-cell {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-sm);
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

:deep(.inactive) {
  opacity: 0.5;
  background-color: var(--bg-elevated);
}

:deep(.inactive td) {
  text-decoration: line-through;
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
