//! User-configurable preferences.

use serde::{Deserialize, Serialize};

use crate::category::Category;

/// A per-category daily time target (e.g. "4 hours of Coding a day").
/// Progress is measured against the local day's tracked time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyGoal {
    pub category: Category,
    /// Target minutes per day
    pub target_minutes: u32,
}

/// Controls whether Fokus tracks everything except exclusions (default)
/// or only whitelisted apps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrackingMode {
    /// Track everything, excluding items in `excluded_apps` (default).
    ExcludeList,

    /// Only track items matching `included_apps`.
    IncludeList,
}

impl Default for TrackingMode {
    fn default() -> Self {
        TrackingMode::ExcludeList
    }
}

/// Application-wide settings that the user can customize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Polling interval in seconds.
    pub polling_interval_secs: u32,

    /// Idle threshold in seconds.
    pub idle_threshold_secs: u32,

    /// Tracking mode: ExcludeList or IncludeList.
    #[serde(default)]
    pub tracking_mode: TrackingMode,

    /// Apps to exclude (ExcludeList mode).
    pub excluded_apps: Vec<String>,

    /// Apps to include (IncludeList mode).
    #[serde(default)]
    pub included_apps: Vec<String>,

    /// Whether to track browser URLs.
    pub track_browser_urls: bool,

    /// Whether to start automatically on boot.
    pub start_on_boot: bool,

    /// Whether to show the system tray icon.
    pub show_tray_icon: bool,

    /// Minimum session duration in seconds.
    pub min_session_duration_secs: u32,

    /// Per-category daily time goals (shown on the Dashboard).
    #[serde(default)]
    pub daily_goals: Vec<DailyGoal>,

    /// UI theme: "system" (follow the OS), "light" or "dark".
    #[serde(default = "default_appearance")]
    pub appearance: String,
}

fn default_appearance() -> String {
    "system".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            polling_interval_secs: 5,
            idle_threshold_secs: 300,
            tracking_mode: TrackingMode::ExcludeList,
            // Odacla itself is always excluded by a hardcoded check in the
            // collector — no self-entry needed here.
            excluded_apps: vec![
                "1Password".to_string(),
                "KeePass".to_string(),
                "Bitwarden".to_string(),
                "LastPass".to_string(),
                "LockApp".to_string(),
                "SearchHost".to_string(),
            ],
            included_apps: vec![
                "Code".to_string(),
                "Visual Studio".to_string(),
                "IntelliJ".to_string(),
                "WebStorm".to_string(),
                "Notion".to_string(),
                "Obsidian".to_string(),
                "Anki".to_string(),
                "Coursera".to_string(),
                "Udemy".to_string(),
                "Khan Academy".to_string(),
                "edX".to_string(),
                ".pdf".to_string(),
                "OneNote".to_string(),
                "Slack".to_string(),
                "Discord".to_string(),
                "Teams".to_string(),
                "Terminal".to_string(),
                "WindowsTerminal".to_string(),
                "cmd.exe".to_string(),
                "powershell".to_string(),
            ],
            track_browser_urls: true,
            start_on_boot: false,
            show_tray_icon: true,
            min_session_duration_secs: 10,
            daily_goals: Vec::new(),
            appearance: default_appearance(),
        }
    }
}
