<template>
  <n-drawer
    :show="show"
    :placement="isPhone ? 'bottom' : 'right'"
    :width="isPhone ? undefined : 560"
    :height="isPhone ? '92%' : undefined"
    @update:show="onUpdateShow"
  >
    <n-drawer-content :title="editingId ? '编辑套装' : '新建套装'" closable>
      <!-- 原页头副标题：解释「套装是什么」「同货主为什么是硬规则」。嵌在工作台里
           的子页没有页头，把这段话放在抽屉内，摊主配之前一定看得到。 -->
      <p class="drawer-hint">
        套装 = 从一组候选商品里按一个总价卖。选「这几样各 1 件凑齐」就是甲+乙合购； 选「任选 N
        件、可以拿同款」就是同一本也能买 3 本。
      </p>

      <n-form label-placement="top" @submit.prevent>
        <n-form-item label="套装名称" required>
          <n-input v-model:value="form.name" placeholder="如「本子任选3本100」" />
        </n-form-item>
      </n-form>

      <!-- 一句话式配置：件数和总价嵌在句子里，读出来就是套装的规则。 -->
      <div class="rule-sentence">
        <span>顾客从下面选</span>
        <n-input-number
          v-model:value="form.pickCount"
          class="rule-num"
          :min="1"
          :precision="0"
          aria-label="要选几件"
        />
        <span>件，一共付</span>
        <n-input-number
          v-model:value="form.priceYuan"
          class="rule-price"
          :min="0"
          :precision="2"
          :show-button="false"
          placeholder="总价"
          aria-label="总价（元）"
        >
          <template #prefix>¥</template>
        </n-input-number>
      </div>

      <!-- 「怎么算凑满」：两张大卡片二选一，按单选组的语义与键盘行为实现。 -->
      <div class="mode-cards" role="radiogroup" aria-label="怎么算凑满">
        <button
          v-for="m in MODES"
          :key="String(m.value)"
          type="button"
          role="radio"
          class="mode-card"
          :class="{ 'mode-card--active': form.allowRepeat === m.value }"
          :aria-checked="form.allowRepeat === m.value"
          :tabindex="form.allowRepeat === m.value ? 0 : -1"
          @click="form.allowRepeat = m.value"
          @keydown.left.prevent="form.allowRepeat = !form.allowRepeat"
          @keydown.right.prevent="form.allowRepeat = !form.allowRepeat"
        >
          <span class="mode-icon" aria-hidden="true">{{ m.icon }}</span>
          <span class="mode-title">{{ m.title }}</span>
          <span class="mode-desc">{{ m.desc }}</span>
          <span class="mode-example">{{ m.example }}</span>
        </button>
      </div>

      <section class="block">
        <h4 class="block-title">候选商品</h4>
        <LotCandidatePicker v-model="form.candidateIds" :products="eventDetailStore.products" />
        <p v-if="notEnoughCandidates" class="block-warn">
          「各 1 件凑齐」至少要选 {{ form.pickCount }} 件候选商品，现在只选了
          {{ form.candidateIds.length }} 件——这样的套装永远凑不出来。
        </p>
      </section>

      <!-- 配置的后果本来是黑箱：摊主配完只能等顾客来薅。把「顾客最多 / 最少能怎么拿」
           做成小票摆在最下面，**切换上面的模式时数字当场变**——语义靠看见后果
           理解，不靠读文字解释「重复」是什么意思。 -->
      <section v-if="previewError || preview" class="receipt" aria-live="polite">
        <h4 class="receipt-title">顾客会怎么拿</h4>
        <p v-if="previewError" class="receipt-problem">{{ previewError }}</p>
        <template v-else-if="preview">
          <div v-for="s in scenarioLines" :key="s.kind" class="receipt-row">
            <div class="receipt-label">{{ s.label }}</div>
            <div class="receipt-items">{{ describeMembers(s.members) }}</div>
            <div class="receipt-amounts">
              <template v-if="s.discount > 0">
                <Money :value="s.original_amount" strike size="sm" />
                <span class="receipt-arrow">→</span>
                <Money :value="s.lot_price" />
                <span class="receipt-save">省 {{ formatYuan(s.discount) }}</span>
              </template>
              <template v-else>
                <Money :value="s.original_amount" size="sm" />
                <span class="receipt-skip">比原价贵，不会套用</span>
              </template>
            </div>
          </div>
          <p v-for="w in preview.warnings" :key="w.code" class="receipt-warn">⚠ {{ w.message }}</p>
        </template>
      </section>

      <template #footer>
        <n-space justify="end">
          <n-button @click="emit('update:show', false)">取消</n-button>
          <n-button type="primary" :loading="isBusy" @click="handleSubmit">
            {{ editingId ? '保存修改' : '新建' }}
          </n-button>
        </n-space>
      </template>
    </n-drawer-content>
  </n-drawer>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  NButton,
  NDrawer,
  NDrawerContent,
  NForm,
  NFormItem,
  NInput,
  NInputNumber,
  NSpace,
} from 'naive-ui'
import { useFeedback } from '@/composables/useFeedback'
import { useViewport } from '@/composables/useViewport'
import { useLotStore } from '@/stores/lotStore'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import type { Schemas } from '@/api/client'
import { formatYuan, toCents, fromCents } from '@/utils/money'
import { Money } from '@/components/ui'
import LotCandidatePicker from './LotCandidatePicker.vue'

