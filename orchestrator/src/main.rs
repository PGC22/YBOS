use anyhow::Result;
use std::sync::Arc;
use tracing::info;
use ybos_orchestrator::agent::AgentContext;
use ybos_orchestrator::registry::AgentRegistry;
use ybos_orchestrator::runtime::InProcessRuntime;
use ybos_inference::mock::MockInference;
use ybos_inference::Inference;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("YBOS Orchestrator starting...");

    let inference: Arc<dyn Inference> = Arc::new(MockInference::new(vec!["42".to_string()]));
    let context = AgentContext { inference };

    let registry = Arc::new(AgentRegistry::new());
    let _runtime = InProcessRuntime::new(registry, context);

    // Minimal daemon loop
    tokio::signal::ctrl_c().await?;
    info!("YBOS Orchestrator shutting down.");
    Ok(())
}
