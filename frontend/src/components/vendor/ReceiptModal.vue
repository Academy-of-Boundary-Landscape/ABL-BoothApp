<template>
  <AppModal :show="show" @close="$emit('cancel')">
    <template #header><h3>确认收款</h3></template>
    <template #body>
      <div class="price-block">
        <div v-if="grossAmount !== effectiveSolved" class="price-row subtle">
          <span>原价</span>
          <span class="struck">{{ formatYuan(grossAmount) }}</span>
        </div>
        <div class="price-row total">
          <span>应收</span>
          <span>{{ formatYuan(effectiveSolved) }}</span>
        </div>
      </div>

      <!-- 已套用的套装，逐个可拆。取消勾选 = 这一单不套用它，成分回到原价。
           这和「直接改实收」不是一回事：改实收把差额记成手工折让、整笔落本社团，
           而拆套装是纠错，钱回到真正的货主头上（spec 4.3 的 2026-09-23 修正）。 -->
      <div v-if="lots.length" class="lot-block">
        <p class="lot-title">已套用的套装</p>
        <label v-for="lot in lots" :key="lot.id" class="lot-row">
          <n-checkbox :checked="!unapplied.includes(lot.id)" @update:checked="toggle(lot.id)" />
          <span class="lot-name">{{ lot.name }}</span>
          <span class="lot-saved">−{{ formatYuan(lot.original_amount - lot.price) }}</span>
        </label>
        <p v-if="unapplied.length" class="lot-note">
          已拆掉 {{ unapplied.length }} 个套装，这些商品按原价计算。
        </p>
      </div>

      <label class="field">
        <span class="field-label">实收（元）</span>
        <n-input-number v-model:value="finalYuan" :min="0" :precision="2" class="field-input" />
      </label>
      <p v-if="adjustment !== 0" class="adjustment">
        {{ adjustment > 0 ? '手工折让' : '手工加价' }} {{ formatYuan(Math.abs(adjustment)) }}
        <span class="adjustment-note">——全部算在本社团头上，代卖社团按自己的定价结算</span>
      </p>

      <n-radio-group v-model:value="channel" name="payment-channel">
        <n-space>
          <n-radio v-for="c in CHANNELS" :key="c" :value="c">{{ c }}</n-radio>
        </n-space>
      </n-radio-group>

      <!-- spec 第 11 节的不可破坏项：不得让复式记账制造出「钱已到账」的错觉。
           系统始终不知道顾客有没有真付，摊主点的是「我看到到账提示了」。
           这个弹窗现在还能改金额、还能拆套装，**看起来**更像在处理真钱——
           所以这句话比以前更不能删。 -->
      <p class="disclosure">这只是记账。请先确认手机上真的收到了到账提示，再点确认。</p>
    </template>
    <template #footer>
      <n-space>
        <n-button @click="$emit('cancel')">取消</n-button>
        <n-button type="primary" @click="handleConfirm">确认</n-button>
      </n-space>
    </template>
  </AppModal>
</template>

<script setup>
import { ref, computed, watch } from 'vue'
import { NRadioGroup, NRadio, NSpace, NButton, NInputNumber, NCheckbox } from 'naive-ui'
import AppModal from '@/components/shared/AppModal.vue'
import { formatYuan, toCents, fromCents } from '@/utils/money'

// 现场一场展会里收款渠道基本不变，上次选的记 localStorage 做默认值。
const CHANNELS = ['现金', '微信', '支付宝']
const CHANNEL_STORAGE_KEY = 'last_payment_channel'

const props = defineProps({
  show: { type: Boolean, default: false },
  /** 原价合计（分） */
  grossAmount: { type: Number, default: 0 },
  /** 服务端算出的应收（分） */
  solvedAmount: { type: Number, default: 0 },
  /** 这一单套用的套装实例：[{ id, name, price, original_amount }] */
  lots: { type: Array, default: () => [] },
})
const emit = defineEmits(['confirm', 'cancel'])

const channel = ref('微信')
const finalYuan = ref(0)
/** 被取消勾选的套装实例 id */
const unapplied = ref([])

/**
 * 拆掉几个套装之后的应收。
 *
 * 在本地算而不是再问一次服务端：`original_amount` 就是「这个实例的成分按原价合计」，
 * 拆掉它应收就回到那个数。服务端在确认时会用同一套规则重算一遍，本地这个数只用于显示。
 */
const effectiveSolved = computed(() =>
  props.lots.reduce(
    (sum, lot) => (unapplied.value.includes(lot.id) ? sum + lot.original_amount - lot.price : sum),
    props.solvedAmount
  )
)

const adjustment = computed(() => effectiveSolved.value - toCents(finalYuan.value))

function reset() {
  const saved = localStorage.getItem(CHANNEL_STORAGE_KEY)
  channel.value = CHANNELS.includes(saved) ? saved : '微信'
  unapplied.value = []
  finalYuan.value = fromCents(props.solvedAmount)
}

watch(
  () => props.show,
  (val) => {
    // 每次打开都重置：上一单改过的数字或拆过的勾留在这里是真事故。
    if (val) reset()
  }
)

function toggle(lotId) {
  unapplied.value = unapplied.value.includes(lotId)
    ? unapplied.value.filter((id) => id !== lotId)
    : [...unapplied.value, lotId]
  // 勾选一变就把实收拉回新的应收。摊主的手势是「先决定套不套装，再决定让不让价」，
  // 反过来保留旧数字只会让人对着一个过期的金额点确认。
  finalYuan.value = fromCents(effectiveSolved.value)
}

function handleConfirm() {
  localStorage.setItem(CHANNEL_STORAGE_KEY, channel.value)
  emit('confirm', {
    channel: channel.value,
    finalAmount: toCents(finalYuan.value),
    unapplyLotIds: [...unapplied.value],
  })
}
</script>

<style scoped>
.price-block {
  margin-bottom: 1rem;
}
.price-row {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  padding: 2px 0;
}
.price-row.subtle {
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.price-row.subtle .struck {
  text-decoration: line-through;
}
.price-row.total {
  font-weight: 600;
  font-size: var(--font-lg);
}

.lot-block {
  margin-bottom: 1rem;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
}
.lot-title {
  margin: 0 0 0.25rem;
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.lot-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 4px 0;
  cursor: pointer;
}
.lot-name {
  flex: 1;
  min-width: 0;
}
.lot-saved {
  color: var(--success-color);
  white-space: nowrap;
}
.lot-note {
  margin: 0.25rem 0 0;
  font-size: var(--font-sm);
  color: var(--warning-color);
}

.field {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.5rem;
}
.field-label {
  white-space: nowrap;
  color: var(--text-muted);
}
.field-input {
  flex: 1;
}
.adjustment {
  margin: 0 0 1rem;
  font-size: var(--font-sm);
  color: var(--accent-color);
  line-height: 1.5;
}
.adjustment-note {
  color: var(--text-muted);
}
.disclosure {
  margin: 16px 0 0;
  font-size: var(--font-sm);
  line-height: 1.5;
  color: var(--text-muted);
}
</style>
