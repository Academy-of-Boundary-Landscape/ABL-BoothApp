-- 收摊闭环：记下「盘过了」和「钱清点过了」这两个事实。
--
-- 为什么不能靠反查 journal 的存在性：**零差异的清点写不出 journal**。
-- post_journal 拒绝空腿（②-1 的原话：记空 journal 会造出「造得出却冲不掉」
-- 的幽灵 journal），而零金额的腿本来就会被静默滤掉。于是
--   「盘了一遍，全对」 vs 「根本没盘」
--   「数了现金盒，分文不差」 vs 「还没数」
-- 两组都会长得一模一样——而结算单上「未盘点，剩余数为账面推算」和
-- 「微信 1,540 ✓」正好要分开它们。
--
-- 两列都顺带供结算单打印时间（「盘点于 10-01 18:23」）。
ALTER TABLE events ADD COLUMN stocktaken_at DATETIME;
ALTER TABLE events ADD COLUMN reconciled_at DATETIME;
