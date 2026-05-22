//! L0 SACRED list and boot tripwire.
//!
//! The list includes both source files that enforce identity safety and sealed
//! identity artifacts under `${YBOS_DATA}/identity/...`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use super::paths::{self, SacredRoots};

pub const L0_SACRED_HASHES_REL: &str = "identity/l0_sacred.hashes.json";

pub const L0_SACRED: &[&str] = &[
    "l0/src/identity/sacred.rs",
    "l0/src/identity/paths.rs",
    "l0/src/identity/blob.rs",
    "l0/src/identity/mod.rs",
    "l0/src/main.rs",
    "identity/identity_core.bin",
    "identity/identity_core.salt",
    "identity/bip39.lock",
    "identity/k_envelope_a.bin",
    "identity/k_envelope_b.bin",
    "identity/k_envelope_c.bin",
    L0_SACRED_HASHES_REL,
];

/// SHA-256 of the sorted L0_SACRED list joined by `|`.
pub const L0_SACRED_LIST_HASH: &str =
    "c6eb88cb6bca554e8c185c56bb255e3a927c9fcfb89aa705bb839fa7f618d9bd";

#[derive(Debug, Error)]
pub enum Error {
    #[error("sacred list tampered")]
    SacredListTampered,
    #[error("sacred file tampered: {0}")]
    SacredFileTampered(PathBuf),
    #[error("sacred write refused: {0}")]
    SacredViolation(PathBuf),
    #[error("io error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("json error for {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedHashes {
    pub sacred_list_hash: String,
    pub files: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct IntegrityReport {
    pub alerts: Vec<String>,
}

pub fn hash_l0_sacred_list() -> String {
    let mut sorted: Vec<&str> = L0_SACRED.to_vec();
    sorted.sort();
    let joined = sorted.join("|");
    let mut hasher = Sha256::new();
    hasher.update(joined.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn verify_l0_list_integrity() -> bool {
    hash_l0_sacred_list() == L0_SACRED_LIST_HASH
}

fn hash_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut h = Sha256::new();
    h.update(bytes);
    Ok(hex::encode(h.finalize()))
}

fn hashable_sacred() -> impl Iterator<Item = &'static str> {
    L0_SACRED
        .iter()
        .copied()
        .filter(|rel| *rel != L0_SACRED_HASHES_REL)
}

pub fn compute_l0_files_hash() -> Result<BTreeMap<String, String>> {
    compute_l0_files_hash_with_roots(&SacredRoots::current())
}

pub fn compute_l0_files_hash_with_roots(roots: &SacredRoots) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for rel in hashable_sacred() {
        let full = roots.resolve_sacred_rel(rel);
        let value = if full.exists() {
            hash_file(&full)?
        } else {
            "MISSING".to_string()
        };
        out.insert(rel.to_string(), value);
    }
    Ok(out)
}

pub fn boot_integrity_check() -> Result<IntegrityReport> {
    boot_integrity_check_with_roots(&SacredRoots::current())
}

pub fn boot_integrity_check_with_roots(roots: &SacredRoots) -> Result<IntegrityReport> {
    if !verify_l0_list_integrity() {
        return Err(Error::SacredListTampered);
    }

    let current = compute_l0_files_hash_with_roots(roots)?;
    let hashes_path = roots.hashes_file();

    if !hashes_path.exists() {
        write_hash_manifest(&hashes_path, &current)?;
        return Ok(IntegrityReport {
            alerts: vec!["L0 baseline created".to_string()],
        });
    }

    let saved = read_hash_manifest(&hashes_path)?;
    if saved.sacred_list_hash != L0_SACRED_LIST_HASH {
        return Err(Error::SacredListTampered);
    }

    for rel in hashable_sacred() {
        let cur = current
            .get(rel)
            .cloned()
            .unwrap_or_else(|| "MISSING".to_string());
        let sav = saved
            .files
            .get(rel)
            .cloned()
            .unwrap_or_else(|| "MISSING".to_string());
        if cur != sav {
            return Err(Error::SacredFileTampered(roots.resolve_sacred_rel(rel)));
        }
    }

    Ok(IntegrityReport { alerts: Vec::new() })
}

pub fn update_l0_baseline(force: bool) -> Result<()> {
    update_l0_baseline_with_roots(force, &SacredRoots::current())
}

