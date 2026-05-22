use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Capabilities {
    #[serde(default)]
    pub net_domains: Vec<String>,
    #[serde(default)]
    pub fs_paths: Vec<PathBuf>,
    #[serde(default)]
    pub data_types: Vec<String>,
    #[serde(default)]
    pub data_user_prefs: AccessLevel,
    #[serde(default)]
    pub llm: bool,
    #[serde(default)]
    pub memory: MemoryAccess,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAccess {
    #[default]
    None,
    Read,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    #[default]
    None,
    Read,
    ReadWrite,
}
