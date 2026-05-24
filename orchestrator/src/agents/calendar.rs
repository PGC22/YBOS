use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use ybos_calendar::{CalendarStore, Event, EventQuery, EventType};
use ybos_inference::CompleteRequest;
use ybos_user_context::{ContextCategory, ContextQuery};

use crate::agent::{Agent, AgentCall, AgentContext, AgentResponse};
use crate::capability::{self, Operation};
use crate::manifest::{AccessLevel, Capabilities, Manifest, MemoryAccess};

pub struct CalendarAgent {
    manifest: Manifest,
    store: Arc<dyn CalendarStore>,
}

#[derive(Deserialize)]
struct AddPayload {
    title: String,
    description: Option<String>,
    start_ts: u64,
    end_ts: u64,
    location: Option<String>,
    event_type: EventType,
    attendees: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct ListPayload {
    from_ts: Option<u64>,
    to_ts: Option<u64>,
    event_type: Option<EventType>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct NextPayload {
    within_secs: Option<u64>,
}

#[derive(Deserialize)]
struct NlQueryPayload {
    question: String,
    window_secs: Option<u64>,
}

#[derive(Deserialize)]
struct RemindPayload {
    within_secs: Option<u64>,
}

impl CalendarAgent {
    pub fn new(name: &str, store: Arc<dyn CalendarStore>) -> Self {
        Self {
            manifest: Manifest {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                capabilities: Capabilities {
                    llm: true,
                    memory: MemoryAccess::None,
                    data_user_prefs: AccessLevel::Read,
                    net_domains: vec![],
                    ..Default::default()
                },
            },
            store,
        }
    }
}

#[async_trait]
impl Agent for CalendarAgent {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn invoke(&self, call: AgentCall, ctx: &AgentContext) -> Result<AgentResponse> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        match call.method.as_str() {
            "add" => {
                let p: AddPayload = serde_json::from_slice(&call.payload)?;
                let id = Uuid::new_v4();
                let event = Event {
                    id,
                    title: p.title,
                    description: p.description,
                    start_ts: p.start_ts,
                    end_ts: p.end_ts,
                    location: p.location,
                    event_type: p.event_type,
                    attendees: p.attendees.unwrap_or_default(),
                    created_at: now,
                    updated_at: now,
                };
                self.store.put(event).await.map_err(|e| anyhow!("CalendarStore error: {}", e))?;
                Ok(AgentResponse::json(json!({ "id": id })))
            }
            "list" => {
                let p: ListPayload = serde_json::from_slice(&call.payload).unwrap_or(ListPayload {
                    from_ts: None,
                    to_ts: None,
                    event_type: None,
                    limit: None,
                });
                let q = EventQuery {
                    from_ts: p.from_ts,
                    to_ts: p.to_ts,
                    event_type: p.event_type,
                    limit: p.limit.unwrap_or(0),
                };
                let events = self.store.list(q).await.map_err(|e| anyhow!("CalendarStore error: {}", e))?;
                Ok(AgentResponse::json(json!(events)))
            }
            "next" => {
                let p: NextPayload = serde_json::from_slice(&call.payload).unwrap_or(NextPayload { within_secs: None });
                let within_secs = p.within_secs.unwrap_or(86400);
                let q = EventQuery {
                    from_ts: Some(now),
                    to_ts: Some(now + within_secs),
                    limit: 1,
                    ..Default::default()
                };
                let events = self.store.list(q).await.map_err(|e| anyhow!("CalendarStore error: {}", e))?;
                if let Some(event) = events.first() {
                    Ok(AgentResponse::json(json!(event)))
                } else {
                    Ok(AgentResponse::json(json!({ "message": "No upcoming events" })))
                }
            }
            "nl_query" => {
                capability::enforce(&self.manifest, &Operation::LlmCall)?;
                let p: NlQueryPayload = serde_json::from_slice(&call.payload)?;
                let window_secs = p.window_secs.unwrap_or(86400);
                let events = self.store.list(EventQuery {
                    from_ts: Some(now),
                    to_ts: Some(now + window_secs),
                    ..Default::default()
                }).await.map_err(|e| anyhow!("CalendarStore error: {}", e))?;

                if events.is_empty() {
                    return Ok(AgentResponse::text("No events found in window".to_string()));
                }

                let mut events_text = String::new();
                for e in events {
                    events_text.push_str(&format!("- start={} end={} type={:?} title={}\n",
                        e.start_ts, e.end_ts, e.event_type, e.title));
                }

                let prompt = format!(
                    "Times below are unix epoch seconds (UTC).\nCurrent time: {}\nEvents:\n{}\nUser question: {}\nAnswer in one sentence.",
                    now, events_text, p.question
                );

                let llm_res = ctx.inference.complete(CompleteRequest {
                    prompt,
                    max_tokens: 128,
                    temperature: 0.7,
                    top_p: 0.9,
                    stop: vec![],
                    seed: None,
                }).await?;

                Ok(AgentResponse::text(llm_res.text))
            }
            "remind" => {
                capability::enforce(&self.manifest, &Operation::UserContextRead)?;
                let p: RemindPayload = serde_json::from_slice(&call.payload).unwrap_or(RemindPayload { within_secs: None });
                let within_secs = p.within_secs.unwrap_or(3600);

                let uc_entries = ctx.user_context.query(ContextQuery {
                    category: Some(ContextCategory::Recurrence),
                    key_prefix: Some("calendar.reminder.".to_string()),
                    limit: 0,
                    ..Default::default()
                }).await.unwrap_or_default();

                let mut internal_offset = 15;
                let mut external_offset = 60;
                let mut personal_offset = 5;

                for entry in uc_entries {
                    match entry.key.as_str() {
                        "calendar.reminder.internal_meeting" => {
                            if let Some(v) = entry.value.as_u64() { internal_offset = v; }
                        }
                        "calendar.reminder.external_meeting" => {
                            if let Some(v) = entry.value.as_u64() { external_offset = v; }
                        }
                        "calendar.reminder.personal" => {
                            if let Some(v) = entry.value.as_u64() { personal_offset = v; }
                        }
                        _ => {}
                    }
                }

                let events = self.store.list(EventQuery {
                    from_ts: Some(now),
                    to_ts: Some(now + within_secs),
                    ..Default::default()
                }).await.map_err(|e| anyhow!("CalendarStore error: {}", e))?;

                let mut due = Vec::new();
                for e in events {
                    let lead_mins = match e.event_type {
                        EventType::Internal => internal_offset,
                        EventType::External => external_offset,
                        EventType::Personal => personal_offset,
                        EventType::Other => internal_offset,
                    };
                    let lead_secs = lead_mins * 60;
                    if now + lead_secs >= e.start_ts {
                        due.push(e);
                    }
                }

                Ok(AgentResponse::json(json!({ "due": due })))
            }
            _ => Err(anyhow!("Unknown method: {}", call.method)),
        }
    }
}
