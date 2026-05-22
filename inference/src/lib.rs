pub mod trait_def;
pub mod types;
pub mod mock;
pub mod remote_api;

#[cfg(feature = "local_llama")]
pub mod local_llama;

pub use trait_def::Inference;
pub use types::*;
pub use mock::MockInference;
pub use remote_api::RemoteAPI;

#[cfg(feature = "local_llama")]
pub use local_llama::{LocalLlama, LlamaParams};
