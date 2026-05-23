use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Top-level grouping of user-context entries. Kept as a small enum
/// rather than free strings so capability audit logs are bounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextCategory {
    Preference,     // "travel.airline = Lufthansa"
    Recurrence,     // "calendar.reminder.external_meeting = 60min"
    Alias,          // "mama -> Maria Popescu, +40..."
    Pattern,        // "friday -> often orders pizza"
    Observation,    // free-form agent-suggested note
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: Uuid,
    pub category: ContextCategory,
    pub key: String,                 // e.g. "travel.airline"
    pub value: serde_json::Value,    // structured value
    pub note: Option<String>,        // optional free-text rationale
    pub confidence: f32,             // 0.0..=1.0 (agent-observed = lower, user-confirmed = 1.0)
    pub created_at: u64,             // unix seconds
    pub updated_at: u64,             // unix seconds
}

#[derive(Debug, Clone, Default)]
pub struct ContextQuery {
    pub category: Option<ContextCategory>,   // None = all
    pub key_prefix: Option<String>,          // exact-prefix match
    pub text: Option<String>,                // optional fuzzy semantic search (requires embedder) - DEFERRED
    pub limit: usize,
}

#[derive(thiserror::Error, Debug)]
pub enum UserContextError {
    #[error("Storage error: {0}")] Storage(String),
    #[error("Not found")] NotFound,
    #[error("Invalid input: {0}")] Invalid(String),
    #[error("Unknown error: {0}")] Unknown(String),
}
