//! Path helpers for L0 identity data.
//!
//! Runtime identity artifacts live under `${YBOS_DATA}/identity/...`. Source
//! files that are part of the boot tripwire are resolved from the repository
//! root, while generated identity files are resolved from `YBOS_DATA`.

use std::env;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

pub const YBOS_DATA_ENV: &str = "YBOS_DATA";
pub const YBOS_REPO_ROOT_ENV: &str = "YBOS_REPO_ROOT";

fn detect_ybos_data_root() -> PathBuf {
    if let Ok(v) = env::var(YBOS_DATA_ENV) {
        return PathBuf::from(v);
    }
    if let Ok(cwd) = env::current_dir() {
        return cwd.join(".ybos-data");
    }
    PathBuf::from("/var/lib/ybos")
}

fn detect_repo_root() -> PathBuf {
    if let Ok(v) = env::var(YBOS_REPO_ROOT_ENV) {
        return PathBuf::from(v);
    }
    if let Ok(cwd) = env::current_dir() {
        if cwd.file_name().and_then(|s| s.to_str()) == Some("l0") {
            if let Some(parent) = cwd.parent() {
                return parent.to_path_buf();
            }
        }
        return cwd;
    }
    PathBuf::from(".")
}

static YBOS_DATA_ROOT: OnceLock<PathBuf> = OnceLock::new();
static REPO_ROOT: OnceLock<PathBuf> = OnceLock::new();

pub fn ybos_data_root() -> &'static Path {
    YBOS_DATA_ROOT.get_or_init(detect_ybos_data_root)
}

pub fn repo_root() -> &'static Path {
    REPO_ROOT.get_or_init(detect_repo_root)
}

pub fn identity_dir() -> PathBuf {
    ybos_data_root().join("identity")
}

pub fn identity_blob() -> PathBuf {
    identity_dir().join("identity_core.bin")
}

pub fn identity_salt() -> PathBuf {
    identity_dir().join("identity_core.salt")
}

pub fn bip39_lock() -> PathBuf {
    identity_dir().join("bip39.lock")
}

pub fn envelope_a() -> PathBuf {
    identity_dir().join("k_envelope_a.bin")
}

#[allow(dead_code)]
pub fn envelope_b() -> PathBuf {
    identity_dir().join("k_envelope_b.bin")
}

#[allow(dead_code)]
pub fn envelope_c() -> PathBuf {
    identity_dir().join("k_envelope_c.bin")
}

pub fn l0_sacred_hashes() -> PathBuf {
    identity_dir().join("l0_sacred.hashes.json")
}

/// Normalize a path lexically without touching the filesystem.
///
/// This keeps tests and pre-create checks deterministic for paths that do not
/// exist yet. Symlink hardening belongs to the platform policy layer.
pub fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                out.push(c.as_os_str());
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct SacredRoots {
    pub repo_root: PathBuf,
    pub ybos_data_root: PathBuf,
}

impl SacredRoots {
    pub fn current() -> Self {
        Self {
            repo_root: repo_root().to_path_buf(),
            ybos_data_root: ybos_data_root().to_path_buf(),
        }
    }

    pub fn identity_dir(&self) -> PathBuf {
        self.ybos_data_root.join("identity")
    }

    pub fn hashes_file(&self) -> PathBuf {
        self.identity_dir().join("l0_sacred.hashes.json")
    }

    pub fn resolve_sacred_rel(&self, rel: &str) -> PathBuf {
        if rel.starts_with("identity/") {
            self.ybos_data_root.join(rel)
        } else {
            self.repo_root.join(rel)
        }
    }
}

use super::sacred::L0_SACRED;

/// Syntactic L0 SACRED check for absolute paths or paths relative to the repo
/// root / YBOS_DATA root. Errors and ambiguous ownership fail closed.
#[allow(dead_code)]
pub fn is_l0_sacred(path: &Path) -> bool {
    is_l0_sacred_with_roots(path, &SacredRoots::current())
}

