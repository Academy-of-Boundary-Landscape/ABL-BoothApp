-- ② 领域模型：库存与资金的复式账
--
-- 这是一次性的大爆炸切换，不是增量加列。依据是路线图 D1「历史数据知情清零」：
-- 老用户的展会、订单、order_items、当前库存**不迁移**到新模型。
--
-- 保留什么、丢什么：
--   保留 settings（管理员/摊主密码）、master_products（全局商品库，加一列归属）、
--        master_product_images / image_embeddings / vision_index_meta（AI 识别资产）
--   丢弃 events 的行（表结构保留并加列）、products、orders、order_items
--
-- 用户侧的兜底由 src-tauri/src/db/mod.rs 的一次性 v1 备份负责（sale_system.db.v1-backup），
-- 外加 ① 建好的迁移前快照 + 失败回滚（db/snapshot.rs）。

-- ========== 社团：货主的单位 ==========
-- 账户名和外键都用 id 而不是名字，因为社团会改名（spec 3.1）。
CREATE TABLE societies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    is_home INTEGER NOT NULL DEFAULT 0 CHECK (is_home IN (0, 1)),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 「本社团」有且只有一个。偏索引是关键：普通 UNIQUE 会让第二个 is_home=0 也插不进去。
CREATE UNIQUE INDEX idx_societies_home ON societies(is_home) WHERE is_home = 1;

INSERT INTO societies (id, name, is_home) VALUES (1, '本社团', 1);

-- ========== 商品库加归属（默认值，选品时会被快照到 event_products）==========
-- 故意不写 REFERENCES societies(id)：SQLite 在 foreign_keys=ON 下拒绝
-- 「带 REFERENCES 且默认值非 NULL」的 ADD COLUMN（实测报错）。改成可空 + UPDATE
-- 又会让 NOT NULL 语义丢失，权衡之后这一列不做 FK 约束，由应用层保证。
ALTER TABLE master_products ADD COLUMN owner_society_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX idx_master_products_owner ON master_products(owner_society_id);

-- ========== 丢弃旧业务表（子表在前，否则 FK 拦截）==========
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;
DELETE FROM events;

-- ========== 展会 ==========
-- 状态值从 未进行/进行中/已结束 改成 筹备/进行中/已结算（spec 6.4），
-- events 的行刚被清空，所以不需要数据转换。
--
-- 注意：**默认值没有改、也不在这里改**。`events.status` 的 DEFAULT 至今仍是
-- '未进行'，而 SQLite 改不了列默认值（要整表重建，收益不值这个风险）。
-- 新模型的正确状态靠 handler 显式写入——全仓 3 处 INSERT 都显式给了 status，
-- `create_event` 写的是 '筹备'。
ALTER TABLE events ADD COLUMN stocktake_skipped INTEGER NOT NULL DEFAULT 0
    CHECK (stocktake_skipped IN (0, 1));

-- ========== 摊位商品 ==========
-- 没有 current_stock —— 可售余额是 SUM(到现场仓) − SUM(离现场仓)，
-- 不存在第二个可以漂移的数字。这是整个设计最直接的体现。
-- owner_society_id 是选品时从 master_products 抄来的**快照**：否则展会结算完
-- 之后有人改了全局商品库的归属，冻结的账就跟着变。
CREATE TABLE event_products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    master_product_id INTEGER NOT NULL,
    owner_society_id INTEGER NOT NULL,
    product_code TEXT NOT NULL,          -- 冗余快照，防商品库改名后账面混乱
    name TEXT NOT NULL,                  -- 同上
    unit_price INTEGER NOT NULL CHECK (unit_price >= 0),  -- 分
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
    FOREIGN KEY (master_product_id) REFERENCES master_products(id),
    FOREIGN KEY (owner_society_id) REFERENCES societies(id),
    UNIQUE (event_id, master_product_id)
);
CREATE INDEX idx_event_products_event ON event_products(event_id);

-- ========== Lot（②-2 才写数据，表先建好）==========
-- 候选集必须同一货主，由应用层在配置时校验（spec 4.1）；SQL 层表达不了。
CREATE TABLE lots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    pick_count INTEGER NOT NULL CHECK (pick_count > 0),
    total_price INTEGER NOT NULL CHECK (total_price >= 0),
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
);
CREATE TABLE lot_candidates (
    lot_id INTEGER NOT NULL,
    event_product_id INTEGER NOT NULL,
    PRIMARY KEY (lot_id, event_product_id),
    FOREIGN KEY (lot_id) REFERENCES lots(id) ON DELETE CASCADE,
    FOREIGN KEY (event_product_id) REFERENCES event_products(id) ON DELETE CASCADE
);

