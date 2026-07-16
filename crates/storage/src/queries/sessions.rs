//! Session queries.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use odacla_domain::{Category, Session};

use crate::database::Database;
use crate::error::StorageError;

impl Database {
    /// Insert a new session into the database.
    ///
    /// Called when the collector creates a new session (i.e., the user
    /// switched to a different app or came back from idle).
    pub fn insert_session(&self, session: &Session) -> Result<(), StorageError> {
        let category_json = serde_json::to_string(&session.category)?;

        self.conn.execute(
            r#"INSERT INTO sessions
                (id, start_time, end_time, app_name, window_title,
                 category, url, activity_count, idle_seconds_total)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            rusqlite::params![
                session.id.to_string(),
                session.start_time.to_rfc3339(),
                session.end_time.map(|t| t.to_rfc3339()),
                session.app_name,
                session.window_title,
                category_json,
                session.url,
                session.activity_count,
                session.idle_seconds_total,
            ],
        )?;
        Ok(())
    }

    /// Update an existing session (extend its end_time, update counts).
    ///
    /// Called periodically as the collector extends the current active session.
    pub fn update_session(&self, session: &Session) -> Result<(), StorageError> {
        let category_json = serde_json::to_string(&session.category)?;

        self.conn.execute(
            r#"UPDATE sessions SET
                end_time = ?2,
                window_title = ?3,
                category = ?4,
                activity_count = ?5,
                idle_seconds_total = ?6
               WHERE id = ?1"#,
            rusqlite::params![
                session.id.to_string(),
                session.end_time.map(|t| t.to_rfc3339()),
                session.window_title,
                category_json,
                session.activity_count,
                session.idle_seconds_total,
            ],
        )?;
        Ok(())
    }

    /// Get sessions within an absolute UTC datetime range.
    /// Callers derive day boundaries in the user's local timezone.
    pub fn get_sessions_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Session>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, start_time, end_time, app_name, window_title,
                      category, url, activity_count, idle_seconds_total
               FROM sessions
               WHERE start_time >= ?1 AND start_time < ?2
               ORDER BY start_time ASC"#,
        )?;

        let sessions = stmt
            .query_map(
                rusqlite::params![start.to_rfc3339(), end.to_rfc3339()],
                |row| Ok(Self::row_to_session(row)),
            )?
            .filter_map(|r| r.ok())
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }

    /// Delete a session by ID.
    ///
    /// Used by the collector to discard sessions shorter than the minimum
    /// duration — keeping them would make session-derived stats disagree
    /// with the rollups, which never include them.
    pub fn delete_session(&self, session_id: &Uuid) -> Result<(), StorageError> {
        self.conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            [session_id.to_string()],
        )?;
        Ok(())
    }

    /// Close active (unclosed) sessions from previous app runs.
    /// The end time is estimated from the number of observed polls times
    /// the configured polling interval.
    pub fn close_stale_sessions(&self, polling_interval_secs: u32) -> Result<u32, StorageError> {
        let count = self.conn.execute(
            r#"UPDATE sessions
               SET end_time = strftime('%Y-%m-%dT%H:%M:%S+00:00', start_time, '+' || (activity_count * ?1) || ' seconds')
               WHERE end_time IS NULL"#,
            [polling_interval_secs],
        )?;
        Ok(count as u32)
    }

    /// Get every closed session. Used by the category resync pass:
    /// whenever the rule set changes, history is re-run through the
    /// classifier so sessions always reflect the CURRENT rules.
    pub fn get_closed_sessions(&self) -> Result<Vec<Session>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, start_time, end_time, app_name, window_title,
                      category, url, activity_count, idle_seconds_total
               FROM sessions
               WHERE end_time IS NOT NULL
               ORDER BY start_time ASC"#,
        )?;

        let sessions = stmt
            .query_map([], |row| Ok(Self::row_to_session(row)))?
            .filter_map(|r| r.ok())
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }

    /// Get the currently active (unclosed) session, if any.
    pub fn get_active_session(&self) -> Result<Option<Session>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, start_time, end_time, app_name, window_title,
                      category, url, activity_count, idle_seconds_total
               FROM sessions
               WHERE end_time IS NULL
               ORDER BY start_time DESC
               LIMIT 1"#,
        )?;

        let session = stmt
            .query_map([], |row| Ok(Self::row_to_session(row)))?
            .filter_map(|r| r.ok())
            .filter_map(|r| r.ok())
            .next();

        Ok(session)
    }

    /// Get distinct application names that have been tracked, ordered by usage.
    pub fn get_detected_apps(&self) -> Result<Vec<String>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT app_name, COUNT(*) as session_count
               FROM sessions
               GROUP BY app_name
               ORDER BY session_count DESC"#,
        )?;

        let apps = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(apps)
    }

    /// Helper: convert a SQLite row into a Session struct.
    fn row_to_session(row: &rusqlite::Row) -> Result<Session, StorageError> {
        let id_str: String = row.get(0).map_err(StorageError::Database)?;
        let start_str: String = row.get(1).map_err(StorageError::Database)?;
        let end_str: Option<String> = row.get(2).map_err(StorageError::Database)?;
        let category_json: String = row.get(5).map_err(StorageError::Database)?;

        let id = Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4());
        let start_time = DateTime::parse_from_rfc3339(&start_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let end_time = end_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });
        let category: Category =
            serde_json::from_str(&category_json).unwrap_or(Category::Uncategorized);

        Ok(Session {
            id,
            start_time,
            end_time,
            app_name: row.get(3).map_err(StorageError::Database)?,
            window_title: row.get(4).map_err(StorageError::Database)?,
            category,
            url: row.get(6).map_err(StorageError::Database)?,
            activity_count: row.get::<_, u32>(7).map_err(StorageError::Database)?,
            idle_seconds_total: row.get::<_, u32>(8).map_err(StorageError::Database)?,
        })
    }
}
