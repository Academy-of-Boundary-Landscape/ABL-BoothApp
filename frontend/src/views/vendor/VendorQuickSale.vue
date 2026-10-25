<template>
  <PageShell embedded width="wide">
    <div class="quick-sale">
      <header class="quick-sale__intro">
        <div>
          <h2>快售收银</h2>
          <p>摊主选商品，系统自动算套装优惠；确认实际到账后，一次完成整单。</p>
        </div>
      </header>

      <section class="quick-sale__channel" aria-label="收款渠道">
        <strong>收款渠道</strong>
        <div class="quick-sale__channels">
          <button
            v-for="option in CHANNELS"
            :key="option"
            type="button"
            :aria-pressed="channel === option"
            :class="{ 'quick-sale__channel-button--active': channel === option }"
            :disabled="busy || !!pending"
            @click="selectChannel(option)"
          >
            {{ option }}
          </button>
          <button
            type="button"
            :aria-pressed="showOtherChannel"
            :class="{ 'quick-sale__channel-button--active': showOtherChannel }"
            :disabled="busy || !!pending"
            @click="showOtherChannel = true"
          >
            其他…
          </button>
        </div>
        <ChannelSelect
          v-if="showOtherChannel && !busy && !pending"
          :model-value="channel"
          class="quick-sale__other-channel"
          @update:model-value="selectOtherChannel"
        />
        <span v-if="!channel" class="quick-sale__hint">先选渠道</span>
        <n-button @click="showPaymentCodes = true">展示收款码</n-button>
      </section>

      <details v-if="activeLots.length" class="quick-sale__deals">
        <summary>本场套装优惠（{{ activeLots.length }}）</summary>
        <ul>
          <li v-for="lot in activeLots" :key="lot.id">
            <span>{{ lot.name }} · 选 {{ lot.pick_count }} 件</span>
            <strong>{{ formatYuan(lot.total_price) }}</strong>
          </li>
        </ul>
      </details>
      <p v-else-if="lotStore.error" class="quick-sale__hint">
        优惠列表暂未加载；购物车仍会向服务端重新报价。
      </p>

      <div v-if="pending" class="quick-sale__pending" role="alert">
        <div>
          <strong>上一单结果待确认</strong>
          <p v-if="pending.eventId === eventId">
            {{ pending.items.reduce((total, item) => total + item.quantity, 0) }} 件 ·
            {{ pending.channel }} · 实收 {{ formatYuan(pending.finalAmount) }}。
            请重试原请求，避免重复记账。
          </p>
          <p v-else>展会 #{{ pending.eventId }} 的一单待确认，请先回到那场展会。</p>
        </div>
        <button
          v-if="pending.eventId === eventId"
          type="button"
          :disabled="busy"
          @click="retryPending"
        >
          重试上一单
        </button>
      </div>

      <div v-if="lastSale" class="quick-sale__success" role="status">
        <div>
          <strong>已记：{{ lastSale.summary }} · {{ formatYuan(lastSale.amount) }}</strong>
          <span>{{ lastSale.channel }} · 订单 #{{ lastSale.orderId }}</span>
        </div>
        <button v-if="canUndo" type="button" :disabled="busy" @click="undoLastSale">
          撤销误记
        </button>
      </div>

      <section id="quick-sale-cart" class="quick-sale__cart" aria-label="这单商品">
        <div class="quick-sale__section-title">
          <h3>这单</h3>
          <span>{{ cartCount }} 件</span>
        </div>
        <p v-if="!cartItems.length" class="quick-sale__empty">点下面的商品加入这单</p>
        <div v-else class="quick-sale__cart-lines">
          <div v-for="item in cartItems" :key="item.product.id" class="quick-sale__cart-line">
            <div>
              <strong>{{ item.product.name }}</strong>
              <span>{{ formatYuan(item.product.unit_price) }} / 件</span>
            </div>
            <div class="quick-sale__quantity">
              <button
                type="button"
                :disabled="busy || !!pending"
                :aria-label="'减少' + item.product.name"
                @click="changeQuantity(item.product, -1)"
              >
                −
              </button>
              <span>{{ item.quantity }}</span>
              <button
                type="button"
                :disabled="busy || !!pending || item.quantity >= item.product.onsite_qty"
                :aria-label="'增加' + item.product.name"
                @click="changeQuantity(item.product, 1)"
              >
                ＋
              </button>
            </div>
          </div>
        </div>

        <div v-if="cartItems.length" class="quick-sale__totals">
          <p v-if="quotePending" class="quick-sale__hint">正在计算本场优惠…</p>
          <p v-else-if="quoteError" class="quick-sale__quote-error">
            {{ quoteError }}
            <button type="button" @click="retryQuote">重新报价</button>
          </p>
          <template v-else-if="quote">
            <div v-if="quoteSummary.gross !== quoteSummary.payable" class="quick-sale__total-row">
              <span>原价</span><span>{{ formatYuan(quoteSummary.gross) }}</span>
            </div>
            <div
              v-for="discount in quoteSummary.discounts"
              :key="discount.name"
              class="quick-sale__total-row quick-sale__discount"
            >
              <span
                >{{ discount.name
                }}<template v-if="discount.count > 1"> ×{{ discount.count }}</template></span
              >
              <span>−{{ formatYuan(discount.saved) }}</span>
            </div>
            <div class="quick-sale__total-row quick-sale__payable">
              <strong>应收</strong><strong>{{ formatYuan(quoteSummary.payable) }}</strong>
            </div>
            <label class="quick-sale__actual">
              <span>实际收到（元）</span>
              <input
                v-model.number="actualYuan"
                type="number"
                min="0"
                step="0.01"
                inputmode="decimal"
                :disabled="busy || !!pending"
              />
            </label>
            <p v-if="manualDifference !== 0" class="quick-sale__hint">
              {{ manualDifference > 0 ? '手工让价' : '手工加价' }}
              {{ formatYuan(cents(Math.abs(manualDifference))) }}，差额由本社团承担。
            </p>
          </template>
        </div>
      </section>

      <section class="quick-sale__catalog" aria-label="本场商品">
        <div class="quick-sale__toolbar">
          <label for="quick-sale-search">本场商品</label>
          <input
            id="quick-sale-search"
            v-model.trim="search"
            type="search"
            placeholder="搜名称或编号"
            autocomplete="off"
          />
          <span>{{ visibleProducts.length }} 种</span>
        </div>
        <p v-if="eventDetailStore.isLoading" class="quick-sale__empty">正在加载商品…</p>
        <p v-else-if="eventDetailStore.error" class="quick-sale__empty">
          商品加载失败，请稍后重试。
        </p>
        <p v-else-if="!visibleProducts.length" class="quick-sale__empty">
          {{ search ? '没有找到商品' : '本场还没有上架商品' }}
        </p>
        <div v-else class="quick-sale__grid">
          <button
            v-for="product in visibleProducts"
            :key="product.id"
            type="button"
            class="quick-sale__product"
            :disabled="busy || !!pending || selectedQuantity(product.id) >= product.onsite_qty"
            :aria-label="'加一件' + product.name + '，' + formatYuan(product.unit_price)"
            @click="changeQuantity(product, 1)"
          >
            <span class="quick-sale__product-name">{{ product.name }}</span>
            <span class="quick-sale__product-meta"
              >{{ product.product_code }} · 剩 {{ product.onsite_qty }}</span
            >
            <span class="quick-sale__product-price">{{ formatYuan(product.unit_price) }}</span>
            <span v-if="selectedQuantity(product.id)" class="quick-sale__selected"
              >已选 {{ selectedQuantity(product.id) }}</span
            >
          </button>
        </div>
      </section>

      <div class="quick-sale__checkout">
        <div>
          <span>{{ cartCount }} 件 · {{ channel || '未选渠道' }}</span>
          <strong>{{
            actualAmount !== null && quote && !quotePending
              ? '实收 ' + formatYuan(actualAmount)
              : '待报价'
          }}</strong>
          <span v-if="quote && quoteSummary.gross > quoteSummary.payable">
            套装省 {{ formatYuan(cents(quoteSummary.gross - quoteSummary.payable)) }} ·
            <a href="#quick-sale-cart">查看明细</a>
          </span>
        </div>
        <button type="button" :disabled="!canCheckout" @click="checkout">确认已收，记这单</button>
      </div>
      <p class="quick-sale__footnote">
        这一步只是记账。请先核对真实到账；录错用“撤销误记”，真实退货去订单页登记。
      </p>
    </div>
  </PageShell>
  <Teleport to="body">
    <PaymentModal
      :show="showPaymentCodes"
      :qr-code-urls="paymentCodeUrls"
      :total="quote && !quotePending && !quoteError ? actualAmount : null"
      close-label="返回快售"
      @close="showPaymentCodes = false"
    />
  </Teleport>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { NButton } from 'naive-ui'
