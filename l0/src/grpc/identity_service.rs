//! `IdentityService.GetIdentity` — citeste identitatea verificata cache-uita
//! de `identity::boot_check()` la pornire.
//!
//! Daca identity-ul nu a fost deblocat prin envelope A/B/C, returnam
//! `Status::failed_precondition`.

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
                "identity is sealed or missing; run onboarding or unlock an envelope",
            )
        })?;

        let resp = Identity {
            name: verified.identity.name,
            uuid: verified.identity.uuid.to_string(),
            biometric_template_public: verified.identity.biometric_template_public,
            created_at: verified.identity.created_at,
            version: verified.header_version,
        };
        Ok(Response::new(resp))
    }
}
