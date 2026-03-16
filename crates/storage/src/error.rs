//! Storage error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Data not found: {0}")]
    NotFound(String),

    #[error("Migration error: {0}")]
    Migration(String),
}
