import { describe, it, expect, vi } from 'vitest'

vi.mock('naive-ui', () => ({
  createDiscreteApi: () => ({ dialog: { warning: vi.fn() } }),
}))

const { bytesFromMb, validateFileSize, normalizeUploadError, IMAGE_UPLOAD_LIMIT_MB } = await import(
  './upload'
)

describe('bytesFromMb', () => {
  it('按 1024 换算', () => {
    expect(bytesFromMb(1)).toBe(1048576)
    expect(bytesFromMb(10)).toBe(10485760)
  })
})

describe('validateFileSize', () => {
  it('没有文件时不通过', () => {
    expect(validateFileSize(null, 10)).toEqual({ ok: false, message: '未选择文件。' })
  })

  it('恰好等于上限时通过（边界是闭区间）', () => {
    expect(validateFileSize({ size: bytesFromMb(10) }, 10)).toEqual({ ok: true })
  })

  it('超过上限时不通过并带上限数字', () => {
    const r = validateFileSize({ size: bytesFromMb(10) + 1 }, 10)
    expect(r.ok).toBe(false)
    expect(r.message).toContain('10MB')
  })
})

describe('normalizeUploadError', () => {
  // 组件里 catch 到的是新 client 的 ApiRequestError（不再有 axios 的 err.response）
  it('ApiRequestError：413 翻译成体积超限提示', async () => {
    const { ApiRequestError } = await import('@/api/core')
    const err = new ApiRequestError(413, 'length limit exceeded')
    expect(normalizeUploadError(err, 10)).toContain('10MB')
  })

  it('ApiRequestError：优先取后端返回的 error 字段', async () => {
    const { ApiRequestError } = await import('@/api/core')
    expect(normalizeUploadError(new ApiRequestError(400, { error: '后端说不行' }), 10)).toBe(
      '后端说不行'
    )
  })

  it('ApiRequestError：纯文本 multipart 解析错误也翻译成体积超限', async () => {
    const { ApiRequestError } = await import('@/api/core')
    const err = new ApiRequestError(400, 'Error parsing `multipart/form-data` request')
    expect(normalizeUploadError(err, 10)).toContain('10MB')
  })

  it('优先取后端返回的 error 字段', () => {
    const err = { response: { data: { error: '后端说不行' } } }
    expect(normalizeUploadError(err, 10)).toBe('后端说不行')
  })

  it('413 翻译成体积超限提示', () => {
    const err = { response: { status: 413, data: {} } }
    expect(normalizeUploadError(err, 10)).toContain('10MB')
  })

  it('multipart 解析错误也翻译成体积超限提示', () => {
    const err = { message: 'Error parsing `multipart/form-data` request' }
    expect(normalizeUploadError(err, 10)).toContain('10MB')
  })

  it('什么都没有时给兜底文案', () => {
    expect(normalizeUploadError({}, 10)).toBe('上传失败，请稍后重试。')
  })
})

describe('常量', () => {
  it('图片上限是 10MB', () => {
    expect(IMAGE_UPLOAD_LIMIT_MB).toBe(10)
  })
})
