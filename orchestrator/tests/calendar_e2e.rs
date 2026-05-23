use anyhow::Result;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::json;

use ybos_orchestrator::agent::{Agent, AgentCall, AgentContext};
use ybos_orchestrator::agents::calendar::CalendarAgent;
use ybos_orchestrator::capability::Operation;
use ybos_orchestrator::http::MockHttpClient;
use ybos_inference::mock::MockInference;
use ybos_memory::{MockVectorStore, MockEmbedder};
use ybos_user_context::{MockUserContextStore, ContextEntry, ContextCategory, UserContextStore};
use ybos_calendar::{MockCalendarStore, Event, EventType};

#[tokio::test]
async fn test_calendar_agent_local_e2e() -> Result<()> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    // 1. Build MockUserContextStore and seed preferences
    let user_context = Arc::new(MockUserContextStore::new());
    user_context.put(ContextEntry {
        id: uuid::Uuid::new_v4(),
        category: ContextCategory::Recurrence,
        key: "calendar.reminder.internal_meeting".to_string(),
        value: json!(30),
        note: None,
        confidence: 1.0,
        created_at: now,
        updated_at: now,
    }).await?;
    user_context.put(ContextEntry {
        id: uuid::Uuid::new_v4(),
        category: ContextCategory::Recurrence,
        key: "calendar.reminder.external_meeting".to_string(),
        value: json!(90),
        note: None,
        confidence: 1.0,
        created_at: now,
        updated_at: now,
    }).await?;

    // 2. Build MockCalendarStore
    let store = Arc::new(MockCalendarStore::new());

    // 3. Build AgentContext
    let ctx = AgentContext {
        inference: Arc::new(MockInference::new(vec!["LLM response".to_string()])),
        memory: Arc::new(MockVectorStore::new()),
        embedder: Arc::new(MockEmbedder::new(8)),
        http: Arc::new(MockHttpClient::new(vec![])),
        user_context,
    };

    // 4. Construct CalendarAgent
    let agent = CalendarAgent::new("cal", store.clone());

    // 5. Invoke add three times
    let add_call = |title: &str, et: EventType, start: u64| -> AgentCall {
        AgentCall {
            method: "add".to_string(),
            payload: serde_json::to_vec(&json!({
                "title": title,
                "start_ts": start,
                "end_ts": start + 3600,
                "event_type": et,
            })).unwrap(),
        }
    };

    let res1 = agent.invoke(add_call("Internal", EventType::Internal, now + 1800), &ctx).await?; // +30 min
    let res2 = agent.invoke(add_call("External", EventType::External, now + 3600), &ctx).await?; // +1 hour
    let res3 = agent.invoke(add_call("Personal", EventType::Personal, now + 7200), &ctx).await?; // +2 hours

    assert!(res1.payload.len() > 0);
    assert!(res2.payload.len() > 0);
    assert!(res3.payload.len() > 0);

    // 6. Invoke list
    let list_res = agent.invoke(AgentCall {
        method: "list".to_string(),
        payload: serde_json::to_vec(&json!({ "limit": 0 })).unwrap(),
    }, &ctx).await?;
    let events: Vec<Event> = serde_json::from_slice(&list_res.payload)?;
    assert_eq!(events.len(), 3);
    assert!(events[0].start_ts <= events[1].start_ts);
    assert!(events[1].start_ts <= events[2].start_ts);

    // 7. Invoke next
    let next_res = agent.invoke(AgentCall {
        method: "next".to_string(),
        payload: vec![],
    }, &ctx).await?;
    let next_event: Event = serde_json::from_slice(&next_res.payload)?;
    assert_eq!(next_event.title, "Internal");

    // 8. Invoke remind
    let remind_res = agent.invoke(AgentCall {
        method: "remind".to_string(),
        payload: serde_json::to_vec(&json!({ "within_secs": 7200 })).unwrap(),
    }, &ctx).await?;
    let remind_payload: serde_json::Value = serde_json::from_slice(&remind_res.payload)?;
    let due = remind_payload["due"].as_array().unwrap();

    // Internal event starts at now + 1800 (30 min). Offset is 30 min. now + 30m >= now + 30m. Should be due.
    // External event starts at now + 3600 (1h). Offset is 90 min. now + 90m >= now + 1h. Should be due.
    // Personal event starts at now + 7200 (2h). Default offset is 5 min. now + 5m < now + 2h. Should NOT be due.
    assert!(due.iter().any(|e| e["title"] == "Internal"));
    assert!(due.iter().any(|e| e["title"] == "External"));
    assert!(!due.iter().any(|e| e["title"] == "Personal"));

    // 9. Invoke nl_query
    let nl_res = agent.invoke(AgentCall {
        method: "nl_query".to_string(),
        payload: serde_json::to_vec(&json!({ "question": "what's on?" })).unwrap(),
    }, &ctx).await?;
    let nl_text = String::from_utf8(nl_res.payload)?;
    assert!(nl_text.contains("LLM response"));

    Ok(())
}

#[tokio::test]
async fn test_calendar_capability_denies_llm_without_declaration() {
    let store = Arc::new(MockCalendarStore::new());
    let agent = CalendarAgent::new("cal", store);

    // Manifest WITHOUT llm: true
    let mut manifest = agent.manifest().clone();
    manifest.capabilities.llm = false;

    // Try to enforce LlmCall with this restricted manifest
    let res = ybos_orchestrator::capability::enforce(&manifest, &Operation::LlmCall);
    assert!(res.is_err());
}
