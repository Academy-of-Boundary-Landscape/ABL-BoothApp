<template>
  <n-drawer
    :show="show"
    :placement="placement"
    :width="drawerWidth"
    :height="drawerHeight"
    :mask-closable="!submitting"
    @update:show="onUpdateShow"
  >
    <n-drawer-content title="从商品库选" closable>
      <div class="pick-toolbar">
        <n-input v-model:value="search" placeholder="搜索名称或编号…" clearable />
        <n-select
          v-model:value="category"
          :options="categoryOptions"
          clearable
          placeholder="全部分类"
        />
      </div>

      <AsyncState
        :loading="loading"
        :error="error"
        :empty="!filteredProducts.length"
        loading-text="加载商品库…"
      >
        <div class="pick-grid">
          <button
            v-for="p in filteredProducts"
            :key="p.id"
            type="button"
            class="pick-card"
            :class="{
              'pick-card--picked': picked.has(p.id),
              'pick-card--existing': existingMasterIds.has(p.id),
            }"
            :aria-pressed="picked.has(p.id)"
            :disabled="existingMasterIds.has(p.id)"
            @click="togglePick(p)"
          >
            <div class="pick-card__media">
              <n-image
                v-if="p.image_url"
                :src="p.image_url"
                :alt="p.name"
                class="pick-card__img"
                preview-disabled
              />
              <div v-else class="pick-card__placeholder">{{ p.name.slice(0, 2) }}</div>
            </div>
            <div class="pick-card__name">{{ p.name }}</div>
            <div class="pick-card__code">{{ p.product_code }}</div>
            <div v-if="societyLabel(p)" class="pick-card__society">{{ societyLabel(p) }}</div>
            <div class="pick-card__price">{{ libraryPriceText(p) }}</div>
            <div v-if="existingMasterIds.has(p.id)" class="pick-card__note">已在本场</div>
            <div v-else-if="picked.has(p.id)" class="pick-card__note pick-card__note--on">已选</div>
          </button>
        </div>

        <template #empty>
          <EmptyState
            icon="🔍"
            title="没有可上架的商品"
            desc="商品库里的商品要么都已经在本场，要么与当前筛选不匹配。"
          />
        </template>
      </AsyncState>

      <PendingProductsTable
        v-if="pendingItems.length"
        :items="pendingItems"
        @update:price="setPrice"
        @update:stock="setStock"
        @remove="removePicked"
      />

      <p v-if="missingPriceNames.length" class="prep-drawer__note" role="status">
        请先填写售价：{{ missingPriceNames.join('、') }}
      </p>
      <p v-if="submitError" class="prep-drawer__error" role="alert">{{ submitError }}</p>

      <template #footer>
        <div class="pick-footer">
          <span class="pick-footer__summary">已选 {{ picked.size }} 件</span>
          <n-space>
            <n-button @click="onUpdateShow(false)">取消</n-button>
            <n-button
              type="primary"
              :disabled="!picked.size || missingPriceNames.length > 0"
              :loading="submitting"
              @click="submit"
            >
              上架 {{ picked.size }} 件
            </n-button>
          </n-space>
        </div>
      </template>
    </n-drawer-content>
  </n-drawer>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { NButton, NDrawer, NDrawerContent, NImage, NInput, NSelect, NSpace } from 'naive-ui'
import { AsyncState, EmptyState } from '@/components/ui'
import PendingProductsTable from './PendingProductsTable.vue'
import { api, errorMessage, unwrap, type Schemas } from '@/api/client'
import { useFeedback } from '@/composables/useFeedback'
import { useViewport } from '@/composables/useViewport'
import { formatYuan, fromCents, toCents, type Cents } from '@/utils/money'
import {
  buildLibraryImportRequest,
  type LibraryPickItem,
  type LibraryProduct,
} from '@/utils/importPlan'

