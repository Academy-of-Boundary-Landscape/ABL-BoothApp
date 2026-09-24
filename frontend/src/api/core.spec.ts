// @vitest-environment node
// 用 node 环境而不是 jsdom：jsdom 的 FormData 与 Node 的 Request 不互通，
// 在 jsdom 里测 multipart 会被序列化成 "[object FormData]"，测的就不是真实行为了。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  createApiClient,
  unwrap,
  ApiRequestError,
  errorMessage,
  type UploadErrorLike,
} from './core'

type Handler = (req: Request, signal: AbortSignal) => Response | Promise<Response>
const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } })

let onUnauthorized: ReturnType<typeof vi.fn<() => void>>
let onUploadError: ReturnType<typeof vi.fn<(url: string, errLike: UploadErrorLike) => void>>
let currentPath: string
let seen: Request[]

function make(h: Handler, timeoutMs = 30_000) {
  onUnauthorized = vi.fn<() => void>()
  onUploadError = vi.fn<(url: string, errLike: UploadErrorLike) => void>()
  seen = []
  return createApiClient({
    baseUrl: 'http://127.0.0.1:5140/api',
    fetch: async (req, signal) => {
      seen.push(req)
      return h(req, signal)
    },
    timeoutMs,
    getToken: () => 'tok',
    currentPath: () => currentPath,
    onUnauthorized,
    onUploadError,
  })
}

beforeEach(() => {
  currentPath = '/admin'
})

