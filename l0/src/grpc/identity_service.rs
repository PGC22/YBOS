//! `IdentityService.GetIdentity` — citeste identitatea verificata cache-uita
//! de `identity::boot_check()` la pornire.
//!
//! Daca boot-ul a rulat in mod UNVERIFIED (identity_core.bin lipsa la prima
//! rulare), returnam `Status::failed_precondition`. L1 trebuie sa ruleze
//! `python tools/identity_gen.py` si sa reporneasca daemonul.

use tonic::{Request, Response, Status};
use tracing::debug;

use super::pb::identity_service_server::IdentityService;
use super::pb::{GetIdentityRequest, Identity};
use crate::identity;

#[derive(Default)]
pub struct IdentitySvc;

#[tonic::async_trait]
impl IdentityService for IdentitySvc {
    async fn get_identity(
        &self,
        _req: Request<GetIdentityRequest>,
    ) -> Result<Response<Identity>, Status> {
        debug!("[L0/grpc] IdentityService.GetIdentity");

        let verified = identity::current_identity().ok_or_else(|| {
            Status::failed_precondition(
                "identitate ne-verificata — identity_core.bin lipsa sau invalida. \
                 Ruleaza: python tools/identity_gen.py",
            )
        })?;

        let resp = Identity {
            remus_id: verified.payload.remus_id,
            device_id: verified.payload.device_id,
            device_role: verified.payload.device_role,
            creator: verified.payload.creator,
            nucleus: verified.payload.nucleus,
            generated_at: verified.payload.generated_at as u64,
            version: verified.header_version,
        };
        Ok(Response::new(resp))
    }
}
