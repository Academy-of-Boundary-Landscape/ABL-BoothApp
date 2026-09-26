/**
 * 扫码枪（spec §7 第二步）：扫码枪在系统里就是一串「极短间隔的可打印字符 + 回车」，
 * 这里在 window 上监听 keydown，把这种输入识别成一次扫码。
 *
 * 与摄像头扫码互补：不依赖摄像头、不改全局对象。输入框聚焦时完全不拦截，免得用户
 * 正常打字（包括按回车提交表单）被吞掉。
 */
import { onScopeDispose } from 'vue'
import type { Ref } from 'vue'

const DEFAULT_MAX_INTERVAL_MS = 50
const DEFAULT_MIN_LENGTH = 4

/** 事件目标在可编辑控件里时完全不处理（输入框 / 文本域 / 下拉 / contenteditable）。 */
function isEditableTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null
  if (!el || typeof el.tagName !== 'string') return false
  const tag = el.tagName.toLowerCase()
  if (tag === 'input' || tag === 'textarea' || tag === 'select') return true
  return el.isContentEditable === true
}

export interface ScanGunOptions {
  onCode: (code: string) => void
  /** 相邻按键间隔上限；超过就认为上一段输入结束，缓冲重置。默认 50ms。 */
  maxIntervalMs?: number
  /** 触发一次扫码所需的最短长度。默认 4。 */
  minLength?: number
  /** 为 false 时完全忽略按键。默认始终启用。 */
  enabled?: Ref<boolean>
}

export function useScanGun(opts: ScanGunOptions): void {
  const maxIntervalMs = opts.maxIntervalMs ?? DEFAULT_MAX_INTERVAL_MS
  const minLength = opts.minLength ?? DEFAULT_MIN_LENGTH

  let buffer = ''
  let lastAt = 0

  function reset() {
    buffer = ''
    lastAt = 0
  }

  function onKeydown(event: KeyboardEvent) {
    if (opts.enabled && !opts.enabled.value) return
    // 系统快捷键（复制粘贴等）不是扫码枪输入，不参与缓冲。
    if (event.ctrlKey || event.metaKey || event.altKey) return
    if (isEditableTarget(event.target)) return

    const key = event.key

    if (key === 'Enter') {
      if (buffer.length >= minLength) {
        const code = buffer
        reset()
        event.preventDefault()
        opts.onCode(code)
      } else {
        reset()
      }
      return
    }

    // 只收可打印单字符；功能键 / 修饰键把缓冲打断。
    if (key.length !== 1) {
      reset()
      return
    }

    const now = Date.now()
    if (lastAt !== 0 && now - lastAt > maxIntervalMs) {
      // 人手速度：上一段输入已结束，从头开始。
      buffer = key
    } else {
      buffer += key
    }
    lastAt = now
  }

  window.addEventListener('keydown', onKeydown, true)
  onScopeDispose(() => {
    window.removeEventListener('keydown', onKeydown, true)
  })
}
