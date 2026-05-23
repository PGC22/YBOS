use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;
use crate::types::*;
use crate::trait_def::UserContextStore;

pub struct SqliteUserContextStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteUserContextStore {
    pub fn open(path: PathBuf) -> Result<Self, UserContextError> {
        let conn = Connection::open(path).map_err(|e| UserContextError::Storage(e.to_string()))?;
        Self::init(conn)
    }

    pub fn in_memory() -> Result<Self, UserContextError> {
        let conn = Connection::open_in_memory().map_err(|e| UserContextError::Storage(e.to_string()))?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self, UserContextError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_context (
              id BLOB PRIMARY KEY,             -- UUID v4 16 bytes
              category INTEGER NOT NULL,       -- enum discriminant
              key TEXT NOT NULL,
              value TEXT NOT NULL,             -- serde_json::Value as text
              note TEXT,
              confidence REAL NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              UNIQUE (category, key)
            );",
            [],
        ).map_err(|e| UserContextError::Storage(e.to_string()))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_context_category ON user_context(category);",
            [],
        ).map_err(|e| UserContextError::Storage(e.to_string()))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_context_updated ON user_context(updated_at DESC);",
            [],
        ).map_err(|e| UserContextError::Storage(e.to_string()))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl UserContextStore for SqliteUserContextStore {
    async fn put(&self, entry: ContextEntry) -> Result<(), UserContextError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("SqliteUserContextStore: connection lock poisoned");

            let category_val = match entry.category {
                ContextCategory::Preference => 0,
                ContextCategory::Recurrence => 1,
                ContextCategory::Alias => 2,
                ContextCategory::Pattern => 3,
                ContextCategory::Observation => 4,
            };

            let value_str = serde_json::to_string(&entry.value).map_err(|e| UserContextError::Invalid(e.to_string()))?;

            // Upsert logic: preserve created_at, update others
            conn.execute(
                "INSERT INTO user_context (id, category, key, value, note, confidence, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(category, key) DO UPDATE SET
                    value = excluded.value,
                    note = excluded.note,
                    confidence = excluded.confidence,
                    updated_at = excluded.updated_at",
                params![
                    entry.id.as_bytes(),
                    category_val,
                    entry.key,
                    value_str,
                    entry.note,
                    entry.confidence,
                    entry.created_at,
                    entry.updated_at,
                ],
            ).map_err(|e| UserContextError::Storage(e.to_string()))?;

            Ok(())
        }).await.map_err(|e| UserContextError::Unknown(e.to_string()))?
    }

    async fn get(&self, category: ContextCategory, key: &str) -> Result<Option<ContextEntry>, UserContextError> {
        let conn = self.conn.clone();
        let key = key.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("SqliteUserContextStore: connection lock poisoned");

            let category_val = match category {
                ContextCategory::Preference => 0,
                ContextCategory::Recurrence => 1,
                ContextCategory::Alias => 2,
                ContextCategory::Pattern => 3,
                ContextCategory::Observation => 4,
            };

            let mut stmt = conn.prepare(
                "SELECT id, category, key, value, note, confidence, created_at, updated_at
                 FROM user_context WHERE category = ?1 AND key = ?2"
            ).map_err(|e| UserContextError::Storage(e.to_string()))?;

            let entry = stmt.query_row(params![category_val, key], |row| {
                let id_bytes: Vec<u8> = row.get(0)?;
                let id = Uuid::from_slice(&id_bytes).map_err(|_| rusqlite::Error::InvalidQuery)?;
                let cat_int: i32 = row.get(1)?;
                let category = match cat_int {
                    0 => ContextCategory::Preference,
                    1 => ContextCategory::Recurrence,
                    2 => ContextCategory::Alias,
                    3 => ContextCategory::Pattern,
                    4 => ContextCategory::Observation,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                let key: String = row.get(2)?;
                let value_str: String = row.get(3)?;
                let value: serde_json::Value = serde_json::from_str(&value_str).map_err(|_| rusqlite::Error::InvalidQuery)?;
                let note: Option<String> = row.get(4)?;
                let confidence: f32 = row.get(5)?;
                let created_at: u64 = row.get(6)?;
                let updated_at: u64 = row.get(7)?;

                Ok(ContextEntry {
                    id, category, key, value, note, confidence, created_at, updated_at
                })
            }).optional().map_err(|e| UserContextError::Storage(e.to_string()))?;

            Ok(entry)
        }).await.map_err(|e| UserContextError::Unknown(e.to_string()))?
    }

    async fn query(&self, q: ContextQuery) -> Result<Vec<ContextEntry>, UserContextError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("SqliteUserContextStore: connection lock poisoned");

            let mut sql = "SELECT id, category, key, value, note, confidence, created_at, updated_at FROM user_context".to_string();
            let mut conditions = Vec::new();
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql + Send>> = Vec::new();

            if let Some(cat) = q.category {
                conditions.push("category = ?");
                let cat_val = match cat {
                    ContextCategory::Preference => 0,
                    ContextCategory::Recurrence => 1,
                    ContextCategory::Alias => 2,
                    ContextCategory::Pattern => 3,
                    ContextCategory::Observation => 4,
                };
                params_vec.push(Box::new(cat_val));
            }

            if let Some(prefix) = q.key_prefix {
                conditions.push("key LIKE ?");
                params_vec.push(Box::new(format!("{}%", prefix)));
            }

            if !conditions.is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(&conditions.join(" AND "));
            }

            sql.push_str(" ORDER BY updated_at DESC");

            if q.limit > 0 {
                sql.push_str(&format!(" LIMIT {}", q.limit));
            }

            let mut stmt = conn.prepare(&sql).map_err(|e| UserContextError::Storage(e.to_string()))?;

            let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref() as &dyn rusqlite::ToSql).collect();
            let rows = stmt.query_map(rusqlite::params_from_iter(param_refs), |row| {
                let id_bytes: Vec<u8> = row.get(0)?;
                let id = Uuid::from_slice(&id_bytes).map_err(|_| rusqlite::Error::InvalidQuery)?;
                let cat_int: i32 = row.get(1)?;
                let category = match cat_int {
                    0 => ContextCategory::Preference,
                    1 => ContextCategory::Recurrence,
                    2 => ContextCategory::Alias,
                    3 => ContextCategory::Pattern,
                    4 => ContextCategory::Observation,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                let key: String = row.get(2)?;
                let value_str: String = row.get(3)?;
                let value: serde_json::Value = serde_json::from_str(&value_str).map_err(|_| rusqlite::Error::InvalidQuery)?;
                let note: Option<String> = row.get(4)?;
                let confidence: f32 = row.get(5)?;
                let created_at: u64 = row.get(6)?;
                let updated_at: u64 = row.get(7)?;

                Ok(ContextEntry {
                    id, category, key, value, note, confidence, created_at, updated_at
                })
            }).map_err(|e| UserContextError::Storage(e.to_string()))?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row.map_err(|e| UserContextError::Storage(e.to_string()))?);
            }
            Ok(results)
        }).await.map_err(|e| UserContextError::Unknown(e.to_string()))?
    }

    async fn delete(&self, category: ContextCategory, key: &str) -> Result<bool, UserContextError> {
        let conn = self.conn.clone();
        let key = key.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("SqliteUserContextStore: connection lock poisoned");
            let category_val = match category {
                ContextCategory::Preference => 0,
                ContextCategory::Recurrence => 1,
                ContextCategory::Alias => 2,
                ContextCategory::Pattern => 3,
                ContextCategory::Observation => 4,
            };

            let rows = conn.execute(
                "DELETE FROM user_context WHERE category = ?1 AND key = ?2",
                params![category_val, key],
            ).map_err(|e| UserContextError::Storage(e.to_string()))?;

            Ok(rows > 0)
        }).await.map_err(|e| UserContextError::Unknown(e.to_string()))?
    }

    async fn count(&self) -> Result<usize, UserContextError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("SqliteUserContextStore: connection lock poisoned");
            let count: usize = conn.query_row("SELECT count(*) FROM user_context", [], |row| row.get(0))
                .map_err(|e| UserContextError::Storage(e.to_string()))?;
            Ok(count)
        }).await.map_err(|e| UserContextError::Unknown(e.to_string()))?
    }
}
