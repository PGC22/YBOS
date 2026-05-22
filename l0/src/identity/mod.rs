//! Identity enrollment, sealed identity blob verification, and boot integrity.

#![allow(dead_code)] // Y1 exposes hooks that L1/UI will call in later phases.

pub mod blob;
pub mod envelope;
pub mod onboarding;
pub mod paths;
pub mod sacred;
pub mod session;

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::sync::RwLock;
use tracing::info;

use blob::VerifiedIdentity;
use envelope::{open_envelope_a, EnvelopeAFile, EnvelopeAParams};

static IDENTITY: RwLock<Option<VerifiedIdentity>> = RwLock::new(None);

#[allow(dead_code)]
pub fn current_identity() -> Option<VerifiedIdentity> {
    IDENTITY.read().ok().and_then(|g| g.clone())
}

/// Boot check blocks on L0 tripwire mismatches. Identity HMAC verification
/// requires K-master, so it happens after an explicit envelope unlock.
pub async fn boot_check() -> Result<()> {
    let report = sacred::boot_integrity_check()?;
    for alert in &report.alerts {
        info!("[L0/identity] {}", alert);
    }

    if paths::identity_blob().exists() {
        info!("[L0/identity] Sealed identity present; waiting for unlock");
    } else {
        info!("[L0/identity] No sealed identity yet; onboarding required");
    }

    Ok(())
}

#[allow(dead_code)]
pub fn unlock_with_pin(
    pin: &str,
    biometric_template: Option<&[u8]>,
    device_fingerprint: &[u8; 32],
) -> Result<VerifiedIdentity> {
    let envelope_path = paths::envelope_a();
    let salt_path = paths::identity_salt();

    let envelope = EnvelopeAFile::from_bytes(
        &fs::read(&envelope_path).with_context(|| format!("read {}", envelope_path.display()))?,
    )?;
    let salt_bytes =
        fs::read(&salt_path).with_context(|| format!("read {}", salt_path.display()))?;
    let salt: [u8; 16] = salt_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("identity_core.salt has invalid length"))?;

    let master_key = open_envelope_a(
        &envelope,
        pin,
        biometric_template,
        device_fingerprint,
        &salt,
        EnvelopeAParams::production(),
    )?;
    let verified = blob::load_and_verify(master_key.expose())?;
    if let Ok(mut guard) = IDENTITY.write() {
        *guard = Some(verified.clone());
    }
    Ok(verified)
}
