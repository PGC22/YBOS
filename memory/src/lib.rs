pub mod types;
pub mod trait_def;

#[cfg(feature = "mock_store")]
pub mod mock_store;
#[cfg(feature = "mock_embedder")]
pub mod mock_embedder;
#[cfg(feature = "sqlite_vec")]
pub mod sqlite_vec_store;
#[cfg(feature = "fastembed")]
pub mod fastembed_embedder;

pub use types::*;
pub use trait_def::*;

#[cfg(feature = "mock_store")]
pub use mock_store::MockVectorStore;
#[cfg(feature = "mock_embedder")]
pub use mock_embedder::MockEmbedder;
#[cfg(feature = "sqlite_vec")]
pub use sqlite_vec_store::SqliteVecStore;
#[cfg(feature = "fastembed")]
pub use fastembed_embedder::FastEmbedEmbedder;
