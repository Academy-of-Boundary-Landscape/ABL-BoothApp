// composables/useWorkbenchEvent.ts —— 管理端展会工作台外壳加载一场展会，子页 inject 共用。
// 外壳 provide 一次，工作台内的子页（商品 / 套装 / 订单 / 统计 / 结算）不再各自按 :id 查。
import {
  inject,
  provide,
  ref,
  toValue,
  watch,
  type InjectionKey,
  type MaybeRefOrGetter,
  type Ref,
} from 'vue'
import { ApiRequestError, api, errorMessage, unwrap, type Schemas } from '@/api/client'

export interface WorkbenchEventContext {
  event: Ref<Schemas['EventResponse'] | null>
  loading: Ref<boolean>
  error: Ref<string | null>
  reload: () => Promise<void>
}

export const WORKBENCH_EVENT: InjectionKey<WorkbenchEventContext> = Symbol('workbench-event')

const NOT_FOUND_TEXT = '展会不存在或已被删除。'

/**
 * 外壳专用：按 id 加载展会并 provide 给子路由。
 * 404 时保留 error（不把 event 当空壳），让 `WorkbenchIndex` 渲染错误态而不是死循环重定向。
 */
export function provideWorkbenchEvent(eventId: MaybeRefOrGetter<number>): WorkbenchEventContext {
  const event = ref<Schemas['EventResponse'] | null>(null)
  const loading = ref(true)
  const error = ref<string | null>(null)

  async function reload(): Promise<void> {
    loading.value = true
    error.value = null
    try {
      event.value = await unwrap(
        api.GET('/events/{id}', { params: { path: { id: toValue(eventId) } } })
      )
    } catch (e) {
      event.value = null
      error.value =
        e instanceof ApiRequestError && e.status === 404
          ? NOT_FOUND_TEXT
          : errorMessage(e, '加载展会失败')
    } finally {
      loading.value = false
    }
  }

  watch(
    () => toValue(eventId),
    () => void reload()
  )
  void reload()

  const ctx: WorkbenchEventContext = { event, loading, error, reload }
  provide(WORKBENCH_EVENT, ctx)
  return ctx
}

/** 子页专用：不在展会工作台内调用直接抛错，避免拿到 `undefined` 后静默失效。 */
export function useWorkbenchEvent(): WorkbenchEventContext {
  const ctx = inject(WORKBENCH_EVENT)
  if (!ctx) throw new Error('useWorkbenchEvent() 只能在 AdminEventWorkbench 内使用')
  return ctx
}