const props = defineProps<{
  show: boolean
  eventId: number
  /** 本场已上架的 master_product_id：置灰不可再选。 */
  existingMasterIds: Set<number>
  /** 商品库列表（productStore 已经映射过 image_url）。 */
  products: Schemas['MasterProduct'][]
  /** 归属社团：master 只带 owner_society_id，由页面映射成名字。 */
  societyNames: Map<number, string>
  loading?: boolean
  error?: string | null
}>()

const emit = defineEmits<{
  (e: 'update:show', v: boolean): void
  (e: 'imported'): void
}>()

const fb = useFeedback()
const { isPhone } = useViewport()

const placement = computed(() => (isPhone.value ? 'bottom' : 'right'))
const drawerWidth = computed(() => (isPhone.value ? undefined : 560))
const drawerHeight = computed(() => (isPhone.value ? '85%' : undefined))

const search = ref('')
const category = ref<string | null>(null)
const picked = ref<Set<number>>(new Set())
/** 用户填的售价（元）；null = 用商品库现价。 */
const priceYuan = ref<Map<number, number | null>>(new Map())
/** 用户填的库存；null = 没填，提交时按 0（不预填）。 */
const stockByMaster = ref<Map<number, number | null>>(new Map())
const submitError = ref('')
const submitting = ref(false)

function libraryCents(p: Schemas['MasterProduct']): Cents | null {
  // schema 把 default_price 标成必填 double，但线上确实出现过空值，防御性按可空处理。
  const raw: number | null | undefined = p.default_price
  return raw === null || raw === undefined ? null : toCents(raw)
}

function libraryPriceText(p: Schemas['MasterProduct']): string {
  const value = libraryCents(p)
  return value === null ? '--' : formatYuan(value)
}

function societyLabel(p: Schemas['MasterProduct']): string {
  if (p.owner_society_id === null || p.owner_society_id === undefined) return ''
  return props.societyNames.get(p.owner_society_id) ?? ''
}

const categoryOptions = computed(() => {
  const cats = props.products
    .map((p) => p.category)
    .filter((c): c is string => !!c && c.trim() !== '')
  return [...new Set(cats)].map((c) => ({ label: c, value: c }))
})

const filteredProducts = computed(() => {
  const q = search.value.trim().toLowerCase()
  return props.products.filter((p) => {
    if (category.value && (p.category ?? '') !== category.value) return false
    if (!q) return true
    return p.name.toLowerCase().includes(q) || p.product_code.toLowerCase().includes(q)
  })
})

const byMaster = computed(() => {
  const map = new Map<number, LibraryProduct & { libraryPrice?: Cents | null }>()
  for (const p of props.products) {
    map.set(p.id, {
      masterProductId: p.id,
      name: p.name,
      productCode: p.product_code,
      libraryPrice: libraryCents(p),
    })
  }
  return map
})

const pendingItems = computed<LibraryPickItem[]>(() => {
  const items: LibraryPickItem[] = []
  for (const id of picked.value) {
    const p = props.products.find((x) => x.id === id)
    if (!p) continue
    items.push({
      masterProductId: id,
      name: p.name,
      productCode: p.product_code,
      imageUrl: p.image_url ?? null,
      price: priceYuan.value.get(id) ?? null,
      stock: stockByMaster.value.get(id) ?? null,
      libraryPrice: libraryCents(p),
    })
  }
  return items
})

/** 商品库现价为空、用户又没填售价的行：不拦下来就会以 0 分上架。 */
const missingPriceNames = computed(() =>
  pendingItems.value
    .filter((item) => item.price === null && item.libraryPrice === null)
    .map((item) => item.name)
)

function togglePick(p: Schemas['MasterProduct']) {
  if (props.existingMasterIds.has(p.id)) return
  const next = new Set(picked.value)
  const prices = new Map(priceYuan.value)
  const stocks = new Map(stockByMaster.value)
  if (next.has(p.id)) {
    next.delete(p.id)
    prices.delete(p.id)
    stocks.delete(p.id)
  } else {
    next.add(p.id)
    const lib = libraryCents(p)
    prices.set(p.id, lib === null ? null : fromCents(lib))
    stocks.set(p.id, null)
  }
  picked.value = next
  priceYuan.value = prices
  stockByMaster.value = stocks
}

