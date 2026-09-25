<!--
  收摊向导（母 spec 6.4，spec §5.6）：清 pending → 盘点 → 带回 → 转已结算。

  当前在第几屏**完全由后端状态推出来**，不存本地 step 变量——存了就会出现
  「退出重进回到第一步、但货已经带回了」这种错位。四步之间没有会话状态，
  每个动作成功后重新拉一次 `GET /closing`。

  能不能进下一步也由后端的 `blockers` 说了算：结算按钮在 blockers 非空时禁用，
  其余动作把后端那句错误原文显示出来即可。

  这是**页面组件**（不是弹窗）：挂载即加载，不会退出；手机上按手机优先设计——
  步骤指示缩成一行字、盘点每行一个大号数字键盘、主按钮固定在底部 tab 栏之上。
-->
<template>
  <div class="closing-wizard">
    <AsyncState
      :loading="store.isLoading && !store.state"
      :error="loadError"
      loading-text="正在加载收摊状态…"
      @retry="reload"
    >
      <template v-if="store.state">
        <!-- 步骤指示：宽屏用 n-steps；手机上缩成一行，省掉横向空间。 -->
        <div v-if="isPhone" class="step-line">第 {{ step }} 步 / 共 4 步 · {{ stepTitle }}</div>
        <n-steps v-else :current="step" size="small" class="steps">
          <n-step v-for="title in STEP_TITLES" :key="title" :title="title" />
        </n-steps>

        <!-- ① 清 pending -->
        <section v-if="step === 1" class="screen">
          <p class="screen-hint">
            还有 {{ store.state.pending_orders.length }} 单待处理，逐单完成或取消之后才能盘点。
          </p>
          <EmptyState
            v-if="!store.state.pending_orders.length"
            compact
            title="没有待处理的订单，可以进入盘点。"
          />
          <div v-for="o in store.state.pending_orders" :key="o.id" class="row">
            <div class="row-info">
              <span class="row-title">#{{ o.id }}</span>
              <span>{{ formatTimestamp(o.created_at, false) }}</span>
              <span>{{ o.item_count }} 件</span>
              <span class="row-amount">{{ formatYuan(o.final_amount) }}</span>
            </div>
            <n-space size="small">
              <n-button size="small" :disabled="isBusy" @click="cancelOne(o)">取消</n-button>
              <n-button size="small" type="primary" :disabled="isBusy" @click="completeOne(o)">
                完成
              </n-button>
            </n-space>
          </div>
          <div class="screen-actions" :class="{ 'screen-actions--fixed': isPhone }">
            <n-button
              type="error"
              tertiary
              :loading="isBusy"
              :disabled="!store.state.pending_orders.length"
              @click="cancelAll"
            >
              全部取消
            </n-button>
          </div>
        </section>

        <!-- ② 盘点 -->
        <section v-else-if="step === 2" class="screen">
          <!-- 粘性条：随时知道还差几行；「只看未填」只看没填的，避免一屏划到底。 -->
          <div class="stocktake-bar">
            <span class="stocktake-count"
              >已盘 {{ countedCount }} / {{ stocktakeRows.length }}</span
            >
            <label class="stocktake-filter">
              <n-switch v-model:value="onlyUnfilled" size="small" />
              <span>只看未填</span>
            </label>
          </div>
          <p class="screen-hint">
            盘点现场仓。<strong>数过一致的也要报</strong>——「我数了，一致」和「我没数」是两件事。
          </p>
          <EmptyState
            v-if="onlyUnfilled && !visibleRows.length"
            compact
            title="都盘完了，可以提交。"
          />
          <div
            v-for="p in visibleRows"
            :key="p.event_product_id"
            class="row stocktake-row"
            data-test="stocktake-row"
          >
            <div class="row-info">
              <span class="row-title">{{ p.name }}</span>
              <span class="row-code">{{ p.product_code }}</span>
              <span class="row-muted">{{ p.owner_name }} · 账面 {{ p.qty }} 件</span>
            </div>
            <n-input-number
              v-model:value="counts[p.event_product_id]"
              :min="0"
              :precision="0"
              :show-button="false"
              :input-props="{ inputmode: 'numeric' }"
              placeholder="实数"
              class="count-input"
            />
          </div>
          <p class="screen-note">跳过盘点之后，结算单上会写「未盘点，剩余数为账面推算」。</p>
          <div class="screen-actions" :class="{ 'screen-actions--fixed': isPhone }">
            <n-button :disabled="isBusy" @click="skipStocktake">跳过盘点</n-button>
            <n-button type="primary" :loading="isBusy" @click="submitStocktake">
              提交盘点
            </n-button>
          </div>
        </section>

        <!-- ③ 带回 -->
        <section v-else-if="step === 3" class="screen">
          <p class="screen-hint">以下商品将带回，共 {{ takebackTotal }} 件。</p>
          <EmptyState
            v-if="!store.state.onsite_remaining.length"
            compact
            title="现场仓已经空了。"
          />
          <div v-for="p in store.state.onsite_remaining" :key="p.event_product_id" class="row">
            <div class="row-info">
              <span class="row-title">{{ p.name }}</span>
              <span class="row-code">{{ p.product_code }}</span>
              <span class="row-muted">{{ p.owner_name }}</span>
              <span class="row-amount">×{{ p.qty }}</span>
            </div>
          </div>
          <div class="screen-actions" :class="{ 'screen-actions--fixed': isPhone }">
            <n-button type="primary" :loading="isBusy" @click="doTakeback">确认带回</n-button>
          </div>
        </section>

        <!-- ④ 结算 -->
        <section v-else class="screen">
          <div v-if="store.state.status === '已结算'" class="settled-note">
            <p>
              <strong>账本已冻结。</strong>
              之后仍然可以补垫付、结算调整和收摊清点，其余都改不了了。 下面就是这一场的结算单。
            </p>
          </div>
          <template v-else>
            <div v-if="store.state.blockers.length" class="blockers">
              <p v-for="(b, i) in store.state.blockers" :key="i" class="blocker-line">⚠ {{ b }}</p>
            </div>
            <p v-else class="screen-hint">没有拦路的项了，确认无误后结束展会。</p>
            <div class="screen-actions" :class="{ 'screen-actions--fixed': isPhone }">
              <n-button
                type="primary"
                :loading="isBusy"
                :disabled="store.state.blockers.length > 0"
                @click="doSettle"
              >
                结束展会
              </n-button>
            </div>
          </template>
        </section>
      </template>
    </AsyncState>

    <!-- 第①屏「完成」复用现成的收款弹窗补录渠道。 -->
    <ReceiptModal
      :show="showReceipt"
      :gross-amount="receiptOrder?.gross_amount"
      :solved-amount="receiptOrder?.solved_amount"
      :lots="receiptOrder?.lots ?? []"
      @confirm="onReceiptConfirm"
      @cancel="closeReceipt"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NButton, NInputNumber, NSpace, NStep, NSteps, NSwitch } from 'naive-ui'
