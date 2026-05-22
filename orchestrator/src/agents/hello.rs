use anyhow::Result;
use async_trait::async_trait;
use crate::agent::{Agent, AgentCall, AgentContext, AgentResponse};
use crate::capability::{self, Operation};
use crate::manifest::{Capabilities, Manifest};
use ybos_inference::CompleteRequest;

pub struct HelloAgent {
    manifest: Manifest,
    use_llm: bool,
}

impl HelloAgent {
    pub fn new(name: &str, manifest: Option<Manifest>) -> Self {
        Self {
            manifest: manifest.unwrap_or(Manifest {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                capabilities: Default::default(),
            }),
            use_llm: false,
        }
    }

    pub fn new_with_llm(name: &str) -> Self {
        Self {
            manifest: Manifest {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                capabilities: Capabilities {
                    llm: true,
                    ..Default::default()
                },
            },
            use_llm: true,
        }
    }
}

#[async_trait]
impl Agent for HelloAgent {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn invoke(&self, call: AgentCall, ctx: &AgentContext) -> Result<AgentResponse> {
        if self.use_llm {
            capability::enforce(&self.manifest, &Operation::LlmCall)?;

            let prompt = format!(
                "Reply with one word: {}",
                String::from_utf8_lossy(&call.payload)
            );
            let llm_res = ctx
                .inference
                .complete(CompleteRequest {
                    prompt,
                    max_tokens: 16,
                    temperature: 0.1,
                    top_p: 0.9,
                    stop: vec![],
                    seed: Some(42),
                })
                .await?;

            Ok(AgentResponse::text(format!(
                "hello from {}: {}",
                self.manifest.name, llm_res.text
            )))
        } else {
            Ok(AgentResponse::text(format!(
                "hello from {}",
                self.manifest.name
            )))
        }
    }
}
