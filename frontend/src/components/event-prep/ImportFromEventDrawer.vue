<template>
  <n-drawer
    :show="show"
    :placement="placement"
    :width="drawerWidth"
    :height="drawerHeight"
    :mask-closable="!submitting"
    @update:show="onUpdateShow"
  >
    <n-drawer-content title="从上一场导入" closable>
      <div class="import-toolbar">
        <span class="import-toolbar__label">源展会</span>
        <n-select
          v-model:value="sourceEventId"
          :options="sourceOptions"
          :disabled="loading"
          placeholder="选择要复制的展会"
          filterable
          @update:value="onSourceChange"
        />
      </div>

      <AsyncState
        :loading="loading"
        :error="loadError"
        :empty="sourceEventId === null"
        loading-text="加载源展会数据…"
      >
        <p class="import-hint">
          勾选要复制的商品与套装；套装会自动带上它在本场没有的候选商品。库存留空按 0。
        </p>

        <SectionCard title="商品">
          <n-data-table
            :columns="productColumns"
            :data="sourceProductList"
            :row-key="productRowKey"
            :checked-row-keys="pickedProductKeys"
            :bordered="false"
            size="small"
            :scroll-x="760"
            @update:checked-row-keys="onProductChecked"
          />
        </SectionCard>

        <SectionCard title="套装">
          <n-data-table
            :columns="lotColumns"
            :data="sourceLots"
            :row-key="lotRowKey"
            :checked-row-keys="pickedLotKeys"
            :bordered="false"
            size="small"
            :scroll-x="620"
            @update:checked-row-keys="onLotChecked"
          />
        </SectionCard>

        <p v-if="submitError" class="prep-drawer__error" role="alert">{{ submitError }}</p>

        <template #empty>
          <EmptyState
            icon="📭"
            title="没有可导入的上一场"
            desc="除了本场之外还没有别的展会，先从商品库选吧。"
          />
        </template>
      </AsyncState>

      <template #footer>
        <div class="import-footer">
          <span class="import-footer__summary">{{ summaryText }}</span>
          <n-space>
            <n-button @click="onUpdateShow(false)">取消</n-button>
            <n-button type="primary" :disabled="!canSubmit" :loading="submitting" @click="submit">
              导入
            </n-button>
          </n-space>
        </div>
      </template>
    </n-drawer-content>
  </n-drawer>
</template>

<script setup lang="ts">
import { computed, h, ref, watch } from 'vue'
import {
  NButton,
  NDataTable,
  NDrawer,
  NDrawerContent,
  NInputNumber,
  NRadioButton,
  NRadioGroup,
  NSelect,
  NSpace,
  type DataTableColumns,
} from 'naive-ui'
import { AsyncState, EmptyState, SectionCard } from '@/components/ui'
import { api, errorMessage, unwrap, type Schemas } from '@/api/client'
import { useFeedback } from '@/composables/useFeedback'
import { useViewport } from '@/composables/useViewport'
import { formatYuan, type Cents } from '@/utils/money'
import {
  buildImportRequest,
  emptyPlan,
  lotAvailability,
  priceOptions,
  toggleLot,
  toggleProduct,
  type PlanState,
  type SourceLot,
  type SourceProduct,
} from '@/utils/importPlan'

const props = defineProps<{
  show: boolean
  eventId: number
  /** 本场已有的 master_product_id：这些商品不用勾也能被套装映射到。 */
  targetMasterIds: Set<number>
  /** master_product_id → 商品库现价（分，可为空），由页面从 productStore 传下来。 */
  libraryPrices: Map<number, Cents | null>
}>()

const emit = defineEmits<{
  (e: 'update:show', v: boolean): void
  (e: 'imported'): void
}>()

const fb = useFeedback()
const { isPhone } = useViewport()

const placement = computed(() => (isPhone.value ? 'bottom' : 'right'))
const drawerWidth = computed(() => (isPhone.value ? undefined : 720))
const drawerHeight = computed(() => (isPhone.value ? '90%' : undefined))

