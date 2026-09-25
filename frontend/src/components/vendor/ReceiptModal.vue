<template>
  <AppModal :show="show" title="确认收款" size="md" @update:show="(v) => !v && $emit('cancel')">
    <!-- 应收：摊主一眼要看到的数，大号粗字；有套装折让时原价划线跟在旁边 -->
    <div class="due">
      <span class="due-label">应收</span>
      <span class="due-amount"><Money :value="effectiveSolved" size="lg" /></span>
      <span v-if="grossAmount !== effectiveSolved" class="due-gross">
        <Money :value="grossAmount" strike size="sm" />
      </span>
    </div>

    <!-- 已套用的套装，逐个可拆。关掉开关 = 这一单不套用它，成分回到原价。
         这和「直接改实收」不是一回事：改实收把差额记成手工折让、整笔落本社团，
         而拆套装是纠错，钱回到真正的货主头上（spec 4.3 的 2026-09-23 修正）。 -->
    <section v-if="lots.length" class="block">
      <h4 class="block-title">已套用的套装</h4>
      <div
        v-for="lot in lots"
        :key="lot.id"
        class="lot-row"
        :class="{ 'lot-row--off': unapplied.includes(lot.id) }"
        role="switch"
        :aria-checked="!unapplied.includes(lot.id)"
        tabindex="0"
        @click="toggle(lot.id)"
        @keydown.enter.prevent="toggle(lot.id)"
        @keydown.space.prevent="toggle(lot.id)"
      >
        <span class="lot-name">{{ lot.name }}</span>
        <span class="lot-saved">−{{ formatYuan(cents(lot.original_amount - lot.price)) }}</span>
        <n-switch
          :value="!unapplied.includes(lot.id)"
          size="large"
          aria-hidden="true"
          tabindex="-1"
          @click.stop="toggle(lot.id)"
        />
      </div>
      <p v-if="unapplied.length" class="lot-note">
        已拆掉 {{ unapplied.length }} 个套装，这些商品按原价计算。
      </p>
    </section>

    <section class="block">
      <h4 class="block-title">实收</h4>
      <div class="received">
        <n-input-number
          v-model:value="finalYuan"
          size="large"
          :min="0"
          :precision="2"
          :show-button="false"
          :input-props="{ inputmode: 'decimal' }"
          class="received-input"
        >
          <template #prefix>¥</template>
        </n-input-number>
        <n-button
          size="large"
          class="received-reset"
          :disabled="finalYuan === fromCents(effectiveSolved)"
          @click="finalYuan = fromCents(effectiveSolved)"
        >
          = 应收
        </n-button>
      </div>
      <p v-if="adjustment !== 0" class="adjustment">
        {{ adjustment > 0 ? '手工折让' : '手工加价' }} {{ formatYuan(cents(Math.abs(adjustment))) }}
        <span class="adjustment-note">——全部算在本社团头上，代卖社团按自己的定价结算</span>
      </p>
    </section>

    <!-- 收款渠道：常用三个做成大按钮一点即选；其他渠道仍走可输入的下拉（防渠道名分裂）。 -->
    <section class="block">
      <h4 class="block-title">收款渠道</h4>
      <div class="channels" role="radiogroup" aria-label="收款渠道">
        <button
          v-for="c in QUICK_CHANNELS"
          :key="c"
          type="button"
          role="radio"
          class="channel-btn"
          :class="{ 'channel-btn--active': channel === c && !showOtherChannel }"
          :aria-checked="channel === c && !showOtherChannel"
          @click="pickChannel(c)"
        >
          {{ c }}
        </button>
        <button
          type="button"
          role="radio"
          class="channel-btn"
          :class="{ 'channel-btn--active': showOtherChannel }"
          :aria-checked="showOtherChannel"
          @click="showOtherChannel = true"
        >
          其他…
        </button>
      </div>
      <ChannelSelect v-if="showOtherChannel" v-model="channel" class="channel-other" />
    </section>

    <!-- spec 第 11 节的不可破坏项：不得让复式记账制造出「钱已到账」的错觉。
         系统始终不知道顾客有没有真付，摊主点的是「我看到到账提示了」。
         这个弹窗现在还能改金额、还能拆套装，**看起来**更像在处理真钱——
         所以这句话比以前更不能删。 -->
    <p class="disclosure">这只是记账。请先确认手机上真的收到了到账提示，再点确认。</p>

    <template #footer>
      <div class="actions">
        <n-button size="large" class="btn-cancel" @click="$emit('cancel')">取消</n-button>
        <n-button type="primary" size="large" class="btn-confirm" @click="handleConfirm">
          确认收款
          <template v-if="finalYuan !== null && Number.isFinite(finalYuan)">
            ¥{{ finalYuan.toFixed(2) }}
          </template>
        </n-button>
      </div>
    </template>
  </AppModal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NButton, NInputNumber, NSwitch } from 'naive-ui'
