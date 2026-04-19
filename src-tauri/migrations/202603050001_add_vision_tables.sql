CREATE TABLE IF NOT EXISTS master_product_images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    master_product_id INTEGER NOT NULL,
    image_url TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'gallery',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (master_product_id) REFERENCES master_products(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_mp_images_product_id
ON master_product_images(master_product_id);

CREATE TABLE IF NOT EXISTS image_embeddings (
    image_id INTEGER NOT NULL,
    model_version TEXT NOT NULL,
    dim INTEGER NOT NULL,
    vector BLOB NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (image_id, model_version),
    FOREIGN KEY (image_id) REFERENCES master_product_images(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS vision_index_meta (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    model_version TEXT NOT NULL,
    index_version INTEGER NOT NULL DEFAULT 1,
    built_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
