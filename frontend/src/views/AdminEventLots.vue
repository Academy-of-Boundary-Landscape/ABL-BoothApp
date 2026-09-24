<template>
  <PageShell
    title="套装与优惠"
    subtitle="套装 = 从一组候选商品里按一个总价卖。选「这几样各 1 件凑齐」就是甲+乙合购； 选「任选 N 件、可以拿同款」就是同一本也能买 3 本。 候选商品必须属于同一个货主——替别的社团让价不是摊主能单方面决定的。"
    width="content"
  >
    <SectionCard
      title="新建套装"
      collapsible
      v-model:collapsed="isFormCollapsed"
      class="form-section"
    >
      <!-- 每个框都带标签，不靠 placeholder：**编辑时所有框都是填好的，
           placeholder 根本不会显示**，只靠占位提示等于没有提示。 -->
      <div class="form-grid">
        <label class="field field-wide">
          <span class="field-label">套装名称</span>
          <n-input v-model:value="form.name" placeholder="如「本子任选3本100」" />
        </label>

        <label class="field">
          <span class="field-label">要选几件</span>
          <n-input-number v-model:value="form.pickCount" :min="1" :precision="0" />
        </label>

        <label class="field">
          <span class="field-label">总价（元）</span>
          <n-input-number v-model:value="form.priceYuan" :min="0" :precision="2" />
        </label>

        <!-- select 和单选组用 div 不用 label：naive-ui 这两个控件不一定渲染出
             可关联的原生 input，套 label 会做出一个点了没反应的假热区。 -->
        <div class="field field-wide">
          <span class="field-label">候选商品</span>
          <n-select
            v-model:value="form.candidateIds"
            multiple
            filterable
            :options="candidateOptions"
            placeholder="可多选。必须属于同一个货主"
          />
        </div>

        <div class="field field-wide">
          <span class="field-label">怎么算「凑满」</span>
          <n-radio-group v-model:value="form.allowRepeat">
            <n-space vertical :size="10">
              <n-radio :value="false">
                这几样各 1 件凑齐
                <span class="mode-hint">固定组合。「甲 + 乙 一起 50」是这一类</span>
              </n-radio>
              <n-radio :value="true">
                任选 N 件，可以拿同款
                <span class="mode-hint">「同一本买 3 本 80」「本子任选 3 本 100」是这一类</span>
              </n-radio>
            </n-space>
          </n-radio-group>
        </div>

        <div class="field-wide actions">
          <n-button type="primary" :disabled="isBusy" @click="handleSubmit">
            {{ editingId ? '保存修改' : '新建' }}
          </n-button>
          <n-button v-if="editingId" quaternary @click="resetForm">取消编辑</n-button>
        </div>
      </div>

      <!-- 配置的后果本来是黑箱：摊主配完只能等顾客来薅。把「顾客最多 / 最少能怎么拿」
           摆在表单正下方，**切换上面那个模式时这几个数字当场变**——语义靠看见后果
           理解，不靠读文字解释「重复」是什么意思。 -->
      <div v-if="previewError" class="preview preview-problem">{{ previewError }}</div>
      <div v-else-if="preview" class="preview">
        <p class="preview-line muted">
          候选：{{
            preview.candidates.map((c) => `${c.name} ${formatYuan(c.unit_price)}`).join(' · ')
          }}
        </p>
        <p v-for="s in scenarioLines" :key="s.kind" class="preview-line">
          {{ s.label }}：<strong>{{ describeMembers(s.members) }}</strong> 原价
          {{ formatYuan(s.original_amount) }} → 付 {{ formatYuan(s.lot_price) }}
          <span v-if="s.discount > 0" class="gave">你让 {{ formatYuan(s.discount) }}</span>
          <span v-else class="not-applied">这种组合不会套用（比原价贵）</span>
        </p>
        <p v-for="w in preview.warnings" :key="w.code" class="preview-warn">⚠ {{ w.message }}</p>
      </div>
    </SectionCard>

    <AsyncState
      :loading="store.isLoading"
      :error="store.error"
      :empty="!store.lots.length"
      loading-text="正在加载套装列表..."
    >
      <div class="table-scroll">
        <table class="lot-table">
          <thead>
            <tr>
              <th>名称</th>
              <th>任选</th>
              <th>模式</th>
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
              <td>{{ lot.allow_repeat ? '可同款' : '各 1 件' }}</td>
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

      <template #empty>
        <EmptyState title="还没有套装" hint="配一个套装，顾客的购物车就会自动套用最省的那一种。" />
      </template>
    </AsyncState>
  </PageShell>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { NInput, NInputNumber, NSelect, NButton, NSpace, NRadioGroup, NRadio } from 'naive-ui'
