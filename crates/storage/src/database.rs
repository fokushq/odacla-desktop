//! Database connection and initialization.

use rusqlite::Connection;
use std::path::Path;
use tracing::info;

use crate::error::StorageError;
use crate::schema;

/// The main database handle for Fokus.
pub struct Database {
    pub conn: Connection,
}

impl Database {
    /// Open or create the database at the given path.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        info!("Opening database at: {}", path.display());

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                StorageError::Database(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(1),
                    Some(format!("Failed to create directory: {}", e)),
                ))
            })?;
        }

        let conn = Connection::open(path)?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA synchronous=NORMAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        conn.execute_batch("PRAGMA cache_size=2000;")?;

        let db = Self { conn };
        db.initialize_schema()?;

        info!("Database initialized successfully");
        Ok(db)
    }

    /// Create an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Run schema creation and seed default data.
    fn initialize_schema(&self) -> Result<(), StorageError> {
        // Create all tables (IF NOT EXISTS makes this idempotent)
        self.conn.execute_batch(schema::SCHEMA_SQL)?;

        // Seed default rules if this is a fresh database
        // We check if any rules exist first to avoid overwriting user changes
        let rule_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM rules",
            [],
            |row| row.get(0),
        )?;

        if rule_count == 0 {
            info!("Fresh database — seeding default classification rules");
            self.conn.execute_batch(schema::DEFAULT_RULES_SQL)?;
        } else {
            info!("Database already has {} rules, skipping seed", rule_count);
        }

        // Versioned rule additions — bring existing databases up to date
        // without resurrecting rules the user deleted from older batches.
        let seed_version: i64 = self
            .conn
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'rules_seed_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(1);

        if seed_version < 2 {
            self.conn.execute_batch(schema::RULES_V2_SQL)?;
            self.conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES ('rules_seed_version', '2')",
                [],
            )?;
            info!("Applied classification rules migration v2");
        }

        // Migration: sessions.source column (manual time entries). CREATE
        // TABLE IF NOT EXISTS won't touch existing databases, so add the
        // column when it's missing.
        let has_source: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'source'",
            [],
            |row| row.get(0),
        )?;
        if has_source == 0 {
            self.conn.execute_batch(
                "ALTER TABLE sessions ADD COLUMN source TEXT NOT NULL DEFAULT 'auto';",
            )?;
            info!("Added sessions.source column (manual entry support)");
        }

        // Migration: drop tables from earlier schema versions that were
        // never written to by any code path (safe — always empty).
        self.conn.execute_batch(
            "DROP TABLE IF EXISTS raw_events; DROP TABLE IF EXISTS categories;",
        )?;

        // Migration: PascalCase → snake_case categories
        self.conn.execute_batch(r#"
            UPDATE rules SET category = REPLACE(category, '"Coding"', '"coding"')       WHERE category LIKE '%"Coding"%';
            UPDATE rules SET category = REPLACE(category, '"Study"', '"study"')         WHERE category LIKE '%"Study"%';
            UPDATE rules SET category = REPLACE(category, '"NoteTaking"', '"note_taking"') WHERE category LIKE '%"NoteTaking"%';
            UPDATE rules SET category = REPLACE(category, '"Productive"', '"productive"') WHERE category LIKE '%"Productive"%';
            UPDATE rules SET category = REPLACE(category, '"Entertainment"', '"entertainment"') WHERE category LIKE '%"Entertainment"%';
            UPDATE rules SET category = REPLACE(category, '"Communication"', '"communication"') WHERE category LIKE '%"Communication"%';
            UPDATE rules SET category = REPLACE(category, '"Idle"', '"idle"')           WHERE category LIKE '%"Idle"%';
            UPDATE rules SET category = REPLACE(category, '"Uncategorized"', '"uncategorized"') WHERE category LIKE '%"Uncategorized"%';

            UPDATE sessions SET category = REPLACE(category, '"Coding"', '"coding"')       WHERE category LIKE '%"Coding"%';
            UPDATE sessions SET category = REPLACE(category, '"Study"', '"study"')         WHERE category LIKE '%"Study"%';
            UPDATE sessions SET category = REPLACE(category, '"NoteTaking"', '"note_taking"') WHERE category LIKE '%"NoteTaking"%';
            UPDATE sessions SET category = REPLACE(category, '"Productive"', '"productive"') WHERE category LIKE '%"Productive"%';
            UPDATE sessions SET category = REPLACE(category, '"Entertainment"', '"entertainment"') WHERE category LIKE '%"Entertainment"%';
            UPDATE sessions SET category = REPLACE(category, '"Communication"', '"communication"') WHERE category LIKE '%"Communication"%';
            UPDATE sessions SET category = REPLACE(category, '"Idle"', '"idle"')           WHERE category LIKE '%"Idle"%';
            UPDATE sessions SET category = REPLACE(category, '"Uncategorized"', '"uncategorized"') WHERE category LIKE '%"Uncategorized"%';

            UPDATE daily_rollups SET category = REPLACE(category, '"Coding"', '"coding"')       WHERE category LIKE '%"Coding"%';
            UPDATE daily_rollups SET category = REPLACE(category, '"Study"', '"study"')         WHERE category LIKE '%"Study"%';
            UPDATE daily_rollups SET category = REPLACE(category, '"NoteTaking"', '"note_taking"') WHERE category LIKE '%"NoteTaking"%';
            UPDATE daily_rollups SET category = REPLACE(category, '"Productive"', '"productive"') WHERE category LIKE '%"Productive"%';
            UPDATE daily_rollups SET category = REPLACE(category, '"Entertainment"', '"entertainment"') WHERE category LIKE '%"Entertainment"%';
            UPDATE daily_rollups SET category = REPLACE(category, '"Communication"', '"communication"') WHERE category LIKE '%"Communication"%';
            UPDATE daily_rollups SET category = REPLACE(category, '"Idle"', '"idle"')           WHERE category LIKE '%"Idle"%';
            UPDATE daily_rollups SET category = REPLACE(category, '"Uncategorized"', '"uncategorized"') WHERE category LIKE '%"Uncategorized"%';
        "#)?;

        // Migration: strip self-referential entries (both the Fokus era and
        // Odacla) from the stored app lists. Self-exclusion is hardcoded in
        // the collector, so these entries are redundant and only confuse the
        // Settings UI with stale branding.
        let legacy_self = ["fokus-desktop", "fokus", "odacla", "odacla-desktop"];
        let stored: Result<String, _> = self.conn.query_row(
            "SELECT value FROM settings WHERE key = 'app_settings'",
            [],
            |row| row.get(0),
        );
        if let Ok(json) = stored {
            if let Ok(mut s) = serde_json::from_str::<odacla_domain::Settings>(&json) {
                let before = s.excluded_apps.len() + s.included_apps.len();
                s.excluded_apps
                    .retain(|a| !legacy_self.contains(&a.to_lowercase().as_str()));
                s.included_apps
                    .retain(|a| !legacy_self.contains(&a.to_lowercase().as_str()));
                if s.excluded_apps.len() + s.included_apps.len() != before {
                    let new_json =
                        serde_json::to_string(&s).map_err(StorageError::Serialization)?;
                    self.conn.execute(
                        "UPDATE settings SET value = ?1 WHERE key = 'app_settings'",
                        [&new_json],
                    )?;
                    info!("Cleaned legacy self-referential entries from settings");
                }
            }
        }

        // Seed default settings if none exist
        let settings_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM settings WHERE key = 'app_settings'",
            [],
            |row| row.get(0),
        )?;

        if settings_count == 0 {
            let default_settings = odacla_domain::Settings::default();
            let json = serde_json::to_string(&default_settings)
                .map_err(StorageError::Serialization)?;
            self.conn.execute(
                "INSERT INTO settings (key, value) VALUES ('app_settings', ?1)",
                [&json],
            )?;
            info!("Seeded default application settings");
        }

        Ok(())
    }
}
