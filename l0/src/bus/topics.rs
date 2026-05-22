//! MQTT topic constants. Sincronizate cu doc-string-ul din `mod.rs`.

/// Binary status published with retain.
pub const STATUS: &str = "ybos/status";

/// CPU telemetry JSON `{ usage_percent, freq_mhz_avg, core_count }`.
pub const TELEM_CPU: &str = "ybos/telemetry/cpu";

/// Memory telemetry JSON MemoryStats.
pub const TELEM_MEM: &str = "ybos/telemetry/mem";

/// Battery telemetry JSON array of BatteryStats.
pub const TELEM_BATTERY: &str = "ybos/telemetry/battery";

/// Thermal telemetry JSON array of ThermalZone.
pub const TELEM_THERMAL: &str = "ybos/telemetry/thermal";

/// Backlight telemetry JSON BacklightStats.
pub const TELEM_BACKLIGHT: &str = "ybos/telemetry/backlight";

/// Full JSON TelemetrySnapshot.
pub const TELEM_FULL: &str = "ybos/telemetry/full";

#[allow(dead_code)]
pub const HW_EVENT: &str = "ybos/hw/event";
