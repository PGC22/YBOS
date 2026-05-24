use serde_json::json;
use std::sync::Arc;
use ybos_inference::MockInference;
use ybos_memory::{MockVectorStore, MockEmbedder};
use ybos_user_context::{MockUserContextStore, ContextCategory, ContextEntry, UserContextStore};
use ybos_orchestrator::agent::{Agent, AgentCall, AgentContext};
use ybos_orchestrator::agents::trip::TripPlannerAgent;
use ybos_orchestrator::http::{MockHttpClient, HttpResponse};
use ybos_orchestrator::capability::{Operation, enforce};

#[tokio::test]
async fn test_trip_planner_mock_e2e() {
    let uc = MockUserContextStore::new();
    uc.put(ContextEntry {
        id: uuid::Uuid::new_v4(),
        category: ContextCategory::Preference,
        key: "travel.airline".to_string(),
        value: json!("Lufthansa"),
        note: None,
        confidence: 1.0,
        created_at: 0,
        updated_at: 0,
    }).await.unwrap();
    uc.put(ContextEntry {
        id: uuid::Uuid::new_v4(),
        category: ContextCategory::Preference,
        key: "travel.seat".to_string(),
        value: json!("aisle"),
        note: None,
        confidence: 1.0,
        created_at: 0,
        updated_at: 0,
    }).await.unwrap();

    let http = MockHttpClient::new(vec![
        ("http://example.com/advisories".to_string(), HttpResponse {
            status: 200,
            headers: vec![],
            body: r#"
                <rss version="2.0">
                  <channel>
                    <title>Travel Advisories</title>
                    <item>
                      <title>TXL airport reopens for civil flights</title>
                      <description>Berlin Brandenburg (BER, ex-TXL area) ...</description>
                      <link>http://example.com/1</link>
                    </item>
                  </channel>
                </rss>
            "#.as_bytes().to_vec(),
        })
    ]);

    let inference = Arc::new(MockInference::new(vec![
        "- Direct LH to BER\n- Hotel near Alexanderplatz\n- Day 1: museum island\n- Avoid taxis from BER\n- Pack layers".into()
    ]));

    let memory = Arc::new(MockVectorStore::new());
    let embedder = Arc::new(MockEmbedder::new(8));

    let ctx = AgentContext {
        inference,
        memory,
        embedder,
        http: Arc::new(http),
        user_context: Arc::new(uc),
    };

    let agent = TripPlannerAgent::new("trip", vec!["http://example.com/advisories".to_string()]);

    // 1. Plan
    let res = agent.invoke(AgentCall {
        method: "plan".to_string(),
        payload: serde_json::to_vec(&json!({
            "destination": "TXL",
            "depart_date": "2026-06-15",
            "purpose": "business"
        })).unwrap(),
    }, &ctx).await.expect("plan failed");

    let res_json: serde_json::Value = serde_json::from_slice(&res.payload).unwrap();
    assert!(res_json.get("trip_id").is_some());
    assert_eq!(res_json.get("brief").unwrap().as_str().unwrap(),
        "- Direct LH to BER\n- Hotel near Alexanderplatz\n- Day 1: museum island\n- Avoid taxis from BER\n- Pack layers");

    // 2. List
    let res = agent.invoke(AgentCall {
        method: "list".to_string(),
        payload: vec![],
    }, &ctx).await.expect("list failed");

    let list_json: serde_json::Value = serde_json::from_slice(&res.payload).unwrap();
    assert_eq!(list_json.as_array().unwrap().len(), 1);
    assert_eq!(list_json[0]["metadata"]["destination"], "TXL");

    // 3. Recall
    let res = agent.invoke(AgentCall {
        method: "recall".to_string(),
        payload: serde_json::to_vec(&json!({
            "query": "Berlin"
        })).unwrap(),
    }, &ctx).await.expect("recall failed");

    let recall_json: serde_json::Value = serde_json::from_slice(&res.payload).unwrap();
    assert!(recall_json.as_array().unwrap().len() >= 1);
    assert_eq!(recall_json[0]["metadata"]["type"], "trip");
}

#[tokio::test]
async fn test_trip_capability_denies_net_without_declaration() {
    let agent = TripPlannerAgent::new("trip", vec!["http://example.com/advisories".to_string()]);
    let mut manifest = agent.manifest().clone();
    manifest.capabilities.net_domains = vec![];

    let op = Operation::NetConnect("example.com".into());
    let res = enforce(&manifest, &op);
    assert!(res.is_err(), "Should deny connect when domain is not in manifest");
}
