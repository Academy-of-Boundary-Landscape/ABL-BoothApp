<template>
  <div class="page">
    <header class="page-header">
      <h1>套装与优惠</h1>
      <p>
        套装 = 从一组候选商品里任选 N 件，按一个总价卖。「全套 5 本 100」是候选集正好 5 件的特例。
        <strong>候选商品必须属于同一个货主</strong>——替别的社团让价不是摊主能单方面决定的。
      </p>
    </header>

    <main class="page-body">
      <CollapsibleSection title="新建套装" v-model:collapsed="isFormCollapsed" class="form-section">
        <div class="form-grid">
          <n-input v-model:value="form.name" placeholder="套装名称，如「本子任选3本100」" />
          <n-input-number
            v-model:value="form.pickCount"
            :min="1"
            :precision="0"
            placeholder="要选几件"
          />
          <n-input-number
            v-model:value="form.priceYuan"
            :min="0"
            :precision="2"
            placeholder="总价（元）"
          />
          <n-select
            v-model:value="form.candidateIds"
            multiple
            filterable
            :options="candidateOptions"
            placeholder="候选商品"
            class="candidates"
          />
          <n-button type="primary" :disabled="isBusy" @click="handleSubmit">
            {{ editingId ? '保存修改' : '新建' }}
          </n-button>
          <n-button v-if="editingId" quaternary @click="resetForm">取消编辑</n-button>
        </div>
      </CollapsibleSection>

      <div v-if="store.isLoading" class="loading-message">正在加载套装列表...</div>
      <div v-else-if="store.error" class="error-message">{{ store.error }}</div>
      <EmptyGuide
        v-else-if="!store.lots.length"
        title="还没有套装"
        hint="配一个套装，顾客的购物车就会自动套用最省的那一种。"
      />

      <div v-else class="table-wrapper">
        <table class="lot-table">
          <thead>
            <tr>
              <th>名称</th>
              <th>任选</th>
              <th>总价</th>
              <th>货主</th>
              <th>候选商品</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="lot in store.lots" :key="lot.id">
              <td>{{ lot.name }}</td>
              <td>{{ lot.pick_count }} 件</td>
              <td>{{ formatYuan(lot.total_price) }}</td>
              <td>{{ lot.owner_society_name }}</td>
              <td class="candidates-cell">{{ candidateNames(lot) }}</td>
              <td>
                <n-space size="small" justify="end">
                  <n-button size="small" :disabled="isBusy" @click="startEdit(lot)">编辑</n-button>
                  <n-button
                    size="small"
                    type="error"
                    quaternary
                    :disabled="isBusy"
                    @click="handleDelete(lot)"
                  >
                    删除
                  </n-button>
                </n-space>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </main>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { NInput, NInputNumber, NSelect, NButton, NSpace, useDialog, useMessage } from 'naive-ui'
import CollapsibleSection from '@/components/shared/CollapsibleSection.vue'
import EmptyGuide from '@/components/shared/EmptyGuide.vue'
import { useLotStore } from '@/stores/lotStore'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import { formatYuan, toCents, fromCents } from '@/utils/money'

const props = defineProps({ id: { type: [String, Number], required: true } })

const store = useLotStore()
const eventDetailStore = useEventDetailStore()
const dialog = useDialog()
const message = useMessage()

const isFormCollapsed = ref(false)
const isBusy = ref(false)
const editingId = ref(null)
const form = ref({ name: '', pickCount: 1, priceYuan: null, candidateIds: [] })

// 选项标签带上货主名：候选集必须同一货主是后端硬校验，把货主写在标签上
// 能让摊主在点选时就看出来，而不是提交后才吃一个 400。
const candidateOptions = computed(() =>
  eventDetailStore.products.map((p) => ({
    label: `${p.name}（${p.owner_society_name} · ${formatYuan(p.unit_price)}）`,
    value: p.id,
  }))
)

