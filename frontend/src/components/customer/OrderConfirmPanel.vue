<!--
  顾客下单前的确认面板。

  自助点单的平板上，顾客点「去结算」之后看到的是这一屏：买了什么、优惠了多少、要付多少。
  原来是一个系统风格的小确认框（「共 6 件商品，合计 ¥215.00」），字小、按钮小、还是蓝色，
  顾客看不出买了什么就要点确认。手机上 AppModal 自动全屏。
-->
<template>
  <AppModal
    :show="show"
    title="确认下单"
    size="md"
    :mask-closable="!submitting"
    :close-on-esc="!submitting"
    :closable="!submitting"
    @update:show="(v) => !v && emit('cancel')"
  >
    <ul class="items">
      <li v-for="item in cart" :key="item.id" class="item">
        <span class="thumb">
          <img v-if="item.image_url" :src="item.image_url" :alt="item.name" class="thumb-img" />
          <span v-else class="thumb-fallback">{{ item.name.slice(0, 1) }}</span>
        </span>
        <span class="name">{{ item.name }}</span>
        <span class="qty">×{{ item.quantity }}</span>
        <span class="subtotal"><Money :value="cents(item.unit_price * item.quantity)" /></span>
      </li>
    </ul>

    <div class="summary">
      <div v-if="payable !== total" class="row subtle">
        <span>原价</span>
        <Money :value="total" strike />
      </div>
      <div v-for="d in discounts" :key="d.name" class="row discount">
        <span
          >已应用：{{ d.name }}<template v-if="d.count > 1"> ×{{ d.count }}</template></span
        >
        <span>−{{ formatYuan(d.saved) }}</span>
      </div>
      <div class="row total">
        <span>共 {{ itemCount }} 件，应付</span>
        <Money :value="payable" size="lg" />
      </div>
    </div>

    <template #footer>
      <div class="actions">
        <n-button size="large" class="btn-back" :disabled="submitting" @click="emit('cancel')">
          再看看
        </n-button>
        <n-button
          type="primary"
          size="large"
          class="btn-confirm"
          :loading="submitting"
          @click="emit('confirm')"
        >
          确认下单
        </n-button>
      </div>
    </template>
  </AppModal>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton } from 'naive-ui'
import { AppModal, Money } from '@/components/ui'
import { cents, formatYuan, type Cents } from '@/utils/money'
import type { Schemas } from '@/api/client'
import type { QuoteDiscount } from '@/utils/quote'

type CartItem = Schemas['ProductEventProduct'] & { quantity: number }

const props = withDefaults(
  defineProps<{
    show: boolean
    cart: CartItem[]
    /** 原价合计（分） */
    total: Cents
    /** 折后应付（分） */
    payable: Cents
    discounts?: QuoteDiscount[]
    /** 下单请求在途：按钮转圈、不许关 */
    submitting?: boolean
  }>(),
  { discounts: () => [], submitting: false }
)

const emit = defineEmits<{ (e: 'confirm'): void; (e: 'cancel'): void }>()

const itemCount = computed(() => props.cart.reduce((n, i) => n + i.quantity, 0))
</script>

<style scoped>
.items {
  max-height: 50vh;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}

.item {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto auto;
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-sm) 0;
  border-bottom: 1px solid var(--divider-color);
  font-size: var(--font-md);
}

.thumb {
  display: flex;
  align-items: center;
  justify-content: center;
  width: var(--space-2xl);
  height: var(--space-2xl);
  overflow: hidden;
  border-radius: var(--radius-sm);
  background: var(--bg-color);
}

.thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.thumb-fallback {
  color: var(--text-muted);
}

.name {
  display: -webkit-box;
  overflow: hidden;
  line-height: var(--leading-tight);
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.qty {
  color: var(--secondary-text-color);
}

.subtotal {
  min-width: 5em;
  text-align: right;
}

.summary {
  margin-top: var(--space-md);
}

.row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-md);
  padding: var(--space-xs) 0;
}

.row.subtle {
  color: var(--text-muted);
}

.row.discount {
  color: var(--success-color);
  font-size: var(--font-sm);
}

.row.total {
  margin-top: var(--space-sm);
  padding-top: var(--space-md);
  border-top: 1px solid var(--border-color);
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
}

.actions {
  display: grid;
  grid-template-columns: 1fr 2fr;
  gap: var(--space-md);
  width: 100%;
}

.btn-back,
.btn-confirm {
  min-height: 52px;
  font-size: var(--font-md);
}

.btn-confirm {
  font-weight: var(--weight-bold);
}
</style>
