use async_trait::async_trait;
use tracing::{info, warn};
use crate::trait_def::Judge;
use crate::types::{JudgeContext, JudgeDecision, JudgeError};

pub struct MockJudge {
    rules: Vec<(String, JudgeDecision)>,
}

impl MockJudge {
    pub fn new(rules: Vec<(String, JudgeDecision)>) -> Self {
        Self { rules }
    }
}

#[async_trait]
impl Judge for MockJudge {
    async fn evaluate(
        &self,
        payload: &[u8],
        context: JudgeContext,
    ) -> Result<JudgeDecision, JudgeError> {
        let payload_str = String::from_utf8_lossy(payload);

        let mut decision = JudgeDecision::Allow;
        for (pattern, rule_decision) in &self.rules {
            if payload_str.contains(pattern) {
                decision = rule_decision.clone();
                break;
            }
        }

        let agent = &context.agent_name;
        let destination = &context.destination;
        let policy = "mock";
        let payload_bytes = payload.len();

        match &decision {
            JudgeDecision::Allow => {
                info!(
                    target: "ybos.audit.judge",
                    agent,
                    destination,
                    policy,
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
                    policy,
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
                    policy,
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
                    policy,
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
