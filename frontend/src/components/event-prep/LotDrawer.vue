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

        <n-form-item label="要选几件" required>
          <n-input-number
            v-model:value="form.pickCount"
            class="full-width"
            :min="1"
            :precision="0"
          />
        </n-form-item>

        <n-form-item label="总价（元）" required>
          <n-input-number
            v-model:value="form.priceYuan"
            class="full-width"
            :min="0"
            :precision="2"
          />
        </n-form-item>

        <n-form-item label="候选商品" required>
          <n-select
            v-model:value="form.candidateIds"
            multiple
            filterable
            :options="candidateOptions"
            placeholder="可多选。必须属于同一个货主"
          />
          <template #feedback
            >候选商品必须属于同一个货主——替别的社团让价不是摊主能单方面决定的。</template
          >
        </n-form-item>

        <n-form-item label="怎么算「凑满」">
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
        </n-form-item>
      </n-form>

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
  NRadio,
  NRadioGroup,
  NSelect,
  NSpace,
} from 'naive-ui'
import { useFeedback } from '@/composables/useFeedback'
import { useViewport } from '@/composables/useViewport'
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
  font-size: var(--font-base);
  line-height: 1.6;
}

.full-width {
  width: 100%;
}

.mode-hint {
  display: block;
  font-size: var(--font-sm);
  line-height: 1.5;
  color: var(--text-muted);
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

@media (--phone) {
  .drawer-hint {
    font-size: var(--font-sm);
  }
}
</style>
