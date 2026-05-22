use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;
use crate::types::{VectorId, VectorItem, VectorQuery, VectorMatch, MemoryError};
use crate::trait_def::VectorStore;

pub struct MockVectorStore {
    items: RwLock<HashMap<VectorId, VectorItem>>,
    dimension: RwLock<Option<usize>>,
}

impl MockVectorStore {
    pub fn new() -> Self {
        Self {
            items: RwLock::new(HashMap::new()),
            dimension: RwLock::new(None),
        }
    }

    fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let norm_v1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
        let norm_v2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();

        if norm_v1 == 0.0 || norm_v2 == 0.0 {
            0.0
        } else {
            dot_product / (norm_v1 * norm_v2)
        }
    }
}

#[async_trait]
impl VectorStore for MockVectorStore {
    async fn insert(&self, item: VectorItem) -> Result<VectorId, MemoryError> {
        let mut dim_guard = self.dimension.write().expect("MockVectorStore: dimension lock poisoned");
        if let Some(dim) = *dim_guard {
            if item.embedding.len() != dim {
                return Err(MemoryError::InvalidEmbedding(format!(
                    "Expected dimension {}, got {}",
                    dim,
                    item.embedding.len()
                )));
            }
        } else {
            *dim_guard = Some(item.embedding.len());
        }

        let id = VectorId::new_v4();
        let mut items = self.items.write().expect("MockVectorStore: items lock poisoned");
        items.insert(id, item);
        Ok(id)
    }

    async fn insert_batch(&self, items: Vec<VectorItem>) -> Result<Vec<VectorId>, MemoryError> {
        let mut ids = Vec::new();
        for item in items {
            ids.push(self.insert(item).await?);
        }
        Ok(ids)
    }

    async fn query_top_k(&self, query: VectorQuery, k: usize) -> Result<Vec<VectorMatch>, MemoryError> {
        let items = self.items.read().expect("MockVectorStore: items lock poisoned");
        let mut matches: Vec<VectorMatch> = items.iter().map(|(id, item)| {
            let score = Self::cosine_similarity(&query.embedding, &item.embedding);
            VectorMatch {
                id: *id,
                text: item.text.clone(),
                metadata: item.metadata.clone(),
                score,
            }
        }).collect();

        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(matches.into_iter().take(k).collect())
    }

    async fn delete(&self, id: VectorId) -> Result<(), MemoryError> {
        let mut items = self.items.write().expect("MockVectorStore: items lock poisoned");
        if items.remove(&id).is_some() {
            Ok(())
        } else {
            Err(MemoryError::NotFound)
        }
    }

    async fn count(&self) -> Result<usize, MemoryError> {
        let items = self.items.read().expect("MockVectorStore: items lock poisoned");
        Ok(items.len())
    }
}