import { api, ApiRequestError, errorMessage, unwrap, type Schemas } from '@/api/client'
import { PageShell } from '@/components/ui'
import ChannelSelect from '@/components/shared/ChannelSelect.vue'
import PaymentModal from '@/components/customer/PaymentModal.vue'
import { useFeedback } from '@/composables/useFeedback'
import { getPendingQuickSale, recordQuickSale, retryPendingQuickSale } from '@/services/quickSale'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import { useEventStore } from '@/stores/eventStore'
import { useLotStore } from '@/stores/lotStore'
import { useOrderStore } from '@/stores/orderStore'
import { cents, formatYuan, fromCents, toCents, type Cents } from '@/utils/money'
import { summarizeQuote } from '@/utils/quote'
import { getImageUrl } from '@/services/url'

const props = defineProps<{ id: string | number }>()
const CHANNELS = ['微信', '支付宝', '现金'] as const
const eventId = computed(() => Number(props.id))
const eventDetailStore = useEventDetailStore()
const eventStore = useEventStore()
const showPaymentCodes = ref(false)
const paymentCodeUrls = computed(() => {
  const event = eventStore.events.find((event) => event.id === eventId.value)
  const urls = event?.qrcode_urls?.length
    ? event.qrcode_urls
    : event?.qrcode_url
      ? [event.qrcode_url]
      : []
  return urls.map(getImageUrl)
})
const lotStore = useLotStore()
const orderStore = useOrderStore()
const fb = useFeedback()
const search = ref('')
const busy = ref(false)
const pending = ref(getPendingQuickSale())
const channelKey = computed(() => 'quick_sale_channel:' + eventId.value)
const channel = ref(sessionStorage.getItem(channelKey.value) ?? '')
const showOtherChannel = ref(
  !!channel.value && !CHANNELS.some((option) => option === channel.value)
)
const quantities = ref<Record<number, number>>({})
const quote = ref<Schemas['LotQuoteResponse'] | null>(null)
const quotePending = ref(false)
const quoteError = ref<string | null>(null)
const actualYuan = ref<number | null>(null)
const lastSale = ref<{ orderId: number; summary: string; amount: Cents; channel: string } | null>(
  null
)
const canUndo = ref(false)
let quoteVersion = 0
let quoteTimer: ReturnType<typeof setTimeout> | null = null
let undoTimer: ReturnType<typeof setTimeout> | null = null

