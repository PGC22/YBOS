use std::sync::Arc;
use async_trait::async_trait;
use tracing::{info, warn};
use ybos_inference::{Inference, CompleteRequest};

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

        let agent = &context.agent_name;
        let destination = &context.destination;
        let policy_str = match self.policy {
            JudgePolicy::Strict => "Strict",
            JudgePolicy::Standard => "Standard",
            JudgePolicy::Permissive => "Permissive",
        };
        let payload_bytes = payload.len();

        match &decision {
            JudgeDecision::Allow => {
                info!(
                    target: "ybos.audit.judge",
                    agent,
                    destination,
                    policy = policy_str,
                    decision = "allow",
                    payload_bytes,
                    "Judge evaluation"
                );
            }
            JudgeDecision::Redact { redacted_payload: _, reasons } => {
                info!(
                    target: "ybos.audit.judge",
                    agent,
                    destination,
                    policy = policy_str,
                    decision = "redact",
                    payload_bytes,
                    redact_reasons_count = reasons.len(),
                    "Judge evaluation"
                );
            }
            JudgeDecision::AskUser { prompt: _ } => {
                info!(
                    target: "ybos.audit.judge",
                    agent,
                    destination,
                    policy = policy_str,
                    decision = "ask_user",
                    payload_bytes,
                    "Judge evaluation"
                );
            }
            JudgeDecision::Block { reason } => {
                warn!(
                    target: "ybos.audit.judge",
                    agent,
                    destination,
                    policy = policy_str,
                    decision = "block",
                    payload_bytes,
                    reason = %reason,
                    "Judge evaluation"
                );
            }
        }

        Ok(decision)
    }
}
