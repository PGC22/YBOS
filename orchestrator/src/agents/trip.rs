use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use ybos_inference::CompleteRequest;
use ybos_memory::{VectorItem, VectorQuery};
use ybos_user_context::{ContextCategory, ContextQuery};

use crate::agent::{Agent, AgentCall, AgentContext, AgentResponse};
use crate::capability::{self, Operation};
use crate::manifest::{AccessLevel, Capabilities, Manifest, MemoryAccess};
use crate::news::rss;

pub struct TripPlannerAgent {
    manifest: Manifest,
    advisory_sources: Vec<String>,
}

#[derive(Deserialize)]
struct PlanPayload {
    origin: Option<String>,
    destination: String,
    depart_date: Option<String>,
    return_date: Option<String>,
    budget_eur: Option<u64>,
    purpose: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize)]
struct RecallPayload {
    query: String,
    k: Option<usize>,
}

#[derive(Deserialize)]
struct ListPayload {
    limit: Option<usize>,
}

impl TripPlannerAgent {
    pub fn new(name: &str, advisory_sources: Vec<String>) -> Self {
        let mut net_domains = Vec::new();
        for source in &advisory_sources {
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
                    data_user_prefs: AccessLevel::Read,
                    ..Default::default()
                },
            },
            advisory_sources,
        }
    }
}

#[async_trait]
impl Agent for TripPlannerAgent {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn invoke(&self, call: AgentCall, ctx: &AgentContext) -> Result<AgentResponse> {
        match call.method.as_str() {
            "plan" => {
                let p: PlanPayload = serde_json::from_slice(&call.payload)?;

                capability::enforce(&self.manifest, &Operation::UserContextRead)?;
                let prefs = ctx.user_context.query(ContextQuery {
                    category: Some(ContextCategory::Preference),
                    key_prefix: Some("travel.".to_string()),
                    limit: 0,
                    ..Default::default()
                }).await.unwrap_or_default();

                let mut prefs_prelude = String::new();
                if prefs.is_empty() {
                    prefs_prelude.push_str("(none provided)");
                } else {
                    for entry in prefs {
                        prefs_prelude.push_str(&format!("- {}: {}\n", entry.key, entry.value));
                    }
                }

                let mut matched_advisories = Vec::new();
                let dest_lower = p.destination.to_lowercase();

                for url in &self.advisory_sources {
                    if matched_advisories.len() >= 10 {
                        break;
                    }

                    let uri: hyper::Uri = url.parse().map_err(|e| anyhow!("Invalid source URL: {}", e))?;
                    let host = uri.host().ok_or_else(|| anyhow!("No host in URL: {}", url))?;

                    capability::enforce(&self.manifest, &Operation::NetConnect(host.to_string()))?;

                    let resp = ctx.http.get(url).await?;
                    if resp.status != 200 {
                        tracing::warn!(
                            target: "ybos.trip",
                            url = %url,
                            status = resp.status,
                            "Non-200 response from advisory source, skipping"
                        );
                        continue;
                    }

                    let xml_str = String::from_utf8_lossy(&resp.body);
                    let channel = rss::parse_rss(&xml_str)?;

                    for item in channel.items {
                        if matched_advisories.len() >= 10 {
                            break;
                        }
                        if item.title.to_lowercase().contains(&dest_lower) ||
                           item.description.to_lowercase().contains(&dest_lower) {
                            matched_advisories.push(format!("{}: {}", item.title, item.description));
                        }
                    }
                }

                let advisories_text = if matched_advisories.is_empty() {
                    "(no advisories matched)".to_string()
                } else {
                    matched_advisories.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n")
                };

                let prompt = format!(
                    "You are a trip-planning assistant. Compose a concise brief.\n\
                    User travel preferences:\n\
                    {}\n\n\
                    Trip request:\n\
                    - Origin: {}\n\
                    - Destination: {}\n\
                    - Depart: {}\n\
                    - Return: {}\n\
                    - Budget (EUR): {}\n\
                    - Purpose: {}\n\
                    - Notes: {}\n\n\
                    Recent travel advisories matching destination (up to 10):\n\
                    {}\n\n\
                    Return a 5-bullet plan covering: flights direction, lodging suggestion, day-1 priority, must-avoid, packing reminder.",
                    prefs_prelude,
                    p.origin.as_deref().unwrap_or("(not specified)"),
                    p.destination,
                    p.depart_date.as_deref().unwrap_or("(flexible)"),
                    p.return_date.as_deref().unwrap_or("(flexible)"),
                    p.budget_eur.map(|b| b.to_string()).unwrap_or_else(|| "(unspecified)".to_string()),
                    p.purpose.as_deref().unwrap_or("(general)"),
                    p.notes.as_deref().unwrap_or("(none)"),
                    advisories_text
                );

                capability::enforce(&self.manifest, &Operation::LlmCall)?;
                let llm_res = ctx.inference.complete(CompleteRequest {
                    prompt,
                    max_tokens: 512,
                    temperature: 0.5,
                    top_p: 0.9,
                    stop: vec![],
                    seed: None,
                }).await?;

                let brief = llm_res.text;
                let embedding = ctx.embedder.embed(&brief).await?;

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let trip_id = Uuid::new_v4().to_string();

                capability::enforce(&self.manifest, &Operation::MemoryWrite)?;
                ctx.memory.insert(VectorItem {
                    embedding,
                    text: brief.clone(),
                    metadata: json!({
                        "type": "trip",
                        "destination": p.destination,
                        "depart_date": p.depart_date,
                        "return_date": p.return_date,
                        "purpose": p.purpose,
                        "budget_eur": p.budget_eur,
                        "planned_at": now,
                        "trip_id": trip_id,
                    }),
                }).await?;

                Ok(AgentResponse::json(json!({
                    "trip_id": trip_id,
                    "brief": brief,
                }))?)
            }
            "recall" => {
                let p: RecallPayload = serde_json::from_slice(&call.payload)?;
                let k = p.k.unwrap_or(5);

                let embedding = ctx.embedder.embed(&p.query).await?;
                capability::enforce(&self.manifest, &Operation::MemoryRead)?;
                let matches = ctx.memory.query_top_k(VectorQuery { embedding }, k).await?;

                let filtered: Vec<_> = matches.into_iter()
                    .filter(|m| m.metadata.get("type").and_then(|v| v.as_str()) == Some("trip"))
                    .collect();

                Ok(AgentResponse::json(json!(filtered))?)
            }
            "list" => {
                let p: ListPayload = serde_json::from_slice(&call.payload).unwrap_or(ListPayload { limit: None });
                let limit = p.limit.unwrap_or(10);

                capability::enforce(&self.manifest, &Operation::MemoryRead)?;
                let dummy_embedding = vec![0.0; ctx.embedder.dimension()];
                let matches = ctx.memory.query_top_k(VectorQuery { embedding: dummy_embedding }, 100).await?;

                let mut filtered: Vec<_> = matches.into_iter()
                    .filter(|m| m.metadata.get("type").and_then(|v| v.as_str()) == Some("trip"))
                    .collect();

                filtered.sort_by(|a, b| {
                    let a_time = a.metadata.get("planned_at").and_then(|v| v.as_u64()).unwrap_or(0);
                    let b_time = b.metadata.get("planned_at").and_then(|v| v.as_u64()).unwrap_or(0);
                    b_time.cmp(&a_time)
                });

                if filtered.len() > limit {
                    filtered.truncate(limit);
                }

                Ok(AgentResponse::json(json!(filtered))?)
            }
            _ => Err(anyhow!("Unknown method: {}", call.method)),
        }
    }
}
