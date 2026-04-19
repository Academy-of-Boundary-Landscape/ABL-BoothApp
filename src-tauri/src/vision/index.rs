use std::sync::atomic::{AtomicI64, Ordering};

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct VisionIndex {
    size: AtomicI64,
}

impl VisionIndex {
    pub fn new() -> Self {
        Self {
            size: AtomicI64::new(0),
        }
    }

    pub fn size(&self) -> i64 {
        self.size.load(Ordering::Relaxed)
    }

    pub fn set_size(&self, value: i64) {
        self.size.store(value, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddingCandidate {
    pub master_product_id: i64,
    pub product_code: String,
    pub name: String,
    pub thumb_url: Option<String>,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct ProductSearchHit {
    pub master_product_id: i64,
    pub product_code: String,
    pub name: String,
    pub thumb_url: Option<String>,
    pub score: f32,
}

pub fn decode_embedding_blob(blob: &[u8], dim: i32) -> Option<Vec<f32>> {
    if dim <= 0 {
        return None;
    }

    let expected = dim as usize * 4;
    if blob.len() != expected {
        return None;
    }

    let mut vec = Vec::with_capacity(dim as usize);
    for chunk in blob.chunks_exact(4) {
        vec.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Some(super::model::l2_normalize(vec))
}

pub fn search_top_k(
    query: &[f32],
    candidates: &[EmbeddingCandidate],
    top_k: usize,
) -> Vec<ProductSearchHit> {
    let mut per_product: HashMap<i64, ProductSearchHit> = HashMap::new();

    for candidate in candidates {
        if candidate.vector.len() != query.len() {
            continue;
        }

        let score = cosine(query, &candidate.vector);
        match per_product.get_mut(&candidate.master_product_id) {
            Some(existing) if score > existing.score => {
                existing.score = score;
                existing.thumb_url = candidate.thumb_url.clone();
            }
            Some(_) => {} // 已有更高分，跳过
            None => {
                per_product.insert(
                    candidate.master_product_id,
                    ProductSearchHit {
                        master_product_id: candidate.master_product_id,
                        product_code: candidate.product_code.clone(),
                        name: candidate.name.clone(),
                        thumb_url: candidate.thumb_url.clone(),
                        score,
                    },
                );
            }
        }
    }

    let mut hits = per_product.into_values().collect::<Vec<_>>();
    hits.sort_by(|a, b| b.score.total_cmp(&a.score));
    hits.truncate(top_k);
    hits
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>()
}