import { AppModal, Money } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import ChannelSelect from '@/components/shared/ChannelSelect.vue'
import { formatYuan, cents, toCents, fromCents, type Cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

// 现场一场展会里收款渠道基本不变，上次选的记 localStorage 做默认值。
// 渠道列表本身由 ChannelSelect 从后端拉（预置三个 + 历史用过的）。
const CHANNEL_STORAGE_KEY = 'last_payment_channel'
/** 现场九成以上的单子是这三个渠道：做成大按钮一点即选。 */
const QUICK_CHANNELS = ['微信', '支付宝', '现金'] as const

const props = withDefaults(
  defineProps<{
    show?: boolean
    /** 原价合计（分） */
    grossAmount?: Cents
    /** 服务端算出的应收（分） */
    solvedAmount?: Cents
    /** 这一单套用的套装实例：[{ id, name, price, original_amount }] */
    lots?: Schemas['OrderLotResponse'][]
  }>(),
  {
    show: false,
    grossAmount: cents(0),
    solvedAmount: cents(0),
    lots: () => [],
  }
)
const emit = defineEmits<{
  (e: 'confirm', payload: { channel: string; finalAmount: Cents; unapplyLotIds: number[] }): void
  (e: 'cancel'): void
}>()
const fb = useFeedback()

const channel = ref('微信')
/** 「其他…」展开：上次记住的渠道不在常用三个里时，打开就是展开态。 */
const showOtherChannel = ref(false)
function pickChannel(c: string) {
  channel.value = c
  showOtherChannel.value = false
}
const finalYuan = ref<number | null>(0)
/** 被取消勾选的套装实例 id */
const unapplied = ref<number[]>([])

/**
 * 拆掉几个套装之后的应收。
 *
 * 在本地算而不是再问一次服务端：`original_amount` 就是「这个实例的成分按原价合计」，
 * 拆掉它应收就回到那个数。服务端在确认时会用同一套规则重算一遍，本地这个数只用于显示。
 */
const effectiveSolved = computed(() =>
  props.lots.reduce(
    (sum, lot) =>
      unapplied.value.includes(lot.id) ? cents(sum + lot.original_amount - lot.price) : sum,
    props.solvedAmount
  )
)

const adjustment = computed(() => effectiveSolved.value - toCents(finalYuan.value ?? 0))

function reset() {
  // 非空就用：自定义渠道也该被记住，不能在下次打开时被悄悄换回预置值。
  const saved = localStorage.getItem(CHANNEL_STORAGE_KEY)
  channel.value = saved || '微信'
  showOtherChannel.value = !(QUICK_CHANNELS as readonly string[]).includes(channel.value)
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

function toggle(lotId: number) {
  unapplied.value = unapplied.value.includes(lotId)
    ? unapplied.value.filter((id) => id !== lotId)
    : [...unapplied.value, lotId]
  // 勾选一变就把实收拉回新的应收。摊主的手势是「先决定套不套装，再决定让不让价」，
  // 反过来保留旧数字只会让人对着一个过期的金额点确认。
  finalYuan.value = fromCents(effectiveSolved.value)
}

function handleConfirm() {
  // n-input-number 被清空 → finalYuan 为 null，而 toCents(null) 会回落成 0，
  // 于是静默提交一张 ¥0 的收款单：整单白送，全额记成对本社团的手工折让。
  if (finalYuan.value === null || !Number.isFinite(finalYuan.value)) {
    fb.warning('请填写实收金额')
    return
  }
  localStorage.setItem(CHANNEL_STORAGE_KEY, channel.value)
  emit('confirm', {
    channel: channel.value,
    finalAmount: toCents(finalYuan.value),
    unapplyLotIds: [...unapplied.value],
  })
}
</script>

<style scoped>
.due {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-md);
  margin-bottom: var(--space-lg);
  padding: var(--space-md) var(--space-lg);
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--accent-color) 8%, var(--card-bg-color));
}
.due-label {
  color: var(--secondary-text-color);
  font-size: var(--font-md);
}
.due-amount {
  font-size: var(--font-2xl);
  font-weight: var(--weight-bold);
}
.due-amount :deep(.money) {
  font-size: inherit;
}
.due-gross {
  color: var(--text-muted);
}

