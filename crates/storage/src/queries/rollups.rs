//! Daily rollup queries — Pre-aggregated summaries.

use chrono::NaiveDate;
use fokus_domain::Category;

use crate::database::Database;
use crate::error::StorageError;

/// A daily summary for one category.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyRollup {
    pub date: String,
    pub category: Category,
    pub total_seconds: i64,
    pub session_count: i64,
}

impl Database {
    /// Add time to a daily rollup without incrementing session_count.
    pub fn upsert_daily_rollup_seconds(
        &self,
        date: NaiveDate,
        category: &Category,
        additional_seconds: i64,
    ) -> Result<(), StorageError> {
        let date_str = date.format("%Y-%m-%d").to_string();
        let category_json = serde_json::to_string(category)?;

        self.conn.execute(
            r#"INSERT INTO daily_rollups (date, category, total_seconds, session_count)
               VALUES (?1, ?2, ?3, 0)
               ON CONFLICT(date, category) DO UPDATE SET
                   total_seconds = total_seconds + ?3"#,
            rusqlite::params![date_str, category_json, additional_seconds],
        )?;
        Ok(())
    }

    /// Increment session_count for a daily rollup (on session finalization).
    pub fn increment_daily_rollup_session_count(
        &self,
        date: NaiveDate,
        category: &Category,
    ) -> Result<(), StorageError> {
        let date_str = date.format("%Y-%m-%d").to_string();
        let category_json = serde_json::to_string(category)?;

        self.conn.execute(
            r#"INSERT INTO daily_rollups (date, category, total_seconds, session_count)
               VALUES (?1, ?2, 0, 1)
               ON CONFLICT(date, category) DO UPDATE SET
                   session_count = session_count + 1"#,
            rusqlite::params![date_str, category_json],
        )?;
        Ok(())
    }

    /// Get all rollups for a specific date.
    pub fn get_rollups_for_date(
        &self,
        date: NaiveDate,
    ) -> Result<Vec<DailyRollup>, StorageError> {
        let date_str = date.format("%Y-%m-%d").to_string();

        let mut stmt = self.conn.prepare(
            "SELECT date, category, total_seconds, session_count
             FROM daily_rollups WHERE date = ?1",
        )?;

        let rollups = stmt
            .query_map([&date_str], |row| {
                let category_json: String = row.get(1)?;
                Ok(DailyRollup {
                    date: row.get(0)?,
                    category: serde_json::from_str(&category_json)
                        .unwrap_or(Category::Uncategorized),
                    total_seconds: row.get(2)?,
                    session_count: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rollups)
    }

    /// Get rollups for a date range.
    pub fn get_rollups_in_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<DailyRollup>, StorageError> {
        let start_str = start.format("%Y-%m-%d").to_string();
        let end_str = end.format("%Y-%m-%d").to_string();

        let mut stmt = self.conn.prepare(
            "SELECT date, category, total_seconds, session_count
             FROM daily_rollups WHERE date >= ?1 AND date <= ?2
             ORDER BY date ASC",
        )?;

        let rollups = stmt
            .query_map(rusqlite::params![start_str, end_str], |row| {
                let category_json: String = row.get(1)?;
                Ok(DailyRollup {
                    date: row.get(0)?,
                    category: serde_json::from_str(&category_json)
                        .unwrap_or(Category::Uncategorized),
                    total_seconds: row.get(2)?,
                    session_count: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rollups)
    }
}
