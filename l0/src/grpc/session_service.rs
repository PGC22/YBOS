//! `SessionService` implementation — wraps `identity::session` API.

use std::time::Duration;
use tonic::{Request, Response, Status};
use tracing::debug;
use uuid::Uuid;

use super::pb::session_service_server::SessionService;
use super::pb::{
    IssueTokenRequest, IssueTokenResponse, ListActiveRequest, ListActiveResponse,
    RevokeAllRequest, RevokeAllResponse, RevokeSessionRequest, RevokeSessionResponse,
    SessionInfo,
};
#[cfg(feature = "dev_test_init")]
use super::pb::{InitializeForTestRequest, InitializeForTestResponse};

use crate::identity::session;

#[derive(Default)]
pub struct SessionSvc;

#[tonic::async_trait]
impl SessionService for SessionSvc {
    async fn issue_token(
        &self,
        req: Request<IssueTokenRequest>,
    ) -> Result<Response<IssueTokenResponse>, Status> {
        debug!("[L0/grpc] SessionService.IssueToken");
        let r = req.into_inner();

        let scope = session::ScopeSpec::new(r.capabilities)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let peer_fp: [u8; 32] = r.peer_fingerprint.try_into()
            .map_err(|_| Status::invalid_argument("peer_fingerprint must be 32 bytes"))?;

        let token = session::issue_session_token(
            scope,
            Duration::from_secs(r.expiry_secs),
            peer_fp,
        ).map_err(|e| Status::internal(format!("failed to issue token: {}", e)))?;

        Ok(Response::new(IssueTokenResponse {
            session_id: token.session_id.to_string(),
            key_bytes: token.key.expose().to_vec(),
            expires_at: token.expires_at,
        }))
    }

    async fn revoke_session(
        &self,
        req: Request<RevokeSessionRequest>,
    ) -> Result<Response<RevokeSessionResponse>, Status> {
        debug!("[L0/grpc] SessionService.RevokeSession");
        let session_id = Uuid::parse_str(&req.into_inner().session_id)
            .map_err(|e| Status::invalid_argument(format!("invalid UUID: {}", e)))?;

        session::revoke_session(session_id)
            .map_err(|e| Status::internal(format!("failed to revoke session: {}", e)))?;

        Ok(Response::new(RevokeSessionResponse { revoked: true }))
    }

    async fn revoke_all(
        &self,
        _req: Request<RevokeAllRequest>,
    ) -> Result<Response<RevokeAllResponse>, Status> {
        debug!("[L0/grpc] SessionService.RevokeAll");
        let count = session::list_active().len() as u32;
        session::revoke_all()
            .map_err(|e| Status::internal(format!("failed to revoke all sessions: {}", e)))?;

        Ok(Response::new(RevokeAllResponse {
            revoked_count: count,
        }))
    }

    async fn list_active(
        &self,
        _req: Request<ListActiveRequest>,
    ) -> Result<Response<ListActiveResponse>, Status> {
        debug!("[L0/grpc] SessionService.ListActive");
        let active = session::list_active();
        let sessions = active
            .into_iter()
            .map(|s| SessionInfo {
                session_id: s.session_id.to_string(),
                capabilities: s.scope.capabilities,
                issued_at: s.issued_at,
                expires_at: s.expires_at,
                peer_fingerprint: s.peer_fingerprint.to_vec(),
            })
            .collect();

        Ok(Response::new(ListActiveResponse { sessions }))
    }

    #[cfg(feature = "dev_test_init")]
    async fn initialize_for_test(
        &self,
        req: Request<InitializeForTestRequest>,
    ) -> Result<Response<InitializeForTestResponse>, Status> {
        debug!("[L0/grpc] SessionService.InitializeForTest (DEV ONLY)");
        use crate::identity::envelope::MasterKey;

        let r = req.into_inner();
        let master_key_bytes: [u8; 32] = r.master_key.try_into()
            .map_err(|_| Status::invalid_argument("master_key must be 32 bytes"))?;

        session::init_session_api(MasterKey::from_bytes(master_key_bytes))
            .map_err(|e| Status::internal(format!("failed to init session API: {}", e)))?;

        Ok(Response::new(InitializeForTestResponse {}))
    }

    #[cfg(not(feature = "dev_test_init"))]
    async fn initialize_for_test(
        &self,
        _req: Request<super::pb::InitializeForTestRequest>,
    ) -> Result<Response<super::pb::InitializeForTestResponse>, Status> {
        Err(Status::unimplemented("InitializeForTest is only available in dev/test builds"))
    }
}
