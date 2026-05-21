//! Remus L0 — The Reflex Layer.
//!
//! Daemon kernel-adjacent care:
//!   - verifica identitatea (HMAC pe identity_core.bin) la boot
//!   - citeste hardware direct (/sys, ACPI, cpufreq) [linux only]
//!   - publica telemetrie pe MQTT (`remus/telemetry/*`)
//!   - expune gRPC (IdentityService, TelemetryService) catre L1 (Python)
//!   - executa reflexe sub-ms (CPU throttle, fan curve, brightness)
//!
//! Vezi `docs/L0_SACRED.md`, CLAUDE.md §2 (arhitectura 3-layer brain),
//! CLAUDE.md §4 (decizii arhitecturale), Faza 6 din §6 (acest crate).
//!
//! Sub-sprints:
//!   S6.0  Scaffold (in lucru)
//!   S6.1  Identity + boot integrity (portat din core/paths.py)
//!   S6.2  HAL trait + telemetrie statica
//!   S6.3  MQTT broker (rumqttd embedded)
//!   S6.4  gRPC server (tonic)
//!   S6.5  Reflex actions
//!   S6.6  Python L1 client (`core/l0_client.py`)
//!   S6.7  systemd service + NixOS integration

use anyhow::Result;
use tracing::{info, warn};

mod bus;
mod grpc;
mod hw;
mod identity;
mod reflex;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    // Logging structurat. RUST_LOG=info,remus_l0=debug pentru verbose.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    info!("[L0] Remus Reflex Layer v{} starting", env!("CARGO_PKG_VERSION"));

    // ── Boot sequence ────────────────────────────────────────────────────────
    // S6.1 va popula real. Acum doar log placeholders.
    identity::boot_check().await?;
    hw::init_hal().await?;
    bus::start_mqtt_broker().await?;
    grpc::serve().await?;

    info!("[L0] Boot complete. Entering reflex loop.");

    // ── Reflex loop ──────────────────────────────────────────────────────────
    // Asculta semnale OS, ruleaza telemetrie + reflexe.
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
