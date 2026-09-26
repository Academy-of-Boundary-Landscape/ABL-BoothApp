/**
 * 扫码提示音（spec §5.2）：用 Web Audio 现场生成短促「嘀」，成功 / 失败两种音高。
 * 不复用新单提示音 `notify.mp3`，也不依赖任何音频资源。
 *
 * 音频不可用（无 AudioContext、被浏览器策略拦截、jsdom 测试环境）时静默失败，
 * 绝不因提示音打断连续扫码。
 */

type AudioContextCtor = new () => AudioContext

let ctx: AudioContext | null = null

function audioCtx(): AudioContext | null {
  if (typeof window === 'undefined') return null
  const Ctor =
    window.AudioContext ??
    (window as unknown as { webkitAudioContext?: AudioContextCtor }).webkitAudioContext
  if (!Ctor) return null
  if (!ctx) {
    try {
      ctx = new Ctor()
    } catch {
      return null
    }
  }
  return ctx
}

/**
 * @param ok true = 成功（高音），false = 失败（低音）。
 */
export function playScanBeep(ok: boolean): void {
  const ac = audioCtx()
  if (!ac) return
  try {
    if (ac.state === 'suspended') void ac.resume()
    const oscillator = ac.createOscillator()
    const gain = ac.createGain()
    const start = ac.currentTime
    oscillator.type = 'sine'
    oscillator.frequency.value = ok ? 1200 : 380
    gain.gain.setValueAtTime(0.0001, start)
    gain.gain.exponentialRampToValueAtTime(0.18, start + 0.01)
    gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.12)
    oscillator.connect(gain)
    gain.connect(ac.destination)
    oscillator.start(start)
    oscillator.stop(start + 0.13)
  } catch {
    // 音频不可用不影响扫码。
  }
}
