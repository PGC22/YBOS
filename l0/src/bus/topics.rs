//! MQTT topic constants. Sincronizate cu doc-string-ul din `mod.rs`.

/// Status binar — published cu retain.
pub const STATUS: &str = "remus/status";

/// Telemetrie CPU — JSON `{ usage_percent, freq_mhz_avg, core_count }`.
pub const TELEM_CPU: &str = "remus/telemetry/cpu";

/// Telemetrie memorie — JSON MemoryStats.
pub const TELEM_MEM: &str = "remus/telemetry/mem";

/// Telemetrie baterii — JSON array de BatteryStats.
pub const TELEM_BATTERY: &str = "remus/telemetry/battery";

/// Telemetrie zone termice — JSON array de ThermalZone.
pub const TELEM_THERMAL: &str = "remus/telemetry/thermal";

/// Telemetrie backlight — JSON BacklightStats (sau lipsa).
pub const TELEM_BACKLIGHT: &str = "remus/telemetry/backlight";

/// Snapshot complet — JSON TelemetrySnapshot.
pub const TELEM_FULL: &str = "remus/telemetry/full";

#[allow(dead_code)] // consumat in Faza 7 (udev attach/detach events)
pub const HW_EVENT: &str = "remus/hw/event";
