//! Reflex actions — raspunsuri sub-ms la conditii hardware.
//!
//! Reguli "instinctuale" pe care L0 le poate aplica fara sa consulte L1/L2:
//!   - CPU temp > 90C → throttle imediat (cpufreq → powersave)
//!   - battery < 5% → suspend forced
//!   - thermal throttling activ persistent → fan curve agresiva
//!   - lid close → suspend (politica configurabila)
//!
//! Status: PLACEHOLDER pentru S6.0. Implementare in S6.5.

use anyhow::Result;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::hw;

/// Loop principal de reflexe — ruleaza cat timp daemonul e activ.
///
/// La fiecare tick citeste un snapshot din HAL si logheaza un sumar.
/// S6.3 va publica pe MQTT, S6.5 va aplica reguli de reflex.
pub async fn run() -> Result<()> {
    info!("[L0/reflex] Reflex loop starting (tick = 5s)");
    let mut ticker = tokio::time::interval(Duration::from_secs(5));
    // Primul tick e imediat — il ignoram pentru a permite HAL warmup.
    ticker.tick().await;

    loop {
        ticker.tick().await;
        match hw::snapshot() {
            Ok(snap) => {
                log_snapshot(&snap);
                if let Err(e) = crate::bus::publish_snapshot(&snap).await {
                    warn!("[L0/reflex] mqtt publish failed: {}", e);
                }
            }
            Err(e) => warn!("[L0/reflex] snapshot failed: {}", e),
        }
    }
}

fn log_snapshot(s: &hw::TelemetrySnapshot) {
    // Format compact pe o linie — verbose la nevoie cu RUST_LOG=debug
    let cpu_pct = s
        .cpu
        .as_ref()
        .and_then(|c| c.usage_percent)
        .map(|p| format!("{:.1}%", p))
        .unwrap_or_else(|| "?".to_string());

    let cpu_freq = s
        .cpu
        .as_ref()
        .and_then(|c| c.freq_mhz_avg)
        .map(|f| format!("{:.0}MHz", f))
        .unwrap_or_else(|| "?".to_string());

    let mem_pct = s
        .mem
        .as_ref()
        .map(|m| format!("{:.1}%", m.used_percent))
        .unwrap_or_else(|| "?".to_string());

    let bat = if let Some(b) = s.batteries.first() {
        format!(
            "{}={}% ({})",
            b.name,
            b.capacity_percent,
            if b.plugged { "plugged" } else { "discharging" }
        )
    } else {
        "no battery".to_string()
    };

    let thermal_max = s
        .thermal
        .iter()
        .map(|z| z.temp_c)
        .fold(f64::NEG_INFINITY, f64::max);
    let thermal_str = if thermal_max.is_finite() {
        format!("{:.1}C", thermal_max)
    } else {
        "?".to_string()
    };

    info!(
        "[L0/reflex] cpu={} @ {} | mem={} | thermal_max={} | bat={}",
        cpu_pct, cpu_freq, mem_pct, thermal_str, bat
    );
    debug!("[L0/reflex] full snapshot: {:?}", s);
}