const events = ref<Schemas['EventResponse'][]>([])
const sourceEventId = ref<number | null>(null)
const sourceProducts = ref<Schemas['ProductEventProduct'][]>([])
const rawLots = ref<Schemas['LotResponse'][]>([])
const loadingEvents = ref(false)
const loadingSource = ref(false)
const loadError = ref<string | null>(null)

const plan = ref<PlanState>(emptyPlan())
const stock = ref<Map<number, number | null>>(new Map())
const brokenLots = ref<Set<number>>(new Set())
const submitError = ref('')
const submitting = ref(false)

const loading = computed(() => loadingEvents.value || loadingSource.value)

const sourceOptions = computed(() =>
  events.value.map((e) => ({ label: `${e.name} · ${e.date}`, value: e.id }))
)

const byEp = computed(() => {
  const map = new Map<number, SourceProduct>()
  for (const p of sourceProducts.value) {
    map.set(p.id, {
      eventProductId: p.id,
      masterProductId: p.master_product_id,
      name: p.name,
      lastPrice: p.unit_price,
      stockedQty: p.stocked_qty,
      libraryPrice: props.libraryPrices.get(p.master_product_id) ?? null,
    })
  }
  return map
})

const sourceProductList = computed(() => [...byEp.value.values()])

const sourceLots = computed<SourceLot[]>(() =>
  rawLots.value.map((l) => ({
    lotId: l.id,
    name: l.name,
    candidateEventProductIds: [...l.candidate_ids],
  }))
)

const lotMeta = computed(() => {
  const map = new Map<number, { pickCount: number; allowRepeat: boolean }>()
  for (const l of rawLots.value) {
    map.set(l.id, { pickCount: l.pick_count, allowRepeat: l.allow_repeat })
  }
  return map
})

const pickedProductKeys = computed(() => [...plan.value.pickedProducts])
const pickedLotKeys = computed(() => [...plan.value.pickedLots])
const productRowKey = (row: SourceProduct) => row.eventProductId
const lotRowKey = (row: SourceLot) => row.lotId

const summaryText = computed(
  () => `导入 ${plan.value.pickedProducts.size} 件商品、${plan.value.pickedLots.size} 个套装`
)
const canSubmit = computed(
  () => plan.value.pickedProducts.size > 0 || plan.value.pickedLots.size > 0
)

function onUpdateShow(v: boolean) {
  if (submitting.value) return
  emit('update:show', v)
}

function markBroken(lotIds: Set<number>) {
  const next = new Set(brokenLots.value)
  for (const id of lotIds) next.add(id)
  brokenLots.value = next
}

function clearBroken(lotId: number) {
  if (!brokenLots.value.has(lotId)) return
  const next = new Set(brokenLots.value)
  next.delete(lotId)
  brokenLots.value = next
}

function setChoice(eventProductId: number, choice: 'last' | 'library') {
  const map = new Map(plan.value.priceChoice)
  map.set(eventProductId, choice)
  plan.value = { ...plan.value, priceChoice: map }
}

function setStock(eventProductId: number, value: number | null) {
  const map = new Map(stock.value)
  map.set(eventProductId, value)
  stock.value = map
}

function onProductChecked(keys: (string | number)[]) {
  const next = new Set(keys.map(Number))
  let state = plan.value
  for (const p of byEp.value.values()) {
    const was = state.pickedProducts.has(p.eventProductId)
    const now = next.has(p.eventProductId)
    if (now === was) continue
    const beforeLots = new Set(state.pickedLots)
    state = toggleProduct(
      p.eventProductId,
      state,
      sourceLots.value,
      props.targetMasterIds,
      byEp.value
    )
    if (!now) {
      // 取消商品会连坐取消不再可用的套装，记下来给文案用。
      markBroken(new Set([...beforeLots].filter((id) => !state.pickedLots.has(id))))
    }
  }
  plan.value = state
}