import { PageShell, SectionCard, AsyncState, EmptyState } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import { useLotStore } from '@/stores/lotStore'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import type { Schemas } from '@/api/client'
import { formatYuan, toCents, fromCents } from '@/utils/money'

type LotForm = {
  name: string
  pickCount: number | null
  priceYuan: number | null
  candidateIds: number[]
  allowRepeat: boolean
}

const props = defineProps<{ id: number }>()

const store = useLotStore()
const eventDetailStore = useEventDetailStore()
const fb = useFeedback()

const isFormCollapsed = ref(false)
const isBusy = ref(false)
const editingId = ref<number | null>(null)
const form = ref<LotForm>({
  name: '',
  pickCount: 1,
  priceYuan: null,
  candidateIds: [],
  allowRepeat: false,
})

// 选项标签带上货主名：候选集必须同一货主是后端硬校验，把货主写在标签上
// 能让摊主在点选时就看出来，而不是提交后才吃一个 400。
const candidateOptions = computed(() =>
  eventDetailStore.products.map((p) => ({
    label: `${p.name}（${p.owner_society_name} · ${formatYuan(p.unit_price)}）`,
    value: p.id,
  }))
)

// --- 试算 ---
// 后端 `/lots/preview` 是只读的 dry-run，走的是和创建完全相同的校验。
const preview = ref<Schemas['LotPreviewResponse'] | null>(null)
const previewError = ref<string | null>(null)
let previewSeq = 0
let previewTimer: ReturnType<typeof setTimeout> | null = null

// 表单填到能试算了没有。不完整时不发请求——那只会得到一句「至少要有一个候选商品」，
// 摊主还在填就弹错话，比不说话更烦。
const previewable = computed(
  () =>
    typeof form.value.pickCount === 'number' &&
    Number.isFinite(form.value.pickCount) &&
    form.value.pickCount >= 1 &&
    form.value.candidateIds.length > 0 &&
    typeof form.value.priceYuan === 'number' &&
    Number.isFinite(form.value.priceYuan)
)

function schedulePreview() {
  if (previewTimer) clearTimeout(previewTimer)
  // **在途请求必须作废。** 少了这一句，上一份配置的试算结果会落在新表单上——
  // 顾客端购物车踩过一模一样的坑（见 customerStore 的 scheduleQuote）。
  previewSeq += 1
  if (!previewable.value) {
    preview.value = null
    previewError.value = null
    return
  }
  previewTimer = setTimeout(runPreview, 300)
}

async function runPreview() {
  const seq = (previewSeq += 1)
  const pickCount = form.value.pickCount
  const priceYuan = form.value.priceYuan
  if (typeof pickCount !== 'number' || typeof priceYuan !== 'number') return
  try {
    const data = await store.previewLot(props.id, {
      pick_count: pickCount,
      total_price: toCents(priceYuan),
      allow_repeat: form.value.allowRepeat,
      candidate_ids: form.value.candidateIds,
    })
    if (seq !== previewSeq) return
    preview.value = data
    previewError.value = null
  } catch (error) {
    if (seq !== previewSeq) return
    preview.value = null
    // 后端原文：跨货主、候选数不够、件数超范围，都是摊主按「新建」会看到的同一句话。
    previewError.value = error instanceof Error ? error.message : String(error)
  }
}

// 名字不影响试算结果，所以不进依赖——否则打字时会白发一串请求。
watch(
  () =>
    JSON.stringify([
      form.value.pickCount,
      form.value.priceYuan,
      form.value.allowRepeat,
      form.value.candidateIds,
    ]),
  schedulePreview,
  { immediate: true }
)

/// 只有一种凑法时（固定组合），两个极端是同一个组合——列两行一样的只会让人
/// 以为自己看错了，合成一行并换个说法。
const scenarioLines = computed(() => {
  const sc = preview.value?.scenarios ?? []
  if (sc.length === 2 && sc[0].original_amount === sc[1].original_amount) {
    return [{ ...sc[0], label: '顾客只能这样拿' }]
  }
  return sc.map((s) => ({
    ...s,
    label: s.kind === 'max_discount' ? '顾客最多能这样拿' : '顾客最少能这样拿',
  }))
})

function describeMembers(members: Schemas['LotPreviewMember'][]) {
  return members.map((m) => (m.qty > 1 ? `${m.name} ×${m.qty}` : m.name)).join(' + ')
}

function candidateNames(lot: Schemas['LotResponse']) {
  if (!lot.candidate_ids.length) return '（候选已被删除）'
  const byId = new Map(eventDetailStore.products.map((p): [number, string] => [p.id, p.name]))
  return lot.candidate_ids.map((id) => byId.get(id) || `#${id}`).join('、')
}

