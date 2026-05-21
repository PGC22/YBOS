//! L0 SACRED — lista intangibila + tripwire boot.
//!
//! Portare directa din `core/paths.py` (Python).
//! Vezi `docs/L0_SACRED.md` pentru rationale + reguli.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;

use super::paths::{config_dir, remus_root};

// ─────────────────────────────────────────────────────────────────────────────
// L0_SACRED — lista hardcodata.
//
// SINCRONIZATA cu `core/paths.py::L0_SACRED` (Python).
// Hash hardcodat trebuie sa fie acelasi cu Python `_hash_l0_sacred_list()`.
// ─────────────────────────────────────────────────────────────────────────────

pub const L0_SACRED: &[&str] = &[
    "config/identity_core.bin",
    "config/identity_core.txt",
    "config/sync_key.bin",
    "core/identity.py",
    "core/l0_simulator.py",
    "core/paths.py",
    "tools/identity_gen.py",
];

/// Hash SHA256 al listei L0_SACRED, ordonata si separata cu `|`.
/// Trebuie sa fie acelasi cu cel din Python `core/paths.py::L0_SACRED_LIST_HASH`.
pub const L0_SACRED_LIST_HASH: &str =
    "e63d546799b8c6b626d0a1e977c7de42937ffcb7216c27a025c8e240c8c615a9";

/// Recalculeaza hash-ul listei (folosit la verificarea anti-tamper).
pub fn hash_l0_sacred_list() -> String {
    let mut sorted: Vec<&str> = L0_SACRED.to_vec();
    sorted.sort();
    let joined = sorted.join("|");
    let mut hasher = Sha256::new();
    hasher.update(joined.as_bytes());
    hex::encode(hasher.finalize())
}

/// True daca lista L0_SACRED nu a fost alterata fata de baseline.
pub fn verify_l0_list_integrity() -> bool {
    hash_l0_sacred_list() == L0_SACRED_LIST_HASH
}

// ─────────────────────────────────────────────────────────────────────────────
// Hash-uri ale fisierelor L0 sacred
// ─────────────────────────────────────────────────────────────────────────────

