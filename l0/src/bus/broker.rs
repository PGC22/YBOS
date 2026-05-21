//! Embedded MQTT broker (rumqttd).
//!
//! Bind doar pe `127.0.0.1:11883` — strict local. Auth + TLS vine in S6.x
//! ulterior, cand permitem peer multi-body (alt device Remus pe LAN).

use anyhow::{Context, Result};
use rumqttd::{Broker, Config};
use tracing::{error, info};

/// Adresa pe care broker-ul asculta. Hardcodat 127.0.0.1:11883 in S6.3.
pub const BROKER_LISTEN: &str = "127.0.0.1:11883";

/// Config TOML hardcodat. Format pentru rumqttd 0.20.
/// Bind doar pe loopback; auth gol; max 64 conexiuni (suficient pentru L1+peers).
const BROKER_TOML: &str = r#"
id = 0

[router]
id = 0
max_connections = 64
max_outgoing_packet_count = 200
max_segment_size = 104857600
max_segment_count = 10
custom_segment = {}
initialized_filters = []
shared_subscriptions_strategy = "roundrobin"

[v4.1]
name = "remus-l0"
listen = "127.0.0.1:11883"
next_connection_delay_ms = 1

[v4.1.connections]
connection_timeout_ms = 60000
max_payload_size = 20480
max_inflight_count = 500
dynamic_filters = false
"#;

/// Porneste broker-ul intr-un task tokio dedicat (blocking). Returneaza
/// imediat dupa lansare; eventualele erori in runtime sunt logate.
pub fn spawn() -> Result<()> {
    let cfg: Config = toml::from_str(BROKER_TOML).context("parse rumqttd config TOML")?;
    let mut broker = Broker::new(cfg);

    // rumqttd 0.20 expune `start()` blocking. Il rulam intr-un thread OS
    // (NU intr-un task tokio, pentru ca blochează runtime-ul).
    std::thread::Builder::new()
        .name("remus-l0-mqtt".into())
        .spawn(move || {
            info!("[L0/bus] MQTT broker listening on {}", BROKER_LISTEN);
            if let Err(e) = broker.start() {
                error!("[L0/bus] MQTT broker exited: {e:?}");
            }
        })
        .context("spawn broker thread")?;
    Ok(())
}
