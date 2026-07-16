//! Settings queries.

use odacla_domain::Settings;

use crate::database::Database;
use crate::error::StorageError;

impl Database {
    /// Load the application settings.
    /// Returns defaults if no settings are stored yet.
    pub fn get_settings(&self) -> Result<Settings, StorageError> {
        let result: Result<String, _> = self.conn.query_row(
            "SELECT value FROM settings WHERE key = 'app_settings'",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(json) => Ok(serde_json::from_str(&json)?),
            Err(_) => Ok(Settings::default()),
        }
    }

    /// Save the application settings (replaces the existing value).
    pub fn save_settings(&self, settings: &Settings) -> Result<(), StorageError> {
        let json = serde_json::to_string(settings)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('app_settings', ?1)",
            [&json],
        )?;
        Ok(())
    }
}
