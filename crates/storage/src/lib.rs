//! SQLite persistence layer.

pub mod database;
pub mod error;
pub mod schema;
pub mod queries;

pub use database::Database;
pub use error::StorageError;
