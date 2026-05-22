//! Session token issuance API hook for future laptop pairing.
//!
//! Y1 only exposes the L0-side API and in-memory active-session list. QR/NFC
//! pairing, laptop clients, mTLS wiring, and task offload are later phases.

use anyhow::{anyhow, Result};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::envelope::MasterKey;

const SESSION_INFO: &[u8] = b"ybos-session-v1";

pub type SessionId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeSpec {
    pub capabilities: Vec<String>,
}

impl ScopeSpec {
    pub fn new(capabilities: Vec<String>) -> Result<Self> {
        if capabilities.is_empty() {
            return Err(anyhow!(
                "session scope must declare at least one capability"
            ));
        }
        Ok(Self { capabilities })
    }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SessionKey([u8; 32]);

impl SessionKey {
    pub fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionKey([redacted])")
    }
}

#[derive(Debug)]
pub struct SessionToken {
    pub session_id: SessionId,
    pub key: SessionKey,
    pub salt: [u8; 32],
    pub epoch: u64,
    pub expires_at: u64,
    pub scope: ScopeSpec,
    pub peer_fingerprint: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: SessionId,
    pub scope: ScopeSpec,
    pub issued_at: u64,
    pub expires_at: u64,
    pub peer_fingerprint: [u8; 32],
}

pub struct SessionManager {
    master_key: MasterKey,
    active: RwLock<HashMap<SessionId, SessionInfo>>,
}

static SESSION_MANAGER: OnceLock<SessionManager> = OnceLock::new();

pub fn init_session_api(master_key: MasterKey) -> Result<()> {
    SESSION_MANAGER
        .set(SessionManager::new(master_key))
        .map_err(|_| anyhow!("session API already initialized"))
}

pub fn issue_session_token(
    scope: ScopeSpec,
    expiry: Duration,
    peer_fingerprint: [u8; 32],
) -> Result<SessionToken> {
    session_manager()?.issue_session_token(scope, expiry, peer_fingerprint)
}

pub fn revoke_session(session_id: SessionId) -> Result<()> {
    session_manager()?.revoke_session(session_id)
}

pub fn revoke_all() -> Result<()> {
    session_manager()?.revoke_all()
}

pub fn list_active() -> Vec<SessionInfo> {
    session_manager().map(|m| m.list_active()).unwrap_or_default()
}

fn session_manager() -> Result<&'static SessionManager> {
    SESSION_MANAGER
        .get()
        .ok_or_else(|| anyhow!("session API not initialized"))
}

impl SessionManager {
    pub fn new(master_key: MasterKey) -> Self {
        Self {
            master_key,
            active: RwLock::new(HashMap::new()),
        }
    }

    pub fn issue_session_token(
        &self,
        scope: ScopeSpec,
        expiry: Duration,
        peer_fingerprint: [u8; 32],
    ) -> Result<SessionToken> {
        if expiry.is_zero() {
            return Err(anyhow!("session expiry must be greater than zero"));
        }

        let session_id = Uuid::new_v4();
        let issued_at = now_secs();
        let expires_at = issued_at
            .checked_add(expiry.as_secs())
            .ok_or_else(|| anyhow!("session expiry overflow"))?;
        let mut salt = [0u8; 32];
        OsRng.fill_bytes(&mut salt);
        let key = derive_session_key(
            self.master_key.expose(),
            &salt,
            issued_at,
            &session_id,
            &peer_fingerprint,
        )?;

        let info = SessionInfo {
            session_id,
            scope: scope.clone(),
            issued_at,
            expires_at,
            peer_fingerprint,
        };
        self.active
            .write()
            .map_err(|e| anyhow!("session lock poisoned: {}", e))?
            .insert(session_id, info);

        Ok(SessionToken {
            session_id,
            key: SessionKey(key),
            salt,
            epoch: issued_at,
            expires_at,
            scope,
            peer_fingerprint,
        })
    }

    pub fn revoke_session(&self, session_id: SessionId) -> Result<()> {
        self.active
            .write()
            .map_err(|e| anyhow!("session lock poisoned: {}", e))?
            .remove(&session_id);
        Ok(())
    }

    pub fn revoke_all(&self) -> Result<()> {
        self.active
            .write()
            .map_err(|e| anyhow!("session lock poisoned: {}", e))?
            .clear();
        Ok(())
    }

    pub fn list_active(&self) -> Vec<SessionInfo> {
        match self.active.read() {
            Ok(active) => active.values().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }
}

fn derive_session_key(
    master_key: &[u8; 32],
    salt: &[u8; 32],
    epoch: u64,
    session_id: &SessionId,
    peer_fingerprint: &[u8; 32],
) -> Result<[u8; 32]> {
    let hk = Hkdf::<Sha256>::new(Some(salt), master_key);
    let mut info = Vec::with_capacity(SESSION_INFO.len() + 8 + 16 + 32);
    info.extend_from_slice(SESSION_INFO);
    info.extend_from_slice(&epoch.to_be_bytes());
    info.extend_from_slice(session_id.as_bytes());
    info.extend_from_slice(peer_fingerprint);

    let mut out = [0u8; 32];
    hk.expand(&info, &mut out)
        .map_err(|_| anyhow!("HKDF expand session key failed"))?;
    info.zeroize();
    Ok(out)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> ScopeSpec {
        ScopeSpec::new(vec!["data.user_prefs.read".to_string()]).unwrap()
    }

    #[test]
    fn issue_and_revoke_session_token() {
        let manager = SessionManager::new(MasterKey::from_bytes([3u8; 32]));
        let token = manager
            .issue_session_token(scope(), Duration::from_secs(60), [9u8; 32])
            .unwrap();

        assert_eq!(token.key.expose().len(), 32);
        assert_eq!(manager.list_active().len(), 1);
        assert_eq!(manager.list_active()[0].session_id, token.session_id);

        manager.revoke_session(token.session_id).unwrap();
        assert!(manager.list_active().is_empty());
    }

    #[test]
    fn revoke_all_clears_sessions() {
        let manager = SessionManager::new(MasterKey::from_bytes([3u8; 32]));
        manager
            .issue_session_token(scope(), Duration::from_secs(60), [1u8; 32])
            .unwrap();
        manager
            .issue_session_token(scope(), Duration::from_secs(60), [2u8; 32])
            .unwrap();
        assert_eq!(manager.list_active().len(), 2);

        manager.revoke_all().unwrap();
        assert!(manager.list_active().is_empty());
    }

    #[test]
    fn rejects_empty_scope() {
        let err = ScopeSpec::new(Vec::new()).unwrap_err();
        assert!(err.to_string().contains("scope"));
    }

    #[test]
    fn rejects_zero_expiry() {
        let manager = SessionManager::new(MasterKey::from_bytes([3u8; 32]));
        let err = manager
            .issue_session_token(scope(), Duration::from_secs(0), [9u8; 32])
            .unwrap_err();
        assert!(err.to_string().contains("expiry"));
    }
}
