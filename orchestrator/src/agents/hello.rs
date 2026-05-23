use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use crate::agent::{Agent, AgentCall, AgentContext, AgentResponse};
use crate::capability::{self, Operation};
use crate::manifest::{AccessLevel, Capabilities, Manifest, MemoryAccess};
use ybos_inference::CompleteRequest;
use ybos_memory::{VectorItem, VectorQuery};
use ybos_user_context::{ContextCategory, ContextEntry};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct HelloAgent {
    manifest: Manifest,
    use_llm: bool,
    text_to_remember: Option<String>,
    use_user_context: bool,
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
            text_to_remember: None,
            use_user_context: false,
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
            text_to_remember: None,
            use_user_context: false,
        }
    }

    pub fn new_with_memory(name: &str, text_to_remember: &str) -> Self {
        Self {
            manifest: Manifest {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                capabilities: Capabilities {
                    memory: MemoryAccess::ReadWrite,
                    ..Default::default()
                },
            },
            use_llm: false,
            text_to_remember: Some(text_to_remember.to_string()),
            use_user_context: false,
        }
    }

    pub fn new_with_user_context(name: &str) -> Self {
        Self {
            manifest: Manifest {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                capabilities: Capabilities {
                    data_user_prefs: AccessLevel::ReadWrite,
                    ..Default::default()
                },
            },
            use_llm: false,
            text_to_remember: None,
            use_user_context: true,
        }
    }
}

#[async_trait]
impl Agent for HelloAgent {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn invoke(&self, call: AgentCall, ctx: &AgentContext) -> Result<AgentResponse> {
        if let Some(text) = &self.text_to_remember {
            capability::enforce(&self.manifest, &Operation::MemoryWrite)?;
            let embedding = ctx.embedder.embed(text).await?;

            ctx.memory
                .insert(VectorItem {
                    embedding: embedding.clone(),
                    text: text.clone(),
                    metadata: json!({"agent": self.manifest.name}),
                })
                .await?;

            capability::enforce(&self.manifest, &Operation::MemoryRead)?;
            let matches = ctx
                .memory
                .query_top_k(
                    VectorQuery {
                        embedding,
                    },
                    1,
                )
                .await?;

            let matched_text = matches
                .first()
                .map(|m| m.text.clone())
                .unwrap_or_else(|| "nothing".to_string());

            Ok(AgentResponse::text(format!(
                "hello from {}: remembered {}",
                self.manifest.name, matched_text
            )))
        } else if self.use_user_context {
            capability::enforce(&self.manifest, &Operation::UserContextWrite)?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            ctx.user_context
                .put(ContextEntry {
                    id: uuid::Uuid::new_v4(),
                    category: ContextCategory::Preference,
                    key: "hello.last_invocation_at".to_string(),
                    value: json!(now),
                    note: Some("Tracked by HelloAgent".to_string()),
                    confidence: 1.0,
                    created_at: now,
                    updated_at: now,
                })
                .await?;

            capability::enforce(&self.manifest, &Operation::UserContextRead)?;
            let entry = ctx
                .user_context
                .get(ContextCategory::Preference, "hello.last_invocation_at")
                .await?
                .ok_or_else(|| anyhow::anyhow!("Failed to retrieve user context after put"))?;

            Ok(AgentResponse::text(format!(
                "hello from {}: last invocation at {}",
                self.manifest.name, entry.value
            )))
        } else if self.use_llm {
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