import { AsyncState, EmptyState } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import { useViewport } from '@/composables/useViewport'
import ReceiptModal from '@/components/vendor/ReceiptModal.vue'
import { useClosingStore } from '@/stores/closingStore'
import { useOrderStore } from '@/stores/orderStore'
import { formatYuan, type Cents } from '@/utils/money'
import { formatTimestamp } from '@/utils/dateFormatter'
import { api, unwrap, errorMessage, type Schemas } from '@/api/client'

// 页面组件：不再有 show prop 与 close 事件，挂载即加载；只有 settled 事件。
const props = defineProps<{ eventId: string | number }>()
const emit = defineEmits<{ (e: 'settled'): void }>()

const store = useClosingStore()
const orderStore = useOrderStore()
const fb = useFeedback()
const { isPhone } = useViewport()

const STEP_TITLES = ['清点订单', '盘点', '带回', '结算'] as const

const isBusy = ref(false)
const loadError = ref<string | null>(null)
const counts = ref<Record<number, number | null>>({})
// 「跳过盘点」是唯一一处必要的本地状态：后端没有「跳过」这个事实，
// 而这一步不落库。它只在「还有货、但摊主决定不盘点」时把界面推到带回屏；
// 货一旦带回（onsite_remaining 为空），步骤就纯由后端状态决定，重进不会错位。
// 每次打开都重置是有意的：跳过没有写任何后端事实，关掉再进来等于「什么都还没做」，
// 再问一次合情合理；若把它持久化，摊主上次跳过、这次只想确认一下就再也回不到盘点屏。
const skippedStocktake = ref(false)
// 只看未填：纯界面筛选，不落库。
const onlyUnfilled = ref(false)

