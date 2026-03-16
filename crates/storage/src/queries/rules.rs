//! Rule queries.

use fokus_domain::{Category, Rule, rule::MatchTarget};
use uuid::Uuid;

use crate::database::Database;
use crate::error::StorageError;

impl Database {
    /// Get all rules sorted by priority (lowest number = highest priority).
    pub fn get_all_rules(&self) -> Result<Vec<Rule>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, pattern, target, category, priority, enabled
             FROM rules ORDER BY priority ASC",
        )?;

        let rules = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let target_str: String = row.get(3)?;
                let category_json: String = row.get(4)?;

                Ok(Rule {
                    id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()),
                    name: row.get(1)?,
                    pattern: row.get(2)?,
                    target: match target_str.as_str() {
                        "app_name" => MatchTarget::AppName,
                        "window_title" => MatchTarget::WindowTitle,
                        "url" => MatchTarget::Url,
                        _ => MatchTarget::WindowTitle,
                    },
                    category: serde_json::from_str(&category_json)
                        .unwrap_or(Category::Uncategorized),
                    priority: row.get(5)?,
                    enabled: row.get::<_, i32>(6)? == 1,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rules)
    }

    /// Insert a new rule.
    pub fn insert_rule(&self, rule: &Rule) -> Result<(), StorageError> {
        let category_json = serde_json::to_string(&rule.category)?;
        let target_str = match rule.target {
            MatchTarget::AppName => "app_name",
            MatchTarget::WindowTitle => "window_title",
            MatchTarget::Url => "url",
        };

        self.conn.execute(
            "INSERT INTO rules (id, name, pattern, target, category, priority, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                rule.id.to_string(),
                rule.name,
                rule.pattern,
                target_str,
                category_json,
                rule.priority,
                rule.enabled as i32,
            ],
        )?;
        Ok(())
    }

    /// Update an existing rule.
    pub fn update_rule(&self, rule: &Rule) -> Result<(), StorageError> {
        let category_json = serde_json::to_string(&rule.category)?;
        let target_str = match rule.target {
            MatchTarget::AppName => "app_name",
            MatchTarget::WindowTitle => "window_title",
            MatchTarget::Url => "url",
        };

        self.conn.execute(
            "UPDATE rules SET name=?2, pattern=?3, target=?4, category=?5, priority=?6, enabled=?7
             WHERE id = ?1",
            rusqlite::params![
                rule.id.to_string(),
                rule.name,
                rule.pattern,
                target_str,
                category_json,
                rule.priority,
                rule.enabled as i32,
            ],
        )?;
        Ok(())
    }

    /// Delete a rule by ID.
    pub fn delete_rule(&self, rule_id: &Uuid) -> Result<(), StorageError> {
        self.conn.execute(
            "DELETE FROM rules WHERE id = ?1",
            [rule_id.to_string()],
        )?;
        Ok(())
    }
}
