use anyhow::Result;
use std::time::Duration;
use tonic::transport::Channel;
use crate::pb::l0::session_service_client::SessionServiceClient;
use crate::pb::l0::{IssueTokenRequest, GetIdentityRequest};
use crate::pb::l0::identity_service_client::IdentityServiceClient;

pub struct L0Client {
    session_svc: SessionServiceClient<Channel>,
    identity_svc: IdentityServiceClient<Channel>,
}

pub struct SessionToken {
    pub session_id: String,
    pub key_bytes: Vec<u8>,
    pub expires_at: u64,
}

impl L0Client {
    pub async fn connect(addr: &str) -> Result<Self> {
        let channel = Channel::from_shared(addr.to_string())?
            .connect()
            .await?;

        Ok(Self {
            session_svc: SessionServiceClient::new(channel.clone()),
            identity_svc: IdentityServiceClient::new(channel),
        })
    }

    pub async fn issue_session_token(
        &self,
        capabilities: Vec<String>,
        expiry: Duration,
        peer_fingerprint: [u8; 32],
    ) -> Result<SessionToken> {
        let mut client = self.session_svc.clone();
        let resp = client.issue_token(IssueTokenRequest {
            capabilities,
            expiry_secs: expiry.as_secs(),
            peer_fingerprint: peer_fingerprint.to_vec(),
        }).await?;

        let inner = resp.into_inner();
        Ok(SessionToken {
            session_id: inner.session_id,
            key_bytes: inner.key_bytes,
            expires_at: inner.expires_at,
        })
    }

    pub async fn get_identity(&self) -> Result<crate::pb::l0::Identity> {
        let mut client = self.identity_svc.clone();
        let resp = client.get_identity(GetIdentityRequest {}).await?;
        Ok(resp.into_inner())
    }
}
