-- 点单页条码扫描：给全局商品库加「商业条码」（JAN/EAN-13、ISBN 等）。
--
-- 与 product_code 互补：有商业条码的商品（官方周边、商业本）按 barcode 找；
-- 没有的（同人本、吧唧等手贴标签）仍按 product_code 找（SKU 即条码）。
--
-- 非唯一索引是刻意的：同一个 JAN 对应多件商品不常见但存在（同一印刷品的不同版本），
-- 命中多件时让摊主在界面上挑，不在录入时报错。
ALTER TABLE master_products ADD COLUMN barcode TEXT;
CREATE INDEX IF NOT EXISTS idx_master_products_barcode ON master_products(barcode);