const showReceipt = ref(false)
const receiptOrder = ref<Schemas['OrderResponse'] | null>(null)

const step = computed(() => {
  const s = store.state
  if (!s) return 1
  if (s.status === '已结算') return 4
  if (s.pending_orders.length > 0) return 1
  if (s.onsite_remaining.length > 0) {
    // 盘点过（或被跳过）才去带回，否则先盘点。
    return s.stocktaken_at || skippedStocktake.value ? 3 : 2
  }
  // 现场仓空了：带回已完成（或本来就没货），只剩结算。
  return 4
})

const stepTitle = computed(() => STEP_TITLES[step.value - 1])

const stocktakeRows = computed(() => store.state?.onsite_remaining ?? [])

/** 粘性条上的「已盘 X」：填了数字（含 0）的行数，空输入框不算。 */
const countedCount = computed(
  () => stocktakeRows.value.filter((p) => Number.isFinite(counts.value[p.event_product_id])).length
)

const visibleRows = computed(() => {
  if (!onlyUnfilled.value) return stocktakeRows.value
  return stocktakeRows.value.filter((p) => !Number.isFinite(counts.value[p.event_product_id]))
})

const takebackTotal = computed(() =>
  (store.state?.onsite_remaining || []).reduce((sum, p) => sum + p.qty, 0)
)

// 盘点输入**不预填账面数**；已经动过的值不被后续刷新覆盖。
// 预填会让「什么都不数直接提交」等于声称「每个商品我都数了且都对」，
// 而「我数了、一致」和「我没数这个」正是盘点要分开的两件事——后端那条
// stocktaken_at 迁移存在的全部理由就是这个。同 AdminEventSettlement 的清点输入。
// 「摊主没数」由「跳过盘点」承接，那条路径留下 stocktaken_at = null。
watch(
  () => store.state?.onsite_remaining,
  (rows) => {
    if (!rows) return
    const next: Record<number, number | null> = {}
    for (const p of rows) {
      const existing = counts.value[p.event_product_id]
      next[p.event_product_id] = Number.isFinite(existing) ? existing : null
    }
    counts.value = next
  },
  { immediate: true }
)

async function reload() {
  loadError.value = null
  skippedStocktake.value = false
  onlyUnfilled.value = false
  showReceipt.value = false
  receiptOrder.value = null
  store.resetStore()
  try {
    // 收摊接口只给待处理单的金额摘要；ReceiptModal 要原价/套装，先把订单全量拉回来。
    await orderStore.pollPendingOrders()
    await store.fetchState(Number(props.eventId))
  } catch (err) {
    // 页面不手写错误态，交给 AsyncState 的 error 态（带重试）。
    loadError.value = errorMessage(err, '无法加载收摊状态。')
  }
}

// 页面组件挂载即加载；切换展会时按新的 eventId 重新拉一遍。
watch(() => props.eventId, reload, { immediate: true })

function skipStocktake() {
  skippedStocktake.value = true
}

async function doSubmitStocktake(payload: Schemas['StocktakeRequest']['counts']) {
  isBusy.value = true
  try {
    await store.stocktake(Number(props.eventId), payload)
    fb.success('盘点已提交')
  } catch (err) {
    fb.error(err, '提交盘点失败')
  } finally {
    isBusy.value = false
  }
}

