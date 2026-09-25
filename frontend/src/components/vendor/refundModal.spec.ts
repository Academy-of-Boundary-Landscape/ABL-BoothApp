// 2026-09-26 收摊走查：退货弹窗「实际退款」输入框的边界。
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import RefundModal from './RefundModal.vue'
import { cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

const mocks = vi.hoisted(() => ({
  apiGet: vi.fn(),
  apiPost: vi.fn(),
  fbSuccess: vi.fn(),
  fbWarning: vi.fn(),
  fbError: vi.fn(),
}))

vi.mock('@/composables/useFeedback', () => ({
  useFeedback: () => ({
    success: mocks.fbSuccess,
    info: vi.fn(),
    warning: mocks.fbWarning,
    error: mocks.fbError,
    loading: vi.fn(),
    confirm: vi.fn(),
    alert: vi.fn(),
  }),
}))

vi.mock('@/api/client', () => ({
  api: { GET: mocks.apiGet, POST: mocks.apiPost },
  unwrap: (p: unknown) => p,
  errorMessage: (_e: unknown, fallback: string) => fallback,
}))

/** 走查里那张单的形状：同一商品因套装拆成两行 + 一行代卖。 */
const REFUNDS: Schemas['RefundListResponse'] = {
  history: [],
  lines: [
    {
      order_line_id: 4,
      event_product_id: 3,
      product_code: 'T01',
      name: '本子A',
      lot_name: '本子任选2本45',
      qty: 2,
      refunded_qty: 0,
      remaining_qty: 2,
      remaining_allocated: cents(4500),
      remaining_paid: cents(4249),
    },
    {
      order_line_id: 5,
      event_product_id: 3,
      product_code: 'T01',
      name: '本子A',
      lot_name: null,
      qty: 1,
      refunded_qty: 0,
      remaining_qty: 1,
      remaining_allocated: cents(3000),
      remaining_paid: cents(2834),
    },
  ],
}

const ORDER = {
  id: 3,
  channel: '微信',
  final_amount: cents(8500),
} as unknown as Schemas['OrderResponse']

async function open() {
  const w = mount(RefundModal, {
    props: { show: false, eventId: 3, order: ORDER },
    attachTo: document.body,
  })
  await w.setProps({ show: true })
  await flushPromises()
  return w
}

const qa = (sel: string) => Array.from(document.body.querySelectorAll<HTMLElement>(sel))
const button = (text: string) => qa('button').find((b) => b.textContent?.includes(text))!

async function setInput(el: HTMLInputElement, value: string) {
  el.value = value
  el.dispatchEvent(new Event('input'))
  el.dispatchEvent(new Event('blur'))
  await flushPromises()
}

/** 第 idx 个可退行的件数输入框。 */
const qtyInput = (idx: number) =>
  qa('.line-block')[idx].querySelector<HTMLInputElement>('.line-controls input')!
/** 「实际退款（元）」输入框。 */
const amountInput = () =>
  qa('.field')
    .find((f) => f.textContent?.includes('实际退款'))!
    .querySelector<HTMLInputElement>('input')!

beforeEach(() => {
  mocks.apiGet.mockImplementation((path: string) =>
    Promise.resolve(path === '/channels' ? [] : REFUNDS)
  )
  mocks.apiPost.mockResolvedValue({
    journal_id: 1,
    refund_amount: 0,
    allocated_total: 0,
    paid_total: 0,
  })
})

afterEach(() => {
  document.body.innerHTML = ''
  vi.clearAllMocks()
})

describe('RefundModal 实际退款', () => {
  it('同一商品的两行分别列出，只退套装那一行时默认退款 = 该行按件数切出的实付', async () => {
    const w = await open()
    expect(qa('.line-name').map((e) => e.textContent)).toEqual(['本子A（本子任选2本45）', '本子A'])

    await setInput(qtyInput(0), '1')
    // 4249 按 [1, 1] 切：各 2124，余 1 分给下标 0 → 2125（与后端 apportion 逐分一致）。
    expect(amountInput().value).toBe('21.25')
    w.unmount()
  })

  it('明确改到 0：发出 refund_amount = 0（顾客只退货不要钱，是合法的）', async () => {
    const w = await open()
    await setInput(qtyInput(1), '1')
    await setInput(amountInput(), '0')
    button('确认退货').click()
    await flushPromises()

    expect(mocks.apiPost).toHaveBeenCalledTimes(1)
    const body = mocks.apiPost.mock.calls[0][1].body
    expect(body.refund_amount).toBe(0)
    expect(body.lines).toEqual([{ order_line_id: 5, qty: 1, destination: '现场仓' }])
    w.unmount()
  })

  // bug：输入框被清空（null）时 `toCents(amountYuan ?? 0)` 把它当成「退 0 元」，
  // 静默提交一笔零退款——顾客一分钱没拿回，账上全记成货主留存。
  // 和收款弹窗（ReceiptModal.handleConfirm）的「请填写实收金额」同一条规则。
  it('清空实际退款输入框后提交：应拦下并提示，而不是按 0 元退款提交', async () => {
    const w = await open()
    await setInput(qtyInput(1), '1')
    await setInput(amountInput(), '')
    button('确认退货').click()
    await flushPromises()

    expect(mocks.apiPost).not.toHaveBeenCalled()
    expect(mocks.fbWarning).toHaveBeenCalled()
    w.unmount()
  })
})
