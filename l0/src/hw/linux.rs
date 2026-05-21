//! Linux HAL implementation. Citeste /sys, /proc, ACPI direct.
//!
//! Gated `#[cfg(target_os = "linux")]` din `hw/mod.rs`.
//! Pe Windows acest modul nu se compileaza deloc, deci nu blocheaza `cargo check`.

#![cfg(target_os = "linux")]

use std::fs;
use std::path::{Path, PathBuf};

use super::parse::{
    cpu_usage_delta, is_battery_plugged, microw_to_w, parse_freq_khz, parse_proc_meminfo,
    parse_proc_stat, parse_temp_millicelsius,
};
use super::{
    BacklightStats, BatteryStats, CpuStats, Hal, MemoryStats, TelemetrySnapshot, ThermalZone,
};

const PROC_STAT: &str = "/proc/stat";
const PROC_MEMINFO: &str = "/proc/meminfo";
const SYS_POWER_SUPPLY: &str = "/sys/class/power_supply";
const SYS_THERMAL: &str = "/sys/class/thermal";
const SYS_BACKLIGHT: &str = "/sys/class/backlight";
const SYS_CPUFREQ: &str = "/sys/devices/system/cpu";

pub fn read_snapshot(hal: &mut Hal, timestamp_ms: u64) -> TelemetrySnapshot {
    TelemetrySnapshot {
        timestamp_ms,
        cpu: read_cpu(hal),
        mem: read_memory(),
        batteries: read_batteries(),
        thermal: read_thermal_zones(),
        backlight: read_backlight(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CPU
// ─────────────────────────────────────────────────────────────────────────────

fn read_cpu(hal: &mut Hal) -> Option<CpuStats> {
    let content = fs::read_to_string(PROC_STAT).ok()?;
    let curr = parse_proc_stat(&content)?;

    let usage_percent = hal.last_cpu.and_then(|prev| cpu_usage_delta(prev, curr));
    hal.last_cpu = Some(curr);

    let (core_count, freq_mhz_avg) = read_cpufreq();

    Some(CpuStats {
        usage_percent,
        core_count,
        freq_mhz_avg,
    })
}

/// Returneaza (numar core-uri detectate, frecventa medie MHz).
fn read_cpufreq() -> (usize, Option<f64>) {
    let cpu_dir = Path::new(SYS_CPUFREQ);
    let Ok(entries) = fs::read_dir(cpu_dir) else {
        return (0, None);
    };

    let mut freqs: Vec<f64> = Vec::new();
    let mut cores: usize = 0;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("cpu") {
            continue;
        }
        // accept doar cpuN cu N numeric
        if !name_str["cpu".len()..].chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        cores += 1;
        let path = entry.path().join("cpufreq").join("scaling_cur_freq");
        if let Ok(content) = fs::read_to_string(&path) {
            if let Some(mhz) = parse_freq_khz(&content) {
                freqs.push(mhz);
            }
        }
    }

    let avg = if freqs.is_empty() {
        None
    } else {
        Some(freqs.iter().sum::<f64>() / freqs.len() as f64)
    };
    (cores, avg)
}

// ─────────────────────────────────────────────────────────────────────────────
// Memorie
// ─────────────────────────────────────────────────────────────────────────────

fn read_memory() -> Option<MemoryStats> {
    let content = fs::read_to_string(PROC_MEMINFO).ok()?;
    parse_proc_meminfo(&content)
}

// ─────────────────────────────────────────────────────────────────────────────
// Baterii — /sys/class/power_supply/BAT*
// ─────────────────────────────────────────────────────────────────────────────

fn read_batteries() -> Vec<BatteryStats> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(SYS_POWER_SUPPLY) else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("BAT") {
            continue;
        }
        let path = entry.path();
        if let Some(b) = read_one_battery(&path, &name) {
            out.push(b);
        }
    }
    out
}

fn read_one_battery(dir: &Path, name: &str) -> Option<BatteryStats> {
    let capacity = read_uint_file(&dir.join("capacity"))? as u32;
    let status = read_string_file(&dir.join("status")).unwrap_or_else(|| "Unknown".to_string());
    let plugged = is_battery_plugged(&status);
    let power_draw_w = read_int_file(&dir.join("power_now")).map(microw_to_w);
    Some(BatteryStats {
        name: name.to_string(),
        capacity_percent: capacity.min(100),
        status,
        plugged,
        power_draw_w,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Termal — /sys/class/thermal/thermal_zone*/temp
// ─────────────────────────────────────────────────────────────────────────────

fn read_thermal_zones() -> Vec<ThermalZone> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(SYS_THERMAL) else {
        return out;
    };
    for entry in entries.flatten() {
        let entry_name = entry.file_name().to_string_lossy().to_string();
        if !entry_name.starts_with("thermal_zone") {
            continue;
        }
        let path = entry.path();
        let temp_str = match fs::read_to_string(path.join("temp")) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let Some(temp_c) = parse_temp_millicelsius(&temp_str) else {
            continue;
        };
        let zone_name = read_string_file(&path.join("type")).unwrap_or(entry_name.clone());
        out.push(ThermalZone {
            name: zone_name,
            temp_c,
        });
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Backlight — /sys/class/backlight/*
// ─────────────────────────────────────────────────────────────────────────────

fn read_backlight() -> Option<BacklightStats> {
    let entries = fs::read_dir(SYS_BACKLIGHT).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let current = read_uint_file(&path.join("actual_brightness"))
            .or_else(|| read_uint_file(&path.join("brightness")));
        let max = read_uint_file(&path.join("max_brightness"));
        match (current, max) {
            (Some(c), Some(m)) if m > 0 => {
                return Some(BacklightStats {
                    current: c as u32,
                    max: m as u32,
                    percent: (c as f64 / m as f64) * 100.0,
                });
            }
            _ => {}
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// File helpers
// ─────────────────────────────────────────────────────────────────────────────

fn read_uint_file(p: &PathBuf) -> Option<u64> {
    fs::read_to_string(p).ok()?.trim().parse::<u64>().ok()
}

fn read_int_file(p: &PathBuf) -> Option<i64> {
    fs::read_to_string(p).ok()?.trim().parse::<i64>().ok()
}

fn read_string_file(p: &PathBuf) -> Option<String> {
    fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}
