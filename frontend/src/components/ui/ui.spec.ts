import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia } from 'pinia'
import { NButton, NCard, NModal, NSpin } from 'naive-ui'
import AsyncState from './AsyncState.vue'
import EmptyState from './EmptyState.vue'
import AppModal from './AppModal.vue'
import Money from './Money.vue'
import PageShell from './PageShell.vue'
import SectionCard from './SectionCard.vue'
import StatTile from './StatTile.vue'
import HelpBubble from '@/components/shared/HelpBubble.vue'
import { cents } from '@/utils/money'

const mountOpts = { global: { plugins: [createPinia()] } }
const pageShellOpts = {
  global: {
    plugins: [createPinia()],
    components: { RouterLink: { template: '<a><slot /></a>' } },
  },
}

// Naive 的 clickoutside（evtd）在 mouseup 上判定，所以遮罩点击要发 mousedown + mouseup。
function clickMask(el: Element) {
  el.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }))
  el.dispatchEvent(new MouseEvent('mouseup', { bubbles: true }))
  el.dispatchEvent(new MouseEvent('click', { bubbles: true }))
}

describe('AsyncState', () => {
  const slots = { default: '<p class="ok">ok</p>', empty: '<p class="e">empty</p>' }

  it('loading 优先', () => {
    const w = mount(AsyncState, {
      props: { loading: true, error: 'x', empty: true },
      slots,
      ...mountOpts,
    })
    expect(w.find('.ok').exists()).toBe(false)
    expect(w.find('.e').exists()).toBe(false)
    expect(w.findComponent(NSpin).exists()).toBe(true)
  })

  it('error wins over empty', () => {
    const w = mount(AsyncState, {
      props: { error: '加载失败', empty: true },
      slots,
      ...mountOpts,
    })
    expect(w.text()).toContain('加载失败')
    expect(w.find('.e').exists()).toBe(false)
  })

  it('empty 渲染 empty 插槽', () => {
    const w = mount(AsyncState, { props: { empty: true }, slots, ...mountOpts })
    expect(w.find('.e').exists()).toBe(true)
  })

  it('正常渲染 default', () => {
    const w = mount(AsyncState, { slots, ...mountOpts })
    expect(w.find('.ok').exists()).toBe(true)
  })

  it('没有 retry 监听时不显示重试', () => {
    const w = mount(AsyncState, { props: { error: 'x' }, ...mountOpts })
    expect(w.text()).not.toContain('重试')
  })

  it('有 retry 监听时显示并触发', async () => {
    const onRetry = vi.fn()
    const w = mount(AsyncState, { props: { error: 'x', onRetry }, ...mountOpts })
    await w.find('button').trigger('click')
    expect(onRetry).toHaveBeenCalledOnce()
  })

  it('缺省 empty 插槽渲染紧凑 EmptyState', () => {
    const w = mount(AsyncState, { props: { empty: true }, ...mountOpts })
    expect(w.text()).toContain('暂无数据')
  })

  it('overlay + loading 时 default 内容仍渲染，NSpin show=true', () => {
    const w = mount(AsyncState, {
      props: { loading: true, overlay: true },
      slots,
      ...mountOpts,
    })
    expect(w.find('.ok').exists()).toBe(true)
    expect(w.find('.e').exists()).toBe(false)
    expect(w.findComponent(NSpin).props('show')).toBe(true)
  })

  it('overlay=false 时 loading 仍替换内容（原行为不变）', () => {
    const w = mount(AsyncState, {
      props: { loading: true },
      slots,
      ...mountOpts,
    })
    expect(w.find('.ok').exists()).toBe(false)
    expect(w.find('.async-state__loading').exists()).toBe(true)
  })

  it('overlay 下 error 仍优先于空态，且包在 NSpin 里', () => {
    const w = mount(AsyncState, {
      props: { loading: true, overlay: true, error: '加载失败', empty: true },
      slots,
      ...mountOpts,
    })
    expect(w.text()).toContain('加载失败')
    expect(w.find('.e').exists()).toBe(false)
    expect(w.findComponent(NSpin).props('show')).toBe(true)
  })
})