function candidateNames(lot) {
  if (!lot.candidate_ids.length) return '（候选已被删除）'
  const byId = new Map(eventDetailStore.products.map((p) => [p.id, p.name]))
  return lot.candidate_ids.map((id) => byId.get(id) || `#${id}`).join('、')
}

function resetForm() {
  editingId.value = null
  form.value = { name: '', pickCount: 1, priceYuan: null, candidateIds: [] }
}

function startEdit(lot) {
  editingId.value = lot.id
  form.value = {
    name: lot.name,
    pickCount: lot.pick_count,
    priceYuan: fromCents(lot.total_price),
    candidateIds: [...lot.candidate_ids],
  }
  isFormCollapsed.value = false
}

async function handleSubmit() {
  const name = form.value.name.trim()
  if (!name) return message.warning('请填写套装名称')
  if (!Number.isFinite(form.value.pickCount) || form.value.pickCount < 1)
    return message.warning('「要选几件」至少是 1')
  if (!form.value.candidateIds.length) return message.warning('请至少选一个候选商品')
  if (form.value.priceYuan === null) return message.warning('请填写总价')

  const payload = {
    name,
    pick_count: form.value.pickCount,
    total_price: toCents(form.value.priceYuan),
    candidate_ids: form.value.candidateIds,
  }
  isBusy.value = true
  try {
    if (editingId.value) {
      await store.updateLot(props.id, editingId.value, payload)
      message.success('套装已更新')
    } else {
      await store.createLot(props.id, payload)
      message.success('套装已新建')
    }
    resetForm()
  } catch (error) {
    message.error(error.message || '操作失败')
  } finally {
    isBusy.value = false
  }
}

function handleDelete(lot) {
  dialog.warning({
    title: '确认删除',
    // 快照的存在是这句话成立的理由，不是安慰剧。
    content: `删除套装「${lot.name}」？已经下过的订单不受影响——它们存的是名字和价格的快照。`,
    positiveText: '确认删除',
    negativeText: '取消',
    async onPositiveClick() {
      isBusy.value = true
      try {
        await store.deleteLot(props.id, lot.id)
        message.success('套装已删除')
      } catch (error) {
        message.error(error.message || '删除失败')
      } finally {
        isBusy.value = false
      }
    },
  })
}

onMounted(async () => {
  await eventDetailStore.fetchProductsForEvent(props.id)
  await store.fetchLots(props.id)
})
// 从 A 展会的套装页跳到 B 展会时，先渲染的是上一场的套装（候选名还会显示成 #id），
// 直到 fetchLots 返回。离开就清掉，别让旧展会的数据越界显示。
onUnmounted(() => store.resetStore())
</script>

<style scoped>
.page {
  max-width: 960px;
}
.page-header {
  margin-bottom: 1.5rem;
}
.page-header h1 {
  margin: 0 0 0.25rem;
  font-size: var(--font-xl);
  color: var(--accent-color);
}
.page-header p {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--font-base);
  line-height: 1.6;
}

.form-section {
  margin-bottom: 1.5rem;
}
.form-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: center;
}
.form-grid > * {
  flex: 1 1 160px;
  min-width: 0;
}
.candidates {
  flex: 2 1 320px;
}

.table-wrapper {
  width: 100%;
  overflow-x: auto;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
}
.lot-table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
  font-size: var(--font-base);
}
.lot-table th {
  padding: 12px 16px;
  background-color: var(--card-bg-color);
  color: var(--primary-text-color);
  font-weight: 600;
  border-bottom: 2px solid var(--accent-color);
  white-space: nowrap;
}
.lot-table td {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
  color: var(--text-placeholder);
  vertical-align: middle;
}
.lot-table th:last-child,
.lot-table td:last-child {
  text-align: right;
}
.candidates-cell {
  max-width: 320px;
}

.loading-message,
.error-message {
  padding: 1rem;
  text-align: center;
}
.error-message {
  color: var(--error-color);
}
</style>
