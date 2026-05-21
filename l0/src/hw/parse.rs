//! Pure parsers pentru fisiere `/proc/` si `/sys/`.
//!
//! Functii fara FS touch — primesc `&str` si returneaza structuri. Testabile
//! pe orice OS. Linux impl (`hw::linux`) citeste FS si pasaza continutul aici.
//!
//! Pe Windows `hw::linux` nu se compileaza, deci aceste functii apar
//! ca dead_code. Suprimam warning-ul aici (sunt consumate de Linux backend
//! si testate cross-platform).

#![allow(dead_code)]

use super::{CpuTimes, MemoryStats};

// ─────────────────────────────────────────────────────────────────────────────
// /proc/stat — prima linie `cpu  user nice system idle iowait irq softirq ...`
// ─────────────────────────────────────────────────────────────────────────────

/// Parseaza prima linie din /proc/stat (CPU agregat).
/// Returneaza Some daca formatul e valid, None altfel.
pub fn parse_proc_stat(content: &str) -> Option<CpuTimes> {
    let first = content.lines().next()?;
    let mut parts = first.split_whitespace();
    if parts.next()? != "cpu" {
        return None;
    }
    let nums: Vec<u64> = parts.filter_map(|p| p.parse::<u64>().ok()).collect();
    if nums.len() < 4 {
        return None;
    }
    let user = nums[0];
    let nice = nums[1];
    let system = nums[2];
    let idle = nums[3];
    let iowait = nums.get(4).copied().unwrap_or(0);
    let irq = nums.get(5).copied().unwrap_or(0);
    let softirq = nums.get(6).copied().unwrap_or(0);
    let steal = nums.get(7).copied().unwrap_or(0);

    let total = user + nice + system + idle + iowait + irq + softirq + steal;
    let idle_total = idle + iowait;
    Some(CpuTimes {
        total,
        idle: idle_total,
    })
}

/// Calculeaza procentul utilizat intre doua masuratori. Returneaza None
/// daca delta totala e zero (acelasi sample) sau invalida.
pub fn cpu_usage_delta(prev: CpuTimes, curr: CpuTimes) -> Option<f64> {
    if curr.total <= prev.total {
        return None;
    }
    let dt = (curr.total - prev.total) as f64;
    let di = curr.idle.saturating_sub(prev.idle) as f64;
    if dt <= 0.0 {
        return None;
    }
    Some(((dt - di) / dt * 100.0).clamp(0.0, 100.0))
}

// ─────────────────────────────────────────────────────────────────────────────
// /proc/meminfo
// ─────────────────────────────────────────────────────────────────────────────

