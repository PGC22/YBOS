use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::agent::Agent;
use crate::manifest::Manifest;

type AgentFactory = Box<dyn Fn() -> Arc<dyn Agent> + Send + Sync>;

pub struct AgentRegistry {
    // Map of name -> (Manifest, AgentInstance OR Factory)
    entries: RwLock<HashMap<String, RegistryEntry>>,
}

enum RegistryEntry {
    Static(Arc<dyn Agent>),
    Runtime(Manifest, AgentFactory),
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_static(&self, agent: Arc<dyn Agent>) {
        let name = agent.manifest().name.clone();
        self.entries.write().unwrap().insert(name, RegistryEntry::Static(agent));
    }

    pub fn register_runtime(&self, manifest_toml: &str, factory: AgentFactory) -> anyhow::Result<()> {
        let manifest: Manifest = toml::from_str(manifest_toml)?;
        self.entries.write().unwrap().insert(manifest.name.clone(), RegistryEntry::Runtime(manifest, factory));
        Ok(())
    }

    pub fn list(&self) -> Vec<Manifest> {
        self.entries.read().unwrap().values().map(|entry| {
            match entry {
                RegistryEntry::Static(agent) => agent.manifest().clone(),
                RegistryEntry::Runtime(manifest, _) => manifest.clone(),
            }
        }).collect()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Agent>> {
        let entries = self.entries.read().unwrap();
        let entry = entries.get(name)?;
        match entry {
            RegistryEntry::Static(agent) => Some(agent.clone()),
            RegistryEntry::Runtime(_, factory) => Some(factory()),
        }
    }
}
