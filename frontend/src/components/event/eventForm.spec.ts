import { describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { NDatePicker } from 'naive-ui'
import EventForm from '@/components/event/EventForm.vue'
import { useEventStore } from '@/stores/eventStore'
import type { Schemas } from '@/api/client'

function makeEvent(overrides: Partial<Schemas['EventResponse']> = {}): Schemas['EventResponse'] {
  return {
    id: 5,
    name: '测试展会',
    date: '2026-10-01',
    location: '上海',
    status: '筹备',
    qrcode_url: null,
    qrcode_urls: [],
    ...overrides,
  }
}

describe('EventForm', () => {
  it('编辑模式：密码框初始为空，FormData 里 vendor_password 为空串（留空 = 不修改）', () => {
    const pinia = createPinia()
    const wrapper = mount(EventForm, {
      props: { mode: 'edit', event: makeEvent() },
      global: { plugins: [pinia] },
    })

    const passwordInput = wrapper.find('input[placeholder="留空 = 不修改"]')
    expect(passwordInput.exists()).toBe(true)
    expect((passwordInput.element as HTMLInputElement).value).toBe('')
    expect(passwordInput.attributes('type')).toBe('password')
    expect(wrapper.text()).toContain('留空 = 不修改')

    const formData = wrapper.vm.buildFormData()
    expect(formData).not.toBeNull()
    expect(formData!.get('id')).toBe('5')
    expect(formData!.get('name')).toBe('测试展会')
    expect(formData!.get('date')).toBe('2026-10-01')
    // 关键断言：编辑但不改密码时提交空串，后端据此保留原哈希。
    expect(formData!.get('vendor_password')).toBe('')
  })

  it('编辑模式：submit 调 updateEvent，传下去的 FormData vendor_password 是空串，成功后 emit saved', async () => {
    const pinia = createPinia()
    const wrapper = mount(EventForm, {
      props: { mode: 'edit', event: makeEvent() },
      global: { plugins: [pinia] },
    })
    const store = useEventStore(pinia)
    const spy = vi.spyOn(store, 'updateEvent').mockResolvedValue(makeEvent())

    await wrapper.vm.submit()
    await flushPromises()

    expect(spy).toHaveBeenCalledTimes(1)
    const [eventId, formData] = spy.mock.calls[0]
    expect(eventId).toBe(5)
    expect(formData.get('vendor_password')).toBe('')
    expect(wrapper.emitted('saved')).toHaveLength(1)
  })

  it('创建模式：名称 / 日期为空时不提交，页面显示校验文案', async () => {
    const pinia = createPinia()
    const wrapper = mount(EventForm, {
      props: { mode: 'create' },
      global: { plugins: [pinia] },
    })
    const store = useEventStore(pinia)
    const spy = vi.spyOn(store, 'createEvent')

    const result = await wrapper.vm.submit()

    expect(result).toBeNull()
    expect(spy).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('展会名称和日期不能为空')
    expect(wrapper.emitted('saved')).toBeUndefined()
  })

  it('创建模式：填齐名称和日期后 FormData 不含 id，成功后 emit saved', async () => {
    const pinia = createPinia()
    const wrapper = mount(EventForm, {
      props: { mode: 'create' },
      global: { plugins: [pinia] },
    })
    const store = useEventStore(pinia)
    const spy = vi.spyOn(store, 'createEvent').mockResolvedValue(makeEvent({ id: 9 }))

    await wrapper.find('input[placeholder="例如：COMICUP 31"]').setValue('新展会')
    wrapper.findComponent(NDatePicker).vm.$emit('update:formattedValue', '2026-11-01')
    await flushPromises()

    const result = await wrapper.vm.submit()
    await flushPromises()

    expect(result).not.toBeNull()
    expect(spy).toHaveBeenCalledTimes(1)
    const formData = spy.mock.calls[0][0]
    expect(formData.has('id')).toBe(false)
    expect(formData.get('name')).toBe('新展会')
    expect(formData.get('date')).toBe('2026-11-01')
    expect(formData.get('vendor_password')).toBe('')
    expect(wrapper.emitted('saved')).toHaveLength(1)
  })
})
