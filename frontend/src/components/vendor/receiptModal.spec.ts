import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import ReceiptModal from './ReceiptModal.vue'
import { cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

vi.mock('@/composables/useFeedback', () => ({
  useFeedback: () => ({
    success: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    error: vi.fn(),
    loading: vi.fn(),
    confirm: vi.fn(),
    alert: vi.fn(),
  }),
}))
// ChannelSelect 挂载时会拉 /channels；这里只测快捷按钮，不展开「其他…」。
vi.mock('@/api/client', () => ({
  api: { GET: vi.fn(() => Promise.resolve([])) },
  unwrap: (p: unknown) => p,
}))

const LOT: Schemas['OrderLotResponse'] = {
  id: 5,
  lot_id: 2,
  name: '本子 + 画集 合购',
  price: cents(12000),
  original_amount: cents(14000),
}

async function open(lots: Schemas['OrderLotResponse'][] = [LOT]) {
  const w = mount(ReceiptModal, {
    props: { show: false, grossAmount: cents(14000), solvedAmount: cents(12000), lots },
    attachTo: document.body,
  })
  await w.setProps({ show: true })
  await flushPromises()
  return w
}

const q = (sel: string) => document.body.querySelector<HTMLElement>(sel)!
const qa = (sel: string) => Array.from(document.body.querySelectorAll<HTMLElement>(sel))
const button = (text: string) => qa('button').find((b) => b.textContent?.includes(text))!

afterEach(() => {
  document.body.innerHTML = ''
  localStorage.clear()
})

describe('ReceiptModal', () => {
  it('应收大字显示折后价；关掉套装开关后应收回到原价、实收跟着回填', async () => {
    const w = await open()
    expect(q('.due-amount').textContent).toContain('¥120.00')
    q('.lot-row').click()
    await flushPromises()
    expect(q('.due-amount').textContent).toContain('¥140.00')
    expect(button('确认收款').textContent).toContain('¥140.00')
    w.unmount()
  })

  it('点渠道按钮选中渠道；提交 payload 与改版前一致', async () => {
    const w = await open()
    button('现金').click()
    await flushPromises()
    expect(button('现金').getAttribute('aria-checked')).toBe('true')
    button('确认收款').click()
    await flushPromises()
    expect(w.emitted('confirm')?.[0]).toEqual([
      { channel: '现金', finalAmount: 12000, unapplyLotIds: [] },
    ])
    w.unmount()
  })

  it('「= 应收」把改过的实收填回应收', async () => {
    const w = await open([])
    const input = q('.received-input input') as HTMLInputElement
    input.value = '100'
    input.dispatchEvent(new Event('input'))
    input.dispatchEvent(new Event('blur'))
    await flushPromises()
    expect(document.body.textContent).toContain('手工折让')
    button('= 应收').click()
    await flushPromises()
    expect(document.body.textContent).not.toContain('手工折让')
    w.unmount()
  })
})
