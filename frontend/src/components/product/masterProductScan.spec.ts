import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import CreateMasterProductForm from './CreateMasterProductForm.vue'
import EditMasterProductModal from './EditMasterProductModal.vue'
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

vi.mock('@/services/vision', () => ({
  listProductImages: vi.fn().mockResolvedValue([]),
  addProductImage: vi.fn(),
  deleteProductImage: vi.fn(),
  listModels: vi.fn().mockResolvedValue({ models: [] }),
  searchByImage: vi.fn(),
  getVisionStatus: vi.fn(),
}))

/** 单次模式扫码面板替身：点一下就 emit 一个码。 */
const ScanStub = {
  name: 'BarcodeScanPanel',
  props: {
    products: { type: Array, default: () => [] },
    single: { type: Boolean, default: false },
  },
  emits: ['code', 'close'],
  template: `<button class="stub-scan-code" @click="$emit('code', '4901234567894')">stub</button>`,
}

const stubs = {
  BarcodeScanPanel: ScanStub,
  ImageUploader: true,
  SocietySelect: true,
  ImageCropper: true,
}

function makeMasterProduct(): Schemas['MasterProduct'] {
  return {
    id: 7,
    product_code: 'P001',
    barcode: null,
    name: '本子A',
    default_price: 30,
    category: '本子',
    tags: '',
    is_active: true,
    image_url: null,
  }
}

function bodyButton(text: string): HTMLButtonElement {
  const btn = [...document.body.querySelectorAll('button')].find(
    (b) => b.textContent?.trim() === text
  )
  if (!btn) throw new Error(`找不到按钮：${text}`)
  return btn as HTMLButtonElement
}

afterEach(() => {
  document.body.innerHTML = ''
  vi.restoreAllMocks()
})

describe('商品表单「扫码填入」', () => {
  it('编辑表单：扫码后回填条码输入框', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const wrapper = mount(EditMasterProductModal, {
      props: { show: false, product: makeMasterProduct() },
      global: { plugins: [pinia], stubs },
    })
    await wrapper.setProps({ show: true })
    await flushPromises()

    bodyButton('扫码填入').click()
    await nextTick()
    await flushPromises()

    const stub = document.body.querySelector<HTMLButtonElement>('.stub-scan-code')
    expect(stub).not.toBeNull()
    stub!.click()
    await nextTick()
    await flushPromises()

    const input = document.body.querySelector<HTMLInputElement>('.barcode-field input')
    expect(input?.value).toBe('4901234567894')
    wrapper.unmount()
  })

  it('新建表单：扫码后回填条码输入框', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const wrapper = mount(CreateMasterProductForm, {
      global: { plugins: [pinia], stubs },
    })
    await flushPromises()

    const scanButton = wrapper.findAll('button').find((b) => b.text().trim() === '扫码填入')
    expect(scanButton).toBeDefined()
    await scanButton!.trigger('click')
    await nextTick()

    const stub = document.body.querySelector<HTMLButtonElement>('.stub-scan-code')
    expect(stub).not.toBeNull()
    stub!.click()
    await nextTick()

    const input = wrapper.find<HTMLInputElement>('.barcode-field input')
    expect(input.element.value).toBe('4901234567894')
    wrapper.unmount()
  })
})
