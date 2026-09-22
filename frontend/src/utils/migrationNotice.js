// src/utils/migrationNotice.js

/**
 * 首启迁移提示是否已经展示过一次的标记。
 * 存在 localStorage（按设备）而不是 sessionStorage：提示只该打扰一次。
 */
export const MIGRATION_NOTICE_SEEN_KEY = 'migration_notice_seen'

/**
 * 是否应该弹出 v1 历史数据迁移提示。
 *
 * 抽成纯函数，副作用（读 localStorage）留在调用方 —— 这样测试不需要组件框架，
 * 和 money.spec.js 一样是纯逻辑测试。
 *
 * 条件：没标记看过 && 确实有备份 && 备份里有展会或订单。
 * 只有备份、没有业务数据（用户升级前就没用过）时不打扰。
 *
 * @param {{has_backup?: boolean, event_count?: number, order_count?: number} | null | undefined} status
 * @param {boolean} seen
 * @returns {boolean}
 */
export function shouldShowMigrationNotice(status, seen) {
  if (seen) return false
  if (!status || !status.has_backup) return false
  const eventCount = Number(status.event_count) || 0
  const orderCount = Number(status.order_count) || 0
  return eventCount > 0 || orderCount > 0
}
