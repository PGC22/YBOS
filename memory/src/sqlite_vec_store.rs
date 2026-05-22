use std::path::Path;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};
use sqlite_vec::sqlite3_vec_init;
use uuid::Uuid;
use crate::types::{VectorId, VectorItem, VectorQuery, VectorMatch, MemoryError};
use crate::trait_def::VectorStore;

pub struct SqliteVecStore {
    conn: Arc<Mutex<Connection>>,
    dimension: usize,
}

impl SqliteVecStore {
    fn ensure_extension_registered() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            unsafe {
                // Register sqlite-vec extension to be loaded automatically for every new connection
                let _ = rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
            }
        });
    }

    pub fn open(path: &Path, dimension: usize) -> Result<Self, MemoryError> {
        Self::ensure_extension_registered();
        let conn = Connection::open(path).map_err(|e| MemoryError::Storage(e.to_string()))?;
        Self::init(conn, dimension)
    }

    pub fn in_memory(dimension: usize) -> Result<Self, MemoryError> {
        Self::ensure_extension_registered();
        let conn = Connection::open_in_memory().map_err(|e| MemoryError::Storage(e.to_string()))?;
        Self::init(conn, dimension)
    }

    fn init(conn: Connection, dimension: usize) -> Result<Self, MemoryError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                pk INTEGER PRIMARY KEY AUTOINCREMENT,
                id BLOB NOT NULL,
                text TEXT NOT NULL,
                metadata TEXT NOT NULL,
                embedding BLOB NOT NULL
            )",
            [],
        ).map_err(|e| MemoryError::Storage(e.to_string()))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_id ON memories(id)",
            [],
        ).map_err(|e| MemoryError::Storage(e.to_string()))?;

        conn.execute(
            &format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS memories_vec USING vec0(
                    embedding float[{}]
                )",
                dimension
            ),
            [],
        ).map_err(|e| MemoryError::Storage(e.to_string()))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            dimension,
        })
    }
}

#[async_trait]
impl VectorStore for SqliteVecStore {
    async fn insert(&self, item: VectorItem) -> Result<VectorId, MemoryError> {
        if item.embedding.len() != self.dimension {
            return Err(MemoryError::InvalidEmbedding(format!(
                "Expected dimension {}, got {}",
                self.dimension,
                item.embedding.len()
            )));
        }

        let id = VectorId::new_v4();
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = conn.lock().unwrap();
            let tx = conn.transaction().map_err(|e| MemoryError::Storage(e.to_string()))?;

            let id_bytes = id.as_bytes();
            let metadata_str = serde_json::to_string(&item.metadata).map_err(|e| MemoryError::Storage(e.to_string()))?;
            let embedding_bytes = bytemuck::cast_slice::<f32, u8>(&item.embedding);

            tx.execute(
                "INSERT INTO memories (id, text, metadata, embedding) VALUES (?1, ?2, ?3, ?4)",
                params![id_bytes, item.text, metadata_str, embedding_bytes],
            ).map_err(|e| MemoryError::Storage(e.to_string()))?;

            let pk = tx.last_insert_rowid();

            tx.execute(
                "INSERT INTO memories_vec (rowid, embedding) VALUES (?1, ?2)",
                params![pk, embedding_bytes],
            ).map_err(|e| MemoryError::Storage(e.to_string()))?;

            tx.commit().map_err(|e| MemoryError::Storage(e.to_string()))?;
            Ok(id)
        }).await.map_err(|e| MemoryError::Unknown(e.to_string()))?
    }

    async fn insert_batch(&self, items: Vec<VectorItem>) -> Result<Vec<VectorId>, MemoryError> {
        let mut ids = Vec::new();
        for item in items {
            ids.push(self.insert(item).await?);
        }
        Ok(ids)
    }

    async fn query_top_k(&self, query: VectorQuery, k: usize) -> Result<Vec<VectorMatch>, MemoryError> {
        if query.embedding.len() != self.dimension {
            return Err(MemoryError::InvalidEmbedding(format!(
                "Expected dimension {}, got {}",
                self.dimension,
                query.embedding.len()
            )));
        }

        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let embedding_bytes = bytemuck::cast_slice::<f32, u8>(&query.embedding);

            let mut stmt = conn.prepare(
                "SELECT
                    m.id,
                    m.text,
                    m.metadata,
                    v.distance
                FROM memories_vec v
                JOIN memories m ON m.pk = v.rowid
                WHERE v.embedding MATCH ?1
                  AND k = ?2
                ORDER BY v.distance"
            ).map_err(|e| MemoryError::Storage(e.to_string()))?;

            let rows = stmt.query_map(params![embedding_bytes, k], |row| {
                let id_bytes: Vec<u8> = row.get(0)?;
                let id = Uuid::from_slice(&id_bytes).unwrap();
                let text: String = row.get(1)?;
                let metadata_str: String = row.get(2)?;
                let metadata: serde_json::Value = serde_json::from_str(&metadata_str).unwrap();
                let distance: f32 = row.get(3)?;

                Ok(VectorMatch {
                    id,
                    text,
                    metadata,
                    score: 1.0 - distance,
                })
            }).map_err(|e| MemoryError::Storage(e.to_string()))?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row.map_err(|e| MemoryError::Storage(e.to_string()))?);
            }
            Ok(results)
        }).await.map_err(|e| MemoryError::Unknown(e.to_string()))?
    }

    async fn delete(&self, id: VectorId) -> Result<(), MemoryError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = conn.lock().unwrap();
            let tx = conn.transaction().map_err(|e| MemoryError::Storage(e.to_string()))?;
            let id_bytes = id.as_bytes();

            let pk: Option<i64> = tx.query_row(
                "SELECT pk FROM memories WHERE id = ?1",
                params![id_bytes],
                |row| row.get(0)
            ).optional().map_err(|e| MemoryError::Storage(e.to_string()))?;

            if let Some(pk) = pk {
                tx.execute("DELETE FROM memories WHERE pk = ?1", params![pk])
                    .map_err(|e| MemoryError::Storage(e.to_string()))?;
                tx.execute("DELETE FROM memories_vec WHERE rowid = ?1", params![pk])
                    .map_err(|e| MemoryError::Storage(e.to_string()))?;
                tx.commit().map_err(|e| MemoryError::Storage(e.to_string()))?;
                Ok(())
            } else {
                Err(MemoryError::NotFound)
            }
        }).await.map_err(|e| MemoryError::Unknown(e.to_string()))?
    }

    async fn count(&self) -> Result<usize, MemoryError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let count: usize = conn.query_row("SELECT count(*) FROM memories", [], |row| row.get(0))
                .map_err(|e| MemoryError::Storage(e.to_string()))?;
            Ok(count)
        }).await.map_err(|e| MemoryError::Unknown(e.to_string()))?
    }
}
