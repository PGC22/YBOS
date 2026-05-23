use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;
use crate::types::{Event, EventQuery, CalendarError};
use crate::trait_def::CalendarStore;

pub struct MockCalendarStore {
    events: RwLock<HashMap<Uuid, Event>>,
}

impl MockCalendarStore {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl CalendarStore for MockCalendarStore {
    async fn put(&self, mut event: Event) -> Result<(), CalendarError> {
        let mut events = self.events.write().expect("MockCalendarStore: lock poisoned");
        if let Some(existing) = events.get(&event.id) {
            event.created_at = existing.created_at;
        }
        events.insert(event.id, event);
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<Event>, CalendarError> {
        let events = self.events.read().expect("MockCalendarStore: lock poisoned");
        Ok(events.get(&id).cloned())
    }

    async fn list(&self, q: EventQuery) -> Result<Vec<Event>, CalendarError> {
        let events = self.events.read().expect("MockCalendarStore: lock poisoned");
        let mut result: Vec<Event> = events.values()
            .filter(|e| {
                if let Some(from) = q.from_ts {
                    if e.start_ts < from { return false; }
                }
                if let Some(to) = q.to_ts {
                    if e.start_ts > to { return false; }
                }
                if let Some(et) = q.event_type {
                    if e.event_type != et { return false; }
                }
                true
            })
            .cloned()
            .collect();

        result.sort_by_key(|e| e.start_ts);

        if q.limit > 0 {
            result.truncate(q.limit);
        }

        Ok(result)
    }

    async fn delete(&self, id: Uuid) -> Result<bool, CalendarError> {
        let mut events = self.events.write().expect("MockCalendarStore: lock poisoned");
        Ok(events.remove(&id).is_some())
    }

    async fn count(&self) -> Result<usize, CalendarError> {
        let events = self.events.read().expect("MockCalendarStore: lock poisoned");
        Ok(events.len())
    }
}