-- ========== 订单 ==========
-- 没有 journal_id —— 反过来，journals.order_id 指向订单。一个订单从下单到退货
-- 有多个 journal（下单记货、完成记钱、取消冲正），单向外键放订单上装不下。
CREATE TABLE orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'completed', 'cancelled')),
    channel TEXT,                                   -- 收款渠道，完成时才有
    gross_amount INTEGER NOT NULL CHECK (gross_amount >= 0),    -- 原价合计（分）
    solved_amount INTEGER NOT NULL CHECK (solved_amount >= 0),  -- 求解器价；②-1 恒等于 gross
    final_amount INTEGER NOT NULL CHECK (final_amount >= 0),    -- 手工覆盖后；②-1 恒等于 solved
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
    -- spec 6.2：结算时必须录收款渠道，否则钱那条腿没有对手账户。
    -- 放在 DB 层是因为这条不变量一旦漏掉，账面要到收摊对账才发现。
    CHECK (status <> 'completed' OR channel IS NOT NULL)
);
CREATE INDEX idx_orders_event_status ON orders(event_id, status);

-- Lot 在一单里的**一次**套用。同一个 Lot 套两次 = 两行，各自分摊。
-- lot_id 可空且 ON DELETE SET NULL：Lot 配置被删掉不该让历史订单跟着没。
CREATE TABLE order_lots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    lot_id INTEGER,
    name TEXT NOT NULL,                             -- Lot 名字的快照
    price INTEGER NOT NULL CHECK (price >= 0),      -- Lot 总价的快照
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (lot_id) REFERENCES lots(id) ON DELETE SET NULL
);
CREATE INDEX idx_order_lots_order ON order_lots(order_id);

-- 订单行。粒度是「商品 × Lot 实例」：同一商品 5 件里 3 件进 Lot、2 件原价 ⇒ 两行。
-- 两个金额的分工见 spec 4.5：
--   allocated_amount = 这一行的**货主应得**（Lot 分摊后、手工折让前）
--   paid_amount      = **顾客为这一行实付**（手工折让摊入后）
-- ②-1 里没有 Lot 也没有折让，两者恒等于 unit_price × qty；②-2 才会让它们分道。
CREATE TABLE order_lines (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    event_product_id INTEGER NOT NULL,
    order_lot_id INTEGER,
    qty INTEGER NOT NULL CHECK (qty > 0),
    unit_price INTEGER NOT NULL CHECK (unit_price >= 0),
    allocated_amount INTEGER NOT NULL CHECK (allocated_amount >= 0),
    paid_amount INTEGER NOT NULL CHECK (paid_amount >= 0),
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (event_product_id) REFERENCES event_products(id),
    FOREIGN KEY (order_lot_id) REFERENCES order_lots(id) ON DELETE SET NULL
);
CREATE INDEX idx_order_lines_order ON order_lines(order_id);

-- ========== 账本：一次业务操作 ==========
CREATE TABLE journals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    kind TEXT NOT NULL,   -- 进货|销售|收款|退货|取消|赠送|报废|盘点|带回|拆封|垫付|调整
    occurred_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    order_id INTEGER,                    -- 非空 = 属于某个订单
    reverses_journal_id INTEGER,         -- 冲正时指向原 journal
    note TEXT,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (reverses_journal_id) REFERENCES journals(id)
);
CREATE INDEX idx_journals_event ON journals(event_id, kind);
CREATE INDEX idx_journals_order ON journals(order_id);

-- 一个 journal 只能被冲正一次。没有这条约束，一次网络重试就能把库存退两遍。
CREATE UNIQUE INDEX idx_journals_reverses
    ON journals(reverses_journal_id) WHERE reverses_journal_id IS NOT NULL;

