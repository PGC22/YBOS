use anyhow::{anyhow, Result};
use std::path::PathBuf;
use crate::manifest::{Manifest, AccessLevel, MemoryAccess};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CapabilityError {
    #[error("Capability denied: {0}")]
    Denied(String),
}

#[derive(Debug, Clone)]
pub enum Operation {
    NetConnect(String),
    FsRead(PathBuf),
    FsWrite(PathBuf),
    UserContextRead,
    UserContextWrite,
    LlmCall,
    MemoryRead,
    MemoryWrite,
}

pub fn enforce(manifest: &Manifest, op: &Operation) -> Result<()> {
    let res = match op {
        Operation::NetConnect(domain) => {
            if manifest.capabilities.net_domains.iter().any(|d| d == domain) {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied(format!(
                    "NetConnect({})",
                    domain
                ))))
            }
        }
        Operation::FsRead(path) => {
            // Path normalization prevents ".." bypass attacks by resolving lexical components.
            // Example: "/data/agent/../../etc/passwd" becomes "/etc/passwd".
            let requested_clean = path_clean::clean(path);
            let declared_clean_list: Vec<_> = manifest
                .capabilities
                .fs_paths
                .iter()
                .map(path_clean::clean)
                .collect();

            if declared_clean_list
                .iter()
                .any(|d| requested_clean.starts_with(d))
            {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied(format!(
                    "FsRead({})",
                    requested_clean.display()
                ))))
            }
        }
        Operation::FsWrite(path) => {
            // Path normalization prevents ".." bypass attacks by resolving lexical components.
            // Example: "/data/agent/../../etc/passwd" becomes "/etc/passwd".
            let requested_clean = path_clean::clean(path);
            let declared_clean_list: Vec<_> = manifest
                .capabilities
                .fs_paths
                .iter()
                .map(path_clean::clean)
                .collect();

            if declared_clean_list
                .iter()
                .any(|d| requested_clean.starts_with(d))
            {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied(format!(
                    "FsWrite({})",
                    requested_clean.display()
                ))))
            }
        }
        Operation::UserContextRead => {
            if manifest.capabilities.data_user_prefs == AccessLevel::Read
                || manifest.capabilities.data_user_prefs == AccessLevel::ReadWrite
            {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied(
                    "UserContextRead".to_string()
                )))
            }
        }
        Operation::UserContextWrite => {
            if manifest.capabilities.data_user_prefs == AccessLevel::ReadWrite {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied("UserContextWrite".to_string())))
            }
        }
        Operation::LlmCall => {
            if manifest.capabilities.llm {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied("LlmCall".to_string())))
            }
        }
        Operation::MemoryRead => {
            if manifest.capabilities.memory == MemoryAccess::Read ||
               manifest.capabilities.memory == MemoryAccess::ReadWrite {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied("MemoryRead".to_string())))
            }
        }
        Operation::MemoryWrite => {
            if manifest.capabilities.memory == MemoryAccess::ReadWrite {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied("MemoryWrite".to_string())))
            }
        }
    };

    // Audit log
    let op_log = match op {
        Operation::FsRead(path) => Operation::FsRead(path_clean::clean(path)),
        Operation::FsWrite(path) => Operation::FsWrite(path_clean::clean(path)),
        _ => op.clone(),
    };

    match &res {
        Ok(_) => {
            tracing::info!(
                target: "ybos.audit",
                agent = %manifest.name,
                op = ?op_log,
                outcome = "allow",
                "Capability check"
            );
        }
        Err(e) => {
            let reason_str = e.to_string();
            tracing::warn!(
                target: "ybos.audit",
                agent = %manifest.name,
                op = ?op_log,
                outcome = "deny",
                reason = %reason_str,
                "Capability check denied"
            );
        }
    }

    res
}
