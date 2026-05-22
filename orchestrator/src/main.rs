use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("YBOS Orchestrator starting...");

    // Minimal daemon loop
    tokio::signal::ctrl_c().await?;
    info!("YBOS Orchestrator shutting down.");
    Ok(())
}