describe('api core', () => {
  it('成功时 unwrap 返回 data，并带上 Bearer', async () => {
    const api = make(() => json(200, ['微信']))
    await expect(unwrap(api.GET('/channels'))).resolves.toEqual(['微信'])
    expect(seen[0]!.headers.get('authorization')).toBe('Bearer tok')
    expect(seen[0]!.url).toBe('http://127.0.0.1:5140/api/channels')
  })

  it('路径参数被替换', async () => {
    const api = make(() => json(200, []))
    await unwrap(api.GET('/events/{event_id}/advances', { params: { path: { event_id: 7 } } }))
    expect(seen[0]!.url).toBe('http://127.0.0.1:5140/api/events/7/advances')
  })

  it('JSON 错误体：serverMessage 取 error 字段，旧式 response 兼容读取仍可用', async () => {
    const api = make(() => json(409, { error: '展会已冻结' }))
    const e = await unwrap(api.GET('/channels')).catch((x: unknown) => x)
    expect(e).toBeInstanceOf(ApiRequestError)
    const err = e as ApiRequestError
    expect(err.status).toBe(409)
    expect(err.serverMessage).toBe('展会已冻结')
    expect((err.response.data as { error: string }).error).toBe('展会已冻结')
    expect(errorMessage(err, '加载失败')).toBe('展会已冻结')
  })

  it('plain-text error body：serverMessage 为空，走调用方兜底', async () => {
    const api = make(() => new Response('JSON Parse Error: expected value', { status: 400 }))
    const e = (await unwrap(api.GET('/channels')).catch((x: unknown) => x)) as ApiRequestError
    expect(e.status).toBe(400)
    expect(e.serverMessage).toBeUndefined()
    expect(errorMessage(e, '保存失败')).toBe('保存失败')
  })

  it('network failure becomes status 0', async () => {
    const api = make(() => {
      throw new TypeError('Failed to fetch')
    })
    const e = (await unwrap(api.GET('/channels')).catch((x: unknown) => x)) as ApiRequestError
    expect(e).toBeInstanceOf(ApiRequestError)
    expect(e.status).toBe(0)
    expect(errorMessage(e, '加载失败')).toBe('加载失败')
  })

  it('errorMessage 对非 ApiRequestError 一律给兜底', () => {
    expect(errorMessage(new Error('boom'), '兜底')).toBe('兜底')
    expect(errorMessage('x', '兜底')).toBe('兜底')
  })

  it('empty success body 返回 undefined 而不是抛错', async () => {
    const api = make(() => new Response(null, { status: 204 }))
    await expect(
      unwrap(
        api.DELETE('/events/{event_id}/advances/{id}', { params: { path: { event_id: 1, id: 2 } } })
      )
    ).resolves.toBeUndefined()
    const api2 = make(() => new Response('', { status: 200 }))
    await expect(unwrap(api2.GET('/channels'))).resolves.toBeUndefined()
  })

  it('401 跳登录', async () => {
    const api = make(() => json(401, { error: '无效的令牌' }))
    await unwrap(api.GET('/channels')).catch(() => {})
    expect(onUnauthorized).toHaveBeenCalledOnce()
  })

  it('401 on /login does not redirect，错误照常抛出', async () => {
    currentPath = '/login'
    const api = make(() => json(401, { error: '密码错误' }))
    const e = await unwrap(api.GET('/channels')).catch((x: unknown) => x)
    expect(onUnauthorized).not.toHaveBeenCalled()
    expect(errorMessage(e, '登录失败')).toBe('密码错误')
  })

  it('JSON body 带 application/json，multipart 由运行时补 boundary', async () => {
    const api = make(() => json(201, { id: 1, journal_id: 2 }))
    await unwrap(
      api.POST('/events/{event_id}/advances', {
        params: { path: { event_id: 1 } },
        body: { society_id: 1, label: 'x', amount: 1 as never },
      })
    )
    expect(seen[0]!.headers.get('content-type')).toBe('application/json')
    expect(await seen[0]!.text()).toBe('{"society_id":1,"label":"x","amount":1}')

    const fd = new FormData()
    fd.append('f', new Blob(['x']), 'a.png')
    // @ts-expect-error 此时 openapi.json 里还没有 multipart 路由；这里只测传输层，路径类型无关紧要
    await unwrap(api.POST('/master-products', { body: fd }))
    expect(seen[1]!.headers.get('content-type')).toMatch(/^multipart\/form-data; boundary=/)
  })

  it('413 upload：multipart 请求失败触发上传弹窗，errLike 兼容 normalizeUploadError', async () => {
    const api = make(() => new Response('length limit exceeded', { status: 413 }))
    const fd = new FormData()
    fd.append('f', new Blob(['x']), 'a.png')
    // @ts-expect-error 同上一条：此时还没有 multipart 路由
    await unwrap(api.POST('/master-products', { body: fd })).catch(() => {})
    expect(onUploadError).toHaveBeenCalledOnce()
    const [url, errLike] = onUploadError.mock.calls[0]!
    expect(url).toContain('/master-products')
    expect(errLike.response.status).toBe(413)
  })

  it('非 multipart 请求失败不弹上传框', async () => {
    const api = make(() => json(400, { error: 'x' }))
    await unwrap(api.GET('/channels')).catch(() => {})
    expect(onUploadError).not.toHaveBeenCalled()
  })

  it('caller signal and global timeout both abort', async () => {
    // 模拟真实 fetch：signal 已经 abort 就立刻拒绝，否则等它 abort
    const hang: Handler = (_req, signal) =>
      new Promise((_, reject) => {
        if (signal.aborted) reject(signal.reason)
        signal.addEventListener('abort', () => reject(signal.reason))
      })
    // 全局超时先到
    const e1 = (await unwrap(make(hang, 20).GET('/channels')).catch(
      (x: unknown) => x
    )) as ApiRequestError
    expect(e1.status).toBe(0)
    expect(e1.message).toBe('请求超时')
    // 调用方 signal 先到——请求发出之后才取消（连接探测的真实时序）
    const ctl = new AbortController()
    let p!: Promise<unknown>
    const started = new Promise<void>((resolve) => {
      const api = make((req, signal) => {
        resolve()
        return hang(req, signal)
      }, 60_000)
      p = unwrap(api.GET('/channels', { signal: ctl.signal })).catch((x: unknown) => x)
    })
    await started
    ctl.abort()
    const e2 = (await p) as ApiRequestError
    expect(e2.status).toBe(0)
    expect(e2.message).toBe('请求已取消')
    // 发出之前就已取消的 signal 也一样
    const done = new AbortController()
    done.abort()
    const e3 = (await unwrap(make(hang).GET('/channels', { signal: done.signal })).catch(
      (x: unknown) => x
    )) as ApiRequestError
    expect(e3.message).toBe('请求已取消')
  })
})
