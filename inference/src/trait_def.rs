use std::pin::Pin;
use async_trait::async_trait;
use futures::Stream;
use crate::types::{CompleteRequest, CompleteResponse, Token, ModelInfo, InferenceError};

#[async_trait]
pub trait Inference: Send + Sync {
    async fn complete(&self, req: CompleteRequest) -> Result<CompleteResponse, InferenceError>;
    async fn complete_stream(
        &self,
        req: CompleteRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Token, InferenceError>> + Send>>, InferenceError>;
    fn model_info(&self) -> ModelInfo;
}