const activeLots = computed(() =>
  lotStore.lots.filter((lot) => lot.event_id === eventId.value && lot.candidate_ids.length > 0)
)
const visibleProducts = computed(() => {
  const term = search.value.toLocaleLowerCase()
  return eventDetailStore.products.filter(
    (product) =>
      product.event_id === eventId.value &&
      (!term ||
        product.name.toLocaleLowerCase().includes(term) ||
        product.product_code.toLocaleLowerCase().includes(term))
  )
})
const cartItems = computed(() =>
  eventDetailStore.products
    .filter((product) => product.event_id === eventId.value && selectedQuantity(product.id) > 0)
    .map((product) => ({ product, quantity: selectedQuantity(product.id) }))
)
const requestItems = computed<Schemas['CartItemRequest'][]>(() =>
  cartItems.value.map((item) => ({ product_id: item.product.id, quantity: item.quantity }))
)
const cartCount = computed(() => cartItems.value.reduce((total, item) => total + item.quantity, 0))
const cartGross = computed(() =>
  cents(cartItems.value.reduce((total, item) => total + item.product.unit_price * item.quantity, 0))
)
const quoteSummary = computed(() => summarizeQuote(quote.value, cartGross.value))
const actualAmount = computed<Cents | null>(() =>
  actualYuan.value !== null && Number.isFinite(actualYuan.value) && actualYuan.value >= 0
    ? toCents(actualYuan.value)
    : null
)
const manualDifference = computed(() =>
  quote.value && actualAmount.value !== null ? quote.value.solved_amount - actualAmount.value : 0
)
const stockEnough = computed(() =>
  cartItems.value.every((item) => item.quantity <= item.product.onsite_qty)
)
const canCheckout = computed(
  () =>
    cartCount.value > 0 &&
    !!channel.value &&
    !!quote.value &&
    !quotePending.value &&
    !quoteError.value &&
    actualAmount.value !== null &&
    stockEnough.value &&
    !busy.value &&
    !pending.value
)

