<!--
  套装的候选商品选择器：商品卡片网格 + 多选。

  「候选商品必须属于同一个货主」是后端硬校验（替别的社团让价不是摊主能单方面决定的）。
  这里把规则做进交互：选了第一件之后，别家货主的卡片置灰不可选，而不是等提交再吃一个 400。
  一件都没选时所有卡片都可选。
-->
<template>
  <div class="picker">
    <div class="picker-bar">
      <n-input
        v-model:value="keyword"
        size="small"
        clearable
        placeholder="搜索商品名或编号"
        class="picker-search"
      />
      <span class="picker-summary">
        已选 <strong>{{ modelValue.length }}</strong> 件
        <template v-if="lockedOwnerName"> · 货主：{{ lockedOwnerName }}</template>
      </span>
    </div>

    <EmptyState
      v-if="!products.length"
      compact
      title="本场还没有上架商品"
      hint="先到「展前 · 商品」上架"
    />
    <EmptyState v-else-if="!visible.length" compact title="没有匹配的商品" />

    <div v-else class="picker-grid" role="group" aria-label="候选商品">
      <button
        v-for="p in visible"
        :key="p.id"
        type="button"
        class="card"
        :class="{ 'card--selected': isSelected(p.id), 'card--blocked': isBlocked(p) }"
        :disabled="isBlocked(p)"
        :aria-pressed="isSelected(p.id)"
        :title="isBlocked(p) ? `套装只能选同一个货主的商品（当前：${lockedOwnerName}）` : p.name"
        @click="toggle(p.id)"
      >
        <span class="card-media">
          <img v-if="p.image_url" :src="p.image_url" :alt="p.name" class="card-img" />
          <span v-else class="card-initial">{{ p.name.slice(0, 1) }}</span>
          <span v-if="isSelected(p.id)" class="card-check" aria-hidden="true">✓</span>
        </span>
        <span class="card-name">{{ p.name }}</span>
        <span class="card-meta">
          <span class="card-price">{{ formatYuan(p.unit_price) }}</span>
          <span v-if="showOwner" class="card-owner">{{ p.owner_society_name }}</span>
        </span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { NInput } from 'naive-ui'
import { EmptyState } from '@/components/ui'
import type { Schemas } from '@/api/client'
import { formatYuan } from '@/utils/money'

type Product = Schemas['ProductEventProduct']

const props = defineProps<{
  modelValue: number[]
  products: Product[]
}>()
const emit = defineEmits<{ (e: 'update:modelValue', v: number[]): void }>()

const keyword = ref('')

const byId = computed(() => new Map(props.products.map((p) => [p.id, p])))

/** 已选商品决定的货主；一件都没选时为 null（所有卡片可选）。 */
const lockedOwnerId = computed<number | null>(() => {
  for (const id of props.modelValue) {
    const p = byId.value.get(id)
    if (p) return p.owner_society_id
  }
  return null
})
const lockedOwnerName = computed(() => {
  const id = lockedOwnerId.value
  if (id === null) return ''
  return props.products.find((p) => p.owner_society_id === id)?.owner_society_name ?? ''
})

/** 本场只有一个货主时，卡片上不重复写货主名。 */
const showOwner = computed(() => new Set(props.products.map((p) => p.owner_society_id)).size > 1)

const visible = computed(() => {
  const k = keyword.value.trim().toLowerCase()
  if (!k) return props.products
  return props.products.filter(
    (p) => p.name.toLowerCase().includes(k) || p.product_code.toLowerCase().includes(k)
  )
})

function isSelected(id: number) {
  return props.modelValue.includes(id)
}

function isBlocked(p: Product) {
  return (
    !isSelected(p.id) && lockedOwnerId.value !== null && p.owner_society_id !== lockedOwnerId.value
  )
}

function toggle(id: number) {
  const p = byId.value.get(id)
  if (!p || isBlocked(p)) return
  emit(
    'update:modelValue',
    isSelected(id) ? props.modelValue.filter((x) => x !== id) : [...props.modelValue, id]
  )
}
</script>

<style scoped>
.picker-bar {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-bottom: var(--space-md);
}

.picker-search {
  flex: 1;
  min-width: 0;
}

.picker-summary {
  flex-shrink: 0;
  color: var(--secondary-text-color);
  font-size: var(--font-sm);
}

.picker-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--space-sm);
}

.card {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  padding: var(--space-xs);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--card-bg-color);
  color: var(--primary-text-color);
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition: border-color 0.15s ease;
}

.card:hover:not(:disabled) {
  border-color: var(--accent-color);
}

.card:focus-visible {
  outline: 2px solid var(--accent-color);
  outline-offset: 2px;
}

.card--selected {
  border-color: var(--accent-color);
  /* 选中态加粗描边：用 outline 叠一圈，不改 border 宽度（避免卡片跳动） */
  outline: 1px solid var(--accent-color);
  background: color-mix(in srgb, var(--accent-color) 6%, var(--card-bg-color));
}

.card--blocked {
  opacity: 0.4;
  cursor: not-allowed;
}

.card-media {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  aspect-ratio: 1;
  overflow: hidden;
  border-radius: var(--radius-sm);
  background: var(--bg-color);
}

.card-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.card-initial {
  color: var(--text-muted);
  font-size: var(--font-xl);
}

.card-check {
  position: absolute;
  top: var(--space-xs);
  right: var(--space-xs);
  display: flex;
  align-items: center;
  justify-content: center;
  width: var(--space-xl);
  height: var(--space-xl);
  border-radius: var(--radius-pill);
  background: var(--accent-color);
  color: var(--text-white);
  font-size: var(--font-sm);
  font-weight: var(--weight-bold);
}

.card-name {
  overflow: hidden;
  font-size: var(--font-sm);
  line-height: var(--leading-tight);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-meta {
  display: flex;
  justify-content: space-between;
  gap: var(--space-xs);
  font-size: var(--font-xs);
}

.card-price {
  flex-shrink: 0;
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}

.card-owner {
  overflow: hidden;
  color: var(--text-muted);
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (--phone) {
  .picker-grid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .picker-bar {
    flex-wrap: wrap;
  }
}
</style>
