use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use crate::agent::Agent;
use crate::manifest::Manifest;

type AgentFactory = Box<dyn Fn() -> Arc<dyn Agent> + Send + Sync>;

pub struct AgentRegistry {
    entries: RwLock<HashMap<String, RegistryEntry>>,
}

enum RegistryEntry {
    Static(Arc<dyn Agent>),
    // Runtime entries cache the first instance produced by the factory so that
    // subsequent get() calls return the SAME Arc, matching Static semantics.
    // Without the cache, get() would return a fresh agent each time, which is
    // surprising lifecycle behaviour for callers expecting shared state.
    Runtime {
        manifest: Manifest,
        factory: AgentFactory,
        instance: Mutex<Option<Arc<dyn Agent>>>,
    },
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_static(&self, agent: Arc<dyn Agent>) {
        let name = agent.manifest().name.clone();
        self.entries
            .write()
            .expect("AgentRegistry: entries lock poisoned")
            .insert(name, RegistryEntry::Static(agent));
    }

    pub fn register_runtime(&self, manifest_toml: &str, factory: AgentFactory) -> anyhow::Result<()> {
        let manifest: Manifest = toml::from_str(manifest_toml)?;
        self.entries
            .write()
            .expect("AgentRegistry: entries lock poisoned")
            .insert(
                manifest.name.clone(),
                RegistryEntry::Runtime {
                    manifest,
                    factory,
                    instance: Mutex::new(None),
                },
            );
        Ok(())
    }

    pub fn list(&self) -> Vec<Manifest> {
        self.entries
            .read()
            .expect("AgentRegistry: entries lock poisoned")
            .values()
            .map(|entry| match entry {
                RegistryEntry::Static(agent) => agent.manifest().clone(),
                RegistryEntry::Runtime { manifest, .. } => manifest.clone(),
            })
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Agent>> {
        let entries = self
            .entries
            .read()
            .expect("AgentRegistry: entries lock poisoned");
        let entry = entries.get(name)?;
        match entry {
            RegistryEntry::Static(agent) => Some(agent.clone()),
            RegistryEntry::Runtime { factory, instance, .. } => {
                let mut guard = instance
                    .lock()
                    .expect("AgentRegistry: runtime instance lock poisoned");
                if let Some(existing) = guard.as_ref() {
                    Some(existing.clone())
                } else {
                    let new_agent = factory();
                    *guard = Some(new_agent.clone());
                    Some(new_agent)
                }
            }
        }
    }
}
