use tracing::{info, warn};
use crate::types::JudgeDecision;

pub(crate) fn log_audit_decision(
    decision: &JudgeDecision,
    agent: &str,
    destination: &str,
    policy: &str,
    payload_bytes: usize,
) {
    match decision {
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
        JudgeDecision::Redact { reasons, .. } => {
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
        JudgeDecision::AskUser { .. } => {
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
}