function resetForm() {
  editingId.value = null
  // 默认「各 1 件凑齐」：猜错成可同款会让摊主静默少收钱，代价不对称。
  form.value = { name: '', pickCount: 1, priceYuan: null, candidateIds: [], allowRepeat: false }
}

function startEdit(lot: Schemas['LotResponse']) {
  editingId.value = lot.id
  form.value = {
    name: lot.name,
    pickCount: lot.pick_count,
    priceYuan: fromCents(lot.total_price),
    candidateIds: [...lot.candidate_ids],
    allowRepeat: lot.allow_repeat,
  }
  isFormCollapsed.value = false
}

async function handleSubmit() {
  const name = form.value.name.trim()
  if (!name) return fb.warning('请填写套装名称')
  const pickCount = form.value.pickCount
  if (typeof pickCount !== 'number' || !Number.isFinite(pickCount) || pickCount < 1)
    return fb.warning('「要选几件」至少是 1')
  if (!form.value.candidateIds.length) return fb.warning('请至少选一个候选商品')
  const priceYuan = form.value.priceYuan
  if (priceYuan === null) return fb.warning('请填写总价')

  const payload: Schemas['LotPayload'] = {
    name,
    pick_count: pickCount,
    total_price: toCents(priceYuan),
    candidate_ids: form.value.candidateIds,
    allow_repeat: form.value.allowRepeat,
  }
  isBusy.value = true
  try {
    if (editingId.value) {
      await store.updateLot(props.id, editingId.value, payload)
      fb.success('套装已更新')
    } else {
      await store.createLot(props.id, payload)
      fb.success('套装已新建')
    }
    resetForm()
  } catch (error) {
    fb.error(error, '操作失败')
  } finally {
    isBusy.value = false
  }
}

async function handleDelete(lot: Schemas['LotResponse']) {
  await fb.confirm({
    title: '确认删除',
    // 快照的存在是这句话成立的理由，不是安慰剧。
    content: `删除套装「${lot.name}」？已经下过的订单不受影响——它们存的是名字和价格的快照。`,
    positiveText: '确认删除',
    negativeText: '取消',
    danger: true,
    onConfirm: async () => {
      isBusy.value = true
      try {
        await store.deleteLot(props.id, lot.id)
        fb.success('套装已删除')
      } catch (error) {
        fb.error(error, '删除失败')
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
onUnmounted(() => {
  if (previewTimer) clearTimeout(previewTimer)
  store.resetStore()
})
</script>

<style scoped>
.form-section {
  margin-bottom: var(--space-xl);
}
/* 两列栅格，需要整行的字段跨两列。原来是一行 flex-wrap、每个控件抢 160px，
   七个控件挤在一起，标签无处安放。 */
.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-lg) var(--space-lg);
  align-items: end;
}
.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  min-width: 0;
}
.field-wide {
  grid-column: 1 / -1;
}
.field-label {
  font-size: var(--font-sm);
  color: var(--text-muted);
}
/* 控件铺满自己那一格：n-input-number 默认按内容宽，不铺的话两列会长短不齐。 */
.field > *:not(.field-label) {
  width: 100%;
}
.mode-hint {
  display: block;
  font-size: var(--font-sm);
  line-height: 1.5;
  color: var(--text-muted);
}
.actions {
  display: flex;
  gap: var(--space-md);
  margin-top: var(--space-xs);
}
/* 手机/窄窗口下单列。摊主在现场用平板配套装是真实场景。 */
@media (--phone) {
  .form-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}

.preview {
  margin-top: var(--space-md);
  padding: var(--space-md) var(--space-lg);
  border: 1px solid var(--border-color);
  border-left: 3px solid var(--accent-color);
  border-radius: var(--radius-sm);
  background-color: var(--card-bg-color);
}
.preview-problem {
  border-left-color: var(--error-color);
  color: var(--error-color);
}
.preview-line {
  margin: 0 0 var(--space-xs);
  font-size: var(--font-sm);
  line-height: 1.6;
}
.preview-line.muted {
  color: var(--text-muted);
}
.preview-line:last-child {
  margin-bottom: 0;
}
.gave {
  color: var(--warning-color);
}
.not-applied {
  color: var(--text-disabled);
}
.preview-warn {
  margin: var(--space-sm) 0 0;
  font-size: var(--font-sm);
  line-height: 1.5;
  color: var(--warning-color);
}

.lot-table {
  width: 100%;
  border-collapse: collapse;
  text-align: left;
  font-size: var(--font-base);
}
.lot-table th {
  padding: var(--space-md) var(--space-lg);
  background-color: var(--card-bg-color);
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
  border-bottom: 2px solid var(--accent-color);
  white-space: nowrap;
}
.lot-table td {
  padding: var(--space-md) var(--space-lg);
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
</style>
