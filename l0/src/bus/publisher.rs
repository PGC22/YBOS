//! MQTT publisher — client conectat loopback la broker, publica snapshot-uri.

use anyhow::{Context, Result};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::hw::TelemetrySnapshot;

use super::topics;

const CLIENT_ID: &str = "remus-l0-publisher";

/// Wrapper care expune doar publish-ul, fara sa scape detalii rumqttc.
pub struct Publisher {
    client: AsyncClient,
}

impl Publisher {
    /// Conecteaza un client local la broker. Lanseaza event loop intr-un task
    /// tokio (eventele primite sunt drain-uite — publisherul nu se aboneaza).
    pub fn connect() -> Result<Self> {
        let mut opts = MqttOptions::new(CLIENT_ID, "127.0.0.1", 11883);
        opts.set_keep_alive(Duration::from_secs(30));
        opts.set_clean_session(true);

        // Last Will and Testament: daca publisherul moare brusc,
        // broker-ul publica automat "offline" pe remus/status (retain).
        opts.set_last_will(rumqttc::LastWill::new(
            topics::STATUS,
            b"offline".to_vec(),
            QoS::AtLeastOnce,
            true,
        ));

        let (client, mut event_loop) = AsyncClient::new(opts, 32);

        // Drain event loop intr-un task; logam doar erorile.
        tokio::spawn(async move {
            loop {
                match event_loop.poll().await {
                    Ok(ev) => {
                        debug!("[L0/bus/publisher] event: {:?}", ev);
                    }
                    Err(e) => {
                        warn!("[L0/bus/publisher] event loop error: {}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });

        Ok(Self { client })
    }

    /// Anunta online (retain). Apelat dupa boot complete.
    pub async fn announce_online(&self) -> Result<()> {
        self.client
            .publish(topics::STATUS, QoS::AtLeastOnce, true, b"online".as_ref())
            .await
            .context("publish online")?;
        info!("[L0/bus] published {} = online (retain)", topics::STATUS);
        Ok(())
    }

    /// Anunta offline (retain) — apelat manual la shutdown grácil.
    pub async fn announce_offline(&self) -> Result<()> {
        self.client
            .publish(topics::STATUS, QoS::AtLeastOnce, true, b"offline".as_ref())
            .await
            .context("publish offline")?;
        info!("[L0/bus] published {} = offline (retain)", topics::STATUS);
        Ok(())
    }

    /// Publica un snapshot complet pe multiple topics granulare + unul agregat.
    pub async fn publish_snapshot(&self, snap: &TelemetrySnapshot) -> Result<()> {
        // Topic agregat — JSON complet
        let full = serde_json::to_vec(snap).context("serialize full snapshot")?;
        self.client
            .publish(topics::TELEM_FULL, QoS::AtMostOnce, false, full)
            .await
            .context("publish full")?;

        if let Some(cpu) = &snap.cpu {
            let payload = serde_json::to_vec(cpu).context("serialize cpu")?;
            self.publish_at_most(topics::TELEM_CPU, payload).await?;
        }
        if let Some(mem) = &snap.mem {
            let payload = serde_json::to_vec(mem).context("serialize mem")?;
            self.publish_at_most(topics::TELEM_MEM, payload).await?;
        }
        if !snap.batteries.is_empty() {
            let payload = serde_json::to_vec(&snap.batteries).context("serialize batteries")?;
            self.publish_at_most(topics::TELEM_BATTERY, payload).await?;
        }
        if !snap.thermal.is_empty() {
            let payload = serde_json::to_vec(&snap.thermal).context("serialize thermal")?;
            self.publish_at_most(topics::TELEM_THERMAL, payload).await?;
        }
        if let Some(bl) = &snap.backlight {
            let payload = serde_json::to_vec(bl).context("serialize backlight")?;
            self.publish_at_most(topics::TELEM_BACKLIGHT, payload)
                .await?;
        }
        debug!("[L0/bus] snapshot publicat ({} bytes total agregat)", snap_size(snap));
        Ok(())
    }

    async fn publish_at_most(&self, topic: &str, payload: Vec<u8>) -> Result<()> {
        self.client
            .publish(topic, QoS::AtMostOnce, false, payload)
            .await
            .with_context(|| format!("publish {}", topic))
    }
}

fn snap_size(s: &TelemetrySnapshot) -> usize {
    serde_json::to_vec(s).map(|v| v.len()).unwrap_or(0)
}