function onLotChecked(keys: (string | number)[]) {
  const next = new Set(keys.map(Number))
  let state = plan.value
  for (const lot of sourceLots.value) {
    const was = state.pickedLots.has(lot.lotId)
    const now = next.has(lot.lotId)
    if (now === was) continue
    state = toggleLot(lot, state, props.targetMasterIds, byEp.value)
    if (now) clearBroken(lot.lotId)
  }
  plan.value = state
}

function renderName(row: SourceProduct) {
  const existing = props.targetMasterIds.has(row.masterProductId)
  return h('div', { class: 'src-name' }, [
    h('span', { class: 'src-name__text' }, row.name),
    existing ? h('span', { class: 'src-name__tag' }, '已在本场') : null,
  ])
}

function renderPriceChoice(row: SourceProduct) {
  const opts = priceOptions(row)
  if (!opts.showChoice) {
    return h('span', { class: 'src-muted' }, `沿用上一场 ${formatYuan(opts.defaultPrice)}`)
  }
  return h(
    NRadioGroup,
    {
      value: plan.value.priceChoice.get(row.eventProductId) ?? 'last',
      size: 'small',
      'onUpdate:value': (v: string | number) =>
        setChoice(row.eventProductId, v === 'library' ? 'library' : 'last'),
    },
    {
      default: () => [
        h(NRadioButton, { value: 'last' }, () => '上一场'),
        h(NRadioButton, { value: 'library' }, () => '现价'),
      ],
    }
  )
}

function renderStock(row: SourceProduct) {
  return h('div', { class: 'src-stock' }, [
    h(NInputNumber, {
      value: stock.value.get(row.eventProductId) ?? null,
      min: 0,
      precision: 0,
      size: 'small',
      class: 'src-stock__input',
      placeholder: '留空按 0',
      'onUpdate:value': (v: number | null) => setStock(row.eventProductId, v),
    }),
    h('span', { class: 'src-stock__hint' }, `上一场进货 ${row.stockedQty}`),
  ])
}

const productColumns = computed<DataTableColumns<SourceProduct>>(() => [
  {
    type: 'selection',
    disabled: (row) => props.targetMasterIds.has(row.masterProductId),
  },
  { key: 'name', title: '商品', minWidth: 150, render: renderName },
  {
    key: 'lastPrice',
    title: '上一场售价',
    width: 110,
    render: (row) => h('span', {}, formatYuan(row.lastPrice)),
  },
  {
    key: 'libraryPrice',
    title: '商品库现价',
    width: 110,
    render: (row) =>
      row.libraryPrice === null
        ? h('span', { class: 'src-muted' }, '--')
        : h('span', {}, formatYuan(row.libraryPrice)),
  },
  { key: 'price', title: '本次售价', width: 200, render: renderPriceChoice },
  { key: 'stock', title: '库存', width: 180, render: renderStock },
])

function renderLotRule(row: SourceLot) {
  const meta = lotMeta.value.get(row.lotId)
  if (!meta) return ''
  return meta.allowRepeat
    ? `任选 ${meta.pickCount} 件 · 可同款`
    : `任选 ${meta.pickCount} 件 · 各 1 件`
}

function renderLotCandidates(row: SourceLot) {
  return row.candidateEventProductIds.map((id) => byEp.value.get(id)?.name ?? `#${id}`).join('、')
}

function renderLotHint(row: SourceLot) {
  const res = lotAvailability(row, plan.value, byEp.value, props.targetMasterIds)
  if (res.ok) return null
  const names = res.missing.join('、')
  if (brokenLots.value.has(row.lotId)) {
    return h('span', { class: 'src-lot__warn' }, `已取消：需要先勾选 ${names}`)
  }
  return h('span', { class: 'src-muted' }, `勾选后自动选中 ${names}`)
}

const lotColumns = computed<DataTableColumns<SourceLot>>(() => [
  { type: 'selection' },
  { key: 'name', title: '套装', minWidth: 130, render: (row) => row.name },
  { key: 'rule', title: '规则', width: 150, render: renderLotRule },
  { key: 'candidates', title: '候选商品', minWidth: 200, render: renderLotCandidates },
  { key: 'hint', title: '提示', minWidth: 160, render: renderLotHint },
])

