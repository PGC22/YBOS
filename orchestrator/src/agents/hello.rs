use anyhow::Result;
use async_trait::async_trait;
use crate::agent::{Agent, AgentCall, AgentResponse};
use crate::manifest::Manifest;

pub struct HelloAgent {
    manifest: Manifest,
}

impl HelloAgent {
    pub fn new(name: &str, manifest: Option<Manifest>) -> Self {
        Self {
            manifest: manifest.unwrap_or(Manifest {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                capabilities: Default::default(),
            }),
        }
    }
}

#[async_trait]
impl Agent for HelloAgent {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn invoke(&self, _call: AgentCall) -> Result<AgentResponse> {
        Ok(AgentResponse::text(format!("hello from {}", self.manifest.name)))
    }
}
