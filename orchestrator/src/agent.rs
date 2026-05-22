use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use ybos_inference::Inference;
use ybos_memory::{VectorStore, Embedder};
use crate::manifest::Manifest;

#[derive(Clone)]
pub struct AgentContext {
    pub inference: Arc<dyn Inference>,
    pub memory: Arc<dyn VectorStore>,
    pub embedder: Arc<dyn Embedder>,
}

#[derive(Debug, Clone)]
pub struct AgentCall {
    pub method: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct AgentResponse {
    pub payload: Vec<u8>,
}

impl AgentResponse {
    pub fn text(s: String) -> Self {
        Self {
            payload: s.into_bytes(),
        }
    }
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn manifest(&self) -> &Manifest;
    async fn invoke(&self, call: AgentCall, ctx: &AgentContext) -> Result<AgentResponse>;
}
