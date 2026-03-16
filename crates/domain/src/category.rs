//! Category labels assigned to activities.

use serde::{Deserialize, Serialize};

/// The built-in category types plus a user-defined custom option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Active learning — Coursera, Khan Academy, Udemy, textbook readers
    Study,
    /// Writing code — IDEs, terminals, GitHub
    Coding,
    /// Reading and note-taking — Notion, Obsidian, PDF readers
    NoteTaking,
    /// General productive work — email, calendar, documents
    Productive,
    /// Entertainment and social media — YouTube, Twitter, Reddit, games
    Entertainment,
    /// Communication — Slack, Discord, Teams, WhatsApp
    Communication,
    /// The user is away from the computer (idle timeout exceeded)
    Idle,
    /// Anything that doesn't match a rule. This is the default category
    /// so that no activity goes untagged.
    Uncategorized,
    /// User-defined category with a custom name
    Custom(String),
}

impl Category {
    /// Returns a human-friendly display name for the category.
    /// Used in the UI dashboard and reports.
    pub fn display_name(&self) -> &str {
        match self {
            Category::Study => "Study",
            Category::Coding => "Coding",
            Category::NoteTaking => "Note-taking",
            Category::Productive => "Productive",
            Category::Entertainment => "Entertainment",
            Category::Communication => "Communication",
            Category::Idle => "Idle",
            Category::Uncategorized => "Uncategorized",
            Category::Custom(name) => name.as_str(),
        }
    }

    /// Returns a suggested color for the UI. Each category gets a distinct
    /// color so charts and timelines are easy to read at a glance.
    /// Colors are returned as hex strings for direct use in CSS/Svelte.
    pub fn color(&self) -> &str {
        match self {
            Category::Study => "#3B82F6",        // Blue
            Category::Coding => "#8B5CF6",       // Purple
            Category::NoteTaking => "#06B6D4",   // Cyan
            Category::Productive => "#10B981",   // Green
            Category::Entertainment => "#F59E0B", // Amber
            Category::Communication => "#EC4899", // Pink
            Category::Idle => "#9CA3AF",         // Gray
            Category::Uncategorized => "#D1D5DB", // Light gray
            Category::Custom(_) => "#6366F1",    // Indigo (default for custom)
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
