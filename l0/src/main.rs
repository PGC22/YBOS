//! YBOS L0 - The Reflex Layer.
//!
//! Daemon kernel-adjacent care:
//!   - verifica integritatea L0 sacred la boot
//!   - citeste hardware direct (/sys, ACPI, cpufreq) [linux only]
//!   - publica telemetrie pe MQTT (`ybos/telemetry/*`)
//!   - expune gRPC (IdentityService, TelemetryService) catre L1
//!   - executa reflexe sub-ms (CPU throttle, fan curve, brightness)
//!
//! Vezi `docs/L0_SACRED.md` si `docs/ARCHITECTURE.md`.

use anyhow::Result;
use tracing::{info, warn};

mod bus;
mod grpc;
mod hw;
mod identity;
mod reflex;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    // Structured logging. RUST_LOG=info,ybos_l0=debug for verbose output.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    info!(
        "[L0] YBOS Reflex Layer v{} starting",
        env!("CARGO_PKG_VERSION")
    );

    // Boot sequence.
    identity::boot_check().await?;
    hw::init_hal().await?;
    bus::start_mqtt_broker().await?;
    grpc::serve().await?;

    info!("[L0] Boot complete. Entering reflex loop.");

    // Reflex loop.
    tokio::select! {
        res = reflex::run() => {
            if let Err(e) = res {
                warn!("[L0] Reflex loop exited: {e:?}");
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("[L0] SIGINT received, shutting down");
        }
    }

    // Best-effort shutdown gracious — publish offline retain.
    bus::announce_offline().await;

    info!("[L0] Goodbye.");
    Ok(())
}