/// Parseaza /proc/meminfo. Foloseste MemAvailable (kernel >= 3.14).
pub fn parse_proc_meminfo(content: &str) -> Option<MemoryStats> {
    let mut total: Option<u64> = None;
    let mut available: Option<u64> = None;
    for line in content.lines() {
        let mut parts = line.splitn(2, ':');
        let key = parts.next()?.trim();
        let rest = parts.next()?.trim();
        let value_str = rest.split_whitespace().next()?;
        let value: u64 = value_str.parse().ok()?;
        match key {
            "MemTotal" => total = Some(value),
            "MemAvailable" => available = Some(value),
            _ => {}
        }
        if total.is_some() && available.is_some() {
            break;
        }
    }
    let total = total?;
    let available = available?;
    if total == 0 {
        return None;
    }
    let used_percent = ((total - available.min(total)) as f64 / total as f64) * 100.0;
    Some(MemoryStats {
        total_kb: total,
        available_kb: available,
        used_percent,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Battery
// ─────────────────────────────────────────────────────────────────────────────

/// Status simplificat: orice diferit de "Charging" / "Full" → unplugged.
pub fn is_battery_plugged(status: &str) -> bool {
    matches!(status.trim(), "Charging" | "Full" | "Not charging")
}

/// Converteste microW (din /sys/class/power_supply/.../power_now) la W.
pub fn microw_to_w(microw: i64) -> f64 {
    (microw as f64) / 1_000_000.0
}

// ─────────────────────────────────────────────────────────────────────────────
// Thermal — /sys/class/thermal/thermal_zone*/temp este in milli-Celsius
// ─────────────────────────────────────────────────────────────────────────────

pub fn parse_temp_millicelsius(content: &str) -> Option<f64> {
    let n: i64 = content.trim().parse().ok()?;
    Some(n as f64 / 1000.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// CPU freq — /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq este kHz
// ─────────────────────────────────────────────────────────────────────────────

pub fn parse_freq_khz(content: &str) -> Option<f64> {
    let n: u64 = content.trim().parse().ok()?;
    Some(n as f64 / 1000.0) // MHz
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — toate pure, fara FS
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_stat_basic() {
        let content =
            "cpu  3357 0 4313 1362393 167 0 122 0 0 0\ncpu0 1234 0 ...\nintr 12345\n";
        let times = parse_proc_stat(content).unwrap();
        assert_eq!(times.total, 3357 + 0 + 4313 + 1362393 + 167 + 0 + 122 + 0);
        assert_eq!(times.idle, 1362393 + 167);
    }

    #[test]
    fn proc_stat_minimal_4_fields() {
        // Kernel vechi fara iowait/irq/softirq — doar user/nice/system/idle
        let content = "cpu  100 200 300 400\n";
        let times = parse_proc_stat(content).unwrap();
        assert_eq!(times.total, 1000);
        assert_eq!(times.idle, 400);
    }

    #[test]
    fn proc_stat_rejects_garbage() {
        assert!(parse_proc_stat("garbage").is_none());
        assert!(parse_proc_stat("").is_none());
        assert!(parse_proc_stat("cpu\n").is_none()); // 0 fields after cpu
    }

    #[test]
    fn cpu_delta_basic() {
        let prev = CpuTimes {
            total: 1000,
            idle: 800,
        };
        let curr = CpuTimes {
            total: 1100,
            idle: 850,
        };
        // dt=100, di=50 → (100-50)/100 = 50%
        let usage = cpu_usage_delta(prev, curr).unwrap();
        assert!((usage - 50.0).abs() < 0.01);
    }

    #[test]
    fn cpu_delta_idle_full() {
        let prev = CpuTimes {
            total: 1000,
            idle: 800,
        };
        let curr = CpuTimes {
            total: 1100,
            idle: 900,
        };
        // dt=100, di=100 → 0% usage
        let usage = cpu_usage_delta(prev, curr).unwrap();
        assert!(usage.abs() < 0.01);
    }

    #[test]
    fn cpu_delta_max() {
        let prev = CpuTimes {
            total: 1000,
            idle: 800,
        };
        let curr = CpuTimes {
            total: 1100,
            idle: 800,
        };
        // dt=100, di=0 → 100% usage
        let usage = cpu_usage_delta(prev, curr).unwrap();
        assert!((usage - 100.0).abs() < 0.01);
    }

    #[test]
    fn cpu_delta_no_progress() {
        let same = CpuTimes {
            total: 1000,
            idle: 800,
        };
        assert!(cpu_usage_delta(same, same).is_none());
    }

    #[test]
    fn cpu_delta_clamps_anomaly() {
        // Idle a scazut mai mult decat total — anomalie aritmetica, clamped.
        let prev = CpuTimes {
            total: 1000,
            idle: 900,
        };
        let curr = CpuTimes {
            total: 1010,
            idle: 100,
        };
        let usage = cpu_usage_delta(prev, curr).unwrap();
        assert!(usage <= 100.0 && usage >= 0.0);
    }

    #[test]
    fn meminfo_basic() {
        let content = "\
MemTotal:        8000000 kB
MemFree:          500000 kB
MemAvailable:    4000000 kB
Buffers:          100000 kB
";
        let m = parse_proc_meminfo(content).unwrap();
        assert_eq!(m.total_kb, 8000000);
        assert_eq!(m.available_kb, 4000000);
        assert!((m.used_percent - 50.0).abs() < 0.01);
    }

    #[test]
    fn meminfo_missing_available() {
        let content = "MemTotal: 1000 kB\n";
        assert!(parse_proc_meminfo(content).is_none());
    }

    #[test]
    fn meminfo_empty() {
        assert!(parse_proc_meminfo("").is_none());
    }

    #[test]
    fn battery_plugged_states() {
        assert!(is_battery_plugged("Charging"));
        assert!(is_battery_plugged("Full"));
        assert!(is_battery_plugged("Not charging"));
        assert!(!is_battery_plugged("Discharging"));
        assert!(!is_battery_plugged("Unknown"));
        assert!(!is_battery_plugged(""));
    }

    #[test]
    fn microw_conversion() {
        assert!((microw_to_w(15_000_000) - 15.0).abs() < 0.001);
        assert!((microw_to_w(0)).abs() < 0.001);
        assert!((microw_to_w(-12_500_000) - (-12.5)).abs() < 0.001);
    }

    #[test]
    fn temp_millicelsius_parse() {
        assert!((parse_temp_millicelsius("47000\n").unwrap() - 47.0).abs() < 0.001);
        assert!((parse_temp_millicelsius("89500").unwrap() - 89.5).abs() < 0.001);
        assert!(parse_temp_millicelsius("garbage").is_none());
    }

    #[test]
    fn freq_khz_parse() {
        assert!((parse_freq_khz("2400000\n").unwrap() - 2400.0).abs() < 0.001);
        assert!(parse_freq_khz("garbage").is_none());
    }
}