async function submitStocktake() {
  const rows = store.state?.onsite_remaining
  if (!rows) return
  // n-input-number 被清空时 model 是 null，`?? 0` 会把它当成「数出来 0 件」，
  // 后端照写一条全量盘亏腿——该商品随即从 onsite_remaining 消失，界面再也纠正不了。
  // 收全量：每个商品都要报数，数过一致的也要报（后端漏一个就 400）。
  const blank = rows.filter((p) => !Number.isFinite(counts.value[p.event_product_id]))
  if (blank.length) {
    fb.warning(`这些商品还没填实数：${blank.map((p) => p.name).join('、')}`)
    return
  }
  const payload: Schemas['StocktakeRequest']['counts'] = rows.map((p) => ({
    event_product_id: p.event_product_id,
    counted_qty: counts.value[p.event_product_id] ?? 0,
  }))

  // 盘亏是不可逆的账（只能靠结算调整补救），差异提交前让摊主核对一遍。
  const diffs = rows.filter((p) => counts.value[p.event_product_id] !== p.qty)
  if (diffs.length) {
    const detail = diffs
      .map((p) => `${p.name}：账面 ${p.qty} → 实数 ${counts.value[p.event_product_id]}`)
      .join('；')
    await fb.confirm({
      title: '确认盘点差异',
      content: `以下商品的实数与账面不一致：${detail}。盘点差异提交后不可直接撤销，确认无误再提交。`,
      positiveText: '确认提交',
      negativeText: '返回核对',
      onConfirm: () => doSubmitStocktake(payload),
    })
    return
  }
  await doSubmitStocktake(payload)
}

async function doTakeback() {
  isBusy.value = true
  try {
    await store.takeback(Number(props.eventId))
    fb.success('已确认带回')
  } catch (err) {
    fb.error(err, '确认带回失败')
  } finally {
    isBusy.value = false
  }
}

async function doSettle() {
  isBusy.value = true
  try {
    await store.settle(Number(props.eventId))
    fb.success('展会已结束，账本已冻结')
    emit('settled')
  } catch (err) {
    fb.error(err, '结束展会失败')
  } finally {
    isBusy.value = false
  }
}

async function cancelOne(order: Schemas['ClosingPendingOrderRow']) {
  isBusy.value = true
  try {
    await unwrap(
      api.PUT('/events/{event_id}/orders/{order_id}/status', {
        params: { path: { event_id: Number(props.eventId), order_id: order.id } },
        body: { status: 'cancelled' },
      })
    )
    await store.fetchState(Number(props.eventId))
    await orderStore.pollPendingOrders()
  } catch (err) {
    fb.error(errorMessage(err, `取消 #${order.id} 失败`))
  } finally {
    isBusy.value = false
  }
}

/** 「全部取消」循环调现有端点：每单独立事务，失败的不回滚已成功的。 */
async function cancelAll() {
  const failed: { id: number; msg: string }[] = []
  isBusy.value = true
  try {
    for (const o of [...(store.state?.pending_orders ?? [])]) {
      try {
        await unwrap(
          api.PUT('/events/{event_id}/orders/{order_id}/status', {
            params: { path: { event_id: Number(props.eventId), order_id: o.id } },
            body: { status: 'cancelled' },
          })
        )
      } catch (err) {
        failed.push({ id: o.id, msg: errorMessage(err, '取消失败') })
      }
    }
    await store.fetchState(Number(props.eventId))
    await orderStore.pollPendingOrders()
  } catch (err) {
    // 刷新失败不能把 isBusy 卡在 true，否则整个向导的按钮永久禁用。
    fb.error(err, '刷新收摊状态失败')
  } finally {
    isBusy.value = false
    // 失败的会留在重新拉回来的列表里，逐条把后端那句话显示出来。
    // 放在 finally 里：即使上面的刷新也失败，摊主仍要知道是哪几单没取消掉。
    for (const f of failed) {
      fb.error(`#${f.id}：${f.msg}`, undefined, { duration: 6000 })
    }
  }
}

