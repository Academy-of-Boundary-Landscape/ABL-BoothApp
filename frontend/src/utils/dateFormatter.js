/**
 * 时间格式化工具
 * 确保所有时间都正确显示为 UTC+8 (中国标准时间)
 */

/**
 * 将数据库返回的时间戳转换为 UTC+8 时间字符串
 * @param {string} timestamp - 数据库返回的时间戳（假设为 UTC 时间）
 * @param {boolean} showSeconds - 是否显示秒
 * @returns {string} 格式化后的时间字符串
 */
export function formatTimestamp(timestamp, showSeconds = true) {
  if (!timestamp) return '-'

  // 创建 Date 对象，SQLite 返回的是 UTC 时间字符串
  // 需要添加 'Z' 来明确告诉 JavaScript 这是 UTC 时间
  const dateStr = timestamp.includes('Z') ? timestamp : timestamp + 'Z'
  const date = new Date(dateStr)

  if (isNaN(date.getTime())) {
    console.warn('Invalid timestamp:', timestamp)
    return timestamp
  }

  // 使用 toLocaleString 转换为北京时间
  const options = {
    timeZone: 'Asia/Shanghai',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }

  if (showSeconds) {
    options.second = '2-digit'
  }

  return date.toLocaleString('zh-CN', options)
}

/**
 * 简化版本：只显示日期
 */
export function formatDate(timestamp) {
  if (!timestamp) return '-'

  const dateStr = timestamp.includes('Z') ? timestamp : timestamp + 'Z'
  const date = new Date(dateStr)

  if (isNaN(date.getTime())) {
    return timestamp
  }

  return date.toLocaleString('zh-CN', {
    timeZone: 'Asia/Shanghai',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  })
}

/**
 * 简化版本：只显示时间
 */
export function formatTime(timestamp) {
  if (!timestamp) return '-'

  const dateStr = timestamp.includes('Z') ? timestamp : timestamp + 'Z'
  const date = new Date(dateStr)

  if (isNaN(date.getTime())) {
    return timestamp
  }

  return date.toLocaleString('zh-CN', {
    timeZone: 'Asia/Shanghai',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  })
}

/**
 * 图表专用：显示简短的日期时间（不含秒）
 */
export function formatChartLabel(timestamp) {
  if (!timestamp) return '-'

  const dateStr = timestamp.includes('Z') ? timestamp : timestamp + 'Z'
  const date = new Date(dateStr)

  if (isNaN(date.getTime())) {
    return timestamp
  }

  return date.toLocaleString('zh-CN', {
    timeZone: 'Asia/Shanghai',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  })
}

/**
 * 图表工具提示专用：显示完整的日期时间
 */
export function formatChartTooltip(timestamp) {
  if (!timestamp) return '-'

  const dateStr = timestamp.includes('Z') ? timestamp : timestamp + 'Z'
  const date = new Date(dateStr)

  if (isNaN(date.getTime())) {
    return timestamp
  }

  return date.toLocaleString('zh-CN', {
    timeZone: 'Asia/Shanghai',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  })
}
