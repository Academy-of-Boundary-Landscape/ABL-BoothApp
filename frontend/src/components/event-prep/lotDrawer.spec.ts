import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import LotCandidatePicker from './LotCandidatePicker.vue'
import LotDrawer from './LotDrawer.vue'
import { useLotStore } from '@/stores/lotStore'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import type { Schemas } from '@/api/client'
import { cents } from '@/utils/money'

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

function product(
  id: number,
  name: string,
  owner: number,
  ownerName: string
): Schemas['ProductEventProduct'] {
  return {
    id,
    event_id: 1,
    master_product_id: 100 + id,
    owner_society_id: owner,
    owner_society_name: ownerName,
    product_code: `P${id}`,
    name,
    unit_price: cents(2500),
    stocked_qty: 10,
    onsite_qty: 10,
    image_url: null,
    category: null,
    tags: '',
  }
}

const PRODUCTS = [
  product(1, '本子甲', 1, '本社团'),
  product(2, '本子乙', 1, '本社团'),
  product(3, '代卖挂件', 2, '友社'),
]

afterEach(() => {
  document.body.innerHTML = ''
})

describe('LotCandidatePicker', () => {
  function cards(w: ReturnType<typeof mount>) {
    return w.findAll('button.card')
  }

  it('一件都没选时所有卡片可选', () => {
    const w = mount(LotCandidatePicker, { props: { modelValue: [], products: PRODUCTS } })
    expect(cards(w).every((c) => c.attributes('disabled') === undefined)).toBe(true)
  })

  it('选了一件之后，别家货主的卡片被禁用并说明原因', () => {
    const w = mount(LotCandidatePicker, { props: { modelValue: [1], products: PRODUCTS } })
    const [a, b, other] = cards(w)
    expect(a.attributes('aria-pressed')).toBe('true')
    expect(b.attributes('disabled')).toBeUndefined()
    expect(other.attributes('disabled')).toBeDefined()
    expect(other.attributes('title')).toContain('同一个货主')
    expect(w.text()).toContain('货主：本社团')
  })

  it('点卡片切换选中，点被禁用的卡片不变', async () => {
    const w = mount(LotCandidatePicker, { props: { modelValue: [1], products: PRODUCTS } })
    await cards(w)[1].trigger('click')
    expect(w.emitted('update:modelValue')?.[0]).toEqual([[1, 2]])
    await cards(w)[0].trigger('click')
    expect(w.emitted('update:modelValue')?.[1]).toEqual([[]])
    await cards(w)[2].trigger('click')
    expect(w.emitted('update:modelValue')).toHaveLength(2)
  })
})

describe('LotDrawer', () => {
  async function openWith(lot: Schemas['LotResponse'] | null) {
    const pinia = createPinia()
    setActivePinia(pinia)
    const lotStore = useLotStore()
    const create = vi.spyOn(lotStore, 'createLot').mockResolvedValue(undefined as never)
    const update = vi.spyOn(lotStore, 'updateLot').mockResolvedValue(undefined as never)
    vi.spyOn(lotStore, 'previewLot').mockResolvedValue(undefined as never)
    useEventDetailStore().products = PRODUCTS
    const w = mount(LotDrawer, {
      props: { show: false, eventId: 1, lot },
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await w.setProps({ show: true })
    await flushPromises()
    return { w, create, update }
  }

  function modeCards() {
    return Array.from(document.body.querySelectorAll<HTMLButtonElement>('.mode-card'))
  }

  it('点模式卡片切换 allowRepeat，当前卡片 aria-checked', async () => {
    const { w } = await openWith(null)
    const [fixed, repeat] = modeCards()
    expect(fixed.getAttribute('aria-checked')).toBe('true')
    repeat.click()
    await flushPromises()
    expect(modeCards()[1].getAttribute('aria-checked')).toBe('true')
    expect(modeCards()[0].getAttribute('aria-checked')).toBe('false')
    w.unmount()
  })

  it('编辑后提交的 payload 与改版前一致', async () => {
    const lot: Schemas['LotResponse'] = {
      id: 7,
      event_id: 1,
      name: '本子任选2本45',
      pick_count: 2,
      total_price: cents(4500),
      allow_repeat: true,
      candidate_ids: [1, 2],
      owner_society_id: 1,
      owner_society_name: '本社团',
    }
    const { w, update } = await openWith(lot)
    const save = Array.from(document.body.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('保存修改')
    )
    save!.click()
    await flushPromises()
    expect(update).toHaveBeenCalledWith(1, 7, {
      name: '本子任选2本45',
      pick_count: 2,
      total_price: 4500,
      candidate_ids: [1, 2],
      allow_repeat: true,
    })
    w.unmount()
  })

  it('各 1 件凑齐但候选不够件数时当场提示', async () => {
    const lot: Schemas['LotResponse'] = {
      id: 8,
      event_id: 1,
      name: '三本合购',
      pick_count: 3,
      total_price: cents(6000),
      allow_repeat: false,
      candidate_ids: [1, 2],
      owner_society_id: 1,
      owner_society_name: '本社团',
    }
    const { w } = await openWith(lot)
    expect(document.body.textContent).toContain('至少要选 3 件候选商品')
    w.unmount()
  })
})
