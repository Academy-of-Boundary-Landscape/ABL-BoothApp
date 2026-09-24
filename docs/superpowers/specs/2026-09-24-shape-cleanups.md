# ③b 形状清理清单

阶段 2 各模块迁移时发现的「想改但不许改」的 JSON 形状问题（spec K3）。Task 8 整理、Task 10 统一处理。

| 模块 | 路由 | 现状 | 建议 | 理由 | 前端消费方 |
|---|---|---|---|---|---|
| order | GET `/events/{event_id}/orders` | 内存分组后 `items_map.remove(&oid).unwrap_or_default()` / `lots_map.remove(&oid).unwrap_or_default()` 把「本应存在的 items/lots 行不在 map 里」静默变成空数组 | 分组后断言没有剩余的未消费 map 项，或在缺失时落日志 | 响应形状本身正确，但会掩盖 JOIN/分组键漂移导致的账目错位（订单显示成 0 件） | `frontend/src/stores/orderStore.js`、`frontend/src/views/AdminEventOrders.vue` |
| order | POST/GET/PUT `/events/{event_id}/orders` | `OrderItemResponse.product_id` 实际存的是 `event_product_id` | 改名为 `event_product_id` | 字段名与值指向不同实体，容易被当成全局 `master_products.id` 使用 | 这五个消费方目前都没读它；改名仍是形状变更 |
| order | POST/GET/PUT `/events/{event_id}/orders` | 响应把 `OrderRow`（DB 行）`#[serde(flatten)]` 摊平到顶层，与 `items`/`lots` 同级 | 收进 `order: { ... }` 子对象 | API 形状与 `SELECT *` 的列强耦合，`OrderRow` 加一列就自动变成公开契约 | 全部五个消费方都读顶层 `final_amount`/`gross_amount`/`solved_amount`/`timestamp`/`channel` |
