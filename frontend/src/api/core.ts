/**
 * API client 的核心：不碰 window / router / sessionStorage，全部依赖从参数注入，方便单测。
 * 业务代码不要直接 import 这里——用 `@/api/client`，它组装好了默认实例并重新导出这些名字。
 */
import createClient, { type Middleware } from 'openapi-fetch'
import type { paths } from './schema'

export class ApiRequestError extends Error {
  readonly status: number
  readonly body: unknown
  /** 后端 `{"error": "..."}` 里的那句话；错误体不是这个形状（纯文本、网络错误）时为 undefined。 */
  readonly serverMessage?: string

  constructor(status: number, body: unknown, message?: string) {
    const server =
      body !== null &&
      typeof body === 'object' &&
      typeof (body as { error?: unknown }).error === 'string'
        ? (body as { error: string }).error
        : undefined
    super(server ?? message ?? `请求失败（${status}）`)
    this.name = 'ApiRequestError'
    this.status = status
    this.body = body
    this.serverMessage = server
  }

  /**
   * @deprecated ③b 过渡：store 已切到新 client、组件还是 JS 时，组件里
   * `err.response?.data?.error` 靠它继续工作。Task 12 删除。
   */
  get response(): { status: number; data: unknown } {
    return { status: this.status, data: this.body }
  }
}

/**
 * 给用户看的错误文案。与旧写法 `err.response?.data?.error || fallback` 语义一致：
 * 只有后端明确给了一句话才用后端的，否则用调用方的兜底文案。
 */
export function errorMessage(e: unknown, fallback: string): string {
  return (e instanceof ApiRequestError && e.serverMessage) || fallback
}

/** openapi-fetch 返回值里我们用到的部分。 */
type FetchResult = { data?: unknown; error?: unknown; response: Response }

/**
 * 成功分支的 data 类型。openapi-fetch 的返回值是「成功（data 必填）| 失败（data?: never）」的联合；
 * 直接从 `{ data: T }` 推断会把失败分支的可选 data 也算进来，得到 `T | undefined`。
 * 这里只从 data 必填的那一支取类型。
 */
type SuccessData<R> = R extends { data: infer D } ? D : never

/** 成功返回 data；HTTP 错误、网络错误、超时、取消一律抛 ApiRequestError（后三者 status 为 0）。 */
export async function unwrap<R extends FetchResult>(p: Promise<R>): Promise<SuccessData<R>> {
  let r: R
  try {
    r = await p
  } catch (e) {
    const name = e instanceof Error || e instanceof DOMException ? e.name : ''
    const message =
      name === 'TimeoutError' ? '请求超时' : name === 'AbortError' ? '请求已取消' : '网络错误'
    throw new ApiRequestError(0, null, message)
  }
  if (!r.response.ok) throw new ApiRequestError(r.response.status, r.error)
  return r.data as SuccessData<R>
}

/** 上传失败回调收到的错误对象。形状与旧 `normalizeUploadError(error, …)` 读取的 axios 错误兼容。 */
export interface UploadErrorLike {
  message: string
  response: { status: number; data: unknown }
}

export interface ApiClientOptions {
  baseUrl: string
  /**
   * 真正发请求的函数。`signal` 是合并了调用方 signal 与全局超时的那一个，
   * 必须传给底层 fetch（`fetch(req, { signal })`）——不要依赖 `req.signal`：
   * Request 的信号是「跟随」来的，部分运行时（Node/undici）里跟随的信号不派发 abort 事件。
   */
  fetch: (req: Request, signal: AbortSignal) => Promise<Response>
  timeoutMs: number
  getToken: () => string | null
  currentPath: () => string
  onUnauthorized: () => void
  onUploadError: (url: string, errLike: UploadErrorLike) => void
}

const isRawBody = (b: unknown) =>
  (typeof FormData !== 'undefined' && b instanceof FormData) ||
  (typeof Blob !== 'undefined' && b instanceof Blob) ||
  b instanceof Uint8Array ||
  b instanceof ArrayBuffer

export function createApiClient(o: ApiClientOptions) {
  // 每个请求的控制信号与超时定时器，以 Request 对象为键；响应回来就清掉定时器。
  const controls = new WeakMap<
    Request,
    { signal: AbortSignal; timer: ReturnType<typeof setTimeout> }
  >()

  const client = createClient<paths>({
    baseUrl: o.baseUrl,
    fetch: (req: Request) => o.fetch(req, controls.get(req)?.signal ?? req.signal),
    // FormData / 原始字节原样透传，其余 JSON。
    // 注意：原始字节时 openapi-fetch 仍会默认加 Content-Type: application/json，
    // 调用方必须显式传 headers: { 'Content-Type': 'application/octet-stream' } 之类覆盖它。
    bodySerializer: (body: unknown) =>
      isRawBody(body) ? (body as BodyInit) : JSON.stringify(body),
  })

  const middleware: Middleware = {
    onRequest({ request }) {
      const token = o.getToken()
      if (token) request.headers.set('Authorization', `Bearer ${token}`)
      // 调用方的 signal（如连接探测的 3 秒超时）与全局超时谁先到谁生效。
      const ctl = new AbortController()
      const caller = request.signal
      if (caller.aborted) ctl.abort(caller.reason)
      else caller.addEventListener('abort', () => ctl.abort(caller.reason), { once: true })
      const timer = setTimeout(
        () => ctl.abort(new DOMException('请求超时', 'TimeoutError')),
        o.timeoutMs
      )
      controls.set(request, { signal: ctl.signal, timer })
    },
    onError({ request }) {
      clearTimeout(controls.get(request)?.timer)
    },
    async onResponse({ request, response }) {
      clearTimeout(controls.get(request)?.timer)
      if ((response.status === 401 || response.status === 403) && o.currentPath() !== '/login') {
        o.onUnauthorized()
      }
      const isUpload = (request.headers.get('content-type') ?? '').startsWith('multipart/form-data')
      if (isUpload && !response.ok) {
        const text = await response.clone().text()
        let data: unknown = text
        try {
          data = JSON.parse(text)
        } catch {
          // 纯文本错误体（如 axum 的 413），原样保留
        }
        o.onUploadError(request.url, { message: text, response: { status: response.status, data } })
      }
      return response
    },
  }
  client.use(middleware)
  return client
}

export type ApiClient = ReturnType<typeof createApiClient>
