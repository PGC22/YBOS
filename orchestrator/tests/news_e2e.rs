use std::sync::Arc;
use ybos_orchestrator::agent::{Agent, AgentCall, AgentContext};
use ybos_orchestrator::agents::news::NewsAgent;
use ybos_orchestrator::http::{HttpResponse, MockHttpClient};
use ybos_inference::mock::MockInference;
use ybos_memory::{MockVectorStore, MockEmbedder};

#[tokio::test]
async fn test_news_agent_mock_e2e() {
    let rss_fixture = r#"
        <rss version="2.0">
            <channel>
                <title>Mock News</title>
                <item>
                    <title>Breaking News</title>
                    <description>Something happened today.</description>
                    <link>http://example.com/1</link>
                </item>
            </channel>
        </rss>
    "#;

    let mock_http = Arc::new(MockHttpClient::new(vec![
        ("http://example.com/rss".to_string(), HttpResponse {
            status: 200,
            headers: vec![],
            body: rss_fixture.as_bytes().to_vec(),
        })
    ]));

    let inference = Arc::new(MockInference::new(vec!["Summary of breaking news.".to_string()]));
    let memory = Arc::new(MockVectorStore::new());
    let embedder = Arc::new(MockEmbedder::new(8));

    let ctx = AgentContext {
        inference,
        memory,
        embedder,
        http: mock_http,
    };

    let agent = NewsAgent::new("news-agent", vec!["http://example.com/rss".to_string()]);

    // 1. Fetch
    let resp = agent.invoke(AgentCall {
        method: "fetch".to_string(),
        payload: vec![],
    }, &ctx).await.expect("Fetch failed");
    assert!(String::from_utf8_lossy(&resp.payload).contains("Fetched 1 items"));

    // 2. Query
    let query_payload = serde_json::to_vec(&serde_json::json!({
        "query": "something",
        "k": 1
    })).unwrap();
    let resp = agent.invoke(AgentCall {
        method: "query".to_string(),
        payload: query_payload,
    }, &ctx).await.expect("Query failed");

    let matches: Vec<ybos_memory::VectorMatch> = serde_json::from_slice(&resp.payload).unwrap();
    assert_eq!(matches.len(), 1);
    assert!(matches[0].text.contains("Breaking News"));

    // 3. Summarize
    let resp = agent.invoke(AgentCall {
        method: "summarize".to_string(),
        payload: vec![],
    }, &ctx).await.expect("Summarize failed");
    assert_eq!(String::from_utf8_lossy(&resp.payload), "Summary of breaking news.");
}

#[cfg(feature = "real_rss")]
#[tokio::test]
async fn test_news_agent_real_rss_smoke() {
    // BBC World News RSS is generally very stable.
    let source_url = "https://feeds.bbci.co.uk/news/world/rss.xml";

    let http = Arc::new(ybos_orchestrator::http::HyperHttpClient::new());
    let inference = Arc::new(MockInference::new(vec!["Real news summary".to_string()]));

    // Using real SqliteVecStore + FastEmbed for a proper smoke test if features are enabled,
    // but e2e spec says "Mocks default" and "SqliteVecStore + FastEmbedEmbedder for memory if you want (or mocks)".
    // Let's use SqliteVecStore and FastEmbed if possible to make it a "real" smoke test.

    #[cfg(all(feature = "sqlite_vec", feature = "fastembed"))]
    let (memory, embedder) = {
        let memory: Arc<dyn ybos_memory::VectorStore> = Arc::new(ybos_memory::SqliteVecStore::in_memory(384).unwrap());
        let embedder: Arc<dyn ybos_memory::Embedder> = Arc::new(ybos_memory::FastEmbedEmbedder::load(None).unwrap());
        (memory, embedder)
    };

    #[cfg(not(all(feature = "sqlite_vec", feature = "fastembed")))]
    let (memory, embedder) = {
        let memory: Arc<dyn ybos_memory::VectorStore> = Arc::new(MockVectorStore::new());
        let embedder: Arc<dyn ybos_memory::Embedder> = Arc::new(MockEmbedder::new(384));
        (memory, embedder)
    };

    let ctx = AgentContext {
        inference,
        memory,
        embedder,
        http,
    };

    let agent = NewsAgent::new("news-real", vec![source_url.to_string()]);

    let resp = agent.invoke(AgentCall {
        method: "fetch".to_string(),
        payload: vec![],
    }, &ctx).await.expect("Real fetch failed");

    let resp_text = String::from_utf8_lossy(&resp.payload);
    println!("Real RSS fetch result: {}", resp_text);

    // We expect at least one item
    assert!(resp_text.contains("Fetched") && !resp_text.contains("Fetched 0 items"));
}