describe('AppModal', () => {
  const base = {
    props: { show: true },
    global: { plugins: [createPinia()] },
    attachTo: document.body,
  }

  it('关闭按钮 emit update:show false', async () => {
    const w = mount(AppModal, base)
    await w.findComponent(NButton).trigger('click')
    expect(w.emitted('update:show')?.[0]).toEqual([false])
    w.unmount()
  })

  it('maskClosable=false 时点遮罩不关', async () => {
    const w = mount(AppModal, { ...base, props: { show: true, maskClosable: false } })
    await nextTick()
    const mask = document.querySelector('.n-modal-mask')
    expect(mask).not.toBeNull()
    clickMask(mask!)
    await nextTick()
    expect(w.emitted('update:show')).toBeUndefined()
    w.unmount()
  })

  it('maskClosable 默认点遮罩关闭', async () => {
    const w = mount(AppModal, base)
    await nextTick()
    const mask = document.querySelector('.n-modal-mask')
    expect(mask).not.toBeNull()
    clickMask(mask!)
    await nextTick()
    expect(w.emitted('update:show')?.[0]).toEqual([false])
    w.unmount()
  })

  it('size 映射宽度 480/640/960', () => {
    const cases = [
      ['sm', '480px'],
      ['md', '640px'],
      ['lg', '960px'],
    ] as const
    for (const [size, px] of cases) {
      const w = mount(AppModal, { ...base, props: { show: true, size } })
      expect(w.findComponent(NCard).attributes('style')).toContain(px)
      w.unmount()
    }
  })

  it('closable=false 不渲染关闭按钮', () => {
    const w = mount(AppModal, { ...base, props: { show: true, closable: false } })
    expect(w.findComponent(NCard).find('.app-modal__close').exists()).toBe(false)
    w.unmount()
  })

  it('closable 默认渲染关闭按钮', () => {
    const w = mount(AppModal, base)
    expect(w.findComponent(NCard).find('.app-modal__close').exists()).toBe(true)
    w.unmount()
  })

  it('closeOnEsc 透传到 NModal', () => {
    const on = mount(AppModal, { ...base, props: { show: true } })
    expect(on.findComponent(NModal).props('closeOnEsc')).toBe(true)
    on.unmount()

    const off = mount(AppModal, { ...base, props: { show: true, closeOnEsc: false } })
    expect(off.findComponent(NModal).props('closeOnEsc')).toBe(false)
    off.unmount()
  })

  it('转发 after-enter / after-leave', async () => {
    const w = mount(AppModal, base)
    await w.findComponent(NModal).vm.$emit('after-enter')
    await w.findComponent(NModal).vm.$emit('after-leave')
    expect(w.emitted('after-enter')).toHaveLength(1)
    expect(w.emitted('after-leave')).toHaveLength(1)
    w.unmount()
  })

  it('#header 插槽由新容器接管且不受标题样式包裹', () => {
    const w = mount(AppModal, {
      ...base,
      props: { show: true, title: '默认标题' },
      slots: { header: '<span class="custom-head">自定义头</span>' },
    })
    const card = w.findComponent(NCard)
    const slot = card.find('.app-modal__header-slot')
    expect(slot.exists()).toBe(true)
    expect(slot.find('.custom-head').exists()).toBe(true)
    expect(card.find('.app-modal__title').exists()).toBe(false)
    w.unmount()
  })

  it('无 #header 时照旧渲染 title', () => {
    const w = mount(AppModal, { ...base, props: { show: true, title: '默认标题' } })
    expect(w.findComponent(NCard).find('.app-modal__title').text()).toBe('默认标题')
    w.unmount()
  })
})

describe('Money', () => {
  it.each([
    [0, '¥0.00'],
    [-150, '¥-1.50'],
    [12345, '¥123.45'],
  ])('value %i → %s', (v, want) => {
    const w = mount(Money, { props: { value: cents(v) }, ...mountOpts })
    expect(w.text()).toBe(want)
  })

  it('signed 给正数加 +', () => {
    const w = mount(Money, { props: { value: cents(150), signed: true }, ...mountOpts })
    expect(w.text()).toBe('+¥1.50')
  })

  it('signed 不给负数加 +', () => {
    const w = mount(Money, { props: { value: cents(-150), signed: true }, ...mountOpts })
    expect(w.text()).toBe('¥-1.50')
  })

  it('strike 加删除线类', () => {
    const w = mount(Money, { props: { value: cents(150), strike: true }, ...mountOpts })
    expect(w.find('.money').classes()).toContain('strike')
  })

  it('null 显示 --', () => {
    const w = mount(Money, { props: { value: null }, ...mountOpts })
    expect(w.text()).toBe('--')
  })
})