pub fn is_l0_sacred_with_roots(path: &Path, roots: &SacredRoots) -> bool {
    if path.as_os_str().is_empty() {
        return true;
    }

    let candidates = if path.is_absolute() {
        vec![normalize_lexical(path)]
    } else {
        vec![
            normalize_lexical(&roots.repo_root.join(path)),
            normalize_lexical(&roots.ybos_data_root.join(path)),
        ]
    };

    for candidate in candidates {
        for sacred in L0_SACRED {
            let sacred_abs = normalize_lexical(&roots.resolve_sacred_rel(sacred));
            if candidate == sacred_abs {
                return true;
            }
        }
    }

    false
}

#[allow(dead_code)]
pub fn filter_l0_sacred<'a, I, P>(paths: I) -> Vec<PathBuf>
where
    I: IntoIterator<Item = &'a P>,
    P: AsRef<Path> + 'a + ?Sized,
{
    paths
        .into_iter()
        .map(|p| p.as_ref().to_path_buf())
        .filter(|p| !is_l0_sacred(p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots() -> SacredRoots {
        let base = if cfg!(windows) {
            PathBuf::from(r"C:\ybos-test")
        } else {
            PathBuf::from("/ybos-test")
        };
        SacredRoots {
            repo_root: base.join("repo"),
            ybos_data_root: base.join("data"),
        }
    }

    fn check(rel: &str) -> bool {
        is_l0_sacred_with_roots(Path::new(rel), &roots())
    }

    #[test]
    fn source_sacred_paths_match() {
        assert!(check("l0/src/identity/sacred.rs"));
        assert!(check("l0/src/identity/paths.rs"));
        assert!(check("l0/src/identity/blob.rs"));
        assert!(check("l0/src/identity/mod.rs"));
        assert!(check("l0/src/main.rs"));
    }

    #[test]
    fn identity_sacred_paths_match() {
        assert!(check("identity/identity_core.bin"));
        assert!(check("identity/identity_core.salt"));
        assert!(check("identity/bip39.lock"));
        assert!(check("identity/k_envelope_a.bin"));
        assert!(check("identity/k_envelope_b.bin"));
        assert!(check("identity/k_envelope_c.bin"));
        assert!(check("identity/l0_sacred.hashes.json"));
    }

    #[test]
    fn non_sacred_paths_pass() {
        assert!(!check("l0/src/hw/mod.rs"));
        assert!(!check("identity/profile.json"));
        assert!(!check("settings/settings.json"));
    }

    #[test]
    fn absolute_identity_path_matches() {
        let p = roots()
            .ybos_data_root
            .join("identity")
            .join("identity_core.bin");
        assert!(is_l0_sacred_with_roots(&p, &roots()));
    }

    #[test]
    fn traversal_resolves_to_sacred() {
        let p = Path::new("identity")
            .join("tmp")
            .join("..")
            .join("identity_core.bin");
        assert!(is_l0_sacred_with_roots(&p, &roots()));
    }

    #[test]
    fn outside_roots_not_sacred() {
        let outside = if cfg!(windows) {
            PathBuf::from(r"C:\some\other\dir\identity_core.bin")
        } else {
            PathBuf::from("/tmp/identity_core.bin")
        };
        assert!(!is_l0_sacred_with_roots(&outside, &roots()));
    }

    #[test]
    fn empty_path_fails_closed() {
        assert!(is_l0_sacred_with_roots(Path::new(""), &roots()));
    }

    #[test]
    fn normalize_handles_dotdot() {
        let p = Path::new("a").join("b").join("..").join("c");
        let n = normalize_lexical(&p);
        assert_eq!(n, Path::new("a").join("c"));
    }

    #[test]
    fn normalize_drops_curdir() {
        let p = Path::new(".").join("a").join(".").join("b");
        let n = normalize_lexical(&p);
        assert_eq!(n, Path::new("a").join("b"));
    }
}
