use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Coarse classification used to pick which user-context reminder rule
/// applies. Kept as a small enum (bounded audit log surface, same
/// rationale as ContextCategory in user-context).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    Internal,   // colleagues, internal team
    External,   // customers, vendors, externals
    Personal,   // family, friends, errands
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start_ts: u64,            // unix seconds
    pub end_ts: u64,              // unix seconds
    pub location: Option<String>,
    pub event_type: EventType,
    pub attendees: Vec<String>,   // free-text strings (names, emails)
    pub created_at: u64,
    pub updated_at: u64,
}

/// Time-range filter. Both bounds optional; None means open-ended.
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    pub from_ts: Option<u64>,
    pub to_ts: Option<u64>,
    pub event_type: Option<EventType>,
    pub limit: usize,             // 0 = unlimited, same convention as Y9
}

#[derive(thiserror::Error, Debug)]
pub enum CalendarError {
    #[error("Storage error: {0}")] Storage(String),
    #[error("Not found")] NotFound,
    #[error("Invalid input: {0}")] Invalid(String),
    #[error("Unknown error: {0}")] Unknown(String),
}
