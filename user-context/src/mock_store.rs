use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use crate::types::*;
use crate::trait_def::UserContextStore;

pub struct MockUserContextStore {
    entries: Arc<RwLock<HashMap<(ContextCategory, String), ContextEntry>>>,
}

impl MockUserContextStore {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl UserContextStore for MockUserContextStore {
    async fn put(&self, mut entry: ContextEntry) -> Result<(), UserContextError> {
        let mut entries = self.entries.write().expect("MockUserContextStore: lock poisoned");
        let key = (entry.category, entry.key.clone());

        if let Some(existing) = entries.get(&key) {
            entry.id = existing.id;
            entry.created_at = existing.created_at;
        }

        entries.insert(key, entry);
        Ok(())
    }

    async fn get(&self, category: ContextCategory, key: &str) -> Result<Option<ContextEntry>, UserContextError> {
        let entries = self.entries.read().expect("MockUserContextStore: lock poisoned");
        Ok(entries.get(&(category, key.to_string())).cloned())
    }

    async fn query(&self, q: ContextQuery) -> Result<Vec<ContextEntry>, UserContextError> {
        let entries = self.entries.read().expect("MockUserContextStore: lock poisoned");
        let mut filtered: Vec<ContextEntry> = entries.values()
            .filter(|e| {
                if let Some(cat) = q.category {
                    if e.category != cat { return false; }
                }
                if let Some(ref prefix) = q.key_prefix {
                    if !e.key.starts_with(prefix) { return false; }
                }
                true
            })
            .cloned()
            .collect();

        filtered.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        if q.limit > 0 {
            filtered.truncate(q.limit);
        }

        Ok(filtered)
    }

    async fn delete(&self, category: ContextCategory, key: &str) -> Result<bool, UserContextError> {
        let mut entries = self.entries.write().expect("MockUserContextStore: lock poisoned");
        Ok(entries.remove(&(category, key.to_string())).is_some())
    }

    async fn count(&self) -> Result<usize, UserContextError> {
        let entries = self.entries.read().expect("MockUserContextStore: lock poisoned");
        Ok(entries.len())
    }
}
