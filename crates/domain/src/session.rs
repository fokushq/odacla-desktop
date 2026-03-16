//! A merged block of continuous activity in one application.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::category::Category;

/// A continuous block of activity in a single application/category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier for this session
    pub id: Uuid,

    /// When this session started
    pub start_time: DateTime<Utc>,

    /// When this session ended (None if still active)
    pub end_time: Option<DateTime<Utc>>,

    /// The application that was active during this session
    pub app_name: String,

    /// The most recent window title (titles change within an app,
    /// so we keep the last one as a representative sample)
    pub window_title: String,

    /// The classified category for this session
    pub category: Category,

    /// Optional URL if this was a browser session
    pub url: Option<String>,

    /// How many raw activity observations made up this session.
    /// Useful for debugging and data quality checks.
    pub activity_count: u32,

    /// Total idle time within this session (in seconds).
    /// A session might be 60 minutes long but include 5 minutes of
    /// brief idle periods. This helps calculate "active time" accurately.
    pub idle_seconds_total: u32,
}

impl Session {
    /// Create a new session starting now.
    pub fn start(app_name: String, window_title: String, category: Category, url: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            start_time: Utc::now(),
            end_time: None,
            app_name,
            window_title,
            category,
            url,
            activity_count: 1,
            idle_seconds_total: 0,
        }
    }

    /// Mark this session as finished at the current time.
    pub fn finish(&mut self) {
        self.end_time = Some(Utc::now());
    }

    /// Extend this session with a new activity observation.
    /// Updates the window title (to the latest), increments the count,
    /// and accumulates idle time.
    pub fn extend(&mut self, window_title: String, idle_seconds: u32, is_idle: bool) {
        self.window_title = window_title;
        self.activity_count += 1;
        if is_idle {
            // Add the polling interval worth of idle time
            // (typically 5 seconds between polls)
            self.idle_seconds_total += idle_seconds.min(10);
        }
    }

    /// Calculate the total duration of this session.
    /// If the session is still active, duration is measured to "now".
    pub fn duration(&self) -> Duration {
        let end = self.end_time.unwrap_or_else(Utc::now);
        end - self.start_time
    }

    /// Calculate the "active" duration — total duration minus idle time.
    /// This gives a more accurate picture of actual productive time.
    pub fn active_duration(&self) -> Duration {
        let total = self.duration();
        let idle = Duration::seconds(self.idle_seconds_total as i64);
        // Ensure we don't go negative (shouldn't happen, but be safe)
        if total > idle {
            total - idle
        } else {
            Duration::zero()
        }
    }

    /// Returns true if this session hasn't been closed yet.
    pub fn is_active(&self) -> bool {
        self.end_time.is_none()
    }

    /// Returns the duration formatted as "Xh Ym" for display.
    pub fn duration_display(&self) -> String {
        let secs = self.duration().num_seconds();
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }
}
