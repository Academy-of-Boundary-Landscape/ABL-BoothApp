use sqlx::{FromRow, SqlitePool};

use super::index::{decode_embedding_blob, EmbeddingCandidate};

#[derive(Debug, Clone, FromRow)]
pub struct VisionIndexMeta {
    pub model_version: String,
    pub index_version: i64,
    pub built_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct VisionStore {
    db: SqlitePool,
}

#[derive(Debug, Clone, FromRow)]
struct RawEmbeddingRow {
    master_product_id: i64,
    product_code: String,
    name: String,
    thumb_url: Option<String>,
    dim: i32,
    vector: Vec<u8>,
}

#[derive(Debug, Clone, FromRow)]
pub struct RebuildImageRow {
    pub image_id: i64,
    pub image_url: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct MasterProductImageRow {
    pub id: i64,
    pub master_product_id: i64,
    pub image_url: String,
    pub kind: String,
    pub created_at: String,
}

impl VisionStore {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn ensure_index_meta(&self, model_version: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO vision_index_meta (id, model_version, index_version)
            VALUES (1, ?, 1)
            ON CONFLICT(id) DO NOTHING
            "#,
        )
        .bind(model_version)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn get_index_meta(&self) -> Result<Option<VisionIndexMeta>, sqlx::Error> {
        let meta = sqlx::query_as::<_, VisionIndexMeta>(
            r#"
            SELECT model_version, index_version, built_at, updated_at
            FROM vision_index_meta
            WHERE id = 1
            "#,
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(meta)
    }

    pub async fn update_index_meta(
        &self,
        model_version: &str,
        index_version: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE vision_index_meta
            SET model_version = ?,
                index_version = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = 1
            "#,
        )
        .bind(model_version)
        .bind(index_version)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn bump_index_meta_for_model(&self, model_version: &str) -> Result<i64, sqlx::Error> {
        let current = self
            .get_index_meta()
            .await?
            .map(|item| item.index_version)
            .unwrap_or(0);
        let next = current + 1;

        sqlx::query(
            r#"
            INSERT INTO vision_index_meta (id, model_version, index_version)
            VALUES (1, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                model_version = excluded.model_version,
                index_version = excluded.index_version,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(model_version)
        .bind(next)
        .execute(&self.db)
        .await?;

        Ok(next)
    }

    pub async fn count_embeddings_by_model(&self, model_version: &str) -> Result<i64, sqlx::Error> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) as cnt FROM image_embeddings WHERE model_version = ?")
                .bind(model_version)
                .fetch_one(&self.db)
                .await?;

        Ok(result.0)
    }

    pub async fn master_product_exists(&self, master_product_id: i64) -> Result<bool, sqlx::Error> {
        let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM master_products WHERE id = ?")
            .bind(master_product_id)
            .fetch_optional(&self.db)
            .await?;

        Ok(exists.is_some())
    }

    pub async fn insert_master_product_image(
        &self,
        master_product_id: i64,
        image_url: &str,
        kind: &str,
    ) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO master_product_images (master_product_id, image_url, kind)
            VALUES (?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(master_product_id)
        .bind(image_url)
        .bind(kind)
        .fetch_one(&self.db)
        .await?;

        Ok(row.0)
    }

    pub async fn insert_master_product_image_if_absent(
        &self,
        master_product_id: i64,
        image_url: &str,
        kind: &str,
    ) -> Result<i64, sqlx::Error> {
        let existing: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT id
            FROM master_product_images
            WHERE master_product_id = ?
              AND image_url = ?
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .bind(master_product_id)
        .bind(image_url)
        .fetch_optional(&self.db)
        .await?;

        if let Some((id,)) = existing {
            return Ok(id);
        }

        self.insert_master_product_image(master_product_id, image_url, kind)
            .await
    }

    pub async fn list_master_product_images(
        &self,
        master_product_id: i64,
    ) -> Result<Vec<MasterProductImageRow>, sqlx::Error> {
        sqlx::query_as::<_, MasterProductImageRow>(
            r#"
            SELECT id, master_product_id, image_url, kind, created_at
            FROM master_product_images
            WHERE master_product_id = ?
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(master_product_id)
        .fetch_all(&self.db)
        .await
    }

    pub async fn get_master_product_image(
        &self,
        image_id: i64,
    ) -> Result<Option<MasterProductImageRow>, sqlx::Error> {
        sqlx::query_as::<_, MasterProductImageRow>(
            r#"
            SELECT id, master_product_id, image_url, kind, created_at
            FROM master_product_images
            WHERE id = ?
            "#,
        )
        .bind(image_id)
        .fetch_optional(&self.db)
        .await
    }

    pub async fn update_master_product_image(
        &self,
        image_id: i64,
        image_url: &str,
        kind: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE master_product_images
            SET image_url = ?, kind = ?
            WHERE id = ?
            "#,
        )
        .bind(image_url)
        .bind(kind)
        .bind(image_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn delete_master_product_image(&self, image_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM master_product_images WHERE id = ?")
            .bind(image_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub async fn has_embedding_for_image(
        &self,
        image_id: i64,
        model_version: &str,
    ) -> Result<bool, sqlx::Error> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT image_id FROM image_embeddings WHERE image_id = ? AND model_version = ? LIMIT 1",
        )
        .bind(image_id)
        .bind(model_version)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.is_some())
    }

    pub async fn delete_embeddings_by_image_id(&self, image_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM image_embeddings WHERE image_id = ?")
            .bind(image_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub async fn delete_embeddings_by_image_ids(
        &self,
        image_ids: &[i64],
    ) -> Result<(), sqlx::Error> {
        if image_ids.is_empty() {
            return Ok(());
        }
        let placeholders = image_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("DELETE FROM image_embeddings WHERE image_id IN ({})", placeholders);
        let mut q = sqlx::query(&sql);
        for id in image_ids {
            q = q.bind(*id);
        }
        q.execute(&self.db).await?;
        Ok(())
    }

    pub async fn load_search_candidates(
        &self,
        model_version: &str,
        mode: Option<&str>,
        event_id: Option<i64>,
    ) -> Result<Vec<EmbeddingCandidate>, sqlx::Error> {
        let rows = match mode {
            Some("order") | Some("admin_event") => {
                let event_id = event_id.unwrap_or_default();
                sqlx::query_as::<_, RawEmbeddingRow>(
                    r#"
                    SELECT
                        mp.id as master_product_id,
                        mp.product_code,
                        mp.name,
                        COALESCE(mp.image_url, mpi.image_url) as thumb_url,
                        ie.dim,
                        ie.vector
                    FROM image_embeddings ie
                    JOIN master_product_images mpi ON mpi.id = ie.image_id
                    JOIN master_products mp ON mp.id = mpi.master_product_id
                    JOIN products p ON p.master_product_id = mp.id
                    WHERE ie.model_version = ?
                      AND p.event_id = ?
                      AND mp.is_active = 1
                      AND mpi.kind NOT IN ('feedback_incorrect', 'legacy_main')
                    "#,
                )
                .bind(model_version)
                .bind(event_id)
                .fetch_all(&self.db)
                .await?
            }
            _ => {
                sqlx::query_as::<_, RawEmbeddingRow>(
                    r#"
                    SELECT
                        mp.id as master_product_id,
                        mp.product_code,
                        mp.name,
                        COALESCE(mp.image_url, mpi.image_url) as thumb_url,
                        ie.dim,
                        ie.vector
                    FROM image_embeddings ie
                    JOIN master_product_images mpi ON mpi.id = ie.image_id
                    JOIN master_products mp ON mp.id = mpi.master_product_id
                    WHERE ie.model_version = ?
                      AND mp.is_active = 1
                      AND mpi.kind NOT IN ('feedback_incorrect', 'legacy_main')
                    "#,
                )
                .bind(model_version)
                .fetch_all(&self.db)
                .await?
            }
        };

        let mut out = Vec::new();
        for row in rows {
            if let Some(vector) = decode_embedding_blob(&row.vector, row.dim) {
                out.push(EmbeddingCandidate {
                    master_product_id: row.master_product_id,
                    product_code: row.product_code,
                    name: row.name,
                    thumb_url: row.thumb_url,
                    vector,
                });
            }
        }

        Ok(out)
    }

    pub async fn list_all_product_images(&self) -> Result<Vec<RebuildImageRow>, sqlx::Error> {
        sqlx::query_as::<_, RebuildImageRow>(
            r#"
            SELECT mpi.id as image_id, mpi.image_url
            FROM master_product_images mpi
            WHERE mpi.kind NOT IN ('feedback_incorrect', 'legacy_main')
            ORDER BY mpi.id ASC
            "#,
        )
        .fetch_all(&self.db)
        .await
    }

    pub async fn list_product_images_by_ids(
        &self,
        image_ids: &[i64],
    ) -> Result<Vec<RebuildImageRow>, sqlx::Error> {
        if image_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut sql = String::from(
            "SELECT id as image_id, image_url FROM master_product_images WHERE id IN (",
        );
        for (idx, _) in image_ids.iter().enumerate() {
            if idx > 0 {
                sql.push(',');
            }
            sql.push('?');
        }
        sql.push_str(") ORDER BY id ASC");

        let mut q = sqlx::query_as::<_, RebuildImageRow>(&sql);
        for image_id in image_ids {
            q = q.bind(*image_id);
        }

        q.fetch_all(&self.db).await
    }

    pub async fn list_images_missing_embedding_for_model(
        &self,
        model_version: &str,
    ) -> Result<Vec<RebuildImageRow>, sqlx::Error> {
        sqlx::query_as::<_, RebuildImageRow>(
            r#"
            SELECT mpi.id as image_id, mpi.image_url
            FROM master_product_images mpi
            LEFT JOIN image_embeddings ie
              ON ie.image_id = mpi.id
             AND ie.model_version = ?
            WHERE ie.image_id IS NULL
              AND mpi.kind NOT IN ('feedback_incorrect', 'legacy_main')
            ORDER BY mpi.id ASC
            "#,
        )
        .bind(model_version)
        .fetch_all(&self.db)
        .await
    }

    pub async fn clear_embeddings_for_model(&self, model_version: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM image_embeddings WHERE model_version = ?")
            .bind(model_version)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    pub async fn upsert_embedding(
        &self,
        image_id: i64,
        model_version: &str,
        vector: &[f32],
    ) -> Result<(), sqlx::Error> {
        let mut blob = Vec::with_capacity(vector.len() * 4);
        for v in vector {
            blob.extend_from_slice(&v.to_le_bytes());
        }

        sqlx::query(
            r#"
            INSERT INTO image_embeddings (image_id, model_version, dim, vector)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(image_id, model_version) DO UPDATE SET
                dim = excluded.dim,
                vector = excluded.vector,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(image_id)
        .bind(model_version)
        .bind(vector.len() as i32)
        .bind(blob)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}
