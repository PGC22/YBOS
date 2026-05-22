// Integration tests that do NOT need tracing log capture.
// Audit-log assertions live in the in-crate unit tests inside
// `orchestrator/src/capability.rs` (see the comment there for the rationale).

use std::path::PathBuf;
use ybos_orchestrator::capability::{enforce, Operation};
use ybos_orchestrator::manifest::{Manifest, Capabilities};

#[test]
fn test_path_normalization_blocks_dotdot_bypass() {
    let manifest = Manifest {
        name: "test-agent".to_string(),
        version: "0.1.0".to_string(),
        capabilities: Capabilities {
            fs_paths: vec![PathBuf::from("/data/agent/")],
            ..Default::default()
        },
    };

    // Valid path
    enforce(&manifest, &Operation::FsRead(PathBuf::from("/data/agent/data.txt"))).expect("Should allow valid path");

    // Bypass attempt
    let err = enforce(&manifest, &Operation::FsRead(PathBuf::from("/data/agent/../../etc/passwd"))).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("denied"));

    let err_write = enforce(&manifest, &Operation::FsWrite(PathBuf::from("/data/agent/../../tmp/x"))).unwrap_err();
    assert!(err_write.to_string().to_lowercase().contains("denied"));
}

#[test]
fn test_path_normalization_allows_canonical_subpath() {
    let manifest = Manifest {
        name: "test-agent".to_string(),
        version: "0.1.0".to_string(),
        capabilities: Capabilities {
            fs_paths: vec![PathBuf::from("/data/agent/")],
            ..Default::default()
        },
    };

    enforce(&manifest, &Operation::FsRead(PathBuf::from("/data/agent/./sub/../file.txt"))).expect("Should allow normalized subpath");
}

#[test]
fn test_path_normalization_normalizes_declared() {
    let manifest = Manifest {
        name: "test-agent".to_string(),
        version: "0.1.0".to_string(),
        capabilities: Capabilities {
            fs_paths: vec![PathBuf::from("/data/agent/../sub/")], // normalizes to /data/sub/
            ..Default::default()
        },
    };

    // Allowed because both are cleaned to /data/sub/
    enforce(&manifest, &Operation::FsRead(PathBuf::from("/data/sub/file"))).expect("Should allow matching normalized declared path");

    // Denied because cleaned declared is /data/sub/, not /data/agent/
    let err = enforce(&manifest, &Operation::FsRead(PathBuf::from("/data/agent/file"))).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("denied"));
}
