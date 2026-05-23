use ybos_user_context::{MockUserContextStore, UserContextStore, ContextEntry, ContextCategory, ContextQuery};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_mock_store_roundtrip() {
    let store = MockUserContextStore::new();

    let entry = ContextEntry {
        id: Uuid::new_v4(),
        category: ContextCategory::Preference,
        key: "news.pref".to_string(),
        value: json!({"topic": "tech"}),
        note: Some("User likes tech".to_string()),
        confidence: 1.0,
        created_at: 100,
        updated_at: 100,
    };

    // Put
    store.put(entry.clone()).await.unwrap();
    assert_eq!(store.count().await.unwrap(), 1);

    // Get
    let retrieved = store.get(ContextCategory::Preference, "news.pref").await.unwrap().unwrap();
    assert_eq!(retrieved.key, "news.pref");
    assert_eq!(retrieved.value, json!({"topic": "tech"}));

    // Update
    let mut updated = entry.clone();
    updated.value = json!({"topic": "science"});
    updated.updated_at = 200;
    store.put(updated).await.unwrap();

    assert_eq!(store.count().await.unwrap(), 1);
    let retrieved2 = store.get(ContextCategory::Preference, "news.pref").await.unwrap().unwrap();
    assert_eq!(retrieved2.value, json!({"topic": "science"}));
    assert_eq!(retrieved2.created_at, 100);
    assert_eq!(retrieved2.updated_at, 200);

    // Query
    let results = store.query(ContextQuery {
        category: Some(ContextCategory::Preference),
        key_prefix: Some("news.".to_string()),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(results.len(), 1);

    // Delete
    let deleted = store.delete(ContextCategory::Preference, "news.pref").await.unwrap();
    assert!(deleted);
    assert_eq!(store.count().await.unwrap(), 0);
}
