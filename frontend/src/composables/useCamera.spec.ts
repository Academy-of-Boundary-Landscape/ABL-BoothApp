import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { effectScope } from 'vue'
import type { EffectScope } from 'vue'
import { useCamera, type Facing } from './useCamera'

interface FakeTrack {
  stop: ReturnType<typeof vi.fn>
  applyConstraints: ReturnType<typeof vi.fn>
  getCapabilities: ReturnType<typeof vi.fn>
}

function makeTrack(overrides: Partial<FakeTrack> = {}): FakeTrack {
  return {
    stop: vi.fn(),
    applyConstraints: vi.fn().mockResolvedValue(undefined),
    getCapabilities: vi.fn(() => ({})),
    ...overrides,
  }
}

function makeStream(track: FakeTrack): MediaStream {
  return {
    getTracks: () => [track],
    getVideoTracks: () => [track],
  } as unknown as MediaStream
}

/** 手动控制 resolve 时机的 getUserMedia：用于「取流还没回来」的场景。 */
function deferredGum() {
  const resolvers: Array<(s: MediaStream) => void> = []
  const gum = vi.fn(
    () =>
      new Promise<MediaStream>((resolve) => {
        resolvers.push(resolve)
      })
  )
  return { gum, resolvers }
}

function setSecure(secure: boolean) {
  Object.defineProperty(window, 'isSecureContext', { configurable: true, value: secure })
}

function setMediaDevices(getUserMedia: unknown) {
  Object.defineProperty(navigator, 'mediaDevices', {
    configurable: true,
    value: { getUserMedia },
  })
}

function clearMediaDevices() {
  // 删掉测试注入的 own property，恢复 jsdom 原状。
  Reflect.deleteProperty(navigator, 'mediaDevices')
}

let scopes: EffectScope[] = []

function makeCamera(facing: Facing = 'environment') {
  const scope = effectScope()
  scopes.push(scope)
  const cam = scope.run(() => useCamera({ facing }))!
  return { cam, scope }
}

beforeEach(() => {
  setSecure(true)
})

afterEach(() => {
  scopes.forEach((s) => s.stop())
  scopes = []
  clearMediaDevices()
  vi.restoreAllMocks()
})

