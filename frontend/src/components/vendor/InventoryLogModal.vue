<!--
  赠送 / 报废登记弹窗。

  商品候选来自 `GET /events/:id/closing` 的 `onsite_remaining`——**复用它而不是
  另开端点**，那个接口本来就在算这笔余额。只列余额 > 0 的。

  数量上限只是「少一次往返」的前置提示，真正判超卖的是后端；后端说不行就把
  它那句话原样显示出来。
-->
<template>
  <AppModal :show="show" @close="emit('close')">
    <template #header><h3>登记赠送 / 报废</h3></template>
    <template #body>
      <n-tabs v-model:value="activeTab" type="line" animated>
        <n-tab-pane name="gift" tab="赠送" />
        <n-tab-pane name="scrap" tab="报废" />
      </n-tabs>

      <n-spin :show="isLoading">
        <div class="log-form">
          <div class="field">
            <span class="field-label">商品</span>
            <n-select
              v-model:value="form.productId"
              :options="productOptions"
              filterable
              placeholder="只列现场仓还有余额的商品"
              :disabled="!productOptions.length"
            />
          </div>
          <p v-if="!isLoading && !productOptions.length" class="empty-hint">
            现场仓没有可登记的商品。
          </p>

          <div class="field">
            <span class="field-label">数量</span>
            <n-input-number
              v-model:value="form.qty"
              :min="1"
              :max="maxQty"
              :precision="0"
              :disabled="!form.productId"
            />
            <span v-if="selectedProduct" class="field-hint"
              >现场仓 {{ selectedProduct.qty }} 件</span
            >
          </div>

          <div class="field">
            <span class="field-label">备注</span>
            <n-input v-model:value="form.note" placeholder="可不填" />
          </div>

          <div v-if="activeTab === 'gift'" class="field switch-field">
            <n-switch v-model:value="form.vendorPays" />
            <div class="switch-text">
              <span>这笔我自掏（按原价补给货主）</span>
              <span class="field-hint"> 不开 = 货主自己承担，结算单上只会显示送了几件 </span>
            </div>
          </div>

          <div class="actions">
            <n-button type="primary" :loading="isBusy" @click="submit">
              {{ activeTab === 'gift' ? '登记赠送' : '登记报废' }}
            </n-button>
          </div>
        </div>
      </n-spin>

      <div class="entries">
        <p class="entries-title">已登记 · {{ activeTab === 'gift' ? '赠送' : '报废' }}</p>
        <p v-if="!entries.length" class="empty-hint">还没有登记记录。</p>
        <div v-for="entry in entries" :key="entry.journal_id" class="entry">
          <div class="entry-main">
            <span class="entry-name">{{ entry.name }}</span>
            <span class="entry-code">{{ entry.product_code }}</span>
            <span class="entry-qty">×{{ entry.qty }}</span>
            <n-tag v-if="activeTab === 'gift'" size="small" :bordered="false">
              {{ entry.vendor_paid ? '我自掏' : '货主承担' }}
            </n-tag>
          </div>
          <div class="entry-meta">
            <span>{{ entry.owner_name }}</span>
            <span>{{ formatTimestamp(entry.occurred_at, false) }}</span>
            <span v-if="entry.note" class="entry-note">{{ entry.note }}</span>
          </div>
          <n-button size="small" quaternary :disabled="isBusy" @click="undo(entry)">撤销</n-button>
        </div>
      </div>
    </template>
  </AppModal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import {
  NButton,
  NInput,
  NInputNumber,
  NSelect,
  NSpin,
  NSwitch,
  NTabs,
  NTabPane,
  NTag,
  useDialog,
  useMessage,
} from 'naive-ui'
import AppModal from '@/components/shared/AppModal.vue'
import { useInventoryLogStore } from '@/stores/inventoryLogStore'
import { formatTimestamp } from '@/utils/dateFormatter'
import { api, unwrap, errorMessage, type Schemas } from '@/api/client'

const props = withDefaults(defineProps<{ show?: boolean; eventId: string | number }>(), {
  show: false,
})
const emit = defineEmits<{ (e: 'close'): void; (e: 'logged'): void }>()

const store = useInventoryLogStore()
const message = useMessage()
const dialog = useDialog()

const activeTab = ref('gift')
const onsite = ref<Schemas['ClosingOnSiteRow'][]>([])
const isLoading = ref(false)
const isBusy = ref(false)
const form = ref<{
  productId: number | null
  qty: number | null
  note: string
  vendorPays: boolean
}>({ productId: null, qty: 1, note: '', vendorPays: false })

const productOptions = computed(() =>
  onsite.value.map((p) => ({
    label: `${p.name}（${p.owner_name} · 现场仓 ${p.qty}）`,
    value: p.event_product_id,
  }))
)

