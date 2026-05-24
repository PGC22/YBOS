use std::sync::Arc;
use async_trait::async_trait;
use tracing::warn;
use ybos_inference::{Inference, CompleteRequest};

use crate::audit::log_audit_decision;
use crate::trait_def::Judge;
use crate::types::{JudgeContext, JudgeDecision, JudgeError, JudgePolicy};
use crate::prompt::build_prompt;
use crate::parse::parse_judge_output;

pub struct LlmJudge {
    inference: Arc<dyn Inference>,
    policy: JudgePolicy,
}

impl LlmJudge {
    pub fn new(inference: Arc<dyn Inference>, policy: JudgePolicy) -> Self {
        Self { inference, policy }
    }
}

#[async_trait]
impl Judge for LlmJudge {
    async fn evaluate(
        &self,
        payload: &[u8],
        context: JudgeContext,
    ) -> Result<JudgeDecision, JudgeError> {
        let prompt = build_prompt(payload, &context, self.policy);

        let res = self
            .inference
            .complete(CompleteRequest {
                prompt,
                max_tokens: 256,
                temperature: 0.0,
                top_p: 1.0,
                stop: vec![],
                seed: None,
            })
            .await
            .map_err(|e| JudgeError::Inference(e.to_string()))?;

        let decision = match parse_judge_output(&res.text) {
            Ok(d) => d,
            Err(e) => {
                warn!(
                    target: "ybos.audit.judge",
                    error = %e,
                    "Judge LLM output unparseable; defaulting to block under fail-closed policy"
                );
                JudgeDecision::Block {
                    reason: "Judge LLM output unparseable; defaulting to block under fail-closed policy"
                        .to_string(),
                }
            }
        };

        let policy_str = match self.policy {
            JudgePolicy::Strict => "Strict",
            JudgePolicy::Standard => "Standard",
            JudgePolicy::Permissive => "Permissive",
        };

        log_audit_decision(
            &decision,
            &context.agent_name,
            &context.destination,
            policy_str,
            payload.len(),
        );

        Ok(decision)
    }
}
