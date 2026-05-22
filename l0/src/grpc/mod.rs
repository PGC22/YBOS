//! gRPC server — expune identitatea, telemetria si reflexele catre L1 (Python).
//!
//! Services (vezi `proto/l0.proto`):
//!   - `IdentityService.GetIdentity()` — returneaza nucleu identitar verificat
//!   - `TelemetryService.Subscribe()` — server-streaming pe ultimele citiri HW
//!   - `ReflexService.RequestAction()` — L1 cere actiune (throttle, brightness)
//!
//! Bind doar pe `127.0.0.1:50051` (local-only, fara TLS in S6.4 init —
//! la fel ca MQTT). Auth + TLS vin in S6.x ulterior, cand permitem peer
//! multi-body pe alt device.
//!
//! Server-ul ruleaza intr-un tokio task spawnat — `serve()` returneaza imediat
//! (pattern paritar cu `bus::start_mqtt_broker`).

use anyhow::{Context, Result};
use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::{info, warn};

mod convert;
mod identity_service;
mod reflex_service;
mod telemetry_service;

/// Codegen prost — includem modulul generat de `tonic-build` (vezi `build.rs`).
pub mod pb {
    tonic::include_proto!("ybos.l0.v1");
}

pub const GRPC_LISTEN: &str = "127.0.0.1:50051";

use pb::identity_service_server::IdentityServiceServer;
use pb::reflex_service_server::ReflexServiceServer;
use pb::telemetry_service_server::TelemetryServiceServer;

/// Porneste serverul gRPC ca task background. Returneaza dupa ce s-a bind-uit
/// pe `GRPC_LISTEN`. Daca bind-ul esueaza, returneaza eroare.
pub async fn serve() -> Result<()> {
    let addr: SocketAddr = GRPC_LISTEN
        .parse()
        .with_context(|| format!("parse gRPC listen addr {}", GRPC_LISTEN))?;

    let identity_svc = identity_service::IdentitySvc::default();
    let telemetry_svc = telemetry_service::TelemetrySvc::default();
    let reflex_svc = reflex_service::ReflexSvc::default();

    tokio::spawn(async move {
        let res = Server::builder()
            .add_service(IdentityServiceServer::new(identity_svc))
            .add_service(TelemetryServiceServer::new(telemetry_svc))
            .add_service(ReflexServiceServer::new(reflex_svc))
            .serve(addr)
            .await;
        if let Err(e) = res {
            warn!("[L0/grpc] server exited: {}", e);
        }
    });

    info!("[L0/grpc] gRPC server listening on {}", GRPC_LISTEN);
    Ok(())
}