const selectedProduct = computed(() =>
  onsite.value.find((p) => p.event_product_id === form.value.productId)
)
const maxQty = computed(() => selectedProduct.value?.qty || 1)

const entries = computed(() => (activeTab.value === 'gift' ? store.gifts : store.scraps))

function resetForm() {
  form.value = { productId: null, qty: 1, note: '', vendorPays: false }
}

/** 只重拉现场仓余额：登记 / 撤销之后用它刷新下拉，别留一个过期的账面数。 */
async function loadOnsite() {
  try {
    const data = await unwrap(
      api.GET('/events/{event_id}/closing', {
        params: { path: { event_id: Number(props.eventId) } },
      })
    )
    onsite.value = (data?.onsite_remaining || []).filter((p) => p.qty > 0)
  } catch (err) {
    onsite.value = []
    message.error(errorMessage(err, '无法加载现场仓余额。'))
  }
}

async function load() {
  isLoading.value = true
  await loadOnsite()
  await Promise.all([
    store.fetchGifts(Number(props.eventId)),
    store.fetchScraps(Number(props.eventId)),
  ])
  isLoading.value = false
}

watch(
  () => props.show,
  (val) => {
    if (val) {
      activeTab.value = 'gift'
      resetForm()
      load()
    }
  }
)

// 换商品或换 tab 都把数量拉回 1：上一个商品的件数套到新商品上没有意义。
watch(
  () => [form.value.productId, activeTab.value],
  () => {
    form.value.qty = 1
  }
)

async function submit() {
  const productId = form.value.productId
  if (!productId) return message.warning('请选择商品')
  const qty = form.value.qty
  if (qty === null || !Number.isFinite(qty) || qty <= 0) {
    return message.warning('数量必须为正')
  }
  if (qty > maxQty.value) {
    // 只是少一次往返；真正的余额判据在后端事务里。
    return message.warning(`现场仓只剩 ${maxQty.value} 件`)
  }

  const payload: Schemas['InventoryLogRequest'] = { event_product_id: productId, qty }
  if (form.value.note.trim()) payload.note = form.value.note.trim()

  isBusy.value = true
  try {
    if (activeTab.value === 'gift') {
      payload.vendor_pays = form.value.vendorPays
      await store.logGift(Number(props.eventId), payload)
      message.success('已登记赠送')
    } else {
      await store.logScrap(Number(props.eventId), payload)
      message.success('已登记报废')
    }
    resetForm()
    // 账面数变了：刷新下拉，别让已经送光的商品还挂着旧余额。
    await loadOnsite()
    // 通知外面刷新库存 / 统计。
    emit('logged')
  } catch (err) {
    message.error((err instanceof Error && err.message) || '操作失败')
  } finally {
    isBusy.value = false
  }
}

function undo(entry: Schemas['InventoryLogEntry']) {
  dialog.warning({
    title: '确认撤销',
    content: `撤销这条${activeTab.value === 'gift' ? '赠送' : '报废'}登记？货会回到现场仓。`,
    positiveText: '撤销',
    negativeText: '返回',
    async onPositiveClick() {
      isBusy.value = true
      try {
        await store.reverse(Number(props.eventId), entry.journal_id)
        message.success('已撤销')
        await loadOnsite()
        emit('logged')
      } catch (err) {
        message.error((err instanceof Error && err.message) || '撤销失败')
      } finally {
        isBusy.value = false
      }
    },
  })
}
</script>

<style scoped>
.log-form {
  margin-top: 0.75rem;
}
.field {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}
.field-label {
  flex: 0 0 3.5em;
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.field > .n-select,
.field > .n-input,
.field > .n-input-number {
  flex: 1;
  min-width: 0;
}
.field-hint {
  color: var(--text-muted);
  font-size: var(--font-sm);
  white-space: nowrap;
}
.switch-field {
  align-items: flex-start;
}
.switch-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: var(--font-sm);
}
.actions {
  margin: 0.25rem 0 1rem;
}
.empty-hint {
  margin: 0.25rem 0;
  color: var(--text-muted);
  font-size: var(--font-sm);
}

.entries {
  border-top: 1px solid var(--border-color);
  padding-top: 0.75rem;
}
.entries-title {
  margin: 0 0 0.5rem;
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.entry {
  display: grid;
  grid-template-columns: 1fr auto;
  grid-template-areas: 'main action' 'meta action';
  gap: 4px 8px;
  align-items: center;
  padding: 0.4rem 0;
  border-bottom: 1px dashed var(--border-color);
}
.entry-main {
  grid-area: main;
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.entry-name {
  font-weight: 600;
}
.entry-code {
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.entry-qty {
  color: var(--accent-color);
  font-weight: 600;
}
.entry-meta {
  grid-area: meta;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.entry-note {
  color: var(--warning-color);
}
.entry > .n-button {
  grid-area: action;
}
</style>
