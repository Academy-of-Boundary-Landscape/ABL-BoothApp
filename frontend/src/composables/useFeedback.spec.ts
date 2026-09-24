import { describe, it, expect, vi, beforeEach } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { darkTheme } from 'naive-ui'
import { errorText, useFeedback } from './useFeedback'

interface ProviderRef {
  value: { theme: unknown; themeOverrides: unknown }
}
interface CapturedCall {
  includes: string[]
  options: { configProviderProps: ProviderRef }
}
interface DialogCaptured {
  title?: string
  content?: unknown
  positiveText?: string
  negativeText?: string
  onPositiveClick?: () => unknown
  onNegativeClick?: () => void
  onClose?: () => void
  onMaskClick?: () => void
}

const mock = vi.hoisted(() => {
  const calls: CapturedCall[] = []
  const dialogCalls: DialogCaptured[] = []
  const messageCalls: Array<{ method: string; args: unknown[] }> = []
  const instances: Array<{ destroy: ReturnType<typeof vi.fn> }> = []
  const makeReactive = (o: DialogCaptured) => {
    dialogCalls.push(o)
    const instance = { key: 'k', destroy: vi.fn() }
    instances.push(instance)
    return instance
  }
  const dialog = {
    warning: vi.fn(makeReactive),
    error: vi.fn(makeReactive),
    info: vi.fn(makeReactive),
    success: vi.fn(makeReactive),
  }
  const message = {
    success: vi.fn((...a: unknown[]) => {
      messageCalls.push({ method: 'success', args: a })
    }),
    info: vi.fn((...a: unknown[]) => {
      messageCalls.push({ method: 'info', args: a })
    }),
    warning: vi.fn((...a: unknown[]) => {
      messageCalls.push({ method: 'warning', args: a })
    }),
    error: vi.fn((...a: unknown[]) => {
      messageCalls.push({ method: 'error', args: a })
    }),
    loading: vi.fn((...a: unknown[]) => {
      messageCalls.push({ method: 'loading', args: a })
      return { destroy: vi.fn() }
    }),
  }
  return { calls, dialogCalls, messageCalls, instances, dialog, message }
})

vi.mock('naive-ui', async (importOriginal) => {
  const actual = await importOriginal<typeof import('naive-ui')>()
  return {
    ...actual,
    createDiscreteApi: vi.fn((includes: string[], options: CapturedCall['options']) => {
      mock.calls.push({ includes, options })
      return { message: mock.message, dialog: mock.dialog, unmount: vi.fn(), app: {} }
    }),
  }
})

beforeEach(() => {
  localStorage.clear()
  setActivePinia(createPinia())
  // 注意：不清 mock.calls —— createDiscreteApi 只在 api 单例首次建立时调用一次，
  // 清掉就看不到那次捕获的参数了。
  mock.dialogCalls.length = 0
  mock.messageCalls.length = 0
  mock.instances.length = 0
  mock.dialog.warning.mockClear()
  mock.dialog.error.mockClear()
  mock.dialog.info.mockClear()
  mock.message.success.mockClear()
  mock.message.loading.mockClear()
})

describe('errorText', () => {
  const cases: Array<[string, unknown, string | undefined, string]> = [
    ['字符串', '网络断了', undefined, '网络断了'],
    ['Error', new Error('boom'), undefined, 'boom'],
    ['空 message 的 Error', new Error(''), '保存失败', '保存失败'],
    ['未知值', { x: 1 }, undefined, '操作失败'],
    ['未知值 + fallback', 42, '删除失败', '删除失败'],
  ]
  it.each(cases)('errorText: %s', (_n, e, fb, want) => {
    expect(errorText(e, fb)).toBe(want)
  })
})

