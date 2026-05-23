use anyhow::Result;
use std::sync::Arc;
use tracing::info;
use ybos_orchestrator::agent::AgentContext;
use ybos_orchestrator::registry::AgentRegistry;
use ybos_orchestrator::runtime::InProcessRuntime;
use ybos_inference::mock::MockInference;
use ybos_inference::Inference;
use ybos_memory::{MockVectorStore, MockEmbedder, VectorStore, Embedder};
use ybos_user_context::{MockUserContextStore, UserContextStore};
use ybos_calendar::{MockCalendarStore, CalendarStore};
use ybos_orchestrator::http::{HyperHttpClient, HttpClient};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("YBOS Orchestrator starting...");

    let inference: Arc<dyn Inference> = Arc::new(MockInference::new(vec!["42".to_string()]));
    let memory: Arc<dyn VectorStore> = Arc::new(MockVectorStore::new());
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(384));
    let http: Arc<dyn HttpClient> = Arc::new(HyperHttpClient::new());
    let user_context: Arc<dyn UserContextStore> = Arc::new(MockUserContextStore::new());
    let _calendar_store: Arc<dyn CalendarStore> = Arc::new(MockCalendarStore::new());
    let context = AgentContext { inference, memory, embedder, http, user_context };

    let registry = Arc::new(AgentRegistry::new());
    let _runtime = InProcessRuntime::new(registry, context);

    // Minimal daemon loop
    tokio::signal::ctrl_c().await?;
    info!("YBOS Orchestrator shutting down.");
    Ok(())
}