function selectedQuantity(productId: number) {
  return quantities.value[productId] ?? 0
}

function selectChannel(value: string) {
  channel.value = value
  showOtherChannel.value = false
  sessionStorage.setItem(channelKey.value, value)
}

function selectOtherChannel(value: string) {
  channel.value = value
  sessionStorage.setItem(channelKey.value, value)
}

function changeQuantity(product: Schemas['ProductEventProduct'], delta: number) {
  if (busy.value || pending.value) return
  const next = Math.max(0, Math.min(product.onsite_qty, selectedQuantity(product.id) + delta))
  const updated = { ...quantities.value }
  if (next === 0) delete updated[product.id]
  else updated[product.id] = next
  quantities.value = updated
  scheduleQuote()
}

function scheduleQuote() {
  quoteVersion += 1
  const version = quoteVersion
  if (quoteTimer) clearTimeout(quoteTimer)
  quote.value = null
  quoteError.value = null
  actualYuan.value = null
  if (!requestItems.value.length) {
    quotePending.value = false
    return
  }
  quotePending.value = true
  quoteTimer = setTimeout(() => {
    void fetchQuote(version)
  }, 180)
}

async function fetchQuote(version: number) {
  try {
    const result = await unwrap(
      api.POST('/events/{event_id}/quote', {
        params: { path: { event_id: eventId.value } },
        body: { items: requestItems.value },
      })
    )
    if (version !== quoteVersion) return
    quote.value = result
    actualYuan.value = fromCents(result.solved_amount)
  } catch (error) {
    if (version !== quoteVersion) return
    quoteError.value = errorMessage(error, '报价失败，请重试')
  } finally {
    if (version === quoteVersion) quotePending.value = false
  }
}

function retryQuote() {
  scheduleQuote()
}

function showSale(order: Schemas['OrderResponse']) {
  if (undoTimer) clearTimeout(undoTimer)
  const count = order.items.reduce((total, item) => total + item.quantity, 0)
  lastSale.value = {
    orderId: order.id,
    summary: count + ' 件商品',
    amount: order.final_amount,
    channel: order.channel ?? '',
  }
  canUndo.value = true
  undoTimer = setTimeout(() => {
    canUndo.value = false
  }, 10_000)
}

async function refreshAfterSale() {
  await Promise.all([
    eventDetailStore.fetchProductsForEvent(eventId.value),
    orderStore.fetchCompletedOrders(),
    lotStore.fetchLots(eventId.value),
  ])
}

async function checkout() {
  if (!canCheckout.value || !quote.value || actualAmount.value === null) return
  const items = requestItems.value.map((item) => ({ ...item }))
  const expected = quote.value.solved_amount
  const final = actualAmount.value
  busy.value = true
  try {
    const order = await recordQuickSale(eventId.value, items, channel.value, expected, final)
    showSale(order)
    quantities.value = {}
    scheduleQuote()
    fb.success('这单已记入账本')
    await refreshAfterSale()
  } catch (error) {
    fb.error(error, '快售失败')
    if (error instanceof ApiRequestError && error.status === 409) {
      await refreshAfterSale()
      scheduleQuote()
    }
  } finally {
    pending.value = getPendingQuickSale()
    busy.value = false
  }
}

