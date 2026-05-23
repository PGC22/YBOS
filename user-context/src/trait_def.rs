use crate::types::*;

#[async_trait::async_trait]
pub trait UserContextStore: Send + Sync {
    /// Put is an upsert keyed by (category, key).
    /// Preserves created_at and bumps updated_at if entry already exists.
    async fn put(&self, entry: ContextEntry) -> Result<(), UserContextError>;

    async fn get(&self, category: ContextCategory, key: &str)
        -> Result<Option<ContextEntry>, UserContextError>;

    /// Query ordering: by updated_at DESC.
    async fn query(&self, q: ContextQuery)
        -> Result<Vec<ContextEntry>, UserContextError>;

    async fn delete(&self, category: ContextCategory, key: &str)
        -> Result<bool, UserContextError>;

    async fn count(&self) -> Result<usize, UserContextError>;
}
