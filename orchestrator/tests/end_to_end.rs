use std::time::Duration;
use std::sync::Arc;
use uuid::Uuid;
use tonic::transport::Server;
use std::net::SocketAddr;

use ybos_orchestrator::l0_client::L0Client;
use ybos_orchestrator::registry::AgentRegistry;
use ybos_orchestrator::runtime::{InProcessRuntime, AgentRuntime};
use ybos_orchestrator::agent::{Agent, AgentCall};
use ybos_orchestrator::agents::hello::HelloAgent;
use ybos_orchestrator::capability::{enforce, Operation};
use ybos_orchestrator::manifest::Manifest;
use ybos_l0::identity::session;
use ybos_l0::identity::envelope::MasterKey;
use ybos_l0::grpc::pb::session_service_server::SessionServiceServer;
use ybos_l0::grpc::session_service::SessionSvc;

#[tokio::test]
async fn test_end_to_end_orchestrator_l0() {
    // 1. Initialize L0 session API in-process
    session::init_session_api(MasterKey::from_bytes([0u8; 32])).expect("Failed to init session API");

    // 2. Spawn a minimal L0 SessionService on a random port
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let session_svc = SessionSvc::default();

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let server_addr = format!("http://{}", local_addr);

    tokio::spawn(async move {
        Server::builder()
            .add_service(SessionServiceServer::new(session_svc))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    // 3. Connect via gRPC
    let l0_client = L0Client::connect(&server_addr).await.expect("Failed to connect to L0");

    // 4. Call l0_client.issue_session_token(...)
    let token = l0_client.issue_session_token(
        vec!["test.cap".to_string()],
        Duration::from_secs(3600),
        [1u8; 32]
    ).await.expect("Failed to issue session token");

    assert_eq!(token.key_bytes.len(), 32);
    Uuid::parse_str(&token.session_id).expect("Invalid session ID UUID");

    // 5. Build an AgentRegistry, register HelloAgent statically
    let registry = Arc::new(AgentRegistry::new());
    let hello = Arc::new(HelloAgent::new("hello", None));
    registry.register_static(hello.clone());

    // register a second agent from a manifest.toml string at runtime
    let second_agent_toml = r#"
name = "runtime-agent"
version = "0.1.0"
[capabilities]
net_domains = ["example.com"]
"#;
    registry.register_runtime(second_agent_toml, Box::new(|| {
        let manifest: Manifest = toml::from_str(second_agent_toml).unwrap();
        Arc::new(HelloAgent::new("runtime-agent", Some(manifest)))
    })).expect("Failed to register runtime agent");

    assert_eq!(registry.list().len(), 2);

    // 6. Invoke hello and assert the response.
    let runtime = InProcessRuntime::new(registry.clone());
    let handle = runtime.spawn(hello.manifest().clone()).await.expect("Failed to spawn hello agent");
    let resp = runtime.invoke(&handle, AgentCall {
        method: "test".to_string(),
        payload: vec![],
    }).await.expect("Failed to invoke hello agent");

    assert_eq!(String::from_utf8_lossy(&resp.payload), "hello from hello");

    // 7. Test capability enforcement
    let manifest = registry.get("runtime-agent").unwrap().manifest().clone();

    // Allowed
    enforce(&manifest, &Operation::NetConnect("example.com".to_string())).expect("Should allow example.com");

    // Denied
    let err = enforce(&manifest, &Operation::NetConnect("evil.com".to_string())).unwrap_err();
    assert!(err.to_string().contains("Capability denied"));
}