async function retryPending() {
  const request = pending.value
  if (!request || request.eventId !== eventId.value || busy.value) return
  busy.value = true
  try {
    const order = await retryPendingQuickSale()
    showSale(order)
    quantities.value = {}
    scheduleQuote()
    fb.success('上一单已确认')
    await refreshAfterSale()
  } catch (error) {
    fb.error(error, '重试失败')
  } finally {
    pending.value = getPendingQuickSale()
    busy.value = false
  }
}

async function undoLastSale() {
  const sale = lastSale.value
  if (!sale || !canUndo.value || busy.value) return
  await fb.confirm({
    title: '撤销误记',
    content:
      '确认订单 #' +
      sale.orderId +
      ' 只是点错或重复记录？这会冲回库存和账本。真实退货请到订单页登记。',
    positiveText: '撤销这笔记录',
    danger: true,
    onConfirm: async () => {
      busy.value = true
      try {
        await unwrap(
          api.PUT('/events/{event_id}/orders/{order_id}/status', {
            params: { path: { event_id: eventId.value, order_id: sale.orderId } },
            body: { status: 'cancelled' },
          })
        )
        lastSale.value = null
        canUndo.value = false
        await refreshAfterSale()
        fb.success('已撤销误记')
      } catch (error) {
        fb.error(error, '撤销失败')
        return false
      } finally {
        busy.value = false
      }
    },
  })
}

onMounted(() => {
  void lotStore.fetchLots(eventId.value)
})
onBeforeUnmount(() => {
  quoteVersion += 1
  if (quoteTimer) clearTimeout(quoteTimer)
  if (undoTimer) clearTimeout(undoTimer)
})
</script>

