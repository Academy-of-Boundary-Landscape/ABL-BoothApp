<!--
  退货弹窗。

  数据来自 `GET /events/:id/orders/:oid/refunds`（一次请求同时给历史和可退行）。
  **逐行列出，不合并**：同一个商品因为套装归属会拆成多行，退哪一行是摊主的决定，
  不是系统猜的（②-2 交接契约第 3 条）。行标题带上 `lot_name`，否则两行「本子A」
  看着像重复计数。

  退款金额的摊分（`splitRefund`）只为界面实时显示，权威计算在后端；两边取整规则
  一致，免得出现「界面上写退 33，提交完变成 34」。
-->
<template>
  <n-modal :show="show" :mask-closable="false" @update:show="(v) => !v && emit('close')">
    <n-card class="refund-card" :bordered="true" size="medium">
      <template #header>
        <div class="modal-header">
          <h3>退货 · 订单 #{{ order?.id }}</h3>
          <n-button quaternary circle size="small" @click="emit('close')">×</n-button>
        </div>
      </template>

      <n-spin :show="isLoading">
        <template v-if="order">
          <p class="order-meta">
            原实收 {{ formatYuan(order.final_amount) }} · 收款渠道 {{ order.channel || '（未记录）' }}
          </p>

          <!-- ===== 可退行 ===== -->
          <p class="section-title">选择要退的行</p>
          <p v-if="!lines.length" class="empty-hint">没有可退的订单行。</p>
          <div v-for="(line, idx) in lines" :key="line.order_line_id" class="line-block">
            <div class="line-head">
              <span class="line-name">{{ lineTitle(line) }}</span>
              <n-tag v-if="line.remaining_qty === 0" size="small" :bordered="false">已退完</n-tag>
            </div>
            <div class="line-body">
              <div class="line-facts">
                <span>可退 {{ line.remaining_qty }} / {{ line.qty }} 件</span>
                <span>该行剩余实付 {{ formatYuan(line.remaining_paid) }}</span>
              </div>
              <div class="line-controls">
                <n-input-number
                  v-model:value="qtyByLine[line.order_line_id]"
                  :min="0"
                  :max="line.remaining_qty"
                  :precision="0"
                  :disabled="line.remaining_qty === 0"
                />
                <span class="line-refund">
                  本次退 {{ formatYuan(perLineRefund[idx]) }}
                </span>
              </div>
              <n-radio-group
                v-model:value="destinationByLine[line.order_line_id]"
                :disabled="line.remaining_qty === 0"
                size="small"
              >
                <n-space>
                  <n-radio value="现场仓">回现场仓（还能卖）</n-radio>
                  <n-radio value="损耗">进损耗（已损坏）</n-radio>
                </n-space>
              </n-radio-group>
            </div>
          </div>

          <!-- ===== 退款金额与渠道 ===== -->
          <div class="field">
            <span class="field-label">退款渠道</span>
            <ChannelSelect v-model="channel" />
          </div>
          <p class="field-note">默认与收款渠道相同；现金退就选现金，否则收摊清点会对不上</p>

          <div class="field">
            <span class="field-label">实际退款（元）</span>
            <n-input-number
              v-model:value="amountYuan"
              :min="0"
              :precision="2"
              @update:value="amountTouched = true"
            />
          </div>
          <p v-if="overLimit" class="limit-warning">
            不能多于顾客实付，白送钱请走结算调整
          </p>
          <p v-else class="field-note">
            默认按所选各行实付之和（{{ formatYuan(defaultTotal) }}）；改低可以，改高会被后端挡住。
          </p>

          <!-- ===== 历史 ===== -->
          <p class="section-title">退货记录</p>
          <p v-if="!history.length" class="empty-hint">还没有退过货。</p>
          <div v-for="h in history" :key="h.id" class="history-row">
            <span class="history-time">{{ formatTimestamp(h.occurred_at, false) }}</span>
            <span class="history-name">{{ h.name }}</span>
            <span>×{{ h.qty }}</span>
            <span>{{ formatYuan(h.refund_amount) }}</span>
            <span>{{ h.channel }}</span>
            <span>{{ h.destination }}</span>
          </div>
          <p v-if="history.length" class="field-note">
            退货没有撤销：退错了再开一张反向的单，或走结算调整。
          </p>
        </template>
      </n-spin>

      <template #footer>
        <n-space justify="end">
          <n-button @click="emit('close')">关闭</n-button>
          <n-button type="primary" :loading="isBusy" :disabled="overLimit" @click="submit">
            确认退货
          </n-button>
        </n-space>
      </template>
    </n-card>
  </n-modal>
</template>

<script setup>
import { ref, computed, watch } from 'vue'
import {
  NModal,
  NCard,
  NButton,
  NInputNumber,
  NRadioGroup,
  NRadio,
  NSpace,
  NTag,
  NSpin,
  useMessage,
} from 'naive-ui'
import ChannelSelect from '@/components/shared/ChannelSelect.vue'
import { formatYuan, fromCents, toCents } from '@/utils/money'
import { formatTimestamp } from '@/utils/dateFormatter'
import { splitRefund, defaultRefundTotal } from '@/utils/refund'
import api from '@/services/api'

const props = defineProps({
  show: { type: Boolean, default: false },
  eventId: { type: [String, Number], required: true },
  order: { type: Object, default: null },
})
const emit = defineEmits(['close', 'refunded', 'loaded'])

const message = useMessage()

