-- 补充缺失的关键索引，减少查询扫描时间和锁持有时间

-- order_items 按 order_id 查询极频繁（订单详情、统计、取消退库存）
-- 之前没有索引，每次都全表扫描
CREATE INDEX IF NOT EXISTS idx_order_items_order_id
ON order_items(order_id);

-- orders 按 event_id + status 组合查询极频繁（仪表盘统计、订单列表、图表）
CREATE INDEX IF NOT EXISTS idx_orders_event_status
ON orders(event_id, status);
