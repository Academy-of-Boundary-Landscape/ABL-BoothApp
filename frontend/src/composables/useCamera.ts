// composables/useCamera.ts —— 摄像头取流 / 翻转 / 补光灯 / 释放的唯一入口。
//
// 关键约束（spec §6.2）：任何时候卸载都不漏 track —— 取流进行中 stop / 再次 start、
// 连点翻转，拿到的旧流都会被 stop。做法是用一个递增的请求序号标记每次取流，
// getUserMedia resolve 时序号已过期就立刻放掉这条流，onScopeDispose 里统一 stop。
import { ref, shallowRef, shallowReadonly, onScopeDispose } from 'vue'
import type { Ref } from 'vue'

export type Facing = 'user' | 'environment'

/** 默认取景分辨率（沿用 VisionSearch 原值）。 */
const DEFAULT_CONSTRAINTS: MediaTrackConstraints = {
  width: { ideal: 1280 },
  height: { ideal: 960 },
}

/** 非安全上下文的兜底文案，原样搬自 VisionSearch。 */
const INSECURE_CONTEXT_ERROR =
  '当前页面不是安全连接，浏览器禁止访问摄像头。请确认 URL 以 https 开头，' +
  '且首次访问时已点击「高级 → 继续访问」接受证书。' +
  '如仍无法解决，请直接在主机的摊盒桌面应用内拍照。'

interface TorchCapabilities extends MediaTrackCapabilities {
  torch?: boolean
}

export function useCamera(opts: { facing: Facing; constraints?: MediaTrackConstraints }): {
  stream: Readonly<Ref<MediaStream | null>>
  isActive: Readonly<Ref<boolean>>
  facing: Ref<Facing>
  error: Ref<string>
  torchSupported: Readonly<Ref<boolean>>
  torchOn: Readonly<Ref<boolean>>
  start(): Promise<boolean>
  stop(): void
  flip(): Promise<boolean>
  setTorch(on: boolean): Promise<void>
} {
  // MediaStream 必须用 shallowRef：deep ref 会把它包成 reactive 代理，
  // 破坏 `video.srcObject = stream` 的对象身份。
  const stream = shallowRef<MediaStream | null>(null)
  const isActive = ref(false)
  const facing = ref<Facing>(opts.facing)
  const error = ref('')
  const torchSupported = ref(false)
  const torchOn = ref(false)

  // 递增的请求序号：每次 start / stop / dispose 都 +1。
  // getUserMedia resolve 时序号已变，说明这次取流已过期，立即放掉。
  let requestId = 0

  function stopStream(target: MediaStream | null) {
    target?.getTracks().forEach((track) => track.stop())
  }

  function videoTrackOf(target: MediaStream | null): MediaStreamTrack | undefined {
    return target?.getVideoTracks()[0]
  }

  function stop(): void {
    requestId++
    if (stream.value) {
      stopStream(stream.value)
      stream.value = null
    }
    isActive.value = false
    torchSupported.value = false
    torchOn.value = false
  }

  async function start(): Promise<boolean> {
    error.value = ''

    // 防御性兜底：getUserMedia 仅在 secure context（HTTPS / localhost）可用。
    // 摊主在 LAN 浏览器首次访问 https URL 但未接受证书时，或意外通过 http URL
    // 进入时，给清晰提示而不是浏览器内部错误。
    if (
      !window.isSecureContext ||
      !navigator.mediaDevices ||
      typeof navigator.mediaDevices.getUserMedia !== 'function'
    ) {
      error.value = INSECURE_CONTEXT_ERROR
      return false
    }

    const id = ++requestId
    try {
      const next = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: facing.value, ...DEFAULT_CONSTRAINTS, ...opts.constraints },
        audio: false,
      })
      if (id !== requestId) {
        // 取流期间 stop / 又 start / 组件已卸载：这条流没人要了，直接放掉。
        stopStream(next)
        return false
      }
      if (stream.value) stopStream(stream.value)
      stream.value = next
      isActive.value = true
      torchOn.value = false

      const track = videoTrackOf(next)
      const caps = track?.getCapabilities?.() as TorchCapabilities | undefined
      torchSupported.value = Boolean(caps?.torch)
      return true
    } catch (err) {
      if (id !== requestId) return false
      const e = err as { message?: string; name?: string }
      error.value = '无法访问摄像头: ' + (e.message || e.name)
      return false
    }
  }

  async function flip(): Promise<boolean> {
    // 先停旧流再切朝向：很多 Android 设备不能同时打开两个摄像头，先 start 新流会
    // 抛 NotReadableError（拍照识别的「翻转」就回退了）；而且旧流不先停，翻转失败时
    // 摄像头指示灯会一直亮着。stop() 会清掉 stream 并让新 start 拿到新的 requestId。
    stop()
    facing.value = facing.value === 'user' ? 'environment' : 'user'
    return start()
  }

  async function setTorch(on: boolean): Promise<void> {
    const track = videoTrackOf(stream.value)
    if (!track || !torchSupported.value) return
    try {
      const constraints = { advanced: [{ torch: on }] } as unknown as MediaTrackConstraints
      await track.applyConstraints(constraints)
      torchOn.value = on
    } catch {
      // 设备不支持 / 被系统拒绝：静默置回关闭态，不打断扫码。
      torchOn.value = false
    }
  }

  onScopeDispose(stop)

  return {
    stream: shallowReadonly(stream),
    isActive: shallowReadonly(isActive),
    facing,
    error,
    torchSupported: shallowReadonly(torchSupported),
    torchOn: shallowReadonly(torchOn),
    start,
    stop,
    flip,
    setTorch,
  }
}
