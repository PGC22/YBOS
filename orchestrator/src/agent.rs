use anyhow::Result;
use async_trait::async_trait;
use crate::manifest::Manifest;

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
    async fn invoke(&self, call: AgentCall) -> Result<AgentResponse>;
}