describe('useFeedback', () => {
  // 注意：api 是模块级单例，configProviderProps 的 computed 会把创建时使用的
  // Pinia store 记为依赖。所以「跟随主题」这条必须最先跑，创建出的单例才绑定
  // 到本测试的 Pinia。
  it('discrete api follows theme', async () => {
    // useFeedback() 是惰性的：先触发一个方法才会真正 createDiscreteApi。
    useFeedback().success('init')
    const call = mock.calls.at(-1)!
    expect(call.includes).toEqual(['message', 'dialog'])
    const providerProps = call.options.configProviderProps
    expect(providerProps.value.theme).toBeNull()

    const { useThemeStore } = await import('@/stores/themeStore')
    useThemeStore().isDark = true
    await nextTick()
    expect(providerProps.value.theme).toBe(darkTheme)
  })

  it('useFeedback works outside component setup', () => {
    const fb = useFeedback()
    expect(() => fb.success('ok')).not.toThrow()
    expect(mock.message.success).toHaveBeenCalledWith('ok', undefined)
  })

  it('confirm 确认 → true，取消 → false', async () => {
    const fb = useFeedback()

    const confirmed = fb.confirm({ title: '删除展会？', danger: true })
    const dangerOpts = mock.dialogCalls.at(-1)!
    expect(mock.dialog.error).toHaveBeenCalledTimes(1)
    expect(dangerOpts.positiveText).toBe('确定')
    expect(dangerOpts.negativeText).toBe('取消')
    dangerOpts.onPositiveClick?.()
    await expect(confirmed).resolves.toBe(true)

    const cancelled = fb.confirm({ title: '普通' })
    const normalOpts = mock.dialogCalls.at(-1)!
    expect(mock.dialog.warning).toHaveBeenCalledTimes(1)
    normalOpts.onNegativeClick?.()
    await expect(cancelled).resolves.toBe(false)
  })

  it('confirm 被关闭（onClose / onMaskClick）→ false', async () => {
    const fb = useFeedback()

    const closed = fb.confirm({ title: 'x' })
    mock.dialogCalls.at(-1)!.onClose?.()
    await expect(closed).resolves.toBe(false)

    const masked = fb.confirm({ title: 'y' })
    mock.dialogCalls.at(-1)!.onMaskClick?.()
    await expect(masked).resolves.toBe(false)
  })

  it('confirm 只 resolve 一次', async () => {
    const fb = useFeedback()
    const p = fb.confirm({ title: 'z' })
    const opts = mock.dialogCalls.at(-1)!
    opts.onPositiveClick?.()
    opts.onNegativeClick?.()
    await expect(p).resolves.toBe(true)
  })

  it('confirm onConfirm 返回 Promise 时 onPositiveClick 返回同一 Promise，完成后 resolve(true)', async () => {
    const fb = useFeedback()
    let release!: (v: unknown) => void
    const inner = new Promise((r) => {
      release = r
    })
    const p = fb.confirm({ title: 'x', onConfirm: () => inner })
    const returned = mock.dialogCalls.at(-1)!.onPositiveClick?.()
    expect(returned).toBe(inner)
    release(undefined)
    await expect(p).resolves.toBe(true)
  })

  it('confirm onConfirm resolve(false) 时不 resolve、弹窗不关，之后取消才 resolve(false)', async () => {
    const fb = useFeedback()
    let settled = false
    const p = fb.confirm({ title: 'x', onConfirm: () => Promise.resolve(false) })
    p.then(() => {
      settled = true
    })
    const opts = mock.dialogCalls.at(-1)!
    const returned = opts.onPositiveClick?.()
    expect(returned).toBeInstanceOf(Promise)
    await returned
    expect(settled).toBe(false)
    opts.onNegativeClick?.()
    await expect(p).resolves.toBe(false)
  })

  it('confirm onConfirm 同步返回 false 时不 resolve', () => {
    const fb = useFeedback()
    const p = fb.confirm({ title: 'x', onConfirm: () => false })
    mock.dialogCalls.at(-1)!.onPositiveClick?.()
    let settled = false
    void p.then(() => {
      settled = true
    })
    return Promise.resolve().then(() => expect(settled).toBe(false))
  })

  it('confirm onConfirm 同步抛错 → reject 且销毁弹窗', async () => {
    const fb = useFeedback()
    const err = new Error('boom')
    const p = fb.confirm({
      title: 'x',
      onConfirm: () => {
        throw err
      },
    })
    const instance = mock.instances.at(-1)!
    expect(() => mock.dialogCalls.at(-1)!.onPositiveClick?.()).not.toThrow()
    await expect(p).rejects.toBe(err)
    expect(instance.destroy).toHaveBeenCalledOnce()
  })

  it('confirm onConfirm 异步 reject → reject 且销毁弹窗', async () => {
    const fb = useFeedback()
    const err = new Error('async boom')
    const p = fb.confirm({ title: 'x', onConfirm: () => Promise.reject(err) })
    const instance = mock.instances.at(-1)!
    mock.dialogCalls.at(-1)!.onPositiveClick?.()
    await expect(p).rejects.toBe(err)
    expect(instance.destroy).toHaveBeenCalledOnce()
  })

  it('loading 返回销毁函数并调用 destroy', () => {
    const fb = useFeedback()
    const stop = fb.loading('正在重置...')
    expect(mock.message.loading).toHaveBeenCalledWith('正在重置...', { duration: 0 })
    const result = mock.message.loading.mock.results.at(-1)!.value as {
      destroy: ReturnType<typeof vi.fn>
    }
    expect(result.destroy).not.toHaveBeenCalled()
    stop()
    expect(result.destroy).toHaveBeenCalledOnce()
  })

  it('alert 任何关闭都 resolve', async () => {
    const fb = useFeedback()
    const p = fb.alert({ title: '提示', content: '库存不足', type: 'warning' })
    const opts = mock.dialogCalls.at(-1)!
    expect(mock.dialog.warning).toHaveBeenCalledTimes(1)
    opts.onClose?.()
    await expect(p).resolves.toBeUndefined()
  })

  it('toast 可选项原样透传（时长 / 可关闭 / 悬停保持）', () => {
    const fb = useFeedback()
    fb.success('导出成功', { duration: 5000, closable: true })
    expect(mock.messageCalls.at(-1)).toEqual({
      method: 'success',
      args: ['导出成功', { duration: 5000, closable: true }],
    })
    fb.info('收到新订单！', { keepAliveOnHover: true })
    expect(mock.messageCalls.at(-1)!.args[1]).toEqual({ keepAliveOnHover: true })
    fb.error(new Error('boom'), '失败', { duration: 6000 })
    expect(mock.messageCalls.at(-1)).toEqual({
      method: 'error',
      args: ['boom', { duration: 6000 }],
    })
  })

  it('confirm 的 type 覆盖 danger 推出的类型；content 可为渲染函数', () => {
    const fb = useFeedback()
    const render = () => 'x'
    void fb.confirm({ title: '确认下单', type: 'info', danger: true, content: render })
    expect(mock.dialog.info).toHaveBeenCalledTimes(1)
    expect(mock.dialog.error).not.toHaveBeenCalled()
    expect(mock.dialogCalls.at(-1)!.content).toBe(render)
  })

  it('alert 的 positiveText 缺省「确认」，可自定义', () => {
    const fb = useFeedback()
    void fb.alert({ content: 'a' })
    expect(mock.dialogCalls.at(-1)!.positiveText).toBe('确认')
    void fb.alert({ content: 'b', type: 'error', positiveText: '知道了' })
    expect(mock.dialogCalls.at(-1)!.positiveText).toBe('知道了')
  })
})
