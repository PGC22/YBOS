use ybos_memory::{MockVectorStore, MockEmbedder, VectorStore, Embedder, VectorItem, VectorQuery};
use serde_json::json;

#[tokio::test]
async fn test_mock_store_and_embedder_smoke() {
    let embedder = MockEmbedder::new(8);
    let store = MockVectorStore::new();

    let text1 = "The meaning of life is 42";
    let text2 = "Hello world";

    let emb1 = embedder.embed(text1).await.unwrap();
    let emb2 = embedder.embed(text2).await.unwrap();

    assert_eq!(emb1.len(), 8);
    assert_eq!(emb2.len(), 8);
    assert_ne!(emb1, emb2);

    let id1 = store.insert(VectorItem {
        embedding: emb1.clone(),
        text: text1.to_string(),
        metadata: json!({"key": "val1"}),
    }).await.unwrap();

    let id2 = store.insert(VectorItem {
        embedding: emb2.clone(),
        text: text2.to_string(),
        metadata: json!({"key": "val2"}),
    }).await.unwrap();

    assert_eq!(store.count().await.unwrap(), 2);

    // Query for text1
    let matches = store.query_top_k(VectorQuery { embedding: emb1 }, 1).await.unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id, id1);
    assert_eq!(matches[0].text, text1);
    assert!(matches[0].score > 0.99);

    // Delete text2
    store.delete(id2).await.unwrap();
    assert_eq!(store.count().await.unwrap(), 1);
}