type LotForm = {
  name: string
  pickCount: number | null
  priceYuan: number | null
  candidateIds: number[]
  allowRepeat: boolean
}

const props = defineProps<{
  show: boolean
  eventId: number
  lot?: Schemas['LotResponse'] | null
}>()

const emit = defineEmits<{
  (e: 'update:show', v: boolean): void
  (e: 'saved'): void
}>()

const store = useLotStore()
const eventDetailStore = useEventDetailStore()
const fb = useFeedback()
const { isPhone } = useViewport()

const isBusy = ref(false)
const editingId = ref<number | null>(null)
const form = ref<LotForm>({
  name: '',
  pickCount: 1,
  priceYuan: null,
  candidateIds: [],
  allowRepeat: false,
})

/** 两种「凑满」规则。默认「各 1 件凑齐」：猜错成可同款会让摊主静默少收钱，代价不对称。 */
const MODES = [
  {
    value: false,
    icon: '🧩',
    title: '固定组合',
    desc: '这几样各 1 件凑齐',
    example: '「甲 + 乙 一起 50」',
  },
  {
    value: true,
    icon: '🔁',
    title: '任选 N 件',
    desc: '可以拿同款',
    example: '「本子任选 3 本 100」',
  },
] as const

/** 各 1 件凑齐时候选种类必须 ≥ 件数，否则永远凑不出（后端同样会拒）。当场提示，不等提交。 */
const notEnoughCandidates = computed(
  () =>
    !form.value.allowRepeat &&
    form.value.candidateIds.length > 0 &&
    typeof form.value.pickCount === 'number' &&
    form.value.candidateIds.length < form.value.pickCount
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
  // 抽屉关着（新建 / 编辑已结束）时不发请求，避免为看不见的表单白发一批。
  if (!props.show) return
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
    const data = await store.previewLot(props.eventId, {
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

function hydrate() {
  const lot = props.lot
  editingId.value = lot?.id ?? null
  if (lot) {
    form.value = {
      name: lot.name,
      pickCount: lot.pick_count,
      priceYuan: fromCents(lot.total_price),
      candidateIds: [...lot.candidate_ids],
      allowRepeat: lot.allow_repeat,
    }
  } else {
    // 默认「各 1 件凑齐」：猜错成可同款会让摊主静默少收钱，代价不对称。
    form.value = { name: '', pickCount: 1, priceYuan: null, candidateIds: [], allowRepeat: false }
  }
  preview.value = null
  previewError.value = null
  schedulePreview()
}

watch(
  () => props.show,
  (open) => {
    if (open) {
      hydrate()
    } else if (previewTimer) {
      // 抽屉关闭时把在途试算掐掉，别让结果落到下一份表单上。
      clearTimeout(previewTimer)
      previewTimer = null
    }
  }
)

function onUpdateShow(v: boolean) {
  emit('update:show', v)
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
      await store.updateLot(props.eventId, editingId.value, payload)
      fb.success('套装已更新')
    } else {
      await store.createLot(props.eventId, payload)
      fb.success('套装已新建')
    }
    emit('saved')
    emit('update:show', false)
  } catch (error) {
    fb.error(error, '操作失败')
  } finally {
    isBusy.value = false
  }
}
</script>

<style scoped>
.drawer-hint {
  margin: 0 0 var(--space-lg);
  color: var(--text-muted);
  font-size: var(--font-sm);
  line-height: var(--leading-base);
}

/* 一句话配置 */
.rule-sentence {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-sm);
  margin-bottom: var(--space-lg);
  padding: var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-color);
  font-size: var(--font-md);
}

