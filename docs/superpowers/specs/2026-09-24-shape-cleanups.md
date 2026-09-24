# ③b 形状清理清单

阶段 2 各模块迁移时发现的「想改但不许改」的 JSON 形状问题（spec K3）。Task 8 整理、Task 10 统一处理。

| 模块 | 路由 | 现状 | 建议 | 理由 | 前端消费方 |
|---|---|---|---|---|---|
| closing | GET /events/{id}/closing | `status` 是中文自由字符串（进行中/已结算） | 改成稳定枚举值（如 `open`/`settled`） | 前端用 `s.status === '已结算'` 比较原文，文案一改契约就断；字符串没有类型约束 | ClosingWizard.vue（`step` computed） |
| closing | POST /events/{id}/closing/stocktake | `journal_id: Option<i64>`，`null` 兼表「零差异、没写 journal」 | 加 `wrote_journal: bool`，或让 `journal_id` 只在真写了时出现 | `null` 同时是「没有 id」和「没写 journal」两种意思，消费方只能靠 `diffs` 反推 | 无（closingStore.stocktake 丢弃响应体） |
| closing | POST /events/{id}/closing/takeback | `journal_id: Option<i64>`，空摊 no-op 时为 `null` | 同 stocktake，加 `wrote_journal: bool` | 同 stocktake：`null` 语义重载 | 无（closingStore.takeback 丢弃响应体） |
| closing | POST /events/{id}/closing/settle | 成功返回 `200 {"status": "已结算"}`，该字段恒为常量 | 改成 `204 No Content` | 前端在成功后重新拉 `GET /closing`，常量 body 不携带信息 | 无（closingStore.settle 丢弃响应体） |
| closing | POST /events/{id}/closing/stocktake | `book.get(&event_product_id).unwrap_or(&0)` 把「账面查不到该商品」当余额 0 | 显式判缺失并返回 400 | 目前靠后面 `event_products` 的名字查询兜底，但默认 0 会掩盖 `book_balances` 漏项；新调用路径可能绕过 | 无 |

| inventory | `POST /events/{event_id}/scraps` | 请求体沿用 `LogRequest`，其中的 `vendor_pays` 被静默置为 false（不报错） | 报废单独定义一个不含 `vendor_pays` 的请求类型 | 该字段对本接口无意义，但 OpenAPI 把它列成可填，客户端会误以为生效 | `frontend/src/components/vendor/InventoryLogModal.vue`（报废时本就不发该字段） |
| lot | GET /events/{event_id}/lots | 候选被级联删空后，LEFT JOIN 的 NULL 被 `owner.unwrap_or(0)` / `owner_name.unwrap_or_else(...)` 吞掉，`owner_society_id` 输出哨兵 `0`、`owner_society_name` 输出 `"（候选已被删除）"` | 两个字段改 `Option<...>`，无候选时输出 `null`，显示文案交给前端 | 库里不存在 0 号社团；哨兵 `0` 让「没有货主」和「货主 id 恰好是 0」在 JSON 上无法区分，且字符串文案是后端替前端做的显示决定 | frontend/src/views/AdminEventLots.vue（`lot.owner_society_name` 表格列） |
| order | GET `/events/{event_id}/orders` | 内存分组后 `items_map.remove(&oid).unwrap_or_default()` / `lots_map.remove(&oid).unwrap_or_default()` 把「本应存在的 items/lots 行不在 map 里」静默变成空数组 | 分组后断言没有剩余的未消费 map 项，或在缺失时落日志 | 响应形状本身正确，但会掩盖 JOIN/分组键漂移导致的账目错位（订单显示成 0 件） | `frontend/src/stores/orderStore.js`、`frontend/src/views/AdminEventOrders.vue` |
| order | POST/GET/PUT `/events/{event_id}/orders` | `OrderItemResponse.product_id` 实际存的是 `event_product_id` | 改名为 `event_product_id` | 字段名与值指向不同实体，容易被当成全局 `master_products.id` 使用 | 这五个消费方目前都没读它；改名仍是形状变更 |
| order | POST/GET/PUT `/events/{event_id}/orders` | 响应把 `OrderRow`（DB 行）`#[serde(flatten)]` 摊平到顶层，与 `items`/`lots` 同级 | 收进 `order: { ... }` 子对象 | API 形状与 `SELECT *` 的列强耦合，`OrderRow` 加一列就自动变成公开契约 | 全部五个消费方都读顶层 `final_amount`/`gross_amount`/`solved_amount`/`timestamp`/`channel` |
| refund | `POST /events/{event_id}/orders/{order_id}/refunds` | 201 响应带 `allocated_total`、`paid_total` 两个内部账务合计 | 契约里可只留 `journal_id` 与 `refund_amount` | 前端只消费 `refund_amount`，另两个是内部量，固化进契约会让实现细节外泄 | `frontend/src/components/vendor/RefundModal.vue`（只读 `data.refund_amount`） |
| refund | `GET /events/{event_id}/orders/{order_id}/refunds` | `RefundableLine` 的 `event_product_id`、`product_code`、`refunded_qty`、`remaining_allocated`，以及 `RefundHistoryRow` 的 `order_line_id`、`product_code` 前端从不读取 | 可考虑精简未消费字段 | 响应面比消费方需要的宽，字段容易在后续重构中漂移 | `frontend/src/components/vendor/RefundModal.vue`、`frontend/src/utils/refund.js`（只用 `order_line_id`/`remaining_qty`/`remaining_paid` 等） |
