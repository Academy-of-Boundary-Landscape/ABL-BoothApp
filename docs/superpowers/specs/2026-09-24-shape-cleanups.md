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
