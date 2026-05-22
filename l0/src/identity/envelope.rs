//! Key envelopes for K-master.
//!
//! Y1 implements envelope A for Linux-dev onboarding and defines the traits for
//! envelopes B/C. Real TEE and hardware-key implementations are intentionally
//! postponed until the AOSP/device phase, when the platform APIs are known.

use anyhow::{anyhow, Context, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

const ENVELOPE_A_CONTEXT: &[u8] = b"ybos-envelope-a-v1";
const ZERO_BIOMETRIC_TEMPLATE: [u8; 32] = [0u8; 32];

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct MasterKey([u8; 32]);

impl MasterKey {
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self(key)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MasterKey([redacted])")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EnvelopeAParams {
    pub memory_kib: u32,
    pub time_cost: u32,
    pub parallelism: u32,
}

impl EnvelopeAParams {
    pub fn production() -> Self {
        Self {
            memory_kib: 64 * 1024,
            time_cost: 4,
            parallelism: 1,
        }
    }

    #[cfg(test)]
    pub fn test_fast() -> Self {
        Self {
            memory_kib: 64,
            time_cost: 1,
            parallelism: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeAFile {
    pub version: u32,
    pub wrapped_k_hex: String,
    pub tag_hex: String,
}

impl EnvelopeAFile {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec_pretty(self).context("serialize envelope A")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).context("parse envelope A")
    }
}

pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt
}

pub fn seal_envelope_a(
    master_key: &MasterKey,
    pin: &str,
    biometric_template: Option<&[u8]>,
    device_fingerprint: &[u8; 32],
    salt: &[u8; 16],
    params: EnvelopeAParams,
) -> Result<EnvelopeAFile> {
    let wrapping_key =
        derive_envelope_a_key(pin, biometric_template, device_fingerprint, salt, params)?;
    let mut wrapped = [0u8; 32];
    for (i, byte) in wrapped.iter_mut().enumerate() {
        *byte = master_key.expose()[i] ^ wrapping_key[i];
    }
    let tag = envelope_tag(&wrapping_key, salt, &wrapped)?;

    Ok(EnvelopeAFile {
        version: 1,
        wrapped_k_hex: hex::encode(wrapped),
        tag_hex: hex::encode(tag),
    })
}

pub fn open_envelope_a(
    envelope: &EnvelopeAFile,
    pin: &str,
    biometric_template: Option<&[u8]>,
    device_fingerprint: &[u8; 32],
    salt: &[u8; 16],
    params: EnvelopeAParams,
) -> Result<MasterKey> {
    if envelope.version != 1 {
        return Err(anyhow!(
            "unsupported envelope A version: {}",
            envelope.version
        ));
    }

    let wrapping_key =
        derive_envelope_a_key(pin, biometric_template, device_fingerprint, salt, params)?;
    let wrapped_vec = hex::decode(&envelope.wrapped_k_hex).context("decode wrapped K")?;
    let tag_vec = hex::decode(&envelope.tag_hex).context("decode envelope A tag")?;
    let wrapped: [u8; 32] = wrapped_vec
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("wrapped K has invalid length"))?;
    let tag = envelope_tag(&wrapping_key, salt, &wrapped)?;
    if tag_vec.as_slice() != tag {
        return Err(anyhow!("envelope A authentication failed"));
    }

    let mut key = [0u8; 32];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = wrapped[i] ^ wrapping_key[i];
    }
    Ok(MasterKey::from_bytes(key))
}

fn derive_envelope_a_key(
    pin: &str,
    biometric_template: Option<&[u8]>,
    device_fingerprint: &[u8; 32],
    salt: &[u8; 16],
    params: EnvelopeAParams,
) -> Result<[u8; 32]> {
    let argon_params = Params::new(
        params.memory_kib,
        params.time_cost,
        params.parallelism,
        Some(32),
    )
    .map_err(|e| anyhow!("argon2 params invalid: {}", e))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);

    let bio = biometric_template.unwrap_or(&ZERO_BIOMETRIC_TEMPLATE);
    let mut input = Vec::with_capacity(pin.len() + bio.len() + device_fingerprint.len() + 2);
    input.extend_from_slice(pin.as_bytes());
    input.push(0);
    input.extend_from_slice(bio);
    input.push(0);
    input.extend_from_slice(device_fingerprint);

    let mut out = [0u8; 32];
    argon2
        .hash_password_into(&input, salt, &mut out)
        .map_err(|e| anyhow!("argon2id failed: {}", e))?;
    input.zeroize();
    Ok(out)
}

fn envelope_tag(wrapping_key: &[u8; 32], salt: &[u8; 16], wrapped: &[u8; 32]) -> Result<Vec<u8>> {
    let mut mac =
        HmacSha256::new_from_slice(wrapping_key).map_err(|e| anyhow!("hmac init: {}", e))?;
    mac.update(ENVELOPE_A_CONTEXT);
    mac.update(salt);
    mac.update(wrapped);
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Envelope B is the platform TEE binding. The production implementation should
/// call the Android Keystore/StrongBox or vendor TEE API to seal K-master to
/// this device and boot state, then return only an opaque sealed blob. Y1 keeps
/// this as a trait because the exact API depends on the AOSP/device target.
#[allow(dead_code)]
pub trait TeeSeal {
    fn seal_k_master(
        &self,
        master_key: &MasterKey,
        device_fingerprint: &[u8; 32],
    ) -> Result<Vec<u8>>;
    fn unseal_k_master(&self, sealed_blob: &[u8]) -> Result<MasterKey>;
}

/// Envelope C is the optional hardware-key binding. The production
/// implementation should use a YubiKey HMAC challenge over a random nonce and
/// derive a wrapping key from that response. Y1 does not talk to NFC/USB-C.
#[allow(dead_code)]
pub trait YubiKeyHmac {
    fn wrap_k_master(&self, master_key: &MasterKey, challenge: &[u8]) -> Result<Vec<u8>>;
    fn unwrap_k_master(&self, wrapped_blob: &[u8], challenge: &[u8]) -> Result<MasterKey>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp() -> [u8; 32] {
        [4u8; 32]
    }

    #[test]
    fn envelope_a_round_trips() {
        let master = MasterKey::from_bytes([9u8; 32]);
        let salt = [1u8; 16];
        let sealed = seal_envelope_a(
            &master,
            "123456",
            None,
            &fp(),
            &salt,
            EnvelopeAParams::test_fast(),
        )
        .unwrap();
        let opened = open_envelope_a(
            &sealed,
            "123456",
            None,
            &fp(),
            &salt,
            EnvelopeAParams::test_fast(),
        )
        .unwrap();
        assert_eq!(opened.expose(), master.expose());
    }

    #[test]
    fn envelope_a_rejects_wrong_pin() {
        let master = MasterKey::from_bytes([9u8; 32]);
        let salt = [1u8; 16];
        let sealed = seal_envelope_a(
            &master,
            "123456",
            None,
            &fp(),
            &salt,
            EnvelopeAParams::test_fast(),
        )
        .unwrap();
        let err = open_envelope_a(
            &sealed,
            "654321",
            None,
            &fp(),
            &salt,
            EnvelopeAParams::test_fast(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("authentication failed"));
    }
}
