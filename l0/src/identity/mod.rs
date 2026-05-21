//! Identity — verificare HMAC pe `config/identity_core.bin` + boot integrity check.
//!
//! Portare din `core/paths.py` + `core/l0_simulator.py` (Python).
//!
//! Modules:
//!   - `paths`: RemusPaths, is_l0_sacred, normalize_lexical
//!   - `sacred`: L0_SACRED const, boot_integrity_check (tripwire L0)
//!   - `blob`: parse + HMAC verify pentru identity_core.bin

pub mod blob;
pub mod paths;
pub mod sacred;

use anyhow::Result;
use std::sync::RwLock;
use tracing::{error, info, warn};

use blob::VerifiedIdentity;

/// Cache global pentru identitatea verificata. Populat de `boot_check()`.
/// Citit de gRPC `IdentityService.GetIdentity()` (S6.4).
static IDENTITY: RwLock<Option<VerifiedIdentity>> = RwLock::new(None);

/// Returneaza identitatea verificata (sau None daca boot_check nu a fost apelat
/// sau a esuat).
#[allow(dead_code)]
pub fn current_identity() -> Option<VerifiedIdentity> {
    IDENTITY.read().ok().and_then(|g| g.clone())
}

/// Boot check — tripwire L0 + HMAC verify pe identity_core.bin.
///
/// Returneaza `Ok(())` daca boot poate continua. Returneaza `Err` daca:
///   - lista L0_SACRED a fost alterata in runtime
///   - hash-urile L0 sacred files difera fata de baseline
///   - identity_core.bin lipseste/invalid/semnatura proasta
///
/// Daca identity_core.bin lipseste (prima rulare), logghea WARNING dar
/// permite boot in mod "unverified" — paritate cu l0_simulator.py.
pub async fn boot_check() -> Result<()> {
    // Step 1: L0 sacred tripwire
    let report = sacred::boot_integrity_check()?;
    for a in &report.alerts {
        if a.starts_with("CRITIC") || a.starts_with("L0 ALTERAT") {
            error!("[L0/identity] {}", a);
        } else {
            info!("[L0/identity] {}", a);
        }
    }
    if !report.ok {
        return Err(anyhow::anyhow!(
            "Boot blocat: integritate L0 sacred compromisa. Vezi alertele de mai sus."
        ));
    }

    // Step 2: HMAC verify identity blob
    match blob::load_and_verify() {
        Ok(identity) => {
            info!(
                "[L0/identity] Identitate verificata (v{}) — Remus ID: {}...",
                identity.header_version,
                identity.remus_id_short()
            );
            if let Ok(mut guard) = IDENTITY.write() {
                *guard = Some(identity);
            }
        }
        Err(e) => {
            warn!("[L0/identity] {}", e);
            warn!("[L0/identity] Boot in mod UNVERIFIED (paritate cu l0_simulator.py).");
        }
    }

    Ok(())
}