.rule-num {
  width: 7em;
}

.rule-price {
  width: 8em;
}

/* 模式卡片 */
.mode-cards {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-sm);
  margin-bottom: var(--space-lg);
}

.mode-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  padding: var(--space-md);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  background: var(--card-bg-color);
  color: var(--primary-text-color);
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition: border-color 0.15s ease;
}

.mode-card:hover {
  border-color: var(--accent-color);
}

.mode-card:focus-visible {
  outline: 2px solid var(--accent-color);
  outline-offset: 2px;
}

.mode-card--active {
  border-color: var(--accent-color);
  /* 选中态加粗描边：用 outline 叠一圈，不改 border 宽度（避免卡片跳动） */
  outline: 1px solid var(--accent-color);
  background: color-mix(in srgb, var(--accent-color) 6%, var(--card-bg-color));
}

.mode-icon {
  font-size: var(--font-xl);
}

.mode-title {
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
}

.mode-card--active .mode-title {
  color: var(--accent-color);
}

.mode-desc {
  font-size: var(--font-sm);
}

.mode-example {
  color: var(--text-muted);
  font-size: var(--font-xs);
}

/* 候选商品 */
.block {
  margin-bottom: var(--space-lg);
}

.block-title,
.receipt-title {
  margin: 0 0 var(--space-sm);
  font-size: var(--font-base);
  font-weight: var(--weight-bold);
}

.block-warn {
  margin: var(--space-sm) 0 0;
  color: var(--warning-color);
  font-size: var(--font-sm);
  line-height: var(--leading-base);
}

/* 小票预览 */
.receipt {
  padding: var(--space-md) var(--space-lg);
  border: 1px dashed var(--border-color);
  border-radius: var(--radius-md);
  background: var(--card-bg-color);
}

.receipt-row {
  padding: var(--space-sm) 0;
  border-bottom: 1px dashed var(--divider-color);
}

.receipt-row:last-of-type {
  border-bottom: none;
}

.receipt-label {
  color: var(--text-muted);
  font-size: var(--font-xs);
}

.receipt-items {
  margin: var(--space-xs) 0;
  font-size: var(--font-sm);
  line-height: var(--leading-base);
}

.receipt-amounts {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-sm);
}

.receipt-arrow {
  color: var(--text-muted);
}

.receipt-save {
  padding: 0 var(--space-sm);
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, var(--success-color) 14%, transparent);
  color: var(--success-color);
  font-size: var(--font-xs);
  font-weight: var(--weight-bold);
}

.receipt-skip {
  color: var(--text-disabled);
  font-size: var(--font-xs);
}

.receipt-problem {
  margin: 0;
  color: var(--error-color);
  font-size: var(--font-sm);
}

.receipt-warn {
  margin: var(--space-sm) 0 0;
  color: var(--warning-color);
  font-size: var(--font-sm);
  line-height: var(--leading-base);
}

@media (--phone) {
  .rule-sentence {
    font-size: var(--font-base);
  }
}
</style>
