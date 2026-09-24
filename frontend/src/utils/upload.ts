import { ApiRequestError } from '@/api/core'
import { createDiscreteApi } from 'naive-ui'

export const IMAGE_UPLOAD_LIMIT_MB = 10
export const IMAGE_WARN_THRESHOLD_MB = 3
export const SYNC_IMPORT_LIMIT_MB = 1000

const { dialog } = createDiscreteApi(['dialog'])

/** `validateFileSize` 的结果。通过时不带 message。 */
export interface FileSizeValidation {
  ok: boolean
  message?: string
}

export function bytesFromMb(mb: number): number {
  return mb * 1024 * 1024
}

export function validateFileSize(
  file: { size: number } | null | undefined,
  maxMb: number
): FileSizeValidation {
  if (!file) return { ok: false, message: '未选择文件。' }
  if (file.size <= bytesFromMb(maxMb)) return { ok: true }
  return {
    ok: false,
    message: `文件过大，请选择不超过 ${maxMb}MB 的文件。`,
  }
}

export function showUploadDialog(title: string, content: string): void {
  dialog.warning({
    title,
    content,
    positiveText: '知道了',
  })
}

/**
 * 大文件确认对话框，返回 Promise<boolean>
 */
export function confirmLargeFile(fileSizeMb: number): Promise<boolean> {
  return new Promise((resolve) => {
    dialog.warning({
      title: '图片文件较大',
      content: `当前文件大小为 ${fileSizeMb.toFixed(1)}MB，上传较大图片可能影响加载速度。建议压缩到 ${IMAGE_WARN_THRESHOLD_MB}MB 以内。是否继续上传？`,
      positiveText: '继续上传',
      negativeText: '取消',
      onPositiveClick: () => resolve(true),
      onNegativeClick: () => resolve(false),
      onClose: () => resolve(false),
    })
  })
}

/**
 * 将图片强制拉伸到 size×size 正方形，返回压缩后的 File。
 * 用于 AI 识别图等不需要保持宽高比的场景。
 *
 * 尺寸已经正好是目标时，或图片加载失败时，原样返回输入；因此输入是 Blob 时
 * 返回值也可能是 Blob。
 *
 * @param file - 待压缩的图片（File 或 Blob）
 * @param size - 输出正方形边长（像素）
 * @param quality - JPEG 质量 0-1
 */
export function resizeImageFile(
  file: File | Blob,
  size: number = 512,
  quality: number = 0.9
): Promise<File | Blob> {
  return new Promise((resolve) => {
    const img = new Image()
    img.onload = () => {
      const { width, height } = img
      if (width === size && height === size) {
        URL.revokeObjectURL(img.src)
        resolve(file)
        return
      }
      const canvas = document.createElement('canvas')
      canvas.width = size
      canvas.height = size
      // canvas 必定能拿到 2d 上下文；原实现直接解引用，null 时会抛错，这里保持同样语义。
      canvas.getContext('2d')!.drawImage(img, 0, 0, size, size)
      URL.revokeObjectURL(img.src)
      const fileName = (file instanceof File ? file.name : '') || 'image.jpg'
      canvas.toBlob(
        (blob) =>
          resolve(
            new File([blob!], fileName, {
              type: 'image/jpeg',
            })
          ),
        'image/jpeg',
        quality
      )
    }
    img.onerror = () => {
      URL.revokeObjectURL(img.src)
      resolve(file)
    }
    img.src = URL.createObjectURL(file)
  })
}

/** 从任意值里取第一个非空字符串；与旧代码的 `a || b || c` 链对字符串等价。 */
function firstString(...values: unknown[]): string | undefined {
  for (const value of values) {
    if (typeof value === 'string' && value) return value
  }
  return undefined
}

/** 外部错误对象只能逐层收窄——它可能是 axios 错误、ApiRequestError 或别的形状。 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

export function normalizeUploadError(error: unknown, maxMb: number): string {
  // 组件里 catch 到的是新 client 的 ApiRequestError；转成下面按 axios 错误形状写的读法
  if (error instanceof ApiRequestError) {
    const body = error.body
    return normalizeUploadError(
      {
        message: typeof body === 'string' && body ? body : error.message,
        response: { status: error.status, data: body },
      },
      maxMb
    )
  }
  const response = isRecord(error) ? error.response : undefined
  const data = isRecord(response) ? response.data : undefined
  const rawMessage =
    firstString(
      isRecord(data) ? data.error : undefined,
      isRecord(data) ? data.message : undefined,
      isRecord(error) ? error.message : undefined
    ) || '上传失败，请稍后重试。'

  if (
    rawMessage.includes('multipart/form-data') ||
    rawMessage.includes('Error parsing') ||
    (isRecord(response) && response.status === 413)
  ) {
    return `上传失败：文件可能超过当前限制，请将文件控制在 ${maxMb}MB 以内后重试。`
  }

  return rawMessage
}
