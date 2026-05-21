//! MQTT bus — broker embedded `rumqttd` + publisher local.
//!
//! Topics:
//!   - `remus/status` (retain) — "online" / "offline"
//!   - `remus/telemetry/cpu` — CpuStats JSON
//!   - `remus/telemetry/mem` — MemoryStats JSON
//!   - `remus/telemetry/battery` — array BatteryStats JSON
//!   - `remus/telemetry/thermal` — array ThermalZone JSON
//!   - `remus/telemetry/backlight` — BacklightStats JSON
//!   - `remus/telemetry/full` — TelemetrySnapshot complet JSON
//!   - `remus/hw/event` — udev events (Faza 7, neimplementat)
//!
//! Bind doar pe `127.0.0.1:1883`. Auth + TLS — S6.x ulterior (cand permitem
//! peer multi-body pe alt device).

use anyhow::Result;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{info, warn};

mod broker;
mod publisher;
pub mod topics;

pub use publisher::Publisher;

static PUBLISHER: OnceLock<Publisher> = OnceLock::new();

/// Porneste broker + publisher local. Publica retain "online" pe remus/status.
pub async fn start_mqtt_broker() -> Result<()> {
    broker::spawn()?;

    // Asteapta scurt ca broker-ul sa fie ready inainte de connect client.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let pub_ = Publisher::connect()?;
    // Mic delay ca event loop sa stabileasca conexiunea.
    tokio::time::sleep(Duration::from_millis(200)).await;

    if let Err(e) = pub_.announce_online().await {
        warn!("[L0/bus] announce online failed: {}", e);
    }

    if PUBLISHER.set(pub_).is_err() {
        warn!("[L0/bus] publisher already initialized");
    }

    info!("[L0/bus] MQTT online ({})", broker::BROKER_LISTEN);
    Ok(())
}

/// Publica un snapshot pe topic-urile granulare. Apelata de reflex loop.
pub async fn publish_snapshot(snap: &crate::hw::TelemetrySnapshot) -> Result<()> {
    let pub_ = PUBLISHER
        .get()
        .ok_or_else(|| anyhow::anyhow!("publisher uninitialized"))?;
    pub_.publish_snapshot(snap).await
}

/// Anunta offline (retain) inainte de exit. Best-effort.
pub async fn announce_offline() {
    if let Some(p) = PUBLISHER.get() {
        if let Err(e) = p.announce_offline().await {
            warn!("[L0/bus] announce offline failed: {}", e);
        }
    }
}
