//! `TelemetryService.Subscribe` — server-streaming pe snapshot-urile HAL.
//!
//! Clientul cere un interval (ms). 0 = default 5000 ms. Server-ul spawneaza
//! o task care la fiecare tick face `hw::snapshot()`, il converteste in
//! `TelemetryFrame` si il trimite pe canalul mpsc. La disconnect, primul `send`
//! esueaza si task-ul iese.

use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::{debug, warn};

use super::convert::snapshot_to_frame;
use super::pb::telemetry_service_server::TelemetryService;
use super::pb::{SubscribeRequest, TelemetryFrame};
use crate::hw;

const DEFAULT_INTERVAL_MS: u64 = 5000;
const MIN_INTERVAL_MS: u64 = 250;
const CHANNEL_BUFFER: usize = 8;

#[derive(Default)]
pub struct TelemetrySvc;

#[tonic::async_trait]
impl TelemetryService for TelemetrySvc {
    type SubscribeStream = ReceiverStream<Result<TelemetryFrame, Status>>;

    async fn subscribe(
        &self,
        req: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let raw = req.into_inner().interval_ms as u64;
        let interval_ms = if raw == 0 {
            DEFAULT_INTERVAL_MS
        } else {
            raw.max(MIN_INTERVAL_MS)
        };
        debug!(
            "[L0/grpc] TelemetryService.Subscribe (interval={}ms)",
            interval_ms
        );

        let (tx, rx) = mpsc::channel::<Result<TelemetryFrame, Status>>(CHANNEL_BUFFER);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms));
            // Primul tick e instant — il consumam pentru a livra prompt
            // primul frame la subscriber.
            loop {
                ticker.tick().await;
                let frame_result = match hw::snapshot() {
                    Ok(snap) => Ok(snapshot_to_frame(&snap)),
                    Err(e) => {
                        warn!("[L0/grpc] snapshot failed: {}", e);
                        Err(Status::internal(format!("snapshot failed: {}", e)))
                    }
                };
                if tx.send(frame_result).await.is_err() {
                    // Client disconnected.
                    debug!("[L0/grpc] subscriber disconnected, ending stream task");
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
