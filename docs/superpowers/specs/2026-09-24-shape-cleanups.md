# ③b 形状清理清单

阶段 2 各模块迁移时发现的「想改但不许改」的 JSON 形状问题（spec K3）。Task 8 整理、Task 10 统一处理。

| 模块 | 路由 | 现状 | 建议 | 理由 | 前端消费方 |
|---|---|---|---|---|---|
| lot | GET /events/{event_id}/lots | 候选被级联删空后，LEFT JOIN 的 NULL 被 `owner.unwrap_or(0)` / `owner_name.unwrap_or_else(...)` 吞掉，`owner_society_id` 输出哨兵 `0`、`owner_society_name` 输出 `"（候选已被删除）"` | 两个字段改 `Option<...>`，无候选时输出 `null`，显示文案交给前端 | 库里不存在 0 号社团；哨兵 `0` 让「没有货主」和「货主 id 恰好是 0」在 JSON 上无法区分，且字符串文案是后端替前端做的显示决定 | frontend/src/views/AdminEventLots.vue（`lot.owner_society_name` 表格列） |
