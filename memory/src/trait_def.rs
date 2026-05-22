use async_trait::async_trait;
use crate::types::{VectorId, VectorItem, VectorQuery, VectorMatch, EmbedderInfo, MemoryError};

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn insert(&self, item: VectorItem) -> Result<VectorId, MemoryError>;
    async fn insert_batch(&self, items: Vec<VectorItem>) -> Result<Vec<VectorId>, MemoryError>;
    async fn query_top_k(&self, query: VectorQuery, k: usize) -> Result<Vec<VectorMatch>, MemoryError>;
    async fn delete(&self, id: VectorId) -> Result<(), MemoryError>;
    async fn count(&self) -> Result<usize, MemoryError>;
}

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, MemoryError>;
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, MemoryError>;
    fn dimension(&self) -> usize;
    fn model_info(&self) -> EmbedderInfo;
}
