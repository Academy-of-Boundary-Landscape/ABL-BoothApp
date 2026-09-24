import { describe, it, expect } from 'vitest'
import {
  formatTimestamp,
  formatDate,
  formatTime,
  formatChartLabel,
  formatChartTooltip,
} from './dateFormatter'

describe('formatTimestamp', () => {
  it('把无时区标记的时间戳当作 UTC，转成 UTC+8', () => {
    expect(formatTimestamp('2024-01-19 08:30:00')).toBe('2024/01/19 16:30:00')
  })

  it('跨日转换正确', () => {
    expect(formatTimestamp('2024-01-19 16:00:00Z')).toBe('2024/01/20 00:00:00')
  })

  it('接受 ISO 格式', () => {
    expect(formatTimestamp('2024-01-19T08:30:00')).toBe('2024/01/19 16:30:00')
  })

  it('showSeconds=false 时不带秒', () => {
    expect(formatTimestamp('2024-01-19 08:30:00', false)).toBe('2024/01/19 16:30')
  })

  it('空值返回短横线', () => {
    expect(formatTimestamp('')).toBe('-')
    expect(formatTimestamp(null)).toBe('-')
    expect(formatTimestamp(undefined)).toBe('-')
  })

  it('无法解析时原样返回输入', () => {
    expect(formatTimestamp('not-a-date')).toBe('not-a-date')
  })
})

describe('formatDate / formatTime', () => {
  it('formatDate 只给日期', () => {
    expect(formatDate('2024-01-19 08:30:00')).toBe('2024/01/19')
  })

  it('formatTime 只给时间', () => {
    expect(formatTime('2024-01-19 08:30:00')).toBe('16:30:00')
  })

  it('都对空值返回短横线', () => {
    expect(formatDate(null)).toBe('-')
    expect(formatTime(null)).toBe('-')
  })
})

describe('图表格式化', () => {
  it('formatChartLabel 不含年和秒', () => {
    expect(formatChartLabel('2024-01-19 08:30:00')).toBe('01/19 16:30')
  })

  it('formatChartTooltip 含年不含秒', () => {
    expect(formatChartTooltip('2024-01-19 08:30:00')).toBe('2024/01/19 16:30')
  })
})
