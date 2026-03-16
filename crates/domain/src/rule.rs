//! Classification rules mapping patterns to categories.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::category::Category;

/// Defines what part of an activity a rule should match against.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatchTarget {
    /// Match against the application name (e.g., "firefox", "Code")
    AppName,
    /// Match against the window title (e.g., "Coursera | Machine Learning")
    WindowTitle,
    /// Match against the URL (only works for browser activities)
    Url,
}

/// A classification rule that maps an activity pattern to a category.
///
/// The classifier evaluates rules in priority order. When an activity
/// comes in, each rule checks if its pattern appears in the relevant
/// field (app name, title, or URL). The first match determines the
/// category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Unique identifier for this rule
    pub id: Uuid,

    /// Human-readable name for the rule (e.g., "VS Code → Coding")
    pub name: String,

    /// The pattern to search for (case-insensitive substring match).
    pub pattern: String,

    /// Which part of the activity to match against
    pub target: MatchTarget,

    /// The category to assign when this rule matches
    pub category: Category,

    /// Lower numbers are checked first. This lets users create specific
    /// rules that override general ones.
    /// Example: priority 10 "youtube.com/watch?v=...lecture" → Study
    ///          priority 50 "youtube.com" → Entertainment
    pub priority: i32,

    /// Whether this rule is currently active. Users can disable rules
    /// without deleting them, which is useful for experimentation.
    pub enabled: bool,
}

impl Rule {
    /// Create a new rule with default priority and enabled state.
    pub fn new(
        name: String,
        pattern: String,
        target: MatchTarget,
        category: Category,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            pattern,
            target,
            category,
            priority: 100, // Default priority — user can adjust
            enabled: true,
        }
    }

    /// Check if this rule matches the given activity fields.
    /// URL rules fall back to window_title if no URL is provided.
    pub fn matches(&self, app_name: &str, window_title: &str, url: Option<&str>) -> bool {
        if !self.enabled {
            return false;
        }

        let pattern_lower = self.pattern.to_lowercase();

        match self.target {
            MatchTarget::AppName => app_name.to_lowercase().contains(&pattern_lower),
            MatchTarget::WindowTitle => window_title.to_lowercase().contains(&pattern_lower),
            MatchTarget::Url => {
                // Try the actual URL first (if provided by browser extension)
                if let Some(u) = url {
                    if u.to_lowercase().contains(&pattern_lower) {
                        return true;
                    }
                }
                // Fallback: check window_title (browsers show site info in title)
                window_title.to_lowercase().contains(&pattern_lower)
            }
        }
    }
}
