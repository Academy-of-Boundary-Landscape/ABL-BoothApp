import { describe, it, expect, vi, beforeEach } from 'vitest'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }))

beforeEach(() => {
  vi.resetModules()
  invoke.mockReset()
  delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
})

describe('backendOrigin', () => {
  it('Tauri 内用 get_backend_url 给出的实际地址（5140 被占时后端会换端口）', async () => {
    ;(window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {}
    invoke.mockResolvedValue('http://127.0.0.1:5152')
    const m = await import('./backendOrigin')
    await m.initBackendOrigin()
    expect(invoke).toHaveBeenCalledWith('get_backend_url')
    expect(m.backendOrigin()).toBe('http://127.0.0.1:5152')
  })

  it('取不到就退回默认 5140，不阻塞启动', async () => {
    ;(window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {}
    invoke.mockRejectedValue(new Error('no command'))
    const m = await import('./backendOrigin')
    await m.initBackendOrigin()
    expect(m.backendOrigin()).toBe('http://127.0.0.1:5140')
  })

  it('浏览器（LAN 顾客端）里不问 Tauri，保持默认值（API 走同源的 /api，见 client.ts）', async () => {
    const m = await import('./backendOrigin')
    await m.initBackendOrigin()
    expect(invoke).not.toHaveBeenCalled()
    expect(m.backendOrigin()).toBe('http://127.0.0.1:5140')
  })
})
