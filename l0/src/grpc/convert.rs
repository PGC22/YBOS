//! Mapper `hw::TelemetrySnapshot` → `pb::TelemetryFrame`.
//!
//! Cimpurile lipsa din snapshot (ex: `disk_*`, `cpu_temp_pkg_c` cand nu avem
//! thermal zones, `power_draw_w` cand kernel-ul nu expune) primesc 0.0.

use super::pb::{BatteryInfo, TelemetryFrame, ThermalZone as PbThermalZone};
use crate::hw;

const KB_PER_GB: f64 = 1024.0 * 1024.0;

pub fn snapshot_to_frame(s: &hw::TelemetrySnapshot) -> TelemetryFrame {
    let cpu_percent = s
        .cpu
        .as_ref()
        .and_then(|c| c.usage_percent)
        .unwrap_or(0.0);

    let cpu_temp_pkg_c = s
        .thermal
        .iter()
        .map(|z| z.temp_c)
        .fold(f64::NEG_INFINITY, f64::max);
    let cpu_temp_pkg_c = if cpu_temp_pkg_c.is_finite() {
        cpu_temp_pkg_c
    } else {
        0.0
    };

    let (ram_percent, ram_available_gb) = match &s.mem {
        Some(m) => (m.used_percent, (m.available_kb as f64) / KB_PER_GB),
        None => (0.0, 0.0),
    };

    let battery = s.batteries.first().map(|b| BatteryInfo {
        capacity_percent: b.capacity_percent,
        status: b.status.clone(),
        plugged: b.plugged,
        power_draw_w: b.power_draw_w.unwrap_or(0.0),
    });

    let thermal = s
        .thermal
        .iter()
        .map(|z| PbThermalZone {
            name: z.name.clone(),
            temp_c: z.temp_c,
            throttled: false,
        })
        .collect();

    TelemetryFrame {
        timestamp_ms: s.timestamp_ms,
        cpu_percent,
        cpu_temp_pkg_c,
        ram_percent,
        ram_available_gb,
        disk_percent: 0.0,
        disk_free_gb: 0.0,
        battery,
        thermal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::{BatteryStats, CpuStats, MemoryStats, TelemetrySnapshot, ThermalZone};

    #[test]
    fn empty_snapshot_maps_to_zeros() {
        let snap = TelemetrySnapshot {
            timestamp_ms: 42,
            ..Default::default()
        };
        let frame = snapshot_to_frame(&snap);
        assert_eq!(frame.timestamp_ms, 42);
        assert_eq!(frame.cpu_percent, 0.0);
        assert_eq!(frame.cpu_temp_pkg_c, 0.0);
        assert_eq!(frame.ram_percent, 0.0);
        assert_eq!(frame.disk_percent, 0.0);
        assert!(frame.battery.is_none());
        assert!(frame.thermal.is_empty());
    }

    #[test]
    fn full_snapshot_maps_fields() {
        let snap = TelemetrySnapshot {
            timestamp_ms: 1000,
            cpu: Some(CpuStats {
                usage_percent: Some(37.5),
                core_count: 4,
                freq_mhz_avg: Some(2400.0),
            }),
            mem: Some(MemoryStats {
                total_kb: 8 * 1024 * 1024,
                available_kb: 4 * 1024 * 1024,
                used_percent: 50.0,
            }),
            batteries: vec![BatteryStats {
                name: "BAT0".into(),
                capacity_percent: 85,
                status: "Discharging".into(),
                plugged: false,
                power_draw_w: Some(12.3),
            }],
            thermal: vec![
                ThermalZone {
                    name: "x86_pkg_temp".into(),
                    temp_c: 64.5,
                },
                ThermalZone {
                    name: "acpitz".into(),
                    temp_c: 51.0,
                },
            ],
            backlight: None,
        };
        let frame = snapshot_to_frame(&snap);
        assert_eq!(frame.timestamp_ms, 1000);
        assert!((frame.cpu_percent - 37.5).abs() < 1e-9);
        assert!((frame.cpu_temp_pkg_c - 64.5).abs() < 1e-9);
        assert!((frame.ram_percent - 50.0).abs() < 1e-9);
        assert!((frame.ram_available_gb - 4.0).abs() < 1e-9);

        let bat = frame.battery.expect("battery mapped");
        assert_eq!(bat.capacity_percent, 85);
        assert_eq!(bat.status, "Discharging");
        assert!(!bat.plugged);
        assert!((bat.power_draw_w - 12.3).abs() < 1e-9);

        assert_eq!(frame.thermal.len(), 2);
        assert_eq!(frame.thermal[0].name, "x86_pkg_temp");
    }
}
