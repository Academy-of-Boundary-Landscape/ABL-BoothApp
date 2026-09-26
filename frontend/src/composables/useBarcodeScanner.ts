/**
 * 连续扫码（spec §5.2 / §6.2）：懒加载引擎，按固定节奏把取景框区域画到离屏 canvas 后解码，
 * 只对外发「扫到码 X」。防重复、暂停与卸载都在这里收口，面板只管表现。
 */
import { ref, shallowReadonly, onScopeDispose } from 'vue'
import type { Ref } from 'vue'
import { loadDetector, type Detector } from '@/utils/barcodeEngine'

const DEFAULT_INTERVAL_MS = 200
const DEFAULT_COOLDOWN_MS = 1500
const DEFAULT_GONE_FRAMES = 3
/** 送进解码器的画面长边上限（像素），控制单帧耗时。 */
const MAX_DIMENSION = 640

export interface ScanRoi {
  x: number
  y: number
  w: number
  h: number
}

export interface BarcodeScannerOptions {
  video: Ref<HTMLVideoElement | null>
  /** 取景框在视频像素坐标里的位置；由面板按 object-fit: cover 换算。 */
  roi: () => ScanRoi
  onCode: (code: string) => void
  intervalMs?: number
  cooldownMs?: number
  /** 连续这么多帧没看到该码，视为已离开画面。 */
  goneFrames?: number
}

export interface BarcodeScanner {
  running: Readonly<Ref<boolean>>
  loading: Readonly<Ref<boolean>>
  error: Ref<string>
  start(): Promise<void>
  pause(): void
  resume(): void
  stop(): void
}

/**
 * 去重状态必须**按码**记录，而不是只记「最后一个码」：日本书的 ISBN（978…）和
 * 价格码（192…）经常同帧进入取景框，只记最后一个会让两个码交替刷新，每个码
 * 每帧都被当成「新码」重新上报。
 */
interface CodeState {
  lastReportedAt: number
  /** 连续多少帧没再看到这个码。 */
  goneCount: number
}

export function useBarcodeScanner(opts: BarcodeScannerOptions): BarcodeScanner {
  const intervalMs = opts.intervalMs ?? DEFAULT_INTERVAL_MS
  const cooldownMs = opts.cooldownMs ?? DEFAULT_COOLDOWN_MS
  const goneFrames = opts.goneFrames ?? DEFAULT_GONE_FRAMES

  const running = ref(false)
  const loading = ref(false)
  const error = ref('')

  let detector: Detector | null = null
  let timer: ReturnType<typeof setInterval> | null = null
  let canvas: HTMLCanvasElement | null = null
  /** 上一次 detect 还没返回时跳过本拍，避免请求堆积。 */
  let busy = false
  let paused = false
  /** start / stop 的世代号：取引擎期间被 stop，resolve 后不得复活。 */
  let generation = 0

  /**
   * 每个码各自维护「上次上报时间 + 连续未见帧数」。
   */
  const codeStates = new Map<string, CodeState>()

  function stopTimer() {
    if (timer !== null) {
      clearInterval(timer)
      timer = null
    }
  }

  function startTimer() {
    if (timer !== null || paused) return
    timer = setInterval(() => {
      void tick()
    }, intervalMs)
  }

  function process(rawValues: readonly string[]) {
    if (paused || !running.value) return
    const now = Date.now()
    const seen = new Set(rawValues)

    // 本帧没看到的已知码：累计「连续未见」帧数。
    for (const [code, state] of codeStates) {
      if (!seen.has(code)) state.goneCount++
    }

    // 本帧看到的码按 detect 返回顺序逐个判断。
    for (const code of rawValues) {
      // 上一轮 onCode 里可能触发 pause（多件命中）：之后不再处理同一帧剩余的码。
      if (paused || !running.value) return

      const state = codeStates.get(code)
      if (!state) {
        // 首次出现 → 立即上报。
        codeStates.set(code, { lastReportedAt: now, goneCount: 0 })
        opts.onCode(code)
        continue
      }
      if (state.goneCount >= goneFrames && now - state.lastReportedAt >= cooldownMs) {
        // 确认离开过，且距上次上报够久 → 视为重新出现，再报一次。
        state.lastReportedAt = now
        state.goneCount = 0
        opts.onCode(code)
      } else {
        // 本帧仍在画面：清零离开计数，避免同一段离开被反复消费。
        state.goneCount = 0
      }
    }
  }

  function draw(video: HTMLVideoElement): HTMLCanvasElement | null {
    const roi = opts.roi()
    if (!roi || roi.w <= 0 || roi.h <= 0) return null
    const longest = Math.max(roi.w, roi.h)
    const scale = longest > MAX_DIMENSION ? MAX_DIMENSION / longest : 1
    const w = Math.max(1, Math.round(roi.w * scale))
    const h = Math.max(1, Math.round(roi.h * scale))
    if (!canvas) canvas = document.createElement('canvas')
    canvas.width = w
    canvas.height = h
    // 每帧 drawImage 后立刻 detect 会读取像素：声明 willReadFrequently 避免
    // 浏览器反复回读 GPU 画面时的 console 警告，并让实现走 CPU 后备路径。
    const ctx = canvas.getContext('2d', { willReadFrequently: true })
    // jsdom 没有 canvas 实现时 getContext 返回 null：跳过绘制但仍把画面交给 detect。
    ctx?.drawImage(video, roi.x, roi.y, roi.w, roi.h, 0, 0, w, h)
    return canvas
  }

  async function tick() {
    if (!running.value || paused || busy) return
    const video = opts.video.value
    if (!video || !video.videoWidth || !video.videoHeight) return
    if (!detector) return
    const source = draw(video)
    if (!source) return
    busy = true
    try {
      const results = await detector.detect(source)
      process(results.map((result) => result.rawValue))
    } catch {
      // 单帧解码失败不打断连续扫描，也不弹窗。
    } finally {
      busy = false
    }
  }

  async function start(): Promise<void> {
    if (running.value || loading.value) return
    const gen = ++generation
    loading.value = true
    error.value = ''
    try {
      const loaded = await loadDetector()
      if (gen !== generation) return
      detector = loaded
      loading.value = false
      running.value = true
      startTimer()
    } catch (e) {
      if (gen !== generation) return
      loading.value = false
      running.value = false
      error.value = (e instanceof Error && e.message) || '扫码引擎加载失败'
    }
  }

  function pause(): void {
    paused = true
    stopTimer()
  }

  function resume(): void {
    if (!running.value) return
    paused = false
    // 恢复后不重新上报仍在画面里的已知码：把它们的计时推到当前，必须真正离开
    // 画面（≥ goneFrames）再回来（≥ cooldownMs）才算一次新的扫描。否则多件选择
    // 弹窗关闭后商品还在镜头里，会立刻再次命中、再次弹窗。
    const now = Date.now()
    for (const state of codeStates.values()) {
      state.lastReportedAt = now
      state.goneCount = 0
    }
    startTimer()
  }

  function stop(): void {
    generation++
    paused = false
    stopTimer()
    running.value = false
    loading.value = false
    codeStates.clear()
  }

  onScopeDispose(stop)

  return {
    running: shallowReadonly(running),
    loading: shallowReadonly(loading),
    error,
    start,
    pause,
    resume,
    stop,
  }
}
