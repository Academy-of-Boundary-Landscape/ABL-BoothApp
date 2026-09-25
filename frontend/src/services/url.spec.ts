import { describe, it, expect, vi, beforeEach } from 'vitest'

beforeEach(() => {
  vi.resetModules()
  delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
})

describe('toAbsoluteApiUrl', () => {
  it('浏览器（LAN 设备）里保持同源相对路径，不拼 127.0.0.1', async () => {
    const { toAbsoluteApiUrl } = await import('./url')
    expect(toAbsoluteApiUrl('/api/events/3/settlement.xlsx')).toBe('/api/events/3/settlement.xlsx')
    expect(toAbsoluteApiUrl('api/x')).toBe('/api/x')
  })

  it('Tauri 里拼上后端的绝对地址', async () => {
    ;(window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {}
    const { toAbsoluteApiUrl } = await import('./url')
    expect(toAbsoluteApiUrl('/api/x')).toBe('http://127.0.0.1:5140/api/x')
    expect(toAbsoluteApiUrl('api/x')).toBe('http://127.0.0.1:5140/api/x')
  })

  it('已经是绝对地址的原样返回', async () => {
    const { toAbsoluteApiUrl } = await import('./url')
    expect(toAbsoluteApiUrl('https://h:5141/api/x')).toBe('https://h:5141/api/x')
  })
})