/// Pentru fiecare fisier L0 sacred: SHA256 hex sau "MISSING" sau "ERROR:..."
pub fn compute_l0_files_hash() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for rel in L0_SACRED {
        let full = remus_root().join(rel);
        let value = match fs::read(&full) {
            Ok(bytes) => {
                let mut h = Sha256::new();
                h.update(&bytes);
                hex::encode(h.finalize())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => "MISSING".to_string(),
            Err(e) => format!("ERROR:{}", e.kind()),
        };
        out.insert((*rel).to_string(), value);
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Boot integrity check — tripwire L0
// ─────────────────────────────────────────────────────────────────────────────

fn hashes_file_path() -> std::path::PathBuf {
    config_dir().join("l0_sacred.hashes.json")
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SavedHashes(BTreeMap<String, String>);

/// Rezultat al verificarii integritatii L0.
#[derive(Debug)]
pub struct IntegrityReport {
    pub ok: bool,
    pub alerts: Vec<String>,
}

/// Tripwire boot. Vezi `core/paths.py::boot_integrity_check()` pentru paritate.
///
/// Logica:
///   1. Verifica `verify_l0_list_integrity()` (anti-tamper pe lista in sine).
///   2. Daca `config/l0_sacred.hashes.json` lipseste → prima rulare, scrie
///      baseline si returneaza ok.
///   3. Daca exista → compara hash-uri. Diferente → alerte critice.
pub fn boot_integrity_check() -> Result<IntegrityReport> {
    let mut alerts = Vec::new();

    // Step 1: integritatea listei
    if !verify_l0_list_integrity() {
        alerts.push(format!(
            "CRITIC: lista L0_SACRED a fost alterata (hash actual={} expected={})",
            &hash_l0_sacred_list()[..16],
            &L0_SACRED_LIST_HASH[..16]
        ));
        return Ok(IntegrityReport { ok: false, alerts });
    }

    // Step 2 + 3: hash-uri fisiere
    let current = compute_l0_files_hash();
    let hashes_path = hashes_file_path();

    if !hashes_path.exists() {
        // Prima rulare — scrie baseline.
        if let Some(parent) = hashes_path.parent() {
            fs::create_dir_all(parent).context("create config dir")?;
        }
        let saved = SavedHashes(current.clone());
        let json = serde_json::to_string_pretty(&saved.0).context("serialize hashes")?;
        fs::write(&hashes_path, json).context("write l0_sacred.hashes.json")?;
        alerts.push("L0 baseline creat (prima rulare).".to_string());
        return Ok(IntegrityReport { ok: true, alerts });
    }

    // Comparare cu baseline existent.
    let saved_text = fs::read_to_string(&hashes_path).context("read l0_sacred.hashes.json")?;
    let saved: BTreeMap<String, String> =
        serde_json::from_str(&saved_text).context("parse l0_sacred.hashes.json")?;

    let mut ok = true;
    for rel in L0_SACRED {
        let key = (*rel).to_string();
        let cur = current.get(&key).cloned().unwrap_or_else(|| "MISSING".to_string());
        let sav = saved.get(&key).cloned().unwrap_or_else(|| "MISSING".to_string());
        if cur != sav {
            alerts.push(format!(
                "L0 ALTERAT: {} (current={}... saved={}...)",
                rel,
                &cur.chars().take(12).collect::<String>(),
                &sav.chars().take(12).collect::<String>()
            ));
            ok = false;
        }
    }

    Ok(IntegrityReport { ok, alerts })
}

/// Rescrie baseline-ul cu hash-urile actuale. Apelata manual dupa modificare
/// intentionata a unui L0 sacred file.
///
/// `force` trebuie sa fie `true` — protectie minima impotriva apelului accidental.
#[allow(dead_code)]
pub fn update_l0_baseline(force: bool) -> Result<()> {
    if !force {
        return Err(anyhow!("update_l0_baseline necesita force=true"));
    }
    let hashes_path = hashes_file_path();
    if let Some(parent) = hashes_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let current = compute_l0_files_hash();
    let json = serde_json::to_string_pretty(&current)?;
    fs::write(&hashes_path, json)?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_hash_matches_baseline() {
        // Daca acest test pica, L0_SACRED a fost modificat dar
        // L0_SACRED_LIST_HASH nu a fost actualizat. Sau invers.
        // Sau lista nu mai e in sync cu cea Python.
        assert_eq!(
            hash_l0_sacred_list(),
            L0_SACRED_LIST_HASH,
            "L0_SACRED list hash diverged. Recalculate via hash_l0_sacred_list() and update L0_SACRED_LIST_HASH constant. Verifica si paritatea cu Python core/paths.py::L0_SACRED_LIST_HASH."
        );
    }

    #[test]
    fn list_hash_stable() {
        let h1 = hash_l0_sacred_list();
        let h2 = hash_l0_sacred_list();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn list_matches_python() {
        // Hash hardcodat trebuie sa fie identic cu Python.
        // Calculat in Python: sha256("|".join(sorted(L0_SACRED))).hexdigest()
        // Daca aici pica si list_hash_matches_baseline pica → e bug.
        // Daca aici pica si list_hash_matches_baseline trece → Python si Rust
        // au liste diferite.
        let expected = "e63d546799b8c6b626d0a1e977c7de42937ffcb7216c27a025c8e240c8c615a9";
        assert_eq!(hash_l0_sacred_list(), expected);
    }

    #[test]
    fn list_size_matches_python() {
        assert_eq!(L0_SACRED.len(), 7);
    }

    #[test]
    fn list_contains_all_expected() {
        let expected = [
            "core/l0_simulator.py",
            "core/identity.py",
            "core/paths.py",
            "tools/identity_gen.py",
            "config/identity_core.txt",
            "config/identity_core.bin",
            "config/sync_key.bin",
        ];
        for e in expected {
            assert!(L0_SACRED.contains(&e), "lipseste din L0_SACRED: {}", e);
        }
    }
}