.block {
  margin-bottom: var(--space-lg);
}
.block-title {
  margin: 0 0 var(--space-sm);
  color: var(--secondary-text-color);
  font-size: var(--font-sm);
  font-weight: var(--weight-regular);
}

.lot-row {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  min-height: 52px;
  padding: var(--space-sm) var(--space-md);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  cursor: pointer;
}
.lot-row + .lot-row {
  margin-top: var(--space-sm);
}
.lot-row:focus-visible {
  outline: 2px solid var(--accent-color);
  outline-offset: 2px;
}
.lot-row--off {
  background: var(--bg-color);
}
.lot-row--off .lot-name,
.lot-row--off .lot-saved {
  color: var(--text-muted);
  text-decoration: line-through;
}
.lot-name {
  flex: 1;
  min-width: 0;
  font-size: var(--font-md);
}
.lot-saved {
  color: var(--success-color);
  font-weight: var(--weight-bold);
  white-space: nowrap;
}
.lot-note {
  margin: var(--space-sm) 0 0;
  font-size: var(--font-sm);
  color: var(--warning-color);
}

.received {
  display: flex;
  gap: var(--space-sm);
}
.received-input {
  flex: 1;
  font-size: var(--font-lg);
}
.received-reset {
  min-height: 44px;
}
.adjustment {
  margin: var(--space-sm) 0 0;
  font-size: var(--font-sm);
  color: var(--accent-color);
  line-height: var(--leading-base);
}
.adjustment-note {
  color: var(--text-muted);
}

.channels {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--space-sm);
}
.channel-btn {
  min-height: 52px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--card-bg-color);
  color: var(--primary-text-color);
  font: inherit;
  font-size: var(--font-md);
  cursor: pointer;
  transition: border-color 0.15s ease;
}
.channel-btn:hover {
  border-color: var(--accent-color);
}
.channel-btn:focus-visible {
  outline: 2px solid var(--accent-color);
  outline-offset: 2px;
}
.channel-btn--active {
  border-color: var(--accent-color);
  background: var(--accent-color);
  color: var(--text-white);
  font-weight: var(--weight-bold);
}
.channel-other {
  margin-top: var(--space-sm);
}

.disclosure {
  margin: 0;
  font-size: var(--font-sm);
  line-height: var(--leading-base);
  color: var(--text-muted);
}

.actions {
  display: grid;
  grid-template-columns: 1fr 2fr;
  gap: var(--space-md);
  width: 100%;
}
.btn-cancel,
.btn-confirm {
  min-height: 52px;
  font-size: var(--font-md);
}
.btn-confirm {
  font-weight: var(--weight-bold);
}
</style>
