import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { effectScope, ref } from 'vue'
import type { EffectScope } from 'vue'
import { useScanGun, type ScanGunOptions } from './useScanGun'

let scopes: EffectScope[] = []

function makeGun(opts: Partial<ScanGunOptions> = {}) {
  const onCode = vi.fn<(code: string) => void>()
  const scope = effectScope()
  scopes.push(scope)
  scope.run(() => useScanGun({ onCode, ...opts }))
  return { onCode, scope }
}

/** 向 window 派发一次按键，返回事件对象以便检查 defaultPrevented。 */
function press(key: string, target: EventTarget = window): KeyboardEvent {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true })
  target.dispatchEvent(event)
  return event
}

function typeFast(text: string, gapMs = 10) {
  for (const ch of text) {
    press(ch)
    vi.advanceTimersByTime(gapMs)
  }
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

describe('useScanGun', () => {
  it('快速输入一整串 + Enter → onCode 一次', () => {
    const { onCode } = makeGun()
    typeFast('4901234567894')
    press('Enter')

    expect(onCode).toHaveBeenCalledTimes(1)
    expect(onCode).toHaveBeenCalledWith('4901234567894')
  })

  it('人手速度（每键 150ms）输入同样内容 + Enter → 不触发', () => {
    const { onCode } = makeGun()
    typeFast('4901234567894', 150)
    press('Enter')

    expect(onCode).not.toHaveBeenCalled()
  })

  it('焦点在 input 内快速输入 + Enter → 不触发且不 preventDefault', () => {
    const { onCode } = makeGun()
    const input = document.createElement('input')
    document.body.appendChild(input)
    input.focus()

    typeFast('4901234567894')
    const enter = press('Enter', input)

    expect(onCode).not.toHaveBeenCalled()
    expect(enter.defaultPrevented).toBe(false)
    input.remove()
  })

  it('长度不足 minLength → 不触发，Enter 也不拦截', () => {
    const { onCode } = makeGun()
    typeFast('123')
    const enter = press('Enter')

    expect(onCode).not.toHaveBeenCalled()
    expect(enter.defaultPrevented).toBe(false)
  })

  it('卸载后不再监听', () => {
    const { onCode, scope } = makeGun()
    scope.stop()

    typeFast('4901234567894')
    press('Enter')

    expect(onCode).not.toHaveBeenCalled()
  })

  it('enabled 为 false 时忽略按键', () => {
    const enabled = ref(false)
    const { onCode, scope } = makeGun({ enabled })
    typeFast('4901234567894')
    press('Enter')
    expect(onCode).not.toHaveBeenCalled()
    scope.stop()
  })
})
