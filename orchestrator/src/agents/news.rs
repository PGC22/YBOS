use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use ybos_inference::CompleteRequest;
use ybos_memory::{VectorItem, VectorQuery};

use crate::agent::{Agent, AgentCall, AgentContext, AgentResponse};
use crate::capability::{self, Operation};
use crate::manifest::{Capabilities, Manifest, MemoryAccess};
use crate::news::rss;

pub struct NewsAgent {
    manifest: Manifest,
    sources: Vec<String>,
}

#[derive(Deserialize)]
struct SummarizePayload {
    k: Option<usize>,
}

#[derive(Deserialize)]
struct QueryPayload {
    query: String,
    k: Option<usize>,
}

impl NewsAgent {
    pub fn new(name: &str, sources: Vec<String>) -> Self {
        let mut net_domains = Vec::new();
        for source in &sources {
            if let Ok(url) = hyper::Uri::try_from(source) {
                if let Some(host) = url.host() {
                    net_domains.push(host.to_string());
                }
            }
        }

        Self {
            manifest: Manifest {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                capabilities: Capabilities {
                    net_domains,
                    llm: true,
                    memory: MemoryAccess::ReadWrite,
                    ..Default::default()
                },
            },
            sources,
        }
    }
}

#[async_trait]
impl Agent for NewsAgent {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn invoke(&self, call: AgentCall, ctx: &AgentContext) -> Result<AgentResponse> {
        match call.method.as_str() {
            "fetch" => {
                let mut total_inserted = 0;
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                for url in &self.sources {
                    let uri: hyper::Uri = url.parse().map_err(|e| anyhow!("Invalid source URL: {}", e))?;
                    let host = uri.host().ok_or_else(|| anyhow!("No host in URL: {}", url))?;

                    capability::enforce(&self.manifest, &Operation::NetConnect(host.to_string()))?;

                    let resp = ctx.http.get(url).await?;
                    if resp.status != 200 {
                        continue;
                    }

                    let xml_str = String::from_utf8_lossy(&resp.body);
                    let channel = rss::parse_rss(&xml_str)?;

                    for item in channel.items {
                        let text = format!("{}\n{}", item.title, item.description);
                        let embedding = ctx.embedder.embed(&text).await?;

                        capability::enforce(&self.manifest, &Operation::MemoryWrite)?;
                        ctx.memory.insert(VectorItem {
                            embedding,
                            text,
                            metadata: json!({
                                "source": url,
                                "type": "news",
                                "fetched_at": now,
                                "title": item.title,
                                "link": item.link,
                                "pub_date": item.pub_date,
                            }),
                        }).await?;
                        total_inserted += 1;
                    }
                }
                Ok(AgentResponse::text(format!("Fetched {} items", total_inserted)))
            }
            "summarize" => {
                let payload: SummarizePayload = serde_json::from_slice(&call.payload).unwrap_or(SummarizePayload { k: None });
                let k = payload.k.unwrap_or(5);

                capability::enforce(&self.manifest, &Operation::MemoryRead)?;

                // Since VectorStore trait doesn't support sorting by metadata yet,
                // we fetch a larger batch (e.g., 100) and sort in-memory by fetched_at.
                let dummy_embedding = vec![0.0; ctx.embedder.dimension()];
                let mut matches = ctx.memory.query_top_k(VectorQuery { embedding: dummy_embedding }, 100).await?;

                // Sort by fetched_at metadata descending (most recent first)
                matches.sort_by(|a, b| {
                    let a_time = a.metadata.get("fetched_at").and_then(|v| v.as_u64()).unwrap_or(0);
                    let b_time = b.metadata.get("fetched_at").and_then(|v| v.as_u64()).unwrap_or(0);
                    b_time.cmp(&a_time)
                });

                if matches.is_empty() {
                    return Ok(AgentResponse::text("No news items found to summarize.".to_string()));
                }

                let mut prompt = "Summarize the following news items:\n\n".to_string();
                for m in matches.iter().take(k) {
                    prompt.push_str(&format!("- {}\n", m.text));
                }

                capability::enforce(&self.manifest, &Operation::LlmCall)?;
                let llm_res = ctx.inference.complete(CompleteRequest {
                    prompt,
                    max_tokens: 512,
                    temperature: 0.7,
                    top_p: 0.9,
                    stop: vec![],
                    seed: None,
                }).await?;

                Ok(AgentResponse::text(llm_res.text))
            }
            "query" => {
                let payload: QueryPayload = serde_json::from_slice(&call.payload)?;
                let k = payload.k.unwrap_or(5);

                let embedding = ctx.embedder.embed(&payload.query).await?;

                capability::enforce(&self.manifest, &Operation::MemoryRead)?;
                let matches = ctx.memory.query_top_k(VectorQuery { embedding }, k).await?;

                Ok(AgentResponse {
                    payload: serde_json::to_vec(&matches)?,
                })
            }
            _ => Err(anyhow!("Unknown method: {}", call.method)),
        }
    }
}
