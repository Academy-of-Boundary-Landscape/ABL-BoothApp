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
  content?: string
  positiveText?: string
  negativeText?: string
  onPositiveClick?: () => void
  onNegativeClick?: () => void
  onClose?: () => void
  onMaskClick?: () => void
}

const mock = vi.hoisted(() => {
  const calls: CapturedCall[] = []
  const dialogCalls: DialogCaptured[] = []
  const messageCalls: Array<{ method: string; args: unknown[] }> = []
  const makeReactive = (o: DialogCaptured) => {
    dialogCalls.push(o)
    return { key: 'k', destroy: () => {} }
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
  }
  return { calls, dialogCalls, messageCalls, dialog, message }
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
  mock.dialog.warning.mockClear()
  mock.dialog.error.mockClear()
  mock.message.success.mockClear()
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
    expect(mock.message.success).toHaveBeenCalledWith('ok')
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

  it('alert 任何关闭都 resolve', async () => {
    const fb = useFeedback()
    const p = fb.alert({ title: '提示', content: '库存不足', type: 'warning' })
    const opts = mock.dialogCalls.at(-1)!
    expect(mock.dialog.warning).toHaveBeenCalledTimes(1)
    opts.onClose?.()
    await expect(p).resolves.toBeUndefined()
  })
})
