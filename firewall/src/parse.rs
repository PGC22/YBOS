use serde::Deserialize;
use crate::types::{JudgeDecision, JudgeError};

/// Intermediate DTO to handle the mismatch between the JSON string "redacted_payload"
/// and the internal Vec<u8> field. We deserialize into this type first, then
/// convert to JudgeDecision.
#[derive(Debug, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
enum JudgeDecisionRaw {
    Allow,
    Redact {
        redacted_payload: String,
        reasons: Vec<String>,
    },
    Block {
        reason: String,
    },
    AskUser {
        prompt: String,
    },
}

impl From<JudgeDecisionRaw> for JudgeDecision {
    fn from(raw: JudgeDecisionRaw) -> Self {
        match raw {
            JudgeDecisionRaw::Allow => JudgeDecision::Allow,
            JudgeDecisionRaw::Redact {
                redacted_payload,
                reasons,
            } => JudgeDecision::Redact {
                redacted_payload: redacted_payload.into_bytes(),
                reasons,
            },
            JudgeDecisionRaw::Block { reason } => JudgeDecision::Block { reason },
            JudgeDecisionRaw::AskUser { prompt } => JudgeDecision::AskUser { prompt },
        }
    }
}

pub fn parse_judge_output(text: &str) -> Result<JudgeDecision, JudgeError> {
    let first_brace = text.find('{').ok_or_else(|| {
        JudgeError::Parse("No JSON object found in judge output".to_string())
    })?;

    let mut balance = 0;
    let mut last_brace = None;

    for (i, c) in text.char_indices() {
        if c == '{' {
            balance += 1;
        } else if c == '}' {
            balance -= 1;
            if balance < 0 {
                return Err(JudgeError::Parse("Unbalanced braces (stray '}')".to_string()));
            }
            if balance == 0 && first_brace <= i {
                last_brace = Some(i);
                break;
            }
        }
    }

    let last_brace = last_brace.ok_or_else(|| {
        JudgeError::Parse("Unbalanced braces in judge output".to_string())
    })?;

    let json_str = &text[first_brace..=last_brace];
    let raw: JudgeDecisionRaw = serde_json::from_str(json_str)
        .map_err(|e| JudgeError::Parse(format!("Failed to parse JSON: {}", e)))?;

    Ok(raw.into())
}
