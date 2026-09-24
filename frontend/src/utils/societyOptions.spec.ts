import { describe, it, expect } from 'vitest'
import { societyOptions } from './societyOptions'

describe('societyOptions', () => {
  it('本社团排第一并标注，其余按原顺序', () => {
    const opts = societyOptions([
      { id: 2, name: '黄昏堂', is_home: false },
      { id: 1, name: '本社团', is_home: true },
      { id: 3, name: '白夜', is_home: false },
    ])
    expect(opts).toEqual([
      { label: '本社团（本社团）', value: 1 },
      { label: '黄昏堂', value: 2 },
      { label: '白夜', value: 3 },
    ])
  })

  it('空列表给空选项', () => {
    expect(societyOptions([])).toEqual([])
  })
})
