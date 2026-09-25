// composables/useVendorPolling.ts —— 摊主外壳的轮询 + 新单提示音。
//
// 从旧的 `views/VendorView.vue` 原样搬出：3 秒间隔、待处理列表的去重规则
// （orderStore 里按 JSON 比较后再赋值）、以及「第一次加载不算新单、之后数量变大才响」
// 这段 watch，都不改行为。区别只在于现在外壳挂载一次就一直活着——切到库存 / 收摊
// tab 时轮询与提示音不中断（spec §3.6）。
//
// 提示音元素由外壳提供（`<audio src="/notify.mp3">`），这里只拿到它的 ref。
import {
  computed,
  inject,
  onMounted,
  onUnmounted,
  provide,
  ref,
  watch,
  type InjectionKey,
  type Ref,
} from 'vue'
import { useOrderStore } from '@/stores/orderStore'
import { useEventStore } from '@/stores/eventStore'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import { useFeedback } from '@/composables/useFeedback'

export interface VendorPolling {
  /** 待处理订单数，供 tab 角标用。 */
  pendingCount: Ref<number>
  /** 页内「刷新」按钮：拉待处理 / 已完成订单与现场商品，并借机解锁音频播放权限。 */
  refresh: () => Promise<void>
}

/** 外壳 provide，子页 inject；避免把 refresh 一层层当 prop 传。 */
export const VENDOR_POLLING: InjectionKey<VendorPolling> = Symbol('vendor-polling')

/**
 * 只在外壳（`VendorShell`）里调用一次。onMounted 启动、onUnmounted 停止。
 */
export function useVendorPolling(
  eventId: Ref<string>,
  audio: Ref<HTMLAudioElement | null>
): VendorPolling {
  const orderStore = useOrderStore()
  const eventStore = useEventStore()
  const eventDetailStore = useEventDetailStore()
  const fb = useFeedback()

  // 首次加载只是把已存在的待处理单拉进来，不是「新单」，不能响铃。
  const isInitialized = ref(false)

  // 外壳 setup 是同步的、且早于 `<router-view>` 里所有子组件挂载；子组件
  // （LiveStats）的 onMounted 早于外壳的 onMounted。若等 onMounted 里
  // setActiveEvent 才设 activeEventId，子组件首次 fetchCompletedOrders 会因
  // activeEventId 为空直接 return，营业额前几秒显示 ¥0。这里先同步钉上展会 id，
  // 下面 onMounted 仍走 setActiveEvent 启动轮询与提示音初始化。
  orderStore.activeEventId = Number(eventId.value)

  const pendingCount = computed(() => orderStore.pendingOrders.length)

  // 播放声音。现代浏览器要求用户必须先与页面交互过，失败就静默记一条日志。
  function playNoticeSound() {
    const el = audio.value
    if (!el) return
    el.currentTime = 0
    el.play().catch((err) => {
      console.warn('音频播放尝试失败（用户尚未与页面交互或文件路径不正确）:', err)
    })
  }

  // 核心逻辑：只负责「已初始化且数量变大 → 响」。
  // 初始化标志由外壳启动后的首次 pollPendingOrders() 完成后置位（见 onMounted），
  // 不能放在 watch 里：首次拉回 0 条时数量 0 → 0 不触发 watch，第一张真正的新单会被误当初始化吞掉。
  watch(
    () => orderStore.pendingOrders.length,
    (newCount, oldCount) => {
      if (isInitialized.value && newCount > oldCount) {
        playNoticeSound()
        fb.info('收到新订单！', { keepAliveOnHover: true })
      }
    }
  )

  async function refresh(): Promise<void> {
    // 顺便在这里尝试播放一下声音，让浏览器「解锁」音频播放权限（旧 manualRefresh 行为）。
    playNoticeSound()
    try {
      await Promise.all([
        orderStore.pollPendingOrders(),
        eventDetailStore.fetchProductsForEvent(Number(eventId.value)),
        orderStore.fetchCompletedOrders(),
      ])
    } catch (err) {
      console.error('手动刷新失败:', err)
    }
  }

  onMounted(() => {
    if (eventStore.events.length === 0) {
      eventStore.fetchEvents()
    }
    // 首次轮询完成（成功或失败都算）后置 isInitialized，之后数量变大才响。
    void orderStore.setActiveEvent(eventId.value).finally(() => {
      isInitialized.value = true
    })
    eventDetailStore.fetchProductsForEvent(Number(eventId.value))
  })

  onUnmounted(() => {
    orderStore.stopPolling()
  })

  return { pendingCount, refresh }
}

/** 外壳里 provide 这个 key，子页用它拿 refresh。 */
export function provideVendorPolling(polling: VendorPolling): VendorPolling {
  provide(VENDOR_POLLING, polling)
  return polling
}

/** 子页专用：不在外壳里调用直接抛错，避免拿到 undefined 后静默失效。 */
export function useVendorPollingContext(): VendorPolling {
  const ctx = inject(VENDOR_POLLING)
  if (!ctx) throw new Error('useVendorPollingContext() 只能在 VendorShell 内使用')
  return ctx
}
