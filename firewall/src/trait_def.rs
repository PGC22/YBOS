use async_trait::async_trait;
use crate::types::{JudgeContext, JudgeDecision, JudgeError};

#[async_trait]
pub trait Judge: Send + Sync {
    async fn evaluate(
        &self,
        payload: &[u8],
        context: JudgeContext,
    ) -> Result<JudgeDecision, JudgeError>;
}
