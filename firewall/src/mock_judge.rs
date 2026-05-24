use async_trait::async_trait;
use crate::audit::log_audit_decision;
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

        log_audit_decision(
            &decision,
            &context.agent_name,
            &context.destination,
            "mock",
            payload.len(),
        );

        Ok(decision)
    }
}