pub fn update_l0_baseline_with_roots(force: bool, roots: &SacredRoots) -> Result<()> {
    if !force {
        return Err(Error::SacredViolation(roots.hashes_file()));
    }
    if !verify_l0_list_integrity() {
        return Err(Error::SacredListTampered);
    }
    let current = compute_l0_files_hash_with_roots(roots)?;
    write_hash_manifest(&roots.hashes_file(), &current)
}

fn read_hash_manifest(path: &Path) -> Result<SavedHashes> {
    let saved_text = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&saved_text).map_err(|source| Error::Json {
        path: path.to_path_buf(),
        source,
    })
}

fn write_hash_manifest(path: &Path, files: &BTreeMap<String, String>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let saved = SavedHashes {
        sacred_list_hash: L0_SACRED_LIST_HASH.to_string(),
        files: files.clone(),
    };
    let json = serde_json::to_vec_pretty(&saved).map_err(|source| Error::Json {
        path: path.to_path_buf(),
        source,
    })?;
    fs::write(path, json).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Guard used by enrollment writes. Initial onboarding may create missing
/// sacred artifacts, but it must never overwrite a sealed artifact.
pub fn refuse_existing_sacred_write(path: &Path) -> Result<()> {
    refuse_existing_sacred_write_with_roots(path, &SacredRoots::current())
}

pub fn refuse_existing_sacred_write_with_roots(path: &Path, roots: &SacredRoots) -> Result<()> {
    if path.exists() && paths::is_l0_sacred_with_roots(path, roots) {
        return Err(Error::SacredViolation(path.to_path_buf()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn roots() -> (TempDir, SacredRoots) {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        let data = tmp.path().join("data");
        fs::create_dir_all(repo.join("l0/src/identity")).unwrap();
        fs::create_dir_all(repo.join("l0/src")).unwrap();
        fs::create_dir_all(data.join("identity")).unwrap();
        let roots = SacredRoots {
            repo_root: repo,
            ybos_data_root: data,
        };
        for rel in hashable_sacred() {
            let full = roots.resolve_sacred_rel(rel);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&full, format!("fixture:{rel}")).unwrap();
        }
        (tmp, roots)
    }

    #[test]
    fn list_hash_matches_baseline() {
        assert_eq!(hash_l0_sacred_list(), L0_SACRED_LIST_HASH);
    }

    #[test]
    fn list_hash_stable() {
        let h1 = hash_l0_sacred_list();
        let h2 = hash_l0_sacred_list();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn list_contains_ybos_layout() {
        let expected = [
            "l0/src/identity/sacred.rs",
            "l0/src/identity/paths.rs",
            "l0/src/identity/blob.rs",
            "l0/src/identity/mod.rs",
            "l0/src/main.rs",
            "identity/identity_core.bin",
            "identity/identity_core.salt",
            "identity/bip39.lock",
            "identity/k_envelope_a.bin",
            "identity/k_envelope_b.bin",
            "identity/k_envelope_c.bin",
            L0_SACRED_HASHES_REL,
        ];
        assert_eq!(L0_SACRED, expected);
    }

    #[test]
    fn tripwire_detects_file_modification() {
        let (_tmp, roots) = roots();
        boot_integrity_check_with_roots(&roots).unwrap();

        let target = roots.resolve_sacred_rel("l0/src/identity/blob.rs");
        let mut f = fs::OpenOptions::new().append(true).open(&target).unwrap();
        writeln!(f, "tampered").unwrap();

        let err = boot_integrity_check_with_roots(&roots).unwrap_err();
        match err {
            Error::SacredFileTampered(path) => assert_eq!(path, target),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn tripwire_detects_manifest_list_hash_tamper() {
        let (_tmp, roots) = roots();
        boot_integrity_check_with_roots(&roots).unwrap();

        let manifest = roots.hashes_file();
        let mut saved = read_hash_manifest(&manifest).unwrap();
        saved.sacred_list_hash = "0".repeat(64);
        fs::write(&manifest, serde_json::to_vec_pretty(&saved).unwrap()).unwrap();

        let err = boot_integrity_check_with_roots(&roots).unwrap_err();
        assert!(matches!(err, Error::SacredListTampered));
    }
}
