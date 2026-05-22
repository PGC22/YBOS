use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use crate::types::{EmbedderInfo, MemoryError};
use crate::trait_def::Embedder;

pub struct FastEmbedEmbedder {
    model: Arc<TextEmbedding>,
}

impl FastEmbedEmbedder {
    pub fn load(model_name: Option<String>, cache_dir: Option<PathBuf>) -> Result<Self, MemoryError> {
        let model_enum = if let Some(_name) = model_name {
            // Mapping string name to enum might be needed, but fastembed has many models.
            // For now, let's just use BGE-small-en-v1.5 as default or try to parse.
            EmbeddingModel::BGESmallENV15
        } else {
            EmbeddingModel::BGESmallENV15
        };

        let mut options = InitOptions::new(model_enum);
        if let Some(dir) = cache_dir {
            options = options.with_cache_dir(dir);
        } else if let Ok(dir) = std::env::var("YBOS_FASTEMBED_CACHE") {
            options = options.with_cache_dir(PathBuf::from(dir));
        }

        let model = TextEmbedding::try_new(options)
            .map_err(|e| MemoryError::EmbedderError(e.to_string()))?;

        Ok(Self { model: Arc::new(model) })
    }
}

#[async_trait]
impl Embedder for FastEmbedEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, MemoryError> {
        let text = text.to_string();
        let model = self.model.clone();

        tokio::task::spawn_blocking(move || {
            let embeddings = model.embed(vec![text], None)
                .map_err(|e| MemoryError::EmbedderError(e.to_string()))?;

            Ok(embeddings[0].clone())
        }).await.map_err(|e| MemoryError::Unknown(e.to_string()))?
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, MemoryError> {
        let model = self.model.clone();

        tokio::task::spawn_blocking(move || {
            let embeddings = model.embed(texts, None)
                .map_err(|e| MemoryError::EmbedderError(e.to_string()))?;

            Ok(embeddings)
        }).await.map_err(|e| MemoryError::Unknown(e.to_string()))?
    }

    fn dimension(&self) -> usize {
        // BGE-small-en-v1.5 is 384
        384
    }

    fn model_info(&self) -> EmbedderInfo {
        EmbedderInfo {
            backend: "fastembed".into(),
            model_name: "BAAI/bge-small-en-v1.5".into(),
            dimension: self.dimension(),
        }
    }
}
