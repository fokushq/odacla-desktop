//! A single observation of what the user is doing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents the source of an activity observation.
/// Desktop activities come from the OS window tracker.
/// Browser activities come from the browser extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    /// Detected via OS-level window tracking (GetForegroundWindow on Windows)
    Desktop,
    /// Reported by the browser extension (knows the exact URL + tab title)
    Browser,
}

/// A single point-in-time observation of user activity.
/// Multiple consecutive Activities with the same app get merged into a Session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// Unique identifier for this observation
    pub id: Uuid,

    /// When this observation was recorded
    pub timestamp: DateTime<Utc>,

    /// The name of the application (e.g., "Code", "Firefox", "Spotify")
    /// On Windows, this comes from the process name or window class.
    pub app_name: String,

    /// The window title at the time of observation.
    /// This is incredibly useful for classification — e.g., the title
    /// "Coursera | Machine Learning" tells us it's a study activity.
    pub window_title: String,

    /// If this came from the browser extension, the actual URL.
    /// Desktop activities won't have this — only the browser extension
    /// can reliably tell us the URL.
    pub url: Option<String>,

    /// Whether this came from the desktop tracker or browser extension
    pub kind: ActivityKind,

    /// Whether the user was actively using the computer at this moment.
    /// If the idle time exceeds a threshold (e.g., 5 minutes), we mark
    /// the activity as idle. This prevents counting "screen on but away"
    /// as productive time.
    pub is_idle: bool,

    /// How many seconds the user has been idle at the time of this observation.
    /// Useful for determining idle transitions — e.g., if idle_seconds jumps
    /// from 30 to 310, we know the user left around 5 minutes ago.
    pub idle_seconds: u32,
}

impl Activity {
    /// Create a new Activity with the current timestamp.
    pub fn new(
        app_name: String,
        window_title: String,
        url: Option<String>,
        kind: ActivityKind,
        is_idle: bool,
        idle_seconds: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            app_name,
            window_title,
            url,
            kind,
            is_idle,
            idle_seconds,
        }
    }
}