function removePicked(masterProductId: number) {
  const next = new Set(picked.value)
  next.delete(masterProductId)
  picked.value = next
}

function setPrice(masterProductId: number, value: number | null) {
  const next = new Map(priceYuan.value)
  next.set(masterProductId, value)
  priceYuan.value = next
}

function setStock(masterProductId: number, value: number | null) {
  const next = new Map(stockByMaster.value)
  next.set(masterProductId, value)
  stockByMaster.value = next
}

function onUpdateShow(v: boolean) {
  if (submitting.value) return
  emit('update:show', v)
}

function reset() {
  search.value = ''
  category.value = null
  picked.value = new Set()
  priceYuan.value = new Map()
  stockByMaster.value = new Map()
  submitError.value = ''
}

watch(
  () => props.show,
  (v) => {
    if (v) reset()
  },
  { immediate: true }
)

async function submit() {
  if (!pendingItems.value.length) return
  submitError.value = ''
  submitting.value = true
  try {
    const body = buildLibraryImportRequest(
      picked.value,
      priceYuan.value,
      stockByMaster.value,
      byMaster.value
    )
    await unwrap(
      api.POST('/events/{event_id}/products/import', {
        params: { path: { event_id: props.eventId } },
        body,
      })
    )
    fb.success(`已上架 ${body.products?.length ?? 0} 件商品`)
    emit('imported')
    emit('update:show', false)
  } catch (e) {
    // 失败不关抽屉、不清空勾选：错误原文留在抽屉里，用户改完可以直接重试。
    submitError.value = errorMessage(e, '上架失败')
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.pick-toolbar {
  display: flex;
  gap: var(--space-md);
  margin-bottom: var(--space-lg);
}

.pick-toolbar > :first-child {
  flex: 1;
}

.pick-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: var(--space-sm);
}

.pick-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-xs);
  min-height: 44px;
  padding: var(--space-sm);
  background: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  cursor: pointer;
  text-align: center;
}

.pick-card:hover:not(:disabled) {
  border-color: var(--accent-color);
}

.pick-card--picked {
  border-color: var(--accent-color);
  background: var(--accent-color-light);
}

.pick-card--existing {
  opacity: 0.5;
  cursor: not-allowed;
}

.pick-card__media {
  display: flex;
  align-items: center;
  justify-content: center;
}

.pick-card__img,
.pick-card__placeholder {
  width: 56px;
  height: 56px;
  border-radius: var(--radius-sm);
  object-fit: cover;
}

.pick-card__placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--accent-color-light);
  color: var(--accent-color);
  font-size: var(--font-sm);
  font-weight: var(--weight-bold);
}

.pick-card__name {
  width: 100%;
  overflow: hidden;
  color: var(--primary-text-color);
  font-size: var(--font-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pick-card__code {
  color: var(--text-disabled);
  font-size: var(--font-xs);
}

.pick-card__society {
  color: var(--text-muted);
  font-size: var(--font-xs);
}

.pick-card__price {
  color: var(--accent-color);
  font-size: var(--font-xs);
}

.pick-card__note {
  color: var(--text-muted);
  font-size: var(--font-xs);
}

.pick-card__note--on {
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}

.prep-drawer__note {
  margin: var(--space-lg) 0 0;
  color: var(--warning-color);
  font-size: var(--font-sm);
}

.prep-drawer__error {
  margin: var(--space-lg) 0 0;
  padding: var(--space-md);
  color: var(--error-color);
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--font-base);
}

.pick-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  width: 100%;
}

.pick-footer__summary {
  color: var(--text-muted);
  font-size: var(--font-sm);
}

@media (--phone) {
  .pick-toolbar {
    flex-direction: column;
    gap: var(--space-sm);
  }

  .pick-grid {
    grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  }
}
</style>
