<template>
  <PageShell embedded width="full">
    <div class="prep-page__head">
      <div class="prep-page__hint-row">
        <p class="prep-page__hint">为当前展会添加、修改和移除上架商品。</p>
        <HelpBubble page="event-products" />
      </div>
      <div class="prep-page__actions">
        <n-button :disabled="!canEdit" @click="libraryOpen = true">从商品库选</n-button>
        <n-button :disabled="!canEdit" @click="importOpen = true">从上一场导入</n-button>
      </div>
    </div>

    <p v-if="!canEdit" class="prep-page__locked">展会已结算，展前编辑不可用。</p>

    <SectionCard title="已上架商品" collapsible v-model:collapsed="listCollapsed">
      <AsyncState
        :loading="eventDetailStore.isLoading"
        :error="eventDetailStore.error"
        :empty="!products.length"
        loading-text="加载已上架商品…"
        @retry="loadProducts"
      >
        <n-data-table
          :columns="columns"
          :data="products"
          :row-key="rowKey"
          :bordered="false"
          size="small"
          :scroll-x="980"
        />

        <template #empty>
          <EmptyState
            icon="📦"
            title="还没有上架商品"
            desc="从商品库选或从上一场导入，把商品加到本场展会。"
          />
        </template>
      </AsyncState>
    </SectionCard>

    <LibraryPickDrawer
      v-model:show="libraryOpen"
      :event-id="eventId"
      :existing-master-ids="existingMasterIds"
      :products="productStore.masterProducts"
      :society-names="societyNames"
      :loading="productStore.isLoading"
      :error="productStore.error"
      @imported="onImported"
    />

    <ImportFromEventDrawer
      v-model:show="importOpen"
      :event-id="eventId"
      :target-master-ids="existingMasterIds"
      :library-prices="libraryPrices"
      @imported="onImported"
    />

    <AppModal
      :show="restockOpen"
      :title="`补货 — ${activeProduct?.name ?? ''}`"
      size="sm"
      @update:show="(v) => !v && (restockOpen = false)"
    >
      <div v-if="activeProduct" class="prep-form">
        <p class="prep-form__stock">
          当前现场库存 <strong>{{ activeProduct.onsite_qty }}</strong> · 累计进货
          <strong>{{ activeProduct.stocked_qty }}</strong>
        </p>
        <label class="prep-form__label">补货数量</label>
        <n-input-number
          v-model:value="restockQty"
          :min="1"
          :precision="0"
          placeholder="请输入补货数量"
          class="prep-form__input"
        />
      </div>
      <template #footer>
        <n-space>
          <n-button @click="restockOpen = false">取消</n-button>
          <n-button type="primary" :loading="restocking" @click="doRestock">确认补货</n-button>
        </n-space>
      </template>
    </AppModal>

    <AppModal
      :show="priceOpen"
      :title="`改价 — ${activeProduct?.name ?? ''}`"
      size="sm"
      @update:show="(v) => !v && (priceOpen = false)"
    >
      <div v-if="activeProduct" class="prep-form">
        <label class="prep-form__label">展会售价（元）</label>
        <n-input-number
          v-model:value="priceYuanInput"
          :min="0"
          :precision="2"
          :step="0.01"
          placeholder="请输入售价"
          class="prep-form__input"
        />
      </div>
      <template #footer>
        <n-space>
          <n-button @click="priceOpen = false">取消</n-button>
          <n-button type="primary" :loading="savingPrice" @click="doPrice">保存更改</n-button>
        </n-space>
      </template>
    </AppModal>
  </PageShell>
</template>

