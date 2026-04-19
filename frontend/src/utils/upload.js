import { createDiscreteApi } from 'naive-ui'

export const IMAGE_UPLOAD_LIMIT_MB = 10
export const IMAGE_WARN_THRESHOLD_MB = 3
export const SYNC_IMPORT_LIMIT_MB = 1000

const { dialog } = createDiscreteApi(['dialog'])

export function bytesFromMb(mb) {
  return mb * 1024 * 1024
}

export function validateFileSize(file, maxMb) {
  if (!file) return { ok: false, message: '未选择文件。' }
  if (file.size <= bytesFromMb(maxMb)) return { ok: true }
  return {
    ok: false,
    message: `文件过大，请选择不超过 ${maxMb}MB 的文件。`,
  }
}

export function showUploadDialog(title, content) {
  dialog.warning({
    title,
    content,
    positiveText: '知道了',
  })
}

/**
 * 大文件确认对话框，返回 Promise<boolean>
 */
export function confirmLargeFile(fileSizeMb) {
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
 * 将图片强制缩放到指定最大边长，返回压缩后的 File
 * @param {File|Blob} file
 * @param {number} maxSize - 最大边长（像素）
 * @param {number} quality - JPEG 质量 0-1
 * @returns {Promise<File>}
 */
/**
 * 将图片强制拉伸到 size×size 正方形，返回压缩后的 File
 * 用于 AI 识别图等不需要保持宽高比的场景
 * @param {File|Blob} file
 * @param {number} size - 输出正方形边长（像素）
 * @param {number} quality - JPEG 质量 0-1
 * @returns {Promise<File>}
 */
export function resizeImageFile(file, size = 512, quality = 0.90) {
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
      canvas.getContext('2d').drawImage(img, 0, 0, size, size)
      URL.revokeObjectURL(img.src)
      canvas.toBlob(
        (blob) => resolve(new File([blob], file.name || 'image.jpg', { type: 'image/jpeg' })),
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

export function normalizeUploadError(error, maxMb) {
  const rawMessage =
    error?.response?.data?.error ||
    error?.response?.data?.message ||
    error?.message ||
    '上传失败，请稍后重试。'

  if (
    rawMessage.includes('multipart/form-data') ||
    rawMessage.includes('Error parsing') ||
    error?.response?.status === 413
  ) {
    return `上传失败：文件可能超过当前限制，请将文件控制在 ${maxMb}MB 以内后重试。`
  }

  return rawMessage
}
