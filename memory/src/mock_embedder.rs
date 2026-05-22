use async_trait::async_trait;
use sha2::{Sha256, Digest};
use crate::types::{EmbedderInfo, MemoryError};
use crate::trait_def::Embedder;

pub struct MockEmbedder {
    dimension: usize,
}

impl MockEmbedder {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl Default for MockEmbedder {
    fn default() -> Self {
        Self::new(384)
    }
}

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, MemoryError> {
        let mut result = Vec::with_capacity(self.dimension);
        let mut current_input = text.as_bytes().to_vec();

        while result.len() < self.dimension {
            let mut hasher = Sha256::new();
            hasher.update(&current_input);
            let hash = hasher.finalize();

            for &byte in hash.iter() {
                if result.len() >= self.dimension {
                    break;
                }
                // Map u8 [0, 255] to f32 [-1.0, 1.0]
                let val = (byte as f32 / 127.5) - 1.0;
                result.push(val);
            }
            current_input = hash.to_vec();
        }

        Ok(result)
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, MemoryError> {
        let mut results = Vec::new();
        for text in texts {
            results.push(self.embed(&text).await?);
        }
        Ok(results)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_info(&self) -> EmbedderInfo {
        EmbedderInfo {
            backend: "mock".into(),
            model_name: "mock-bge-small".into(),
            dimension: self.dimension,
        }
    }
}
