import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import type { Ref } from 'vue'
import BarcodeScanPanel from '@/components/customer/BarcodeScanPanel.vue'
import { AppModal } from '@/components/ui'
import { playScanBeep } from '@/utils/scanBeep'
import { cents } from '@/utils/money'
import type { Schemas } from '@/api/client'

const scannerMock = vi.hoisted(() => ({
  opts: null as null | { onCode: (code: string) => void },
  start: vi.fn(),
  pause: vi.fn(),
  resume: vi.fn(),
  stop: vi.fn(),
}))

vi.mock('@/composables/useBarcodeScanner', async () => {
  const { ref } = await import('vue')
  return {
    useBarcodeScanner: (opts: { onCode: (code: string) => void }) => {
      scannerMock.opts = opts
      return {
        running: ref(true),
        loading: ref(false),
        error: ref(''),
        start: scannerMock.start,
        pause: scannerMock.pause,
        resume: scannerMock.resume,
        stop: scannerMock.stop,
      }
    },
  }
})

const cameraMock = vi.hoisted(() => ({
  start: vi.fn(),
  stop: vi.fn(),
  flip: vi.fn(),
  setTorch: vi.fn(),
  error: null as Ref<string> | null,
  facing: null as Ref<'user' | 'environment'> | null,
}))

vi.mock('@/composables/useCamera', async () => {
  const { ref, shallowRef } = await import('vue')
  return {
    useCamera: () => {
      const error = ref('')
      const facing = ref<'user' | 'environment'>('environment')
      cameraMock.error = error
      cameraMock.facing = facing
      return {
        stream: shallowRef(null),
        isActive: ref(false),
        facing,
        error,
        torchSupported: ref(false),
        torchOn: ref(false),
        start: cameraMock.start,
        stop: cameraMock.stop,
        flip: cameraMock.flip,
        setTorch: cameraMock.setTorch,
      }
    },
  }
})

vi.mock('@/utils/scanBeep', () => ({ playScanBeep: vi.fn() }))

function makeProduct(
  overrides: Partial<Schemas['ProductEventProduct']> = {}
): Schemas['ProductEventProduct'] {
  return {
    id: 1,
    event_id: 3,
    master_product_id: 11,
    name: '本子A',
    product_code: 'P001',
    barcode: null,
    category: '本子',
    tags: '',
    owner_society_id: 1,
    owner_society_name: '测试社团',
    stocked_qty: 100,
    onsite_qty: 10,
    unit_price: cents(3000),
    image_url: null,
    ...overrides,
  }
}

function mountPanel(
  products: Schemas['ProductEventProduct'][],
  cart: { id: number; quantity: number }[] = []
): VueWrapper {
  return mount(BarcodeScanPanel, {
    props: { products, cart },
  })
}

/** 触发一次「扫到码」，顺带等组件更新。 */
async function scan(code: string) {
  scannerMock.opts?.onCode(code)
  await nextTick()
}

