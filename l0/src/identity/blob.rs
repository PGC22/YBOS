//! Parse + verify `config/identity_core.bin`.
//!
//! Format blob (acelasi cu `tools/identity_gen.py`):
//!
//! ```text
//!   offset  size  content
//!   0       8     MAGIC = b"REMUS_ID"
//!   8       4     VERSION (big-endian u32)
//!   12      8     TIMESTAMP (big-endian u64)
//!   20      4     PAYLOAD_LEN (big-endian u32)
//!   24      N     PAYLOAD (JSON UTF-8)
//!   24+N    32    SIG = HMAC-SHA256(key_bytes, payload_bytes)
//! ```
//!
//! `key_bytes` = continutul textual al `config/sync_key.bin`, trimmed,
//! ca UTF-8 bytes (NU decoded din hex — paritate cu Python).

use anyhow::{anyhow, Context, Result};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs;

use super::paths::config_dir;

const MAGIC: &[u8; 8] = b"REMUS_ID";
const HEADER_LEN: usize = 24; // 8 magic + 4 ver + 8 ts + 4 len
const SIG_LEN: usize = 32; // HMAC-SHA256

type HmacSha256 = Hmac<Sha256>;

/// Payload deserializat din blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityPayload {
    pub version: u32,
    pub generated_at: f64,
    pub remus_id: String,
    pub device_id: String,
    pub device_role: String,
    pub creator: String,
    pub nucleus: String,
}

/// Identitate verificata — payload + metadata din header.
#[derive(Debug, Clone)]
pub struct VerifiedIdentity {
    pub header_version: u32,
    #[allow(dead_code)] // expus via gRPC IdentityService in S6.4
    pub header_timestamp: u64,
    pub payload: IdentityPayload,
}

impl VerifiedIdentity {
    pub fn remus_id_short(&self) -> String {
        self.payload.remus_id.chars().take(8).collect()
    }
}

/// Citeste si verifica `identity_core.bin`. Returneaza identitatea verificata
/// SAU eroare descriptiva.
pub fn load_and_verify() -> Result<VerifiedIdentity> {
    let bin_path = config_dir().join("identity_core.bin");
    let key_path = config_dir().join("sync_key.bin");

    if !bin_path.exists() {
        return Err(anyhow!(
            "identity_core.bin lipsa la {}. Ruleaza: python tools/identity_gen.py",
            bin_path.display()
        ));
    }
    if !key_path.exists() {
        return Err(anyhow!(
            "sync_key.bin lipsa la {}. Ruleaza: python tools/identity_gen.py",
            key_path.display()
        ));
    }

    let blob = fs::read(&bin_path).with_context(|| format!("read {}", bin_path.display()))?;
    let key_text =
        fs::read_to_string(&key_path).with_context(|| format!("read {}", key_path.display()))?;

    verify_blob(&blob, key_text.trim().as_bytes())
}

