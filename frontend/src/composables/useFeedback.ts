// composables/useFeedback.ts —— 反馈（toast / 确认 / 模态提示）的唯一入口。
// 基于 createDiscreteApi：不依赖组件 setup 上下文，store 里也能用；主题跟随 themeStore。
import { computed, type VNodeChild } from 'vue'
import { createDiscreteApi, darkTheme, zhCN, dateZhCN, type ConfigProviderProps } from 'naive-ui'
import { useThemeStore } from '@/stores/themeStore'

type FeedbackApi = ReturnType<typeof createDiscreteApi<'message' | 'dialog'>>

let api: FeedbackApi | null = null

function discrete(): FeedbackApi {
  if (!api) {
    // 在 computed 内部取 store：单例不把某个 Pinia 实例钉死，主题切换（含测试里的
    // 独立 Pinia）都能被响应。createDiscreteApi 接受 Ref 形式的 configProviderProps。
    const configProviderProps = computed<ConfigProviderProps>(() => {
      const theme = useThemeStore()
      return {
        theme: theme.isDark ? darkTheme : null,
        themeOverrides: theme.naiveThemeOverrides,
        locale: zhCN,
        dateLocale: dateZhCN,
      }
    })
    api = createDiscreteApi<'message' | 'dialog'>(['message', 'dialog'], {
      configProviderProps,
    })
  }
  return api
}

/** 从任意异常里取一句可展示的文案（error() 内部用，也导出给测试）。 */
export function errorText(e: unknown, fallback?: string): string {
  if (typeof e === 'string' && e) return e
  if (e instanceof Error && e.message) return e.message
  return fallback ?? '操作失败'
}

/** toast 的可选项，原样透传给 Naive message（只收用得到的几项）。 */
export interface MessageOptions {
  duration?: number
  closable?: boolean
  keepAliveOnHover?: boolean
}

export interface ConfirmOptions {
  title: string
  /** 字符串，或渲染函数（需要换行/富文本时，例如 `() => h('div', { style: 'white-space: pre-line' }, …)`）。 */
  content?: string | (() => VNodeChild)
  positiveText?: string
  negativeText?: string
  danger?: boolean
  /** 对话框类型（图标与确认按钮配色）；缺省按 `danger` 取 error / warning。 */
  type?: 'info' | 'success' | 'warning' | 'error'
  /**
   * 点击确认时执行的操作，接到 Naive dialog 的 `onPositiveClick` 上并原样返回其返回值。
   * 返回 Promise 时确认按钮 loading、弹窗等 Promise 结束才关；返回 / resolve 为 `false`
   * 时弹窗保持打开。抛错（同步 throw 或 reject）时弹窗关闭，`confirm()` reject 该错误。
   */
  onConfirm?: () => unknown | Promise<unknown>
}

export interface Feedback {
  success(msg: string, opts?: MessageOptions): void
  info(msg: string, opts?: MessageOptions): void
  warning(msg: string, opts?: MessageOptions): void
  error(e: unknown, fallback?: string, opts?: MessageOptions): void
  /** 常驻 loading 提示（duration: 0），返回销毁函数。 */
  loading(msg: string): () => void
  confirm(opts: ConfirmOptions): Promise<boolean>
  alert(opts: {
    title?: string
    content: string
    type?: 'info' | 'success' | 'warning' | 'error'
    /** 确认按钮文字，缺省「确认」。 */
    positiveText?: string
  }): Promise<void>
}

export function useFeedback(): Feedback {
  return {
    success(msg, opts) {
      discrete().message.success(msg, opts)
    },
    info(msg, opts) {
      discrete().message.info(msg, opts)
    },
    warning(msg, opts) {
      discrete().message.warning(msg, opts)
    },
    error(e, fallback, opts) {
      discrete().message.error(errorText(e, fallback), opts)
    },
    loading(msg) {
      const instance = discrete().message.loading(msg, { duration: 0 })
      return () => instance.destroy()
    },
    confirm(opts) {
      return new Promise<boolean>((resolve, reject) => {
        let settled = false
        const settle = (v: boolean) => {
          if (settled) return
          settled = true
          resolve(v)
        }
        const fail = (e: unknown) => {
          if (settled) return
          settled = true
          reject(e)
        }
        let instance: { destroy: () => void } | null = null
        const hide = () => instance?.destroy()

        const handlePositiveClick = (): unknown => {
          if (!opts.onConfirm) {
            settle(true)
            return
          }
          let result: unknown
          try {
            result = opts.onConfirm()
          } catch (e) {
            fail(e)
            hide()
            return
          }
          if (result instanceof Promise) {
            result.then(
              (v) => {
                if (v !== false) settle(true)
              },
              (e: unknown) => {
                fail(e)
                hide()
              }
            )
            // 原样返回同一个 Promise，交回 Naive 处理「确认按钮 loading / 等它结束才关」。
            return result
          }
          if (result !== false) settle(true)
          return result
        }

        instance = discrete().dialog[opts.type ?? (opts.danger ? 'error' : 'warning')]({
          title: opts.title,
          content: opts.content,
          positiveText: opts.positiveText ?? '确定',
          negativeText: opts.negativeText ?? '取消',
          onPositiveClick: handlePositiveClick,
          onNegativeClick: () => settle(false),
          onClose: () => settle(false),
          onMaskClick: () => settle(false),
        })
      })
    },
    alert(opts) {
      return new Promise<void>((resolve) => {
        let settled = false
        const settle = () => {
          if (settled) return
          settled = true
          resolve()
        }
        discrete().dialog[opts.type ?? 'info']({
          title: opts.title,
          content: opts.content,
          positiveText: opts.positiveText ?? '确认',
          onPositiveClick: settle,
          onNegativeClick: settle,
          onClose: settle,
          onMaskClick: settle,
        })
      })
    },
  }
}