function reset() {
  events.value = []
  sourceEventId.value = null
  sourceProducts.value = []
  rawLots.value = []
  plan.value = emptyPlan()
  stock.value = new Map()
  brokenLots.value = new Set()
  submitError.value = ''
  loadError.value = null
}

async function loadSource(id: number) {
  loadingSource.value = true
  loadError.value = null
  try {
    const [prods, lots] = await Promise.all([
      unwrap(api.GET('/events/{event_id}/products', { params: { path: { event_id: id } } })),
      unwrap(api.GET('/events/{event_id}/lots', { params: { path: { event_id: id } } })),
    ])
    sourceProducts.value = prods
    rawLots.value = lots
    plan.value = emptyPlan()
    stock.value = new Map()
    brokenLots.value = new Set()
  } catch (e) {
    sourceProducts.value = []
    rawLots.value = []
    loadError.value = errorMessage(e, '加载源展会数据失败')
  } finally {
    loadingSource.value = false
  }
}

function onSourceChange(id: number | null) {
  if (id === null) return
  sourceEventId.value = id
  void loadSource(id)
}

async function init() {
  reset()
  loadingEvents.value = true
  try {
    const list = await unwrap(api.GET('/events'))
    events.value = [...list]
      .filter((e) => e.id !== props.eventId)
      .sort((a, b) => b.date.localeCompare(a.date))
    sourceEventId.value = events.value[0]?.id ?? null
    if (sourceEventId.value !== null) await loadSource(sourceEventId.value)
  } catch (e) {
    loadError.value = errorMessage(e, '加载展会列表失败')
  } finally {
    loadingEvents.value = false
  }
}

watch(
  () => props.show,
  (v) => {
    if (v) void init()
  },
  { immediate: true }
)

async function submit() {
  if (!canSubmit.value) return
  submitError.value = ''
  submitting.value = true
  try {
    const body = buildImportRequest(plan.value, stock.value, byEp.value)
    await unwrap(
      api.POST('/events/{event_id}/products/import', {
        params: { path: { event_id: props.eventId } },
        body,
      })
    )
    fb.success(`已导入 ${body.products?.length ?? 0} 件商品、${body.lots?.length ?? 0} 个套装`)
    emit('imported')
    emit('update:show', false)
  } catch (e) {
    // 失败不关抽屉、不清空勾选：错误原文留在抽屉里，用户改完可以直接重试。
    submitError.value = errorMessage(e, '导入失败')
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.import-toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-bottom: var(--space-lg);
}

.import-toolbar__label {
  flex-shrink: 0;
  color: var(--text-muted);
  font-size: var(--font-sm);
}

.import-toolbar :deep(.n-select) {
  flex: 1;
}

.import-hint {
  margin: 0 0 var(--space-lg);
  color: var(--text-muted);
  font-size: var(--font-sm);
  line-height: var(--leading-base);
}

.src-name {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.src-name__text {
  color: var(--primary-text-color);
}

.src-name__tag {
  flex-shrink: 0;
  padding: 0 var(--space-xs);
  color: var(--text-muted);
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--font-xs);
}

.src-muted {
  color: var(--text-muted);
  font-size: var(--font-sm);
}

.src-lot__warn {
  color: var(--error-color);
  font-size: var(--font-sm);
}

.src-stock {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}

.src-stock__input {
  width: 110px;
}

.src-stock__hint {
  color: var(--text-muted);
  font-size: var(--font-xs);
}

.prep-drawer__error {
  margin: var(--space-lg) 0 0;
  padding: var(--space-md);
  color: var(--error-color);
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--font-base);
}

.import-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  width: 100%;
}

.import-footer__summary {
  color: var(--text-muted);
  font-size: var(--font-sm);
}

@media (--phone) {
  .import-footer {
    flex-direction: column;
    align-items: stretch;
  }

  .import-footer__summary {
    text-align: center;
  }
}
</style>