describe('PageShell', () => {
  it('help 传入时渲染 HelpBubble', () => {
    const w = mount(PageShell, { props: { title: 'T', help: 'events' }, ...pageShellOpts })
    expect(w.findComponent(HelpBubble).exists()).toBe(true)
  })

  it('help 气泡与 h1 同一行（title-row）', () => {
    const w = mount(PageShell, {
      props: { title: 'T', subtitle: 'S', help: 'events' },
      slots: { actions: '<button class="act">操作</button>' },
      ...pageShellOpts,
    })
    const row = w.find('.page-shell__title-row')
    expect(row.find('h1').exists()).toBe(true)
    expect(row.findComponent(HelpBubble).exists()).toBe(true)
    // 副标题不在标题行内，actions 也不在
    expect(row.find('.page-shell__subtitle').exists()).toBe(false)
    expect(row.find('.act').exists()).toBe(false)
    expect(w.find('.page-shell__actions .act').exists()).toBe(true)
  })

  it('width=wide 用 --page-wide', () => {
    const w = mount(PageShell, { props: { title: 'T', width: 'wide' }, ...pageShellOpts })
    expect(w.find('.page-shell').attributes('style')).toContain('--page-wide')
  })

  it('width 默认 content', () => {
    const w = mount(PageShell, { props: { title: 'T' }, ...pageShellOpts })
    expect(w.find('.page-shell').attributes('style')).toContain('--page-content')
  })

  it('title 渲染为 h1', () => {
    const w = mount(PageShell, { props: { title: '展会管理' }, ...pageShellOpts })
    expect(w.find('h1').text()).toBe('展会管理')
  })

  it('subtitle 插槽优先于 subtitle prop（可放富文本）', () => {
    const w = mount(PageShell, {
      props: { title: 'T', subtitle: '纯文本' },
      slots: { subtitle: '说明，<strong class="em">重点</strong>' },
      ...pageShellOpts,
    })
    const sub = w.find('.page-shell__subtitle')
    expect(sub.find('.em').text()).toBe('重点')
    expect(w.text()).not.toContain('纯文本')
  })

  it('embedded 不渲染 h1 / 副标题 / actions，只留内容', () => {
    const w = mount(PageShell, {
      props: { embedded: true, subtitle: '不该出现' },
      slots: { actions: '<button class="act">操作</button>', default: '<p class="body">内容</p>' },
      ...pageShellOpts,
    })
    expect(w.find('h1').exists()).toBe(false)
    expect(w.find('.page-shell__header').exists()).toBe(false)
    expect(w.find('.page-shell__subtitle').exists()).toBe(false)
    expect(w.find('.act').exists()).toBe(false)
    expect(w.find('.body').text()).toBe('内容')
  })
})

describe('SectionCard', () => {
  it('collapsible 点标题切换 collapsed 并 emit', async () => {
    const onUpdate = vi.fn()
    const w = mount(SectionCard, {
      props: { title: '区块', collapsible: true, collapsed: false, 'onUpdate:collapsed': onUpdate },
      slots: { default: '<p class="body">内容</p>' },
      ...mountOpts,
    })
    expect(w.find('.body').isVisible()).toBe(true)
    expect(w.find('.section-card__header').attributes('aria-expanded')).toBe('true')
    await w.find('.section-card__header').trigger('click')
    expect(onUpdate).toHaveBeenCalledWith(true)
    expect(w.find('.section-card__header').attributes('aria-expanded')).toBe('false')
  })

  it('非 collapsible 点标题不 emit', async () => {
    const onUpdate = vi.fn()
    const w = mount(SectionCard, {
      props: { title: '区块', 'onUpdate:collapsed': onUpdate },
      ...mountOpts,
    })
    await w.find('.section-card__header').trigger('click')
    expect(onUpdate).not.toHaveBeenCalled()
  })
})

describe('StatTile', () => {
  it('value 插槽优先于 value prop', () => {
    const w = mount(StatTile, {
      props: { label: '合计', value: '¥10' },
      slots: { value: '<b class="slot-val">¥20</b>' },
      ...mountOpts,
    })
    expect(w.find('.slot-val').text()).toBe('¥20')
    expect(w.text()).not.toContain('¥10')
  })
})

describe('EmptyState', () => {
  it('compact 不渲染图标', () => {
    const w = mount(EmptyState, { props: { title: '暂无数据', compact: true }, ...mountOpts })
    expect(w.find('.empty-state__icon').exists()).toBe(false)
    expect(w.text()).toContain('暂无数据')
  })

  it('默认渲染图标与 action 插槽', () => {
    const w = mount(EmptyState, {
      props: { title: '空' },
      slots: { action: '<button class="do">创建</button>' },
      ...mountOpts,
    })
    expect(w.find('.empty-state__icon').exists()).toBe(true)
    expect(w.find('.do').exists()).toBe(true)
  })

  it('icon 传空串不渲染图标占位', () => {
    const w = mount(EmptyState, { props: { title: '暂无已完成订单', icon: '' }, ...mountOpts })
    expect(w.find('.empty-state__icon').exists()).toBe(false)
  })
})
