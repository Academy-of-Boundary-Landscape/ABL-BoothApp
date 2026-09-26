import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { effectScope, ref } from 'vue'
import type { EffectScope, Ref } from 'vue'
import { useBarcodeScanner, type ScanRoi } from './useBarcodeScanner'

const mocks = vi.hoisted(() => ({
  loadDetector: vi.fn(),
}))

vi.mock('@/utils/barcodeEngine', () => ({
  loadDetector: mocks.loadDetector,
  SCAN_FORMATS: ['ean_13', 'qr_code'],
}))

const ROI: ScanRoi = { x: 0, y: 0, w: 320, h: 128 }

/** 假 video：只要 videoWidth / videoHeight 够 scanner 检查即可（画布绘制在 jsdom 里被跳过）。 */
function makeVideo(): HTMLVideoElement {
  return { videoWidth: 1280, videoHeight: 720 } as unknown as HTMLVideoElement
}

interface DetectResult {
  rawValue: string
}

function makeDetector() {
  const detect = vi.fn<(source: unknown) => Promise<DetectResult[]>>()
  detect.mockResolvedValue([])
  return { detect }
}

let scopes: EffectScope[] = []

function makeScanner(opts?: { intervalMs?: number; cooldownMs?: number; goneFrames?: number }) {
  const scope = effectScope()
  scopes.push(scope)
  const onCode = vi.fn<(code: string) => void>()
  const video: Ref<HTMLVideoElement | null> = ref(makeVideo())
  const scanner = scope.run(() =>
    useBarcodeScanner({ video, roi: () => ROI, onCode, ...opts })
  )!
  return { scanner, onCode, video, scope }
}

