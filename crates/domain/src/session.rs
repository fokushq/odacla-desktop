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
    ///
    /// Idle accounting: at each poll the user has been inactive for
    /// `idle_seconds`. Of the window since the previous poll
    /// (`poll_interval_secs` long), exactly `min(idle_seconds, interval)`
    /// seconds were idle — summing that per poll captures brief pauses
    /// (below the idle threshold) without double counting.
    pub fn extend(&mut self, window_title: String, idle_seconds: u32, poll_interval_secs: u32) {
        self.window_title = window_title;
        self.activity_count += 1;
        self.idle_seconds_total += idle_seconds.min(poll_interval_secs);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> Session {
        Session::start(
            "Code".to_string(),
            "main.rs".to_string(),
            Category::Coding,
            None,
        )
    }

    #[test]
    fn extend_updates_title_and_count() {
        let mut session = make_session();
        session.extend("lib.rs".to_string(), 0, 5);
        assert_eq!(session.window_title, "lib.rs");
        assert_eq!(session.activity_count, 2);
        assert_eq!(session.idle_seconds_total, 0);
    }

    #[test]
    fn extend_accumulates_brief_idle_pauses() {
        let mut session = make_session();
        // User paused for 3s within a 5s poll window → 3s idle
        session.extend("main.rs".to_string(), 3, 5);
        assert_eq!(session.idle_seconds_total, 3);
        // Still idle at the next poll (8s total) → the whole 5s window was idle
        session.extend("main.rs".to_string(), 8, 5);
        assert_eq!(session.idle_seconds_total, 8);
    }

    #[test]
    fn active_duration_subtracts_idle_and_never_goes_negative() {
        let mut session = make_session();
        session.idle_seconds_total = 3600; // more idle than elapsed time
        assert_eq!(session.active_duration(), Duration::zero());
    }
}