<style scoped>
.quick-sale {
  display: grid;
  gap: var(--space-md);
  padding-bottom: var(--space-lg);
}
.quick-sale__intro h2 {
  margin: 0 0 var(--space-xs);
  font-size: var(--font-xl);
}
.quick-sale__intro p,
.quick-sale__footnote {
  margin: 0;
  color: var(--secondary-text-color);
  line-height: 1.6;
}
.quick-sale__channel,
.quick-sale__deals,
.quick-sale__cart,
.quick-sale__pending,
.quick-sale__success {
  background: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  padding: var(--space-md);
}
.quick-sale__channel {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-md);
}
.quick-sale__channels {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-sm);
}
.quick-sale__other-channel {
  min-width: 180px;
  flex: 1;
}
.quick-sale__channels button,
.quick-sale__pending button,
.quick-sale__success button,
.quick-sale__quantity button,
.quick-sale__quote-error button {
  min-height: 44px;
  padding: var(--space-xs) var(--space-md);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  background: var(--bg-color);
  color: var(--primary-text-color);
  cursor: pointer;
}
.quick-sale__channels .quick-sale__channel-button--active {
  background: var(--accent-color);
  color: var(--text-white);
  border-color: var(--accent-color);
}
.quick-sale__hint,
.quick-sale__empty,
.quick-sale__footnote {
  color: var(--secondary-text-color);
}
.quick-sale__deals summary {
  cursor: pointer;
  font-weight: var(--weight-bold);
}
.quick-sale__deals ul {
  list-style: none;
  margin: var(--space-md) 0 0;
  padding: 0;
}
.quick-sale__deals li {
  display: flex;
  justify-content: space-between;
  gap: var(--space-md);
  padding: var(--space-xs) 0;
}
.quick-sale__pending,
.quick-sale__success {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-md);
}
.quick-sale__pending {
  border-color: var(--accent-color);
}
.quick-sale__pending p {
  margin: var(--space-xs) 0 0;
}
.quick-sale__success {
  background: var(--accent-color-light);
}
.quick-sale__success div {
  display: grid;
  gap: var(--space-xs);
}
.quick-sale__success span {
  color: var(--secondary-text-color);
}
.quick-sale__section-title,
.quick-sale__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-sm);
}
.quick-sale__section-title h3 {
  margin: 0;
  font-size: var(--font-lg);
}
.quick-sale__section-title span,
.quick-sale__toolbar span {
  color: var(--secondary-text-color);
}
.quick-sale__cart-lines {
  margin-top: var(--space-sm);
}
.quick-sale__cart-line {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm) 0;
  border-bottom: 1px solid var(--border-color);
}
.quick-sale__cart-line > div:first-child {
  display: grid;
  gap: var(--space-xs);
}
.quick-sale__cart-line span {
  color: var(--secondary-text-color);
}
.quick-sale__quantity {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}
.quick-sale__quantity span {
  min-width: 2ch;
  text-align: center;
  color: var(--primary-text-color);
}
.quick-sale__quantity button {
  width: 44px;
  padding: 0;
  font-size: var(--font-lg);
}
.quick-sale__totals {
  display: grid;
  gap: var(--space-xs);
  margin-top: var(--space-md);
}
.quick-sale__totals p {
  margin: 0;
}
.quick-sale__total-row {
  display: flex;
  justify-content: space-between;
  gap: var(--space-md);
}
.quick-sale__discount {
  color: var(--accent-color);
}
.quick-sale__payable {
  padding-top: var(--space-sm);
  border-top: 1px solid var(--border-color);
  font-size: var(--font-lg);
}
.quick-sale__actual {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-sm);
  margin-top: var(--space-sm);
}
.quick-sale__actual input,
.quick-sale__toolbar input {
  min-height: 44px;
  min-width: 0;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: var(--space-xs) var(--space-sm);
  background: var(--bg-color);
  color: var(--primary-text-color);
}
.quick-sale__actual input {
  width: 8em;
  text-align: right;
  font-size: var(--font-lg);
}
.quick-sale__quote-error {
  color: var(--error-color);
}
.quick-sale__quote-error button {
  margin-left: var(--space-sm);
}
.quick-sale__toolbar input {
  flex: 1;
}
.quick-sale__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: var(--space-sm);
  margin-top: var(--space-sm);
}
.quick-sale__product {
  position: relative;
  display: grid;
  align-content: space-between;
  gap: var(--space-xs);
  min-height: 105px;
  padding: var(--space-md);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  background: var(--card-bg-color);
  color: var(--primary-text-color);
  text-align: left;
  cursor: pointer;
}
.quick-sale__product:not(:disabled):hover,
.quick-sale__product:not(:disabled):focus-visible {
  border-color: var(--accent-color);
  outline-color: var(--accent-color);
}
.quick-sale__product:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
.quick-sale__product-name {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  line-height: 1.3;
}
.quick-sale__product-meta {
  color: var(--secondary-text-color);
  font-size: var(--font-sm);
}
.quick-sale__product-price {
  color: var(--accent-color);
  font-size: var(--font-xl);
  font-weight: var(--weight-bold);
}
.quick-sale__selected {
  position: absolute;
  right: var(--space-sm);
  top: var(--space-sm);
  border-radius: var(--radius-pill);
  background: var(--accent-color-light);
  color: var(--accent-color);
  padding: var(--space-xs);
  font-size: var(--font-xs);
}
.quick-sale__empty {
  margin: var(--space-md) 0;
  text-align: center;
}
.quick-sale__checkout {
  position: sticky;
  bottom: calc(var(--vendor-tabbar-height) + var(--space-xs));
  z-index: 10;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-sm) var(--space-md);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  background: var(--card-bg-color);
}
.quick-sale__checkout > div {
  display: grid;
  gap: var(--space-xs);
}
.quick-sale__checkout span {
  color: var(--secondary-text-color);
  font-size: var(--font-sm);
}
.quick-sale__checkout strong {
  font-size: var(--font-lg);
}
.quick-sale__checkout button {
  min-height: 48px;
  border: 0;
  border-radius: var(--radius-md);
  padding: var(--space-sm) var(--space-lg);
  background: var(--accent-color);
  color: var(--text-white);
  font-weight: var(--weight-bold);
  cursor: pointer;
}
.quick-sale__checkout button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.quick-sale__footnote {
  font-size: var(--font-sm);
}
@media (--phone) {
  .quick-sale__grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .quick-sale__product {
    min-height: 116px;
  }
  .quick-sale__toolbar label {
    display: none;
  }
  .quick-sale__checkout {
    border-radius: var(--radius-md);
  }
  .quick-sale__checkout button {
    padding: var(--space-sm);
  }
}
</style>
