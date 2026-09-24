// composables/useFeedback.ts —— 反馈（toast / 确认 / 模态提示）的唯一入口。
// 基于 createDiscreteApi：不依赖组件 setup 上下文，store 里也能用；主题跟随 themeStore。
import { computed } from 'vue'
import { createDiscreteApi, darkTheme, type ConfigProviderProps } from 'naive-ui'
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

export interface ConfirmOptions {
  title: string
  content?: string
  positiveText?: string
  negativeText?: string
  danger?: boolean
}

export interface Feedback {
  success(msg: string): void
  info(msg: string): void
  warning(msg: string): void
  error(e: unknown, fallback?: string): void
  confirm(opts: ConfirmOptions): Promise<boolean>
  alert(opts: {
    title?: string
    content: string
    type?: 'info' | 'success' | 'warning' | 'error'
  }): Promise<void>
}

export function useFeedback(): Feedback {
  return {
    success(msg) {
      discrete().message.success(msg)
    },
    info(msg) {
      discrete().message.info(msg)
    },
    warning(msg) {
      discrete().message.warning(msg)
    },
    error(e, fallback) {
      discrete().message.error(errorText(e, fallback))
    },
    confirm(opts) {
      return new Promise<boolean>((resolve) => {
        let settled = false
        const settle = (v: boolean) => {
          if (settled) return
          settled = true
          resolve(v)
        }
        discrete().dialog[opts.danger ? 'error' : 'warning']({
          title: opts.title,
          content: opts.content,
          positiveText: opts.positiveText ?? '确定',
          negativeText: opts.negativeText ?? '取消',
          onPositiveClick: () => settle(true),
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
          positiveText: '确认',
          onPositiveClick: settle,
          onNegativeClick: settle,
          onClose: settle,
          onMaskClick: settle,
        })
      })
    },
  }
}
