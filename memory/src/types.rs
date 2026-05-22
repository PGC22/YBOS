use serde::{Deserialize, Serialize};
use uuid::Uuid;
use thiserror::Error;

pub type VectorId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorItem {
    pub embedding: Vec<f32>,
    pub text: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorQuery {
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMatch {
    pub id: VectorId,
    pub text: String,
    pub metadata: serde_json::Value,
    pub score: f32, // Cosine similarity (higher is better)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedderInfo {
    pub backend: String,
    pub model_name: String,
    pub dimension: usize,
}

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Invalid embedding: {0}")]
    InvalidEmbedding(String),

    #[error("Item not found")]
    NotFound,

    #[error("Embedder error: {0}")]
    EmbedderError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
