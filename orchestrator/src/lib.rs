pub mod agent;
pub mod manifest;
pub mod capability;
pub mod runtime;
pub mod registry;
pub mod l0_client;
pub mod agents;

pub mod pb {
    pub mod l0 {
        tonic::include_proto!("ybos.l0.v1");
    }
    pub mod orchestrator {
        tonic::include_proto!("ybos.orchestrator.v1");
    }
}