beforeEach(() => {
  scannerMock.opts = null
  scannerMock.start.mockResolvedValue(undefined)
  cameraMock.start.mockResolvedValue(true)
  cameraMock.flip.mockResolvedValue(true)
  vi.mocked(playScanBeep).mockClear()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('BarcodeScanPanel 命中处理', () => {
  it('命中有货 → emit add 且最近记录出现', async () => {
    const product = makeProduct({ barcode: '4901234567894' })
    const wrapper = mountPanel([product])
    await flushPromises()

    await scan('4901234567894')

    expect(wrapper.emitted('add')).toHaveLength(1)
    expect(wrapper.emitted('add')?.[0]?.[0]).toMatchObject({ id: product.id })
    expect(wrapper.find('.scan-recent').text()).toContain('+1 本子A ¥30.00')
    wrapper.unmount()
  })

  it('已售罄 → 不 emit add，提示行出现「已售罄」，不弹窗', async () => {
    const product = makeProduct({ barcode: '4901234567894', onsite_qty: 0 })
    const wrapper = mountPanel([product])
    await flushPromises()

    await scan('4901234567894')

    expect(wrapper.emitted('add')).toBeUndefined()
    expect(wrapper.find('.scan-hint').text()).toContain('本子A 已售罄')
    expect(wrapper.findComponent(AppModal).props('show')).toBe(false)
    expect(playScanBeep).toHaveBeenCalledWith(false)
    wrapper.unmount()
  })

  it('购物车已占满库存 → 不 emit add、提示「库存不足」、失败音、不弹窗', async () => {
    const product = makeProduct({ barcode: '4901234567894', onsite_qty: 3 })
    const wrapper = mountPanel([product], [{ id: product.id, quantity: 3 }])
    await flushPromises()

    await scan('4901234567894')

    expect(wrapper.emitted('add')).toBeUndefined()
    expect(wrapper.find('.scan-hint').text()).toContain('本子A 库存不足')
    expect(wrapper.findComponent(AppModal).props('show')).toBe(false)
    expect(playScanBeep).toHaveBeenCalledWith(false)
    expect(playScanBeep).not.toHaveBeenCalledWith(true)
    wrapper.unmount()
  })

  it('购物车数量超过库存也按「库存不足」处理', async () => {
    const product = makeProduct({ barcode: '4901234567894', onsite_qty: 2 })
    const wrapper = mountPanel([product], [{ id: product.id, quantity: 5 }])
    await flushPromises()

    await scan('4901234567894')

    expect(wrapper.emitted('add')).toBeUndefined()
    expect(wrapper.find('.scan-hint').text()).toContain('本子A 库存不足')
    expect(wrapper.findComponent(AppModal).props('show')).toBe(false)
    wrapper.unmount()
  })

  it('未找到 → 提示「未找到条码」，且不弹任何对话框', async () => {
    const wrapper = mountPanel([makeProduct({ barcode: '4901234567894' })])
    await flushPromises()

    await scan('0000000000000')

    expect(wrapper.emitted('add')).toBeUndefined()
    expect(wrapper.find('.scan-hint').text()).toContain('未找到条码 0000000000000')
    expect(wrapper.findComponent(AppModal).props('show')).toBe(false)
    wrapper.unmount()
  })

  it('多件命中 → 暂停扫描并弹选择列表，选一件后 emit add 并 resume', async () => {
    const first = makeProduct({ id: 1, name: '版本A', barcode: '4901234567894' })
    const second = makeProduct({ id: 2, name: '版本B', barcode: '4901234567894' })
    const wrapper = mountPanel([first, second])
    await flushPromises()

    await scan('4901234567894')

    expect(scannerMock.pause).toHaveBeenCalled()
    const modal = wrapper.findComponent(AppModal)
    expect(modal.props('show')).toBe(true)
    // NModal 把内容 teleport 到 body，从 body 里取候选按钮。
    const candidateEls = document.body.querySelectorAll<HTMLElement>('.scan-candidate')
    expect(candidateEls).toHaveLength(2)
    expect(wrapper.emitted('add')).toBeUndefined()

    candidateEls[1].click()
    await nextTick()
    await flushPromises()

    expect(wrapper.emitted('add')).toHaveLength(1)
    expect(wrapper.emitted('add')?.[0]?.[0]).toMatchObject({ id: second.id })
    expect(modal.props('show')).toBe(false)
    expect(scannerMock.resume).toHaveBeenCalled()
    wrapper.unmount()
  })

  it('ignored（191/192 价格码）→ 无任何变化', async () => {
    const wrapper = mountPanel([makeProduct({ barcode: '1920123456789' })])
    await flushPromises()

    await scan('1920123456789')

    expect(wrapper.emitted('add')).toBeUndefined()
    expect(wrapper.find('.scan-hint').text()).toBe('将条码对准取景框')
    expect(scannerMock.pause).not.toHaveBeenCalled()
    wrapper.unmount()
  })
})

describe('BarcodeScanPanel 单次模式', () => {
  it('扫到码 → emit code 并停止扫描，不加购', async () => {
    const wrapper = mount(BarcodeScanPanel, { props: { products: [], single: true } })
    await flushPromises()

    await scan('4901234567894')

    expect(wrapper.emitted('code')).toHaveLength(1)
    expect(wrapper.emitted('code')?.[0]?.[0]).toBe('4901234567894')
    expect(wrapper.emitted('add')).toBeUndefined()
    expect(scannerMock.pause).toHaveBeenCalled()
    wrapper.unmount()
  })

  it('ignored 价格码不 emit', async () => {
    const wrapper = mount(BarcodeScanPanel, { props: { products: [], single: true } })
    await flushPromises()

    await scan('1920123456789')

    expect(wrapper.emitted('code')).toBeUndefined()
    expect(scannerMock.pause).not.toHaveBeenCalled()
    wrapper.unmount()
  })
})

describe('BarcodeScanPanel 摄像头不可用', () => {
  it('useCamera.error 非空时显示错误文案与返回按钮，不渲染取景', async () => {
    const wrapper = mountPanel([])
    await flushPromises()

    if (cameraMock.error) cameraMock.error.value = '无法访问摄像头: nope'
    await nextTick()

    expect(wrapper.find('.barcode-scan__error-text').text()).toContain('无法访问摄像头')
    expect(wrapper.find('.scan-viewport').exists()).toBe(false)

    const back = wrapper
      .findAll('button')
      .find((b) => b.text().includes('返回商品列表'))
    await back?.trigger('click')
    expect(wrapper.emitted('close')).toHaveLength(1)
    wrapper.unmount()
  })
})

describe('BarcodeScanPanel 前后摄镜像', () => {
  it('后摄不加镜像；翻转成 user 后视频加水平镜像样式', async () => {
    const wrapper = mountPanel([])
    await flushPromises()

    expect(wrapper.find('.scan-video').classes()).not.toContain('scan-video--mirrored')

    if (cameraMock.facing) cameraMock.facing.value = 'user'
    await nextTick()
    expect(wrapper.find('.scan-video').classes()).toContain('scan-video--mirrored')

    wrapper.unmount()
  })
})
