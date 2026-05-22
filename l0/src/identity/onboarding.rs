//! Single-device onboarding scaffold for Y1.
//!
//! This module models the state machine and writes the sealed identity artifacts
//! for Linux-dev simulation. It has no UI; callers are expected to display the
//! returned BIP39 phrase once and never persist it.

use anyhow::{anyhow, Context, Result};
use bip39::{Language, Mnemonic};
use rand::rngs::OsRng;
use rand::RngCore;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::blob::{self, Identity};
use super::envelope::{generate_salt, seal_envelope_a, EnvelopeAFile, EnvelopeAParams, MasterKey};
use super::paths::SacredRoots;
use super::sacred;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingState {
    Welcome,
    Name,
    Pin,
    Biometric,
    YubiKey,
    KeyGen,
    Bip39Display,
    Sealed,
}

#[derive(Debug, Clone)]
pub struct OnboardingRequest {
    pub name: String,
    pub pin: String,
    pub biometric_template_public: Vec<u8>,
    pub biometric_template_secret: Option<Vec<u8>>,
    pub enable_yubikey: bool,
    pub device_fingerprint: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct OnboardingConfig {
    pub roots: SacredRoots,
    pub argon2_params: EnvelopeAParams,
    pub master_key: Option<[u8; 32]>,
    pub envelope_salt: Option<[u8; 16]>,
    pub entropy: Option<[u8; 32]>,
    pub created_at: Option<u64>,
}

impl Default for OnboardingConfig {
    fn default() -> Self {
        Self {
            roots: SacredRoots::current(),
            argon2_params: EnvelopeAParams::production(),
            master_key: None,
            envelope_salt: None,
            entropy: None,
            created_at: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OnboardingResult {
    pub identity: Identity,
    pub mnemonic: String,
    pub states: Vec<OnboardingState>,
}

pub fn run_onboarding(
    request: OnboardingRequest,
    config: OnboardingConfig,
) -> Result<OnboardingResult> {
    let mut states = Vec::new();
    states.push(OnboardingState::Welcome);

    states.push(OnboardingState::Name);
    validate_name(&request.name)?;

    states.push(OnboardingState::Pin);
    validate_pin(&request.pin)?;

    states.push(OnboardingState::Biometric);
    let biometric_secret = request.biometric_template_secret.as_deref();

    states.push(OnboardingState::YubiKey);
    if request.enable_yubikey {
        return Err(anyhow!(
            "YubiKey envelope is a Y1 trait stub and has no hardware implementation"
        ));
    }

    states.push(OnboardingState::KeyGen);
    let created_at = config.created_at.unwrap_or_else(now_secs);
    let master_key = config
        .master_key
        .map(MasterKey::from_bytes)
        .unwrap_or_else(MasterKey::generate);
    let identity = Identity::new(request.name, request.biometric_template_public, created_at);
    let salt = config.envelope_salt.unwrap_or_else(generate_salt);
    let envelope = seal_envelope_a(
        &master_key,
        &request.pin,
        biometric_secret,
        &request.device_fingerprint,
        &salt,
        config.argon2_params,
    )?;
    let identity_blob = blob::build_blob(&identity, master_key.expose())?;

    states.push(OnboardingState::Bip39Display);
    let entropy = config.entropy.unwrap_or_else(random_entropy);
    let mnemonic = mnemonic_from_entropy(&entropy)?;

    write_identity_artifacts(&config.roots, &salt, &envelope, &identity_blob, created_at)?;
    sacred::update_l0_baseline_with_roots(true, &config.roots)?;

    states.push(OnboardingState::Sealed);
    Ok(OnboardingResult {
        identity,
        mnemonic,
        states,
    })
}

pub fn mnemonic_from_entropy(entropy: &[u8; 32]) -> Result<String> {
    let mnemonic =
        Mnemonic::from_entropy_in(Language::English, entropy).context("generate BIP39 mnemonic")?;
    Ok(mnemonic.to_string())
}

fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(anyhow!("identity name cannot be empty"));
    }
    Ok(())
}

fn validate_pin(pin: &str) -> Result<()> {
    if pin.len() < 6 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("PIN must contain at least 6 digits"));
    }
    Ok(())
}

fn write_identity_artifacts(
    roots: &SacredRoots,
    salt: &[u8; 16],
    envelope: &EnvelopeAFile,
    identity_blob: &[u8],
    created_at: u64,
) -> Result<()> {
    let identity_dir = roots.identity_dir();
    fs::create_dir_all(&identity_dir)
        .with_context(|| format!("create {}", identity_dir.display()))?;

    write_new_sacred(
        &roots.resolve_sacred_rel("identity/identity_core.salt"),
        roots,
        salt,
    )?;
    write_new_sacred(
        &roots.resolve_sacred_rel("identity/k_envelope_a.bin"),
        roots,
        &envelope.to_bytes()?,
    )?;
    write_new_sacred(
        &roots.resolve_sacred_rel("identity/identity_core.bin"),
        roots,
        identity_blob,
    )?;

    let lock_text = format!("seen_at={created_at}\n");
    write_new_sacred(
        &roots.resolve_sacred_rel("identity/bip39.lock"),
        roots,
        lock_text.as_bytes(),
    )?;
    Ok(())
}

fn write_new_sacred(path: &Path, roots: &SacredRoots, bytes: &[u8]) -> Result<()> {
    sacred::refuse_existing_sacred_write_with_roots(path, roots)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(path, bytes).with_context(|| format!("write {}", path.display()))
}

