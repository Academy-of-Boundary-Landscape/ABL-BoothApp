-- 为 master_products 添加 tags 字段（逗号分隔的标签文本）
-- 与 category（商品类型分类）互补，用于角色/IP/色系等多维标签
ALTER TABLE master_products ADD COLUMN tags TEXT NOT NULL DEFAULT '';
