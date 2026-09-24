# ③b 形状清理清单

阶段 2 各模块迁移时发现的「想改但不许改」的 JSON 形状问题（spec K3）。Task 8 整理、Task 10 统一处理。

| 模块 | 路由 | 现状 | 建议 | 理由 | 前端消费方 |
|---|---|---|---|---|---|
| inventory | `POST /events/{event_id}/scraps` | 请求体沿用 `LogRequest`，其中的 `vendor_pays` 被静默置为 false（不报错） | 报废单独定义一个不含 `vendor_pays` 的请求类型 | 该字段对本接口无意义，但 OpenAPI 把它列成可填，客户端会误以为生效 | `frontend/src/components/vendor/InventoryLogModal.vue`（报废时本就不发该字段） |
