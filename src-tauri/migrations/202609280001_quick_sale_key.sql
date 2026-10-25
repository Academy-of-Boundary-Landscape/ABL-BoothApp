-- 快速售出允许网络超时后用同一个请求编号安全重试。
-- 已发布迁移不可修改，只在订单表上追加可空列；普通订单不受影响。
ALTER TABLE orders ADD COLUMN quick_sale_key TEXT;
CREATE UNIQUE INDEX idx_orders_quick_sale_key ON orders(quick_sale_key)
    WHERE quick_sale_key IS NOT NULL;
