import { describe, it, expect } from 'vitest'
import { shouldShowMigrationNotice } from './migrationNotice.js'

describe('shouldShowMigrationNotice', () => {
  const withData = { has_backup: true, event_count: 2, order_count: 3, item_count: 4 }

  it('shows once when there is a backup with events', () => {
    expect(shouldShowMigrationNotice({ ...withData, order_count: 0 }, false)).toBe(true)
  })

  it('shows when there is a backup with orders but no events', () => {
    // 理论上不该出现（订单必属展会），但判定函数不该因此漏弹。
    expect(shouldShowMigrationNotice({ ...withData, event_count: 0 }, false)).toBe(true)
  })

  it('never shows after it has been seen', () => {
    expect(shouldShowMigrationNotice(withData, true)).toBe(false)
  })

  it('does not show when there is no backup', () => {
    expect(
      shouldShowMigrationNotice({ has_backup: false, event_count: 0, order_count: 0 }, false)
    ).toBe(false)
  })

  it('does not disturb a clean install with a backup but no business data', () => {
    expect(
      shouldShowMigrationNotice({ has_backup: true, event_count: 0, order_count: 0 }, false)
    ).toBe(false)
  })

  it('survives a missing or malformed status response', () => {
    expect(shouldShowMigrationNotice(null, false)).toBe(false)
    expect(shouldShowMigrationNotice(undefined, false)).toBe(false)
    expect(shouldShowMigrationNotice({ has_backup: true }, false)).toBe(false)
  })
})