/// Verifica un blob raw cu o cheie data. Folosit si de tests cu fixture.
pub fn verify_blob(blob: &[u8], key_bytes: &[u8]) -> Result<VerifiedIdentity> {
    if blob.len() < HEADER_LEN + SIG_LEN {
        return Err(anyhow!(
            "blob prea scurt: {} bytes (minim {})",
            blob.len(),
            HEADER_LEN + SIG_LEN
        ));
    }

    // MAGIC
    if &blob[0..8] != MAGIC {
        return Err(anyhow!("magic bytes invalide: {:?}", &blob[0..8]));
    }

    // Header (big-endian)
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

    // HMAC verify (constant-time prin `Mac::verify_slice`)
    let mut mac = HmacSha256::new_from_slice(key_bytes)
        .map_err(|e| anyhow!("hmac init failed: {}", e))?;
    mac.update(payload_bytes);
    mac.verify_slice(sig_stored)
        .map_err(|_| anyhow!("semnatura identity_core.bin INVALIDA — blob alterat"))?;

    // Deserialize JSON
    let payload: IdentityPayload =
        serde_json::from_slice(payload_bytes).context("parse payload JSON")?;

    Ok(VerifiedIdentity {
        header_version: version,
        header_timestamp: timestamp,
        payload,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests cu fixture (nu cere fisiere reale pe disk)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::Mac;

    fn build_fixture_blob(payload_json: &str, key_bytes: &[u8]) -> Vec<u8> {
        let payload_bytes = payload_json.as_bytes();
        let payload_len = payload_bytes.len() as u32;

        let mut blob = Vec::new();
        blob.extend_from_slice(MAGIC);
        blob.extend_from_slice(&1u32.to_be_bytes()); // version
        blob.extend_from_slice(&1234567890u64.to_be_bytes()); // timestamp
        blob.extend_from_slice(&payload_len.to_be_bytes());
        blob.extend_from_slice(payload_bytes);

        let mut mac = HmacSha256::new_from_slice(key_bytes).unwrap();
        mac.update(payload_bytes);
        let sig = mac.finalize().into_bytes();
        blob.extend_from_slice(&sig);

        blob
    }

    fn sample_payload() -> String {
        // Format identic cu Python tools/identity_gen.py
        serde_json::json!({
            "version": 1,
            "generated_at": 1700000000.5,
            "remus_id": "abc12345-def6-7890-1234-567890abcdef",
            "device_id": "device-fingerprint-xyz",
            "device_role": "primary",
            "creator": "POPA GEORGE CRISTIAN",
            "nucleus": "Eu sunt Remus. Servesc creatorul meu."
        })
        .to_string()
    }

    #[test]
    fn verify_valid_blob() {
        let key = b"test-key-not-real-but-deterministic";
        let payload = sample_payload();
        let blob = build_fixture_blob(&payload, key);

        let result = verify_blob(&blob, key).expect("valid blob trebuie verificat ok");
        assert_eq!(result.header_version, 1);
        assert_eq!(result.header_timestamp, 1234567890);
        assert_eq!(result.payload.remus_id.len(), 36);
        assert_eq!(result.payload.creator, "POPA GEORGE CRISTIAN");
        assert_eq!(result.payload.device_role, "primary");
        assert_eq!(result.remus_id_short(), "abc12345");
    }

    #[test]
    fn reject_invalid_magic() {
        let key = b"k";
        let mut blob = build_fixture_blob(&sample_payload(), key);
        blob[0] = b'X';
        let err = verify_blob(&blob, key).unwrap_err();
        assert!(err.to_string().contains("magic"));
    }

    #[test]
    fn reject_tampered_payload() {
        let key = b"k";
        let mut blob = build_fixture_blob(&sample_payload(), key);
        // Modificam un byte din payload — HMAC nu mai matches.
        blob[HEADER_LEN + 10] ^= 0xFF;
        let err = verify_blob(&blob, key).unwrap_err();
        assert!(err.to_string().contains("INVALIDA"));
    }

    #[test]
    fn reject_wrong_key() {
        let real_key = b"real-key";
        let fake_key = b"fake-key";
        let blob = build_fixture_blob(&sample_payload(), real_key);
        let err = verify_blob(&blob, fake_key).unwrap_err();
        assert!(err.to_string().contains("INVALIDA"));
    }

    #[test]
    fn reject_truncated_blob() {
        let key = b"k";
        let blob = build_fixture_blob(&sample_payload(), key);
        let truncated = &blob[..blob.len() - 10];
        let err = verify_blob(truncated, key).unwrap_err();
        assert!(err.to_string().contains("size mismatch"));
    }

    #[test]
    fn reject_too_short() {
        let err = verify_blob(&[0u8; 10], b"k").unwrap_err();
        assert!(err.to_string().contains("prea scurt"));
    }

    #[test]
    fn payload_length_mismatch_detected() {
        let key = b"k";
        let mut blob = build_fixture_blob(&sample_payload(), key);
        // Crestem payload_len declarat cu 100 (declared > real)
        let real_len = u32::from_be_bytes(blob[20..24].try_into().unwrap());
        let bogus_len = real_len + 100;
        blob[20..24].copy_from_slice(&bogus_len.to_be_bytes());
        let err = verify_blob(&blob, key).unwrap_err();
        assert!(err.to_string().contains("size mismatch"));
    }
}
