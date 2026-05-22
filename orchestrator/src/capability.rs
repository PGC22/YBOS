use anyhow::{anyhow, Result};
use std::path::PathBuf;
use crate::manifest::{Manifest, AccessLevel};
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
}

pub fn enforce(manifest: &Manifest, op: &Operation) -> Result<()> {
    match op {
        Operation::NetConnect(domain) => {
            if manifest.capabilities.net_domains.iter().any(|d| d == domain) {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied(format!("NetConnect({})", domain))))
            }
        }
        Operation::FsRead(path) => {
            if manifest.capabilities.fs_paths.iter().any(|p| path.starts_with(p)) {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied(format!("FsRead({})", path.display()))))
            }
        }
        Operation::FsWrite(path) => {
            if manifest.capabilities.fs_paths.iter().any(|p| path.starts_with(p)) {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied(format!("FsWrite({})", path.display()))))
            }
        }
        Operation::UserContextRead => {
            if manifest.capabilities.data_user_prefs == AccessLevel::Read ||
               manifest.capabilities.data_user_prefs == AccessLevel::ReadWrite {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied("UserContextRead".to_string())))
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
    }
}
