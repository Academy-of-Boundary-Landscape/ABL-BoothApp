# ③b 形状清理清单

阶段 2 各模块迁移时发现的「想改但不许改」的 JSON 形状问题（spec K3）。Task 8 整理、Task 10 统一处理。

| 模块 | 路由 | 现状 | 建议 | 理由 | 前端消费方 |
|---|---|---|---|---|---|
| refund | `POST /events/{event_id}/orders/{order_id}/refunds` | 201 响应带 `allocated_total`、`paid_total` 两个内部账务合计 | 契约里可只留 `journal_id` 与 `refund_amount` | 前端只消费 `refund_amount`，另两个是内部量，固化进契约会让实现细节外泄 | `frontend/src/components/vendor/RefundModal.vue`（只读 `data.refund_amount`） |
| refund | `GET /events/{event_id}/orders/{order_id}/refunds` | `RefundableLine` 的 `event_product_id`、`product_code`、`refunded_qty`、`remaining_allocated`，以及 `RefundHistoryRow` 的 `order_line_id`、`product_code` 前端从不读取 | 可考虑精简未消费字段 | 响应面比消费方需要的宽，字段容易在后续重构中漂移 | `frontend/src/components/vendor/RefundModal.vue`、`frontend/src/utils/refund.js`（只用 `order_line_id`/`remaining_qty`/`remaining_paid` 等） |
