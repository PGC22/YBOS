use crate::types::{Event, EventQuery, CalendarError};

#[async_trait::async_trait]
pub trait CalendarStore: Send + Sync {
    /// Upsert by id. created_at preserved if entry already exists,
    /// updated_at is bumped (mirror Y9 UserContextStore semantics).
    async fn put(&self, event: Event) -> Result<(), CalendarError>;

    async fn get(&self, id: uuid::Uuid)
        -> Result<Option<Event>, CalendarError>;

    /// Returns events ordered by start_ts ASC (chronological).
    async fn list(&self, q: EventQuery)
        -> Result<Vec<Event>, CalendarError>;

    async fn delete(&self, id: uuid::Uuid) -> Result<bool, CalendarError>;

    async fn count(&self) -> Result<usize, CalendarError>;
}
