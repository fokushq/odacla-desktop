//! Custom category queries.

use odacla_domain::{Category, CustomCategory};
use uuid::Uuid;

use crate::database::Database;
use crate::error::StorageError;

impl Database {
    /// Get all custom categories, alphabetically.
    pub fn get_custom_categories(&self) -> Result<Vec<CustomCategory>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, color FROM custom_categories ORDER BY name COLLATE NOCASE ASC",
        )?;

        let categories = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                Ok(CustomCategory {
                    id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()),
                    name: row.get(1)?,
                    color: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(categories)
    }

    /// Insert a new custom category. Fails if the name already exists
    /// (UNIQUE constraint).
    pub fn insert_custom_category(&self, category: &CustomCategory) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO custom_categories (id, name, color) VALUES (?1, ?2, ?3)",
            rusqlite::params![category.id.to_string(), category.name, category.color],
        )?;
        Ok(())
    }

    /// Update a custom category's name and/or color. When the name changes,
    /// every reference in rules, sessions and rollups is rewritten so
    /// historical data follows the rename.
    pub fn update_custom_category(&self, category: &CustomCategory) -> Result<(), StorageError> {
        let old_name: String = self
            .conn
            .query_row(
                "SELECT name FROM custom_categories WHERE id = ?1",
                [category.id.to_string()],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::NotFound(format!("custom category {}", category.id)))?;

        self.conn.execute(
            "UPDATE custom_categories SET name = ?2, color = ?3 WHERE id = ?1",
            rusqlite::params![category.id.to_string(), category.name, category.color],
        )?;

        if old_name != category.name {
            let old_json = serde_json::to_string(&Category::Custom(old_name))?;
            let new_json = serde_json::to_string(&Category::Custom(category.name.clone()))?;
            for table in ["rules", "sessions", "daily_rollups"] {
                self.conn.execute(
                    &format!("UPDATE {table} SET category = ?2 WHERE category = ?1"),
                    rusqlite::params![old_json, new_json],
                )?;
            }
        }

        Ok(())
    }

    /// Delete a custom category. Refuses when rules still reference it —
    /// the caller should surface that to the user instead of silently
    /// breaking their rule set. (Historical sessions keep the category
    /// label; they just fall back to the default custom color.)
    pub fn delete_custom_category(&self, id: &Uuid) -> Result<(), StorageError> {
        let name: String = self
            .conn
            .query_row(
                "SELECT name FROM custom_categories WHERE id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::NotFound(format!("custom category {id}")))?;

        let category_json = serde_json::to_string(&Category::Custom(name.clone()))?;
        let rules_using: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM rules WHERE category = ?1",
            [&category_json],
            |row| row.get(0),
        )?;

        if rules_using > 0 {
            return Err(StorageError::Migration(format!(
                "Category \"{name}\" is used by {rules_using} rule(s). Delete or reassign those rules first."
            )));
        }

        self.conn.execute(
            "DELETE FROM custom_categories WHERE id = ?1",
            [id.to_string()],
        )?;
        Ok(())
    }
}
