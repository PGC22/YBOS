//! Signed identity blob format.
//!
//! `identity_core.bin` contains only public identity metadata and an
//! HMAC-SHA256 signature over that metadata. The HMAC key is K-master, which is
//! generated during onboarding and recovered through an envelope at unlock time.

use anyhow::{anyhow, Context, Result};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs;
use uuid::Uuid;

use super::paths;

const MAGIC: &[u8; 8] = b"YBOS_ID1";
const HEADER_LEN: usize = 24; // 8 magic + 4 version + 8 timestamp + 4 len
const SIG_LEN: usize = 32;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub uuid: Uuid,
    pub biometric_template_public: Vec<u8>,
    pub created_at: u64,
}

impl Identity {
    pub fn new(
        name: impl Into<String>,
        biometric_template_public: Vec<u8>,
        created_at: u64,
    ) -> Self {
        Self {
            name: name.into(),
            uuid: Uuid::new_v4(),
            biometric_template_public,
            created_at,
        }
    }

    pub fn uuid_short(&self) -> String {
        self.uuid.simple().to_string().chars().take(8).collect()
    }
}

#[derive(Debug, Clone)]
pub struct VerifiedIdentity {
    pub header_version: u32,
    pub header_timestamp: u64,
    pub identity: Identity,
}

pub fn load_and_verify(key: &[u8; 32]) -> Result<VerifiedIdentity> {
    let bin_path = paths::identity_blob();
    if !bin_path.exists() {
        return Err(anyhow!(
            "identity_core.bin missing at {}. Run onboarding first.",
            bin_path.display()
        ));
    }

    let blob = fs::read(&bin_path).with_context(|| format!("read {}", bin_path.display()))?;
    verify_blob(&blob, key)
}

pub fn build_blob(identity: &Identity, key: &[u8; 32]) -> Result<Vec<u8>> {
    let payload_bytes = serde_json::to_vec(identity).context("serialize identity payload")?;
    let payload_len = payload_bytes.len() as u32;

    let mut blob = Vec::with_capacity(HEADER_LEN + payload_bytes.len() + SIG_LEN);
    blob.extend_from_slice(MAGIC);
    blob.extend_from_slice(&1u32.to_be_bytes());
    blob.extend_from_slice(&identity.created_at.to_be_bytes());
    blob.extend_from_slice(&payload_len.to_be_bytes());
    blob.extend_from_slice(&payload_bytes);

    let mut mac =
        HmacSha256::new_from_slice(key).map_err(|e| anyhow!("hmac init failed: {}", e))?;
    mac.update(&payload_bytes);
    let sig = mac.finalize().into_bytes();
    blob.extend_from_slice(&sig);

    Ok(blob)
}

pub fn verify_blob(blob: &[u8], key: &[u8; 32]) -> Result<VerifiedIdentity> {
    if blob.len() < HEADER_LEN + SIG_LEN {
        return Err(anyhow!(
            "blob too short: {} bytes (minimum {})",
            blob.len(),
            HEADER_LEN + SIG_LEN
        ));
    }

    if &blob[0..8] != MAGIC {
        return Err(anyhow!("invalid magic bytes: {:?}", &blob[0..8]));
    }

    let version = u32::from_be_bytes(blob[8..12].try_into().unwrap());
    let timestamp = u64::from_be_bytes(blob[12..20].try_into().unwrap());
    let payload_len = u32::from_be_bytes(blob[20..24].try_into().unwrap()) as usize;

    let expected_total = HEADER_LEN + payload_len + SIG_LEN;
    if blob.len() != expected_total {
        return Err(anyhow!(
            "blob size mismatch: got {}, expected {} (payload_len={})",
            blob.len(),
            expected_total,
            payload_len
        ));
    }

    let payload_bytes = &blob[HEADER_LEN..HEADER_LEN + payload_len];
    let sig_stored = &blob[HEADER_LEN + payload_len..];

    let mut mac =
        HmacSha256::new_from_slice(key).map_err(|e| anyhow!("hmac init failed: {}", e))?;
    mac.update(payload_bytes);
    mac.verify_slice(sig_stored)
        .map_err(|_| anyhow!("identity_core.bin signature invalid"))?;

    let identity: Identity =
        serde_json::from_slice(payload_bytes).context("parse identity payload JSON")?;

    Ok(VerifiedIdentity {
        header_version: version,
        header_timestamp: timestamp,
        identity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> [u8; 32] {
        [7u8; 32]
    }

    fn sample_identity() -> Identity {
        Identity {
            name: "Test User".to_string(),
            uuid: Uuid::parse_str("01890f34-8d7b-7c9c-a00d-111111111111").unwrap(),
            biometric_template_public: vec![1, 2, 3, 4],
            created_at: 1_700_000_000,
        }
    }

    #[test]
    fn verify_valid_blob() {
        let identity = sample_identity();
        let blob = build_blob(&identity, &key()).unwrap();

        let result = verify_blob(&blob, &key()).expect("valid blob should verify");
        assert_eq!(result.header_version, 1);
        assert_eq!(result.header_timestamp, identity.created_at);
        assert_eq!(result.identity, identity);
        assert_eq!(result.identity.uuid_short(), "01890f34");
    }

    #[test]
    fn reject_invalid_magic() {
        let mut blob = build_blob(&sample_identity(), &key()).unwrap();
        blob[0] = b'X';
        let err = verify_blob(&blob, &key()).unwrap_err();
        assert!(err.to_string().contains("magic"));
    }

    #[test]
    fn reject_tampered_payload() {
        let mut blob = build_blob(&sample_identity(), &key()).unwrap();
        blob[HEADER_LEN + 10] ^= 0xFF;
        let err = verify_blob(&blob, &key()).unwrap_err();
        assert!(err.to_string().contains("signature invalid"));
    }

    #[test]
    fn reject_wrong_key() {
        let blob = build_blob(&sample_identity(), &key()).unwrap();
        let err = verify_blob(&blob, &[9u8; 32]).unwrap_err();
        assert!(err.to_string().contains("signature invalid"));
    }

    #[test]
    fn reject_truncated_blob() {
        let blob = build_blob(&sample_identity(), &key()).unwrap();
        let truncated = &blob[..blob.len() - 10];
        let err = verify_blob(truncated, &key()).unwrap_err();
        assert!(err.to_string().contains("size mismatch"));
    }

    #[test]
    fn reject_too_short() {
        let err = verify_blob(&[0u8; 10], &key()).unwrap_err();
        assert!(err.to_string().contains("too short"));
    }
}