-- ========== 货的移动：from/to 均非空 ⇒ 结构上不可能不平衡 ==========
CREATE TABLE stock_movements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    journal_id INTEGER NOT NULL,
    event_product_id INTEGER NOT NULL,
    from_location TEXT NOT NULL,
    to_location TEXT NOT NULL,
    qty INTEGER NOT NULL CHECK (qty > 0),
    FOREIGN KEY (journal_id) REFERENCES journals(id) ON DELETE CASCADE,
    FOREIGN KEY (event_product_id) REFERENCES event_products(id),
    CHECK (from_location <> to_location)
);
CREATE INDEX idx_stock_movements_product ON stock_movements(event_product_id);
CREATE INDEX idx_stock_movements_journal ON stock_movements(journal_id);

-- ========== 钱的移动：同上 ==========
-- 账户是字符串：'摊主自有' | '实收-<渠道>' | '社团往来:<society_id>' | '结算调整' | '对账差异'
-- 不做外键，因为 '实收-微信'、'摊主自有' 这些不挂任何社团。
CREATE TABLE money_movements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    journal_id INTEGER NOT NULL,
    from_account TEXT NOT NULL,
    to_account TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK (amount > 0),
    FOREIGN KEY (journal_id) REFERENCES journals(id) ON DELETE CASCADE,
    CHECK (from_account <> to_account)
);
CREATE INDEX idx_money_movements_journal ON money_movements(journal_id);
CREATE INDEX idx_money_movements_from ON money_movements(from_account);
CREATE INDEX idx_money_movements_to ON money_movements(to_account);

-- ========== 退货（②-3 才写数据）==========
CREATE TABLE refunds (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_line_id INTEGER NOT NULL,
    journal_id INTEGER NOT NULL,
    qty INTEGER NOT NULL CHECK (qty > 0),
    allocated_amount INTEGER NOT NULL CHECK (allocated_amount >= 0),
    paid_amount INTEGER NOT NULL CHECK (paid_amount >= 0),
    refund_amount INTEGER NOT NULL CHECK (refund_amount >= 0),
    channel TEXT NOT NULL,
    destination TEXT NOT NULL,           -- 现场仓|损耗
    FOREIGN KEY (order_line_id) REFERENCES order_lines(id) ON DELETE CASCADE,
    FOREIGN KEY (journal_id) REFERENCES journals(id) ON DELETE CASCADE,
    -- spec 6.3：退多于实付不允许，那是白送钱，走结算调整
    CHECK (refund_amount <= paid_amount)
);
CREATE INDEX idx_refunds_line ON refunds(order_line_id);

-- ========== 垫付 / 结算调整（②-3 才写数据）==========
-- 两张表都带 journal_id：结算单 = 往来账户的余额（spec 5.2），
-- 前提是所有影响往来的东西都在账本里。
CREATE TABLE advances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    owner_society_id INTEGER NOT NULL,
    journal_id INTEGER NOT NULL,
    label TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK (amount > 0),
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
    FOREIGN KEY (owner_society_id) REFERENCES societies(id),
    FOREIGN KEY (journal_id) REFERENCES journals(id) ON DELETE CASCADE
);
CREATE INDEX idx_advances_event ON advances(event_id);

CREATE TABLE settlement_adjustments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    owner_society_id INTEGER NOT NULL,
    journal_id INTEGER NOT NULL,
    label TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK (amount <> 0),   -- 可正可负，但不能是 0
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
    FOREIGN KEY (owner_society_id) REFERENCES societies(id),
    FOREIGN KEY (journal_id) REFERENCES journals(id) ON DELETE CASCADE
);
CREATE INDEX idx_settlement_adjustments_event ON settlement_adjustments(event_id);

-- ========== 外键列索引 ==========
-- 没有这些索引时，级联删除要对子表做全表扫描（已用 EXPLAIN QUERY PLAN 实测确认）。
-- journals 是增长最快的表（每次业务操作一条），删展会的成本会随数据积累明显上涨。
CREATE INDEX idx_lots_event ON lots(event_id);
CREATE INDEX idx_lot_candidates_product ON lot_candidates(event_product_id);
CREATE INDEX idx_order_lots_lot ON order_lots(lot_id);
CREATE INDEX idx_order_lines_product ON order_lines(event_product_id);
CREATE INDEX idx_order_lines_lot ON order_lines(order_lot_id);
CREATE INDEX idx_refunds_journal ON refunds(journal_id);
CREATE INDEX idx_advances_journal ON advances(journal_id);
CREATE INDEX idx_settlement_adjustments_journal ON settlement_adjustments(journal_id);