const history = ref([])
const lines = ref([])
const qtyByLine = ref({})
const destinationByLine = ref({})
const channel = ref('')
const amountYuan = ref(0)
const amountTouched = ref(false)
const isLoading = ref(false)
const isBusy = ref(false)

/** 同商品拆行时带上套装名；`lot_name` 为 null 就只显示商品名。 */
function lineTitle(line) {
  return line.lot_name ? `${line.name}（${line.lot_name}）` : line.name
}

const defaultTotal = computed(() => defaultRefundTotal(lines.value, qtyByLine.value))
const refundCents = computed(() => toCents(amountYuan.value))
const overLimit = computed(() => refundCents.value > defaultTotal.value)

/** 和 `lines` 一一对应；只用于显示，权威值在后端。
 *  和后端一样把「剩余实付」按件数切成本次退的那一份，权重是件数，
 *  所以必须用不带 cap 的 `splitRefund`（拿件数当分的上限会截成几分钱）。 */
const weights = computed(() =>
  lines.value.map((l) => {
    const q = Number(qtyByLine.value[l.order_line_id] || 0)
    if (q <= 0 || l.remaining_qty <= 0) return 0
    return splitRefund(l.remaining_paid, [q, l.remaining_qty - q], false)[0]
  })
)
const perLineRefund = computed(() => splitRefund(refundCents.value, weights.value))

async function load() {
  if (!props.order) return
  isLoading.value = true
  try {
    const { data } = await api.get(
      `/events/${props.eventId}/orders/${props.order.id}/refunds`
    )
    history.value = data?.history || []
    lines.value = data?.lines || []

    const qty = {}
    const dest = {}
    for (const line of lines.value) {
      qty[line.order_line_id] = 0
      dest[line.order_line_id] = '现场仓'
    }
    qtyByLine.value = qty
    destinationByLine.value = dest
    channel.value = props.order.channel || '微信'
    amountTouched.value = false
    amountYuan.value = fromCents(defaultRefundTotal(lines.value, qty))
    emit('loaded', { orderId: props.order.id })
  } catch (err) {
    message.error(err.response?.data?.error || '无法加载退货信息。')
  } finally {
    isLoading.value = false
  }
}

watch(
  () => props.show,
  (val) => {
    if (val) load()
  }
)

// 摊主没动过金额时，跟着所选件数实时回到默认值；动过就不再覆盖他的输入。
watch(defaultTotal, (val) => {
  if (!amountTouched.value) amountYuan.value = fromCents(val)
})

async function submit() {
  const chosen = lines.value.filter(
    (l) => Number(qtyByLine.value[l.order_line_id] || 0) > 0
  )
  if (!chosen.length) return message.warning('请至少选择一行退货数量')
  if (!channel.value) return message.warning('请选择退款渠道')
  if (overLimit.value) return message.warning('不能多于顾客实付，白送钱请走结算调整')

  const payload = {
    channel: channel.value,
    lines: chosen.map((l) => ({
      order_line_id: l.order_line_id,
      qty: Number(qtyByLine.value[l.order_line_id]),
      destination: destinationByLine.value[l.order_line_id] || '现场仓',
    })),
  }
  // 没动过就让后端按各行实付全额退，省掉一次前端取整与后端的对不齐。
  if (amountTouched.value) payload.refund_amount = refundCents.value

  isBusy.value = true
  try {
    const { data } = await api.post(
      `/events/${props.eventId}/orders/${props.order.id}/refunds`,
      payload
    )
    message.success(`已退货，退款 ${formatYuan(data.refund_amount)}`)
    await load()
    emit('refunded', { orderId: props.order.id })
  } catch (err) {
    message.error(err.response?.data?.error || '退货失败。')
  } finally {
    isBusy.value = false
  }
}
</script>

<style scoped>
.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.modal-header h3 {
  margin: 0;
}
.order-meta {
  margin: 0 0 0.75rem;
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.section-title {
  margin: 1rem 0 0.5rem;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--border-color);
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.line-block {
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  padding: 0.5rem 0.75rem;
  margin-bottom: 0.5rem;
}
.line-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.line-name {
  font-weight: 600;
}
.line-body {
  margin-top: 0.4rem;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}
.line-facts {
  display: flex;
  gap: 1rem;
  flex-wrap: wrap;
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.line-controls {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}
.line-refund {
  color: var(--accent-color);
  font-size: var(--font-sm);
  white-space: nowrap;
}
.field {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-top: 0.75rem;
}
.field-label {
  flex: 0 0 7em;
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.field > .n-select,
.field > .n-input-number {
  flex: 1;
  min-width: 0;
}
.field-note {
  margin: 0.25rem 0 0;
  color: var(--text-muted);
  font-size: var(--font-sm);
  line-height: 1.5;
}
.limit-warning {
  margin: 0.25rem 0 0;
  color: var(--error-color);
  font-size: var(--font-sm);
}
.empty-hint {
  margin: 0.25rem 0;
  color: var(--text-muted);
  font-size: var(--font-sm);
}
.history-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  padding: 4px 0;
  border-bottom: 1px dashed var(--border-color);
  font-size: var(--font-sm);
}
.history-time {
  color: var(--text-muted);
}
.history-name {
  font-weight: 600;
}
.refund-card {
  width: 720px;
  max-width: 95%;
}
.refund-card :deep(.n-card__content) {
  max-height: 70vh;
  overflow-y: auto;
}
</style>
