use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::agent::{Agent, AgentCall, AgentResponse};
use crate::manifest::Manifest;
use crate::registry::AgentRegistry;

#[derive(Debug, Clone)]
pub struct RuntimeHandle {
    pub agent_name: String,
}

#[async_trait]
pub trait AgentRuntime: Send + Sync {
    async fn spawn(&self, manifest: Manifest) -> Result<RuntimeHandle>;
    async fn invoke(&self, handle: &RuntimeHandle, call: AgentCall) -> Result<AgentResponse>;
}

pub struct InProcessRuntime {
    registry: Arc<AgentRegistry>,
    active_agents: RwLock<HashMap<String, Arc<dyn Agent>>>,
}

impl InProcessRuntime {
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self {
            registry,
            active_agents: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl AgentRuntime for InProcessRuntime {
    async fn spawn(&self, manifest: Manifest) -> Result<RuntimeHandle> {
        let name = manifest.name.clone();

        // If already active, just return handle
        if self.active_agents.read().unwrap().contains_key(&name) {
            return Ok(RuntimeHandle { agent_name: name });
        }

        let agent = self.registry.get(&name)
            .ok_or_else(|| anyhow!("agent not found in registry: {}", name))?;

        self.active_agents.write().unwrap().insert(name.clone(), agent);
        Ok(RuntimeHandle { agent_name: name })
    }

    async fn invoke(&self, handle: &RuntimeHandle, call: AgentCall) -> Result<AgentResponse> {
        let agent = self.active_agents.read().unwrap().get(&handle.agent_name)
            .cloned()
            .ok_or_else(|| anyhow!("agent not active in runtime: {}", handle.agent_name))?;

        agent.invoke(call).await
    }
}

/// Placeholder for future process isolation.
pub trait SubprocessRuntime: AgentRuntime {}