describe('useCamera', () => {
  it('start 成功 → isActive true；stop → track.stop 被调用、isActive false', async () => {
    const track = makeTrack()
    const stream = makeStream(track)
    const gum = vi.fn().mockResolvedValue(stream)
    setMediaDevices(gum)
    const { cam } = makeCamera()

    await expect(cam.start()).resolves.toBe(true)
    expect(gum).toHaveBeenCalledOnce()
    expect(cam.isActive.value).toBe(true)
    expect(cam.stream.value).toBe(stream)
    expect(track.stop).not.toHaveBeenCalled()

    cam.stop()
    expect(track.stop).toHaveBeenCalledOnce()
    expect(cam.isActive.value).toBe(false)
    expect(cam.stream.value).toBeNull()
  })

  it('getUserMedia 收到 facingMode 与默认分辨率约束，constraints 可覆盖', async () => {
    const gum = vi.fn().mockResolvedValue(makeStream(makeTrack()))
    setMediaDevices(gum)

    const scope = effectScope()
    scopes.push(scope)
    const cam = scope.run(() =>
      useCamera({ facing: 'user', constraints: { width: { ideal: 640 } } })
    )!

    await cam.start()
    expect(gum).toHaveBeenCalledWith({
      video: { facingMode: 'user', width: { ideal: 640 }, height: { ideal: 960 } },
      audio: false,
    })
  })

  it('start 未 resolve 时 scope 被 dispose → resolve 后该流 track 被 stop，isActive 保持 false', async () => {
    const { gum, resolvers } = deferredGum()
    setMediaDevices(gum)
    const { cam, scope } = makeCamera()

    const pending = cam.start()
    scope.stop()

    const track = makeTrack()
    resolvers[0](makeStream(track))

    await expect(pending).resolves.toBe(false)
    expect(track.stop).toHaveBeenCalledOnce()
    expect(cam.isActive.value).toBe(false)
    expect(cam.stream.value).toBeNull()
  })

  it('start 未 resolve 时 stop() → resolve 后该流 track 被 stop', async () => {
    const { gum, resolvers } = deferredGum()
    setMediaDevices(gum)
    const { cam } = makeCamera()

    const pending = cam.start()
    cam.stop()

    const track = makeTrack()
    resolvers[0](makeStream(track))

    await expect(pending).resolves.toBe(false)
    expect(track.stop).toHaveBeenCalledOnce()
    expect(cam.isActive.value).toBe(false)
  })

  it('连续两次 flip（第一次还没 resolve）→ 只剩最后一个流活着，前一个 track 被 stop', async () => {
    const { gum, resolvers } = deferredGum()
    setMediaDevices(gum)
    const { cam } = makeCamera('environment')

    const firstFlip = cam.flip() // → user，取流 #1
    const secondFlip = cam.flip() // → environment，取流 #2
    expect(cam.facing.value).toBe('environment')
    expect(gum).toHaveBeenCalledTimes(2)

    const trackA = makeTrack()
    const trackB = makeTrack()
    const streamB = makeStream(trackB)
    resolvers[0](makeStream(trackA))
    resolvers[1](streamB)

    await expect(firstFlip).resolves.toBe(false)
    await expect(secondFlip).resolves.toBe(true)
    expect(trackA.stop).toHaveBeenCalledOnce()
    expect(trackB.stop).not.toHaveBeenCalled()
    expect(cam.isActive.value).toBe(true)
    expect(cam.stream.value).toBe(streamB)
  })

  it('start 成功后再 flip 成功 → 旧 track 在第二次 getUserMedia 之前已 stop，只剩第二个流活着', async () => {
    const trackA = makeTrack()
    const streamA = makeStream(trackA)
    const trackB = makeTrack()
    const streamB = makeStream(trackB)
    const gum = vi.fn().mockResolvedValueOnce(streamA).mockResolvedValueOnce(streamB)
    setMediaDevices(gum)
    const { cam } = makeCamera('environment')

    await expect(cam.start()).resolves.toBe(true)
    expect(cam.stream.value).toBe(streamA)
    expect(cam.facing.value).toBe('environment')
    expect(trackA.stop).not.toHaveBeenCalled()

    await expect(cam.flip()).resolves.toBe(true)
    expect(cam.facing.value).toBe('user')

    // 先停旧流：trackA.stop 必须发生在第二次 getUserMedia 之前，很多 Android 设备
    // 不能同时开两个摄像头（否则新流 NotReadableError）。
    expect(trackA.stop.mock.invocationCallOrder[0]).toBeLessThan(
      gum.mock.invocationCallOrder[1]
    )
    expect(trackA.stop).toHaveBeenCalledOnce()
    expect(trackB.stop).not.toHaveBeenCalled()
    expect(cam.isActive.value).toBe(true)
    expect(cam.stream.value).toBe(streamB)
  })

  it('flip 时 getUserMedia 失败 → 旧流已停、无活 track、isActive false、error 非空', async () => {
    const trackA = makeTrack()
    const gum = vi
      .fn()
      .mockResolvedValueOnce(makeStream(trackA))
      .mockRejectedValueOnce(new Error('NotReadableError'))
    setMediaDevices(gum)
    const { cam } = makeCamera('environment')

    await expect(cam.start()).resolves.toBe(true)
    expect(cam.isActive.value).toBe(true)

    await expect(cam.flip()).resolves.toBe(false)

    expect(trackA.stop).toHaveBeenCalledOnce()
    expect(cam.stream.value).toBeNull()
    expect(cam.isActive.value).toBe(false)
    expect(cam.error.value).not.toBe('')
  })

  it('非安全上下文 → start 返回 false、error 非空、不调用 getUserMedia', async () => {
    setSecure(false)
    const gum = vi.fn()
    setMediaDevices(gum)
    const { cam } = makeCamera()

    await expect(cam.start()).resolves.toBe(false)
    expect(cam.error.value).not.toBe('')
    expect(gum).not.toHaveBeenCalled()
    expect(cam.isActive.value).toBe(false)
  })

  it('没有 getUserMedia → start 返回 false、error 非空', async () => {
    setMediaDevices(undefined)
    const { cam } = makeCamera()

    await expect(cam.start()).resolves.toBe(false)
    expect(cam.error.value).not.toBe('')
  })

  it('权限被拒 → error 为「无法访问摄像头: ...」', async () => {
    const gum = vi.fn().mockRejectedValue(new Error('Permission denied'))
    setMediaDevices(gum)
    const { cam } = makeCamera()

    await expect(cam.start()).resolves.toBe(false)
    expect(cam.error.value).toBe('无法访问摄像头: Permission denied')
    expect(cam.isActive.value).toBe(false)
  })

  it('track capabilities 含 torch → torchSupported true；setTorch(true) 调用 applyConstraints', async () => {
    const track = makeTrack({ getCapabilities: vi.fn(() => ({ torch: true })) })
    const gum = vi.fn().mockResolvedValue(makeStream(track))
    setMediaDevices(gum)
    const { cam } = makeCamera()

    await cam.start()
    expect(cam.torchSupported.value).toBe(true)
    expect(cam.torchOn.value).toBe(false)

    await cam.setTorch(true)
    expect(track.applyConstraints).toHaveBeenCalledWith({ advanced: [{ torch: true }] })
    expect(cam.torchOn.value).toBe(true)

    await cam.setTorch(false)
    expect(track.applyConstraints).toHaveBeenLastCalledWith({ advanced: [{ torch: false }] })
    expect(cam.torchOn.value).toBe(false)
  })

  it('track 不支持 torch → torchSupported false，setTorch 不报错', async () => {
    const track = makeTrack()
    const gum = vi.fn().mockResolvedValue(makeStream(track))
    setMediaDevices(gum)
    const { cam } = makeCamera()

    await cam.start()
    expect(cam.torchSupported.value).toBe(false)

    await expect(cam.setTorch(true)).resolves.toBeUndefined()
    expect(cam.torchOn.value).toBe(false)
  })

  it('applyConstraints 失败 → torchOn 静默置回 false', async () => {
    const track = makeTrack({
      getCapabilities: vi.fn(() => ({ torch: true })),
      applyConstraints: vi.fn().mockRejectedValue(new Error('nope')),
    })
    const gum = vi.fn().mockResolvedValue(makeStream(track))
    setMediaDevices(gum)
    const { cam } = makeCamera()

    await cam.start()
    await cam.setTorch(true)
    expect(cam.torchOn.value).toBe(false)
  })
})