/** 推进 intervalMs 一帧并 flush 掉 detect 的微任务。 */
async function tick(intervalMs = 200) {
  await vi.advanceTimersByTimeAsync(intervalMs)
}

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  scopes.forEach((s) => s.stop())
  scopes = []
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('useBarcodeScanner', () => {
  it('同码连续 10 帧只上报 1 次', async () => {
    const detector = makeDetector()
    detector.detect.mockResolvedValue([{ rawValue: '4901234567894' }])
    mocks.loadDetector.mockResolvedValue(detector)

    const { scanner, onCode } = makeScanner({ intervalMs: 100 })
    await scanner.start()
    for (let i = 0; i < 10; i++) await tick(100)

    expect(onCode).toHaveBeenCalledTimes(1)
    expect(onCode).toHaveBeenCalledWith('4901234567894')
    scanner.stop()
  })

  it('离开 ≥3 帧且距上次上报 ≥1500ms 后再出现 → 再上报', async () => {
    const detector = makeDetector()
    mocks.loadDetector.mockResolvedValue(detector)

    const { scanner, onCode } = makeScanner({ intervalMs: 100, cooldownMs: 1500 })
    await scanner.start()

    detector.detect.mockResolvedValue([{ rawValue: 'A' }])
    await tick(100) // t=100 上报 A
    expect(onCode).toHaveBeenCalledTimes(1)

    // 连续 16 帧没看到 A（≥ goneFrames），时间也走过 cooldown（t 到 1700）。
    detector.detect.mockResolvedValue([])
    for (let i = 0; i < 16; i++) await tick(100)
    expect(onCode).toHaveBeenCalledTimes(1)

    detector.detect.mockResolvedValue([{ rawValue: 'A' }])
    await tick(100) // t=1800 再次出现
    expect(onCode).toHaveBeenCalledTimes(2)
    expect(onCode).toHaveBeenLastCalledWith('A')
    scanner.stop()
  })

  it('离开 ≥3 帧但距上次上报 <1500ms 就再出现 → 不上报', async () => {
    const detector = makeDetector()
    mocks.loadDetector.mockResolvedValue(detector)

    const { scanner, onCode } = makeScanner({ intervalMs: 100, cooldownMs: 1500 })
    await scanner.start()

    detector.detect.mockResolvedValue([{ rawValue: 'A' }])
    await tick(100) // t=100 上报 A
    expect(onCode).toHaveBeenCalledTimes(1)

    detector.detect.mockResolvedValue([])
    await tick(100)
    await tick(100)
    await tick(100) // t=400，连续 3 帧未见

    detector.detect.mockResolvedValue([{ rawValue: 'A' }])
    await tick(100) // t=500 再出现，距上次仅 400ms
    expect(onCode).toHaveBeenCalledTimes(1)

    // 之后一直可见也不再上报：重新出现那一下没够 cooldown，就算此段结束。
    for (let i = 0; i < 20; i++) await tick(100)
    expect(onCode).toHaveBeenCalledTimes(1)
    scanner.stop()
  })

  it('持续可见 10 秒只上报 1 次', async () => {
    const detector = makeDetector()
    detector.detect.mockResolvedValue([{ rawValue: 'A' }])
    mocks.loadDetector.mockResolvedValue(detector)

    const { scanner, onCode } = makeScanner({ intervalMs: 200, cooldownMs: 1500 })
    await scanner.start()
    for (let i = 0; i < 50; i++) await tick(200) // 10 秒

    expect(onCode).toHaveBeenCalledTimes(1)
    scanner.stop()
  })

  it('不同的码立即上报', async () => {
    const detector = makeDetector()
    mocks.loadDetector.mockResolvedValue(detector)

    const { scanner, onCode } = makeScanner({ intervalMs: 100 })
    await scanner.start()

    detector.detect.mockResolvedValue([{ rawValue: 'A' }])
    await tick(100)
    detector.detect.mockResolvedValue([{ rawValue: 'B' }])
    await tick(100)

    expect(onCode.mock.calls.map((c) => c[0])).toEqual(['A', 'B'])
    scanner.stop()
  })

  it('pause 期间不 detect、不上报；resume 后恢复', async () => {
    const detector = makeDetector()
    detector.detect.mockResolvedValue([{ rawValue: 'A' }])
    mocks.loadDetector.mockResolvedValue(detector)

    const { scanner, onCode } = makeScanner({ intervalMs: 100 })
    await scanner.start()
    await tick(100)
    expect(onCode).toHaveBeenCalledTimes(1)
    const calls = detector.detect.mock.calls.length

    scanner.pause()
    await tick(100)
    await tick(100)
    expect(onCode).toHaveBeenCalledTimes(1)
    expect(detector.detect.mock.calls.length).toBe(calls)

    scanner.resume()
    await tick(100)
    expect(onCode).toHaveBeenCalledTimes(2)
    scanner.stop()
  })

  it('上一次 detect 未返回时跳过本拍，不重入', async () => {
    let resolveDetect!: (value: DetectResult[]) => void
    const detector = makeDetector()
    detector.detect.mockImplementation(
      () =>
        new Promise<DetectResult[]>((resolve) => {
          resolveDetect = resolve
        })
    )
    mocks.loadDetector.mockResolvedValue(detector)

    const { scanner, onCode } = makeScanner({ intervalMs: 100 })
    await scanner.start()

    await tick(100) // 第 1 拍发起 detect，promise 一直挂着
    expect(detector.detect).toHaveBeenCalledTimes(1)
    await tick(100)
    await tick(100)
    await tick(100)
    expect(detector.detect).toHaveBeenCalledTimes(1)

    resolveDetect([{ rawValue: 'A' }])
    await vi.advanceTimersByTimeAsync(0)
    expect(onCode).toHaveBeenCalledTimes(1)
    scanner.stop()
  })

  it('引擎加载失败 → error 非空且不上报', async () => {
    mocks.loadDetector.mockRejectedValue(new Error('引擎炸了'))

    const { scanner, onCode } = makeScanner()
    await scanner.start()
    expect(scanner.error.value).toContain('引擎炸了')
    await tick()
    expect(onCode).not.toHaveBeenCalled()
    scanner.stop()
  })

  it('stop 后不再 detect', async () => {
    const detector = makeDetector()
    detector.detect.mockResolvedValue([{ rawValue: 'A' }])
    mocks.loadDetector.mockResolvedValue(detector)

    const { scanner } = makeScanner({ intervalMs: 100 })
    await scanner.start()
    await tick(100)
    const calls = detector.detect.mock.calls.length
    scanner.stop()
    await tick(100)
    await tick(100)
    expect(detector.detect.mock.calls.length).toBe(calls)
  })
})
