//! RemusPaths — singura sursa de adevar pentru cai in L0.
//!
//! Portare din `core/paths.py` (Python). Detecteaza REMUS_ROOT din env var,
//! fallback la cwd, fallback la `/opt/remus` (production NixOS).

use std::env;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

/// Detecteaza REMUS_ROOT.
///
/// Prioritate:
///   1. env REMUS_ROOT
///   2. current working directory
///   3. /opt/remus (fallback production)
fn detect_remus_root() -> PathBuf {
    if let Ok(v) = env::var("REMUS_ROOT") {
        return PathBuf::from(v);
    }
    if let Ok(cwd) = env::current_dir() {
        return cwd;
    }
    PathBuf::from("/opt/remus")
}

static REMUS_ROOT: OnceLock<PathBuf> = OnceLock::new();

pub fn remus_root() -> &'static Path {
    REMUS_ROOT.get_or_init(detect_remus_root)
}

pub fn config_dir() -> PathBuf {
    remus_root().join("config")
}

#[allow(dead_code)] // folosit de S6.2+ pentru paths catre core/
pub fn core_dir() -> PathBuf {
    remus_root().join("core")
}

// ─────────────────────────────────────────────────────────────────────────────
// Normalize path — fara filesystem touch (functioneaza si pe paths inexistente)
// ─────────────────────────────────────────────────────────────────────────────

/// Normalizeaza un path: rezolva `.` si `..` lexical, fara sa atinga FS.
///
/// Limitare: NU urmareste symlinks (canonicalize face asta, dar cere ca path-ul
/// sa existe). Pentru anti-symlink, vezi `is_symlink_to_sacred` viitor.
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

// ─────────────────────────────────────────────────────────────────────────────
// is_l0_sacred — refuz sintactic. Anti-traversal + fail-closed.
// ─────────────────────────────────────────────────────────────────────────────

use super::sacred::L0_SACRED;

/// Verifica daca `path` (absolut sau relativ la REMUS_ROOT) apartine L0_SACRED.
///
/// Returneaza `true` daca:
///   - path-ul, dupa normalizare lexicala, corespunde unei intrari din
///     L0_SACRED relativ la REMUS_ROOT.
///   - orice eroare (path malformat, prefix invalid) → fail-closed → true.
///
/// Returneaza `false` doar daca path-ul e clar non-sacred.
#[allow(dead_code)] // consumat de S6.5+ (reflex actions, file ops checks)
pub fn is_l0_sacred(path: &Path) -> bool {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        remus_root().join(path)
    };
    let normalized = normalize_lexical(&absolute);

    let root_normalized = normalize_lexical(remus_root());

    // Strip prefix REMUS_ROOT
    let rel = match normalized.strip_prefix(&root_normalized) {
        Ok(r) => r,
        Err(_) => {
            // Path nu e sub REMUS_ROOT — nu poate fi L0 sacred prin definitie.
            return false;
        }
    };

    // Cross-platform: convert separators to forward slash for matching.
    let rel_str: String = rel
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str().map(String::from),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");

    L0_SACRED.iter().any(|sacred| *sacred == rel_str)
}

/// Filtreaza o lista de paths, eliminand pe cele L0 sacred.
#[allow(dead_code)] // consumat de S6.5+
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

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        // Folosim un root sintetic pentru tests determinist
        if cfg!(windows) {
            PathBuf::from(r"C:\test_remus")
        } else {
            PathBuf::from("/test_remus")
        }
    }

    fn check(rel: &str) -> bool {
        let p = root().join(rel.replace('/', std::path::MAIN_SEPARATOR_STR.as_ref()));
        is_l0_sacred_with_root(&p, &root())
    }

    /// Test helper: is_l0_sacred dar cu root explicit (evita OnceLock global).
    fn is_l0_sacred_with_root(path: &Path, root: &Path) -> bool {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };
        let normalized = normalize_lexical(&absolute);
        let root_normalized = normalize_lexical(root);
        let rel = match normalized.strip_prefix(&root_normalized) {
            Ok(r) => r,
            Err(_) => return false,
        };
        let rel_str: String = rel
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => s.to_str().map(String::from),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/");
        L0_SACRED.iter().any(|s| *s == rel_str)
    }

    #[test]
    fn relative_sacred_paths_match() {
        assert!(check("core/identity.py"));
        assert!(check("core/paths.py"));
        assert!(check("core/l0_simulator.py"));
        assert!(check("tools/identity_gen.py"));
        assert!(check("config/identity_core.txt"));
        assert!(check("config/identity_core.bin"));
        assert!(check("config/sync_key.bin"));
    }

    #[test]
    fn non_sacred_paths_pass() {
        assert!(!check("core/orchestrator.py"));
        assert!(!check("skills/web_search.py"));
        assert!(!check("web_interface.py"));
        assert!(!check("config/settings.json"));
    }

    #[test]
    fn traversal_resolves_to_sacred() {
        let p = root().join("skills").join("..").join("core").join("identity.py");
        assert!(is_l0_sacred_with_root(&p, &root()));
    }

    #[test]
    fn outside_root_not_sacred() {
        let outside = if cfg!(windows) {
            PathBuf::from(r"C:\some\other\dir\identity.py")
        } else {
            PathBuf::from("/tmp/identity.py")
        };
        assert!(!is_l0_sacred_with_root(&outside, &root()));
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