<script setup lang="ts">
import { computed, h, onMounted, onUnmounted, ref, watch } from 'vue'
import { NButton, NDataTable, NImage, NInputNumber, NSpace, type DataTableColumns } from 'naive-ui'
import { AppModal, AsyncState, EmptyState, Money, PageShell, SectionCard } from '@/components/ui'
import HelpBubble from '@/components/shared/HelpBubble.vue'
import LibraryPickDrawer from '@/components/event-prep/LibraryPickDrawer.vue'
import ImportFromEventDrawer from '@/components/event-prep/ImportFromEventDrawer.vue'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import { useProductStore } from '@/stores/productStore'
import { useSocietyStore } from '@/stores/societyStore'
import { useFeedback } from '@/composables/useFeedback'
import { useWorkbenchEvent } from '@/composables/useWorkbenchEvent'
import { fromCents, toCents, type Cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

const props = defineProps<{ id: number }>()

const eventDetailStore = useEventDetailStore()
const productStore = useProductStore()
const societyStore = useSocietyStore()
const fb = useFeedback()
const { event } = useWorkbenchEvent()

/** 外壳已经加载了展会；id 用路由参数，保证外壳还没回来时也能取列表。 */
const eventId = computed(() => props.id)
const canEdit = computed(() => event.value?.status !== '已结算')

const products = computed(() => eventDetailStore.products)
const existingMasterIds = computed(() => new Set(products.value.map((p) => p.master_product_id)))

const societyNames = computed(
  () => new Map(societyStore.societies.map((s) => [s.id, s.name] as const))
)

const libraryPrices = computed(() => {
  const map = new Map<number, Cents | null>()
  for (const p of productStore.masterProducts) {
    // schema 标成必填 double，但线上确实出现过空值，防御性按可空处理。
    const raw: number | null | undefined = p.default_price
    map.set(p.id, raw === null || raw === undefined ? null : toCents(raw))
  }
  return map
})

const libraryOpen = ref(false)
const importOpen = ref(false)
const listCollapsed = ref(false)

async function loadProducts() {
  await eventDetailStore.fetchProductsForEvent(eventId.value)
}

async function onImported() {
  // 两个抽屉成功后都回到这里，刷新列表即可（抽屉自己负责关闭与 fb.success）。
  await loadProducts()
}

function productLabel(name: string | null | undefined) {
  return name ? name.slice(0, 3) : '无图'
}

function renderImage(row: Schemas['ProductEventProduct']) {
  if (row.image_url) {
    return h(NImage, {
      src: row.image_url,
      width: 44,
      height: 44,
      previewDisabled: true,
      objectFit: 'cover',
      class: 'prep-table__img',
    })
  }
  return h('div', { class: 'prep-table__placeholder' }, productLabel(row.name))
}

const activeProduct = ref<Schemas['ProductEventProduct'] | null>(null)
const restockOpen = ref(false)
const restockQty = ref<number | null>(null)
const restocking = ref(false)
const priceOpen = ref(false)
const priceYuanInput = ref<number | null>(null)
const savingPrice = ref(false)

function openRestock(row: Schemas['ProductEventProduct']) {
  activeProduct.value = row
  // 补货是「收全量」：数量不预填。
  restockQty.value = null
  restockOpen.value = true
}

function openPrice(row: Schemas['ProductEventProduct']) {
  activeProduct.value = row
  priceYuanInput.value = fromCents(row.unit_price)
  priceOpen.value = true
}

async function doRestock() {
  const product = activeProduct.value
  if (!product) return
  const qty = restockQty.value
  if (typeof qty !== 'number' || !Number.isInteger(qty) || qty <= 0) {
    fb.warning('补货数量必须是正整数')
    return
  }
  restocking.value = true
  try {
    await eventDetailStore.restockEventProduct(eventId.value, product.id, qty)
    fb.success('补货成功')
    restockOpen.value = false
  } catch (e) {
    fb.error(e, '补货失败')
  } finally {
    restocking.value = false
  }
}

async function doPrice() {
  const product = activeProduct.value
  if (!product) return
  const price = priceYuanInput.value
  if (price === null || !Number.isFinite(price)) {
    fb.warning('请输入有效的售价')
    return
  }
  savingPrice.value = true
  try {
    await eventDetailStore.updateEventProduct(product.id, { unit_price: toCents(price) })
    fb.success('售价已更新')
    priceOpen.value = false
  } catch (e) {
    fb.error(e, '改价失败')
  } finally {
    savingPrice.value = false
  }
}

async function confirmRemove(row: Schemas['ProductEventProduct']) {
  await fb.confirm({
    title: '确认下架',
    content: `确定要从该展会下架「${row.name}」吗？此操作不可恢复。`,
    positiveText: '确认下架',
    negativeText: '取消',
    danger: true,
    onConfirm: async () => {
      try {
        await eventDetailStore.deleteEventProduct(row.id)
        fb.success('已下架')
        await loadProducts()
      } catch (e) {
        fb.error(e, '下架失败')
      }
    },
  })
}

function renderActions(row: Schemas['ProductEventProduct']) {
  return h(NSpace, { size: 'small', justify: 'end', wrap: false }, () => [
    h(
      NButton,
      { size: 'small', disabled: !canEdit.value, onClick: () => openRestock(row) },
      () => '补货'
    ),
    h(
      NButton,
      { size: 'small', disabled: !canEdit.value, onClick: () => openPrice(row) },
      () => '改价'
    ),
    h(
      NButton,
      {
        size: 'small',
        type: 'error',
        quaternary: true,
        disabled: !canEdit.value,
        onClick: () => void confirmRemove(row),
      },
      () => '下架'
    ),
  ])
}

const rowKey = (row: Schemas['ProductEventProduct']) => row.id

const columns = computed<DataTableColumns<Schemas['ProductEventProduct']>>(() => [
  { key: 'image', title: '图', width: 64, render: renderImage },
  { key: 'product_code', title: '编号', width: 110 },
  { key: 'name', title: '名称', minWidth: 150 },
  { key: 'owner_society_name', title: '货主', width: 120 },
  {
    key: 'unit_price',
    title: '售价',
    width: 100,
    render: (row) => h(Money, { value: row.unit_price }),
  },
  { key: 'onsite_qty', title: '现场库存', width: 96 },
  { key: 'stocked_qty', title: '累计进货', width: 96 },
  { key: 'actions', title: '操作', width: 210, render: renderActions },
])

onMounted(() => {
  void loadProducts()
  void productStore.fetchMasterProducts()
  void societyStore.fetchSocieties()
})

watch(
  () => props.id,
  () => {
    void loadProducts()
  }
)

onUnmounted(() => {
  eventDetailStore.resetStore()
})
</script>

<style scoped>
.prep-page__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  flex-wrap: wrap;
  margin-bottom: var(--space-lg);
}

.prep-page__hint-row {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  min-width: 0;
}

.prep-page__actions {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.prep-page__hint {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--font-base);
}

.prep-page__locked {
  margin: 0 0 var(--space-lg);
  padding: var(--space-md);
  color: var(--warning-color);
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--font-sm);
}

.prep-table__img {
  border-radius: var(--radius-sm);
}

.prep-table__placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  color: var(--accent-color);
  background: var(--accent-color-light);
  border-radius: var(--radius-sm);
  font-size: var(--font-xs);
  font-weight: var(--weight-bold);
}

.prep-form__stock {
  margin: 0 0 var(--space-lg);
  color: var(--text-muted);
  font-size: var(--font-sm);
}

.prep-form__stock strong {
  color: var(--primary-text-color);
  font-variant-numeric: tabular-nums;
}

.prep-form__label {
  display: block;
  margin-bottom: var(--space-sm);
  color: var(--secondary-text-color);
  font-size: var(--font-sm);
}

.prep-form__input {
  width: 100%;
}

@media (--phone) {
  .prep-page__head {
    flex-direction: column;
    align-items: stretch;
  }

  .prep-page__actions > * {
    flex: 1;
  }
}
</style>
