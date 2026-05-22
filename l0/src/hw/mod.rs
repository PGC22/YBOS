//! Hardware Abstraction Layer.
//!
//! L0 vorbeste cu hardware-ul prin acest layer. Implementarile concrete sunt
//! Linux-only (citire `/sys/`, `/proc/`, ACPI). Pe alte OS-uri folosim stub
//! care returneaza un snapshot gol (doar timestamp).
//!
//! Pattern: `Hal` struct cu state intern (delta CPU) + singleton via OnceLock.
//! API public: `init_hal()`, `snapshot()`, `current_telemetry()`.

use anyhow::{anyhow, Result};
use serde::Serialize;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod parse;

#[cfg(target_os = "linux")]
mod linux;

// ─────────────────────────────────────────────────────────────────────────────
// Telemetry data structures
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize)]
pub struct TelemetrySnapshot {
    pub timestamp_ms: u64,
    pub cpu: Option<CpuStats>,
    pub mem: Option<MemoryStats>,
    pub batteries: Vec<BatteryStats>,
    pub thermal: Vec<ThermalZone>,
    pub backlight: Option<BacklightStats>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CpuStats {
    /// Procent utilizat din momentul ultimei masurari (delta-based).
    /// Pentru prima masurare e None (nu avem cu ce compara).
    pub usage_percent: Option<f64>,
    pub core_count: usize,
    /// Frecventa medie curenta MHz (din scaling_cur_freq).
    pub freq_mhz_avg: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MemoryStats {
    pub total_kb: u64,
    pub available_kb: u64,
    pub used_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatteryStats {
    pub name: String,
    pub capacity_percent: u32,
    /// Charging / Discharging / Full / Not charging / Unknown
    pub status: String,
    pub plugged: bool,
    /// Watts curent (pozitiv = discharging, negativ = charging). Opțional.
    pub power_draw_w: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThermalZone {
    pub name: String,
    pub temp_c: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacklightStats {
    pub current: u32,
    pub max: u32,
    pub percent: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// CPU delta state (pentru calcul usage_percent intre tick-uri)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CpuTimes {
    pub total: u64,
    pub idle: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Hal singleton
// ─────────────────────────────────────────────────────────────────────────────

pub struct Hal {
    last_cpu: Option<CpuTimes>,
}

impl Hal {
    fn new() -> Self {
        Self { last_cpu: None }
    }

    /// Citeste un snapshot complet. Mutable pentru a actualiza state-ul delta CPU.
    pub fn snapshot(&mut self) -> TelemetrySnapshot {
        let ts = now_ms();

        #[cfg(target_os = "linux")]
        {
            linux::read_snapshot(self, ts)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = &mut self.last_cpu; // suprimam unused-mut warning
            TelemetrySnapshot {
                timestamp_ms: ts,
                ..Default::default()
            }
        }
    }
}

static HAL: OnceLock<Mutex<Hal>> = OnceLock::new();

/// Initializeaza HAL singleton. Logghea diagnostic.
pub async fn init_hal() -> Result<()> {
    HAL.set(Mutex::new(Hal::new()))
        .map_err(|_| anyhow!("HAL already initialized"))?;

    tracing::info!("[L0/hw] HAL initialized");

    #[cfg(target_os = "linux")]
    tracing::info!("[L0/hw] Linux backend - reading /sys, /proc, ACPI");
    #[cfg(not(target_os = "linux"))]
    tracing::info!(
        "[L0/hw] Non-Linux backend - stub snapshot with timestamp only. \
         Run on Linux for real telemetry."
    );

    Ok(())
}

/// Citeste un snapshot proaspat. Apelat de reflex loop la fiecare tick si
/// de MQTT publisher (S6.3).
pub fn snapshot() -> Result<TelemetrySnapshot> {
    let mutex = HAL
        .get()
        .ok_or_else(|| anyhow!("HAL nu este initializat. Apeleaza init_hal() la boot."))?;
    let mut hal = mutex
        .lock()
        .map_err(|e| anyhow!("HAL mutex poisoned: {}", e))?;
    Ok(hal.snapshot())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
