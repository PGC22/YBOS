use std::pin::Pin;
use async_trait::async_trait;
use futures::Stream;
use secrecy::SecretString;
use crate::trait_def::Inference;
use crate::types::{CompleteRequest, CompleteResponse, Token, ModelInfo, InferenceError};

pub struct RemoteAPI {
    pub endpoint: String,
    pub api_key: SecretString,
}

impl RemoteAPI {
    pub fn new(endpoint: String, api_key: SecretString) -> Self {
        Self { endpoint, api_key }
    }
}

impl std::fmt::Debug for RemoteAPI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteAPI")
            .field("endpoint", &self.endpoint)
            .field("api_key", &"***REDACTED***")
            .finish()
    }
}

#[async_trait]
impl Inference for RemoteAPI {
    async fn complete(&self, _req: CompleteRequest) -> Result<CompleteResponse, InferenceError> {
        Err(InferenceError::NotImplemented)
    }

    async fn complete_stream(
        &self,
        _req: CompleteRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Token, InferenceError>> + Send>>, InferenceError> {
        Err(InferenceError::NotImplemented)
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            backend: "remote-api".into(),
            model_name: "".into(),
            context_window: 0,
        }
    }
}