fn random_entropy() -> [u8; 32] {
    let mut entropy = [0u8; 32];
    OsRng.fill_bytes(&mut entropy);
    entropy
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
    use crate::identity::envelope::open_envelope_a;
    use tempfile::TempDir;

    fn roots() -> (TempDir, SacredRoots) {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        let data = tmp.path().join("data");
        fs::create_dir_all(repo.join("l0/src/identity")).unwrap();
        fs::create_dir_all(repo.join("l0/src")).unwrap();
        for rel in [
            "l0/src/identity/sacred.rs",
            "l0/src/identity/paths.rs",
            "l0/src/identity/blob.rs",
            "l0/src/identity/mod.rs",
            "l0/src/main.rs",
        ] {
            let full = repo.join(rel);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(full, rel.as_bytes()).unwrap();
        }
        let roots = SacredRoots {
            repo_root: repo,
            ybos_data_root: data,
        };
        (tmp, roots)
    }

    fn request() -> OnboardingRequest {
        OnboardingRequest {
            name: "Ada".to_string(),
            pin: "123456".to_string(),
            biometric_template_public: vec![1, 2, 3],
            biometric_template_secret: None,
            enable_yubikey: false,
            device_fingerprint: [5u8; 32],
        }
    }

    #[test]
    fn mnemonic_generation_is_deterministic_for_entropy() {
        let phrase = mnemonic_from_entropy(&[0u8; 32]).unwrap();
        assert_eq!(
            phrase,
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art"
        );
    }

    #[test]
    fn enrollment_happy_path_writes_sealed_artifacts() {
        let (_tmp, roots) = roots();
        let result = run_onboarding(
            request(),
            OnboardingConfig {
                roots: roots.clone(),
                argon2_params: EnvelopeAParams::test_fast(),
                master_key: Some([8u8; 32]),
                envelope_salt: Some([1u8; 16]),
                entropy: Some([0u8; 32]),
                created_at: Some(1_700_000_000),
            },
        )
        .unwrap();

        assert_eq!(
            result.states,
            vec![
                OnboardingState::Welcome,
                OnboardingState::Name,
                OnboardingState::Pin,
                OnboardingState::Biometric,
                OnboardingState::YubiKey,
                OnboardingState::KeyGen,
                OnboardingState::Bip39Display,
                OnboardingState::Sealed,
            ]
        );
        assert_eq!(result.identity.name, "Ada");
        assert!(roots
            .resolve_sacred_rel("identity/identity_core.bin")
            .exists());
        assert!(roots
            .resolve_sacred_rel("identity/identity_core.salt")
            .exists());
        assert!(roots
            .resolve_sacred_rel("identity/k_envelope_a.bin")
            .exists());
        assert!(roots.resolve_sacred_rel("identity/bip39.lock").exists());
        assert!(roots.hashes_file().exists());
        assert!(
            !fs::read_to_string(roots.resolve_sacred_rel("identity/bip39.lock"))
                .unwrap()
                .contains("abandon")
        );
    }

    #[test]
    fn hmac_verifies_after_unlocking_envelope_a() {
        let (_tmp, roots) = roots();
        run_onboarding(
            request(),
            OnboardingConfig {
                roots: roots.clone(),
                argon2_params: EnvelopeAParams::test_fast(),
                master_key: Some([8u8; 32]),
                envelope_salt: Some([1u8; 16]),
                entropy: Some([0u8; 32]),
                created_at: Some(1_700_000_000),
            },
        )
        .unwrap();

        let envelope = EnvelopeAFile::from_bytes(
            &fs::read(roots.resolve_sacred_rel("identity/k_envelope_a.bin")).unwrap(),
        )
        .unwrap();
        let salt: [u8; 16] = fs::read(roots.resolve_sacred_rel("identity/identity_core.salt"))
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap();
        let key = open_envelope_a(
            &envelope,
            "123456",
            None,
            &[5u8; 32],
            &salt,
            EnvelopeAParams::test_fast(),
        )
        .unwrap();
        let verified = blob::verify_blob(
            &fs::read(roots.resolve_sacred_rel("identity/identity_core.bin")).unwrap(),
            key.expose(),
        )
        .unwrap();
        assert_eq!(verified.identity.name, "Ada");
    }

    #[test]
    fn smoke_boot_onboarding_reboot_integrity_check() {
        let (_tmp, roots) = roots();
        sacred::boot_integrity_check_with_roots(&roots).unwrap();
        sacred::update_l0_baseline_with_roots(true, &roots).unwrap();
        run_onboarding(
            request(),
            OnboardingConfig {
                roots: roots.clone(),
                argon2_params: EnvelopeAParams::test_fast(),
                master_key: Some([8u8; 32]),
                envelope_salt: Some([1u8; 16]),
                entropy: Some([0u8; 32]),
                created_at: Some(1_700_000_000),
            },
        )
        .unwrap();

        sacred::boot_integrity_check_with_roots(&roots).unwrap();
    }

    #[test]
    fn onboarding_refuses_to_overwrite_sealed_identity() {
        let (_tmp, roots) = roots();
        let config = OnboardingConfig {
            roots: roots.clone(),
            argon2_params: EnvelopeAParams::test_fast(),
            master_key: Some([8u8; 32]),
            envelope_salt: Some([1u8; 16]),
            entropy: Some([0u8; 32]),
            created_at: Some(1_700_000_000),
        };
        run_onboarding(request(), config.clone()).unwrap();
        let err = run_onboarding(request(), config).unwrap_err();
        assert!(err.to_string().contains("sacred write refused"));
    }
}
