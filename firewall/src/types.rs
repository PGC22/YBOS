use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JudgePolicy {
    /// Block on any sensitive-content suspicion; redact on doubt.
    Strict,
    /// Redact obvious PII (emails, phones, IDs); allow generic
    /// business prompts; ask user on ambiguous cases.
    Standard,
    /// Allow most content; redact only secrets/keys; never block.
    Permissive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Json,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeContext {
    pub agent_name: String,
    pub destination: String,       // e.g. "<api.openai.com>" or
                                   // "<api.anthropic.com>"
    pub purpose: Option<String>,   // free-text rationale supplied by
                                   // the caller, surfaced in prompts
    pub content_type: ContentType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum JudgeDecision {
    Allow,
    Redact {
        redacted_payload: Vec<u8>,
        reasons: Vec<String>,
    },
    Block {
        reason: String,
    },
    AskUser {
        prompt: String,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum JudgeError {
    #[error("Inference error: {0}")]
    Inference(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Policy violation: {0}")]
    Policy(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}
