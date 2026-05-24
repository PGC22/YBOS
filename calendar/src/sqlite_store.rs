use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use crate::types::{Event, EventType, EventQuery, CalendarError};
use crate::trait_def::CalendarStore;

pub struct SqliteCalendarStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteCalendarStore {
    pub fn open(path: PathBuf) -> Result<Self, CalendarError> {
        let conn = Connection::open(path).map_err(|e| CalendarError::Storage(e.to_string()))?;
        Self::init(conn)
    }

    pub fn in_memory() -> Result<Self, CalendarError> {
        let conn = Connection::open_in_memory().map_err(|e| CalendarError::Storage(e.to_string()))?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self, CalendarError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS calendar_events (
                id BLOB PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                start_ts INTEGER NOT NULL,
                end_ts INTEGER NOT NULL,
                location TEXT,
                event_type INTEGER NOT NULL,
                attendees TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        ).map_err(|e| CalendarError::Storage(e.to_string()))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_calendar_start ON calendar_events(start_ts)",
            [],
        ).map_err(|e| CalendarError::Storage(e.to_string()))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_calendar_type ON calendar_events(event_type)",
            [],
        ).map_err(|e| CalendarError::Storage(e.to_string()))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl CalendarStore for SqliteCalendarStore {
    async fn put(&self, event: Event) -> Result<(), CalendarError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("SqliteCalendarStore: connection lock poisoned");
            let attendees_json = serde_json::to_string(&event.attendees)
                .map_err(|e| CalendarError::Invalid(e.to_string()))?;

            let event_type = event.event_type as i32;

            conn.execute(
                "INSERT INTO calendar_events (
                    id, title, description, start_ts, end_ts, location, event_type, attendees, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    description = excluded.description,
                    start_ts = excluded.start_ts,
                    end_ts = excluded.end_ts,
                    location = excluded.location,
                    event_type = excluded.event_type,
                    attendees = excluded.attendees,
                    updated_at = excluded.updated_at",
                params![
                    event.id.as_bytes(),
                    event.title,
                    event.description,
                    event.start_ts,
                    event.end_ts,
                    event.location,
                    event_type,
                    attendees_json,
                    event.created_at,
                    event.updated_at,
                ],
            ).map_err(|e| CalendarError::Storage(e.to_string()))?;
            Ok(())
        }).await.map_err(|e| CalendarError::Unknown(e.to_string()))?
    }

    async fn get(&self, id: Uuid) -> Result<Option<Event>, CalendarError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("SqliteCalendarStore: connection lock poisoned");
            let mut stmt = conn.prepare(
                "SELECT id, title, description, start_ts, end_ts, location, event_type, attendees, created_at, updated_at
                 FROM calendar_events WHERE id = ?1"
            ).map_err(|e| CalendarError::Storage(e.to_string()))?;

            let row = stmt.query_row(params![id.as_bytes()], |row| {
                let id_bytes: Vec<u8> = row.get(0)?;
                let event_type_int: i32 = row.get(6)?;
                let attendees_str: String = row.get(7)?;

                Ok((id_bytes, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, u64>(3)?,
                    row.get::<_, u64>(4)?, row.get::<_, Option<String>>(5)?, event_type_int, attendees_str,
                    row.get::<_, u64>(8)?, row.get::<_, u64>(9)?))
            }).optional().map_err(|e| CalendarError::Storage(e.to_string()))?;

            if let Some((id_bytes, title, description, start_ts, end_ts, location, event_type_int, attendees_str, created_at, updated_at)) = row {
                let id = Uuid::from_slice(&id_bytes).map_err(|e| CalendarError::Storage(e.to_string()))?;
                let event_type = match event_type_int {
                    0 => EventType::Internal,
                    1 => EventType::External,
                    2 => EventType::Personal,
                    _ => EventType::Other,
                };
                let attendees: Vec<String> = serde_json::from_str(&attendees_str)
                    .map_err(|e| CalendarError::Storage(e.to_string()))?;

                Ok(Some(Event {
                    id, title, description, start_ts, end_ts, location, event_type, attendees, created_at, updated_at
                }))
            } else {
                Ok(None)
            }
        }).await.map_err(|e| CalendarError::Unknown(e.to_string()))?
    }

    async fn list(&self, q: EventQuery) -> Result<Vec<Event>, CalendarError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("SqliteCalendarStore: connection lock poisoned");
            let mut where_clauses = Vec::new();
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(from) = q.from_ts {
                where_clauses.push("start_ts >= ?");
                params_vec.push(Box::new(from));
            }
            if let Some(to) = q.to_ts {
                where_clauses.push("start_ts <= ?");
                params_vec.push(Box::new(to));
            }
            if let Some(et) = q.event_type {
                where_clauses.push("event_type = ?");
                params_vec.push(Box::new(et as i32));
            }

            let where_str = if where_clauses.is_empty() {
                "".to_string()
            } else {
                format!("WHERE {}", where_clauses.join(" AND "))
            };

            let limit_str = if q.limit > 0 {
                format!("LIMIT {}", q.limit)
            } else {
                "".to_string()
            };

            let query_str = format!(
                "SELECT id, title, description, start_ts, end_ts, location, event_type, attendees, created_at, updated_at
                 FROM calendar_events {} ORDER BY start_ts ASC {}",
                where_str, limit_str
            );

            let mut stmt = conn.prepare(&query_str).map_err(|e| CalendarError::Storage(e.to_string()))?;

            let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

            let event_iter = stmt.query_map(rusqlite::params_from_iter(params_refs), |row| {
                let id_bytes: Vec<u8> = row.get(0)?;
                let event_type_int: i32 = row.get(6)?;
                let attendees_str: String = row.get(7)?;

                let id = Uuid::from_slice(&id_bytes).map_err(|_| rusqlite::Error::InvalidQuery)?;
                let event_type = match event_type_int {
                    0 => EventType::Internal,
                    1 => EventType::External,
                    2 => EventType::Personal,
                    _ => EventType::Other,
                };
                let attendees: Vec<String> = serde_json::from_str(&attendees_str).map_err(|_| rusqlite::Error::InvalidQuery)?;

                Ok(Event {
                    id,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    start_ts: row.get(3)?,
                    end_ts: row.get(4)?,
                    location: row.get(5)?,
                    event_type,
                    attendees,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            }).map_err(|e| CalendarError::Storage(e.to_string()))?;

            let mut events = Vec::new();
            for event in event_iter {
                events.push(event.map_err(|e| CalendarError::Storage(e.to_string()))?);
            }
            Ok(events)
        }).await.map_err(|e| CalendarError::Unknown(e.to_string()))?
    }

    async fn delete(&self, id: Uuid) -> Result<bool, CalendarError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("SqliteCalendarStore: connection lock poisoned");
            let rows_affected = conn.execute(
                "DELETE FROM calendar_events WHERE id = ?1",
                params![id.as_bytes()],
            ).map_err(|e| CalendarError::Storage(e.to_string()))?;
            Ok(rows_affected > 0)
        }).await.map_err(|e| CalendarError::Unknown(e.to_string()))?
    }

    async fn count(&self) -> Result<usize, CalendarError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("SqliteCalendarStore: connection lock poisoned");
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM calendar_events",
                [],
                |row| row.get(0),
            ).map_err(|e| CalendarError::Storage(e.to_string()))?;
            Ok(count as usize)
        }).await.map_err(|e| CalendarError::Unknown(e.to_string()))?
    }
}