async function completeOne(order: Schemas['ClosingPendingOrderRow']) {
  let full = orderStore.pendingOrders.find((o) => o.id === order.id)
  if (!full) {
    await orderStore.pollPendingOrders()
    full = orderStore.pendingOrders.find((o) => o.id === order.id)
  }
  if (!full) {
    fb.error('找不到这张订单的完整信息，请先刷新')
    return
  }
  receiptOrder.value = full
  showReceipt.value = true
}

function closeReceipt() {
  showReceipt.value = false
  receiptOrder.value = null
}

async function onReceiptConfirm(payload: {
  channel: string
  finalAmount: Cents
  unapplyLotIds: number[]
}) {
  const order = receiptOrder.value
  showReceipt.value = false
  if (!order) return
  try {
    await orderStore.markOrderAsCompleted(
      order.id,
      payload.channel,
      payload.finalAmount,
      payload.unapplyLotIds
    )
    await store.fetchState(Number(props.eventId))
    fb.success('已记录收款')
  } catch (err) {
    fb.error(err, '操作失败')
  } finally {
    receiptOrder.value = null
  }
}
</script>

<style scoped>
/* 步骤指示 */
.steps {
  margin-bottom: var(--space-lg);
}
.step-line {
  margin-bottom: var(--space-lg);
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-pill);
  background-color: var(--card-bg-color);
  border: 1px solid var(--border-color);
  color: var(--primary-text-color);
  font-size: var(--font-base);
  font-weight: var(--weight-bold);
  text-align: center;
}

.screen-hint {
  margin: 0 0 var(--space-md);
  color: var(--text-muted);
  font-size: var(--font-sm);
  line-height: 1.6;
}
.screen-note {
  margin: var(--space-sm) 0 0;
  color: var(--warning-color);
  font-size: var(--font-sm);
}

/* 盘点粘性条 */
.stocktake-bar {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  padding: var(--space-sm) var(--space-md);
  margin-bottom: var(--space-md);
  background-color: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
}
.stocktake-count {
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
  font-variant-numeric: tabular-nums;
}
.stocktake-filter {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  min-height: 44px;
  color: var(--text-muted);
  font-size: var(--font-sm);
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  padding: var(--space-sm) 0;
  border-bottom: 1px dashed var(--border-color);
}
.row-info {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  flex-wrap: wrap;
  font-size: var(--font-sm);
  color: var(--text-muted);
  min-width: 0;
}
.row-title {
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
}
.row-code {
  color: var(--text-disabled);
}
.row-muted {
  color: var(--text-muted);
}
.row-amount {
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}
.count-input {
  flex: 0 0 120px;
}
/* 盘点输入框要有手机能点的大号码；n-input 默认 34px。 */
.count-input :deep(.n-input) {
  min-height: 44px;
}

.screen-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-md);
  margin-top: var(--space-lg);
}

.blockers {
  border: 1px solid var(--warning-color);
  border-radius: var(--radius-sm);
  padding: var(--space-sm) var(--space-md);
}
.blocker-line {
  margin: var(--space-xs) 0;
  color: var(--warning-color);
  font-size: var(--font-sm);
}
.settled-note {
  border-left: 3px solid var(--accent-color);
  padding: var(--space-sm) var(--space-md);
  background: var(--card-bg-color);
}
.settled-note p {
  margin: 0;
  line-height: 1.6;
}

@media (--phone) {
  /* 主按钮固定在底部 tab 栏之上（外壳在根元素写了 --vendor-tabbar-height）。 */
  .screen-actions--fixed {
    position: fixed;
    left: 0;
    right: 0;
    bottom: var(--vendor-tabbar-height, 0px);
    z-index: 90;
    margin: 0;
    padding: var(--space-sm) var(--space-lg);
    background-color: var(--card-bg-color);
    border-top: 1px solid var(--border-color);
  }
  .screen-actions--fixed > * {
    flex: 1;
  }
  /* 固定条会盖住内容，底部留出高于它的空间。 */
  .screen {
    padding-bottom: calc(var(--space-2xl) + var(--space-lg));
  }
  /* 盘点行：商品信息一行，输入框占满下一行，方便点。 */
  .row {
    flex-wrap: wrap;
  }
  .count-input {
    flex: 1 1 100%;
  }
}
</style>
