<template>
  <div class="pending-products">
    <p class="pending-products__title">待上架（{{ items.length }}）</p>
    <n-data-table
      :columns="columns"
      :data="items"
      :row-key="rowKey"
      :bordered="false"
      size="small"
      :scroll-x="560"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, h } from 'vue'
import { NButton, NDataTable, NImage, NInputNumber, type DataTableColumns } from 'naive-ui'
import { formatYuan } from '@/utils/money'
import type { LibraryPickItem } from '@/utils/importPlan'

defineProps<{ items: LibraryPickItem[] }>()

const emit = defineEmits<{
  (e: 'update:price', masterProductId: number, value: number | null): void
  (e: 'update:stock', masterProductId: number, value: number | null): void
  (e: 'remove', masterProductId: number): void
}>()

const rowKey = (row: LibraryPickItem) => row.masterProductId

function renderName(row: LibraryPickItem) {
  return h('div', { class: 'pending-products__name' }, [
    row.imageUrl
      ? h(NImage, {
          src: row.imageUrl,
          width: 40,
          height: 40,
          previewDisabled: true,
          objectFit: 'cover',
          class: 'pending-products__img',
        })
      : null,
    h('div', { class: 'pending-products__meta' }, [
      h('div', { class: 'pending-products__nm' }, row.name),
      h('div', { class: 'pending-products__code' }, row.productCode),
    ]),
  ])
}

function renderPrice(row: LibraryPickItem) {
  return h(NInputNumber, {
    value: row.price,
    min: 0,
    precision: 2,
    step: 0.01,
    size: 'small',
    class: 'pending-products__input',
    placeholder:
      row.libraryPrice === null ? '请输入售价' : `商品库 ${formatYuan(row.libraryPrice)}`,
    'onUpdate:value': (v: number | null) => emit('update:price', row.masterProductId, v),
  })
}

function renderStock(row: LibraryPickItem) {
  return h(NInputNumber, {
    value: row.stock,
    min: 0,
    precision: 0,
    size: 'small',
    class: 'pending-products__input',
    placeholder: '留空按 0',
    'onUpdate:value': (v: number | null) => emit('update:stock', row.masterProductId, v),
  })
}

function renderRemove(row: LibraryPickItem) {
  return h(
    NButton,
    {
      size: 'small',
      quaternary: true,
      type: 'error',
      onClick: () => emit('remove', row.masterProductId),
    },
    () => '移除'
  )
}

const columns = computed<DataTableColumns<LibraryPickItem>>(() => [
  { key: 'name', title: '商品', minWidth: 170, render: renderName },
  { key: 'price', title: '售价（元）', width: 150, render: renderPrice },
  { key: 'stock', title: '库存', width: 150, render: renderStock },
  { key: 'actions', title: '', width: 72, render: renderRemove },
])
</script>

<style scoped>
.pending-products {
  margin-top: var(--space-lg);
}

.pending-products__title {
  margin: 0 0 var(--space-sm);
  font-size: var(--font-sm);
  font-weight: var(--weight-bold);
  color: var(--text-muted);
}

.pending-products__name {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.pending-products__img {
  border-radius: var(--radius-sm);
  flex-shrink: 0;
}

.pending-products__meta {
  min-width: 0;
}

.pending-products__nm {
  color: var(--primary-text-color);
  font-size: var(--font-base);
}

.pending-products__code {
  color: var(--text-disabled);
  font-size: var(--font-xs);
}

.pending-products__input {
  width: 100%;
}

@media (--phone) {
  .pending-products__title {
    font-size: var(--font-xs);
  }
}
</style>
