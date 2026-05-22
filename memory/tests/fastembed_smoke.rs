use ybos_memory::{FastEmbedEmbedder, SqliteVecStore, VectorStore, Embedder, VectorItem, VectorQuery};
use serde_json::json;
use std::path::PathBuf;

#[tokio::test]
#[cfg(all(feature = "fastembed", feature = "sqlite_vec"))]
async fn test_fastembed_and_sqlite_vec_smoke() {
    let cache_dir = PathBuf::from("target/test-models/fastembed");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let embedder = FastEmbedEmbedder::load(None, Some(cache_dir)).unwrap();
    let dimension = embedder.dimension();
    let store = SqliteVecStore::in_memory(dimension).unwrap();

    let text1 = "The meaning of life is 42";
    let text2 = "Hello world";

    let emb1 = embedder.embed(text1).await.unwrap();
    let emb2 = embedder.embed(text2).await.unwrap();

    assert_eq!(emb1.len(), dimension);
    assert_ne!(emb1, emb2);

    let id1 = store.insert(VectorItem {
        embedding: emb1.clone(),
        text: text1.to_string(),
        metadata: json!({"agent": "test"}),
    }).await.unwrap();

    let id2 = store.insert(VectorItem {
        embedding: emb2.clone(),
        text: text2.to_string(),
        metadata: json!({"agent": "test"}),
    }).await.unwrap();

    assert_eq!(store.count().await.unwrap(), 2);

    // Query for text1
    let matches = store.query_top_k(VectorQuery { embedding: emb1 }, 1).await.unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id, id1);
    assert_eq!(matches[0].text, text1);
    assert!(matches[0].score > 0.9);

    // Delete text2
    store.delete(id2).await.unwrap();
    assert_eq!(store.count().await.unwrap(), 1);
}
