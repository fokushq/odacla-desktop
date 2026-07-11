//! Background activity collection engine.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::watch;
use tracing::{debug, error, info, trace, warn};

use fokus_classifier::Classifier;
use fokus_domain::{Activity, ActivityKind, Category, Session, Settings, TrackingMode};
use fokus_platform::ActivityDetector;
use fokus_storage::Database;

#[cfg(target_os = "windows")]
use fokus_platform_windows::WindowsActivityDetector as PlatformDetector;

#[cfg(target_os = "linux")]
use fokus_platform_linux::LinuxActivityDetector as PlatformDetector;

#[cfg(target_os = "macos")]
use fokus_platform_macos::MacosActivityDetector as PlatformDetector;

/// The background collector service.
pub struct Collector {
    detector: PlatformDetector,
    classifier: Arc<Mutex<Classifier>>,
    db: Arc<Mutex<Database>>,
    current_session: Option<Session>,
    settings: Arc<Mutex<Settings>>,
    polls_since_last_flush: u32,
    last_rollup_seconds: i64,
}

/// How often to flush the current session to the database.
/// At 5-second polling intervals, 6 polls = every 30 seconds.
const FLUSH_INTERVAL_POLLS: u32 = 6;

impl Collector {
    /// Create a new collector with the given dependencies.
    pub fn new(
        db: Arc<Mutex<Database>>,
        classifier: Arc<Mutex<Classifier>>,
        settings: Arc<Mutex<Settings>>,
    ) -> Self {
        Self {
            detector: PlatformDetector::new(),
            classifier,
            db,
            current_session: None,
            settings,
            polls_since_last_flush: 0,
            last_rollup_seconds: 0,
        }
    }

    /// Start the collection loop until shutdown signal is received.
    pub async fn run(&mut self, mut shutdown_rx: watch::Receiver<bool>) {
        let interval_secs = {
            let s = self.settings.lock().unwrap();
            s.polling_interval_secs
        };
        let interval = Duration::from_secs(interval_secs as u64);
        info!(
            interval_secs = interval_secs,
            "Collector starting with polling interval"
        );

        loop {
            if *shutdown_rx.borrow() {
                info!("Collector received shutdown signal");
                self.finalize_current_session();
                break;
            }

            self.collect_once();

            self.polls_since_last_flush += 1;
            if self.polls_since_last_flush >= FLUSH_INTERVAL_POLLS {
                self.flush_current_session();
                self.polls_since_last_flush = 0;
            }

            tokio::select! {
                _ = tokio::time::sleep(interval) => {},
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Collector interrupted during sleep — shutting down");
                        self.finalize_current_session();
                        break;
                    }
                }
            }
        }

        info!("Collector stopped");
    }

    /// Perform one collection cycle.
    fn collect_once(&mut self) {
        // Step 1: Get the active window from the OS
        let window_info = match self.detector.get_active_window() {
            Ok(Some(info)) => info,
            Ok(None) => {
                trace!("No active window detected");
                return;
            }
            Err(e) => {
                warn!("Failed to get active window: {}", e);
                return;
            }
        };

        // Step 2: ALWAYS exclude Fokus itself, regardless of tracking mode.
        // This is hardcoded (not dependent on settings) because Fokus tracking
        // itself pollutes the data with constant self-referential sessions.
        // In IncludeList mode the excluded_apps list is skipped, so without
        // this hardcoded check, Fokus would track itself in whitelist mode.
        {
            let app_lower = window_info.app_name.to_lowercase();
            let title_lower = window_info.window_title.to_lowercase();
            if app_lower.contains("fokus") || title_lower.contains("fokus") {
                trace!(app = %window_info.app_name, "Skipping self (fokus)");
                return;
            }
        }

        // Step 3: Read current settings (may have been updated by UI)
        let settings = self.settings.lock().unwrap().clone();

        // Step 4: Apply tracking mode filter
        match settings.tracking_mode {
            TrackingMode::ExcludeList => {
                if Self::matches_list(&window_info.app_name, &window_info.window_title, &settings.excluded_apps) {
                    debug!(app = %window_info.app_name, mode = "exclude", "Skipping excluded app");
                    return;
                }
            }
            TrackingMode::IncludeList => {
                if !Self::matches_list(&window_info.app_name, &window_info.window_title, &settings.included_apps) {
                    trace!(app = %window_info.app_name, mode = "include", "Not in whitelist");
                    return;
                }
            }
        }

        // Step 5: Get idle time
        let idle_seconds = self.detector.get_idle_seconds().unwrap_or(0);
        let is_idle = idle_seconds >= settings.idle_threshold_secs;

        // Step 6: Create an Activity observation
        let activity = Activity::new(
            window_info.app_name.clone(),
            window_info.window_title.clone(),
            None,
            ActivityKind::Desktop,
            is_idle,
            idle_seconds,
        );

        // Step 7: Classify the activity
        let category = {
            let classifier = self.classifier.lock().unwrap();
            let cat = classifier.classify(&activity);
            debug!(
                app = %window_info.app_name,
                category = %cat,
                idle = is_idle,
                "Classified"
            );
            cat
        };

        // Step 8: Update session state
        self.update_session(activity, category, settings.polling_interval_secs);
    }

    /// Update the session state based on the new activity.
    fn update_session(&mut self, activity: Activity, category: Category, poll_interval_secs: u32) {
        match &mut self.current_session {
            Some(session) => {
                let same_app = session.app_name == activity.app_name;
                let same_category = session.category == category;

                if same_app && same_category && !activity.is_idle {
                    session.extend(activity.window_title, activity.idle_seconds, poll_interval_secs);
                    trace!(app = %session.app_name, polls = session.activity_count, "Extended");
                } else {
                    debug!(
                        old_app = %session.app_name, new_app = %activity.app_name,
                        old_cat = %session.category, new_cat = %category,
                        idle = activity.is_idle, "Session transition"
                    );
                    self.finalize_current_session();

                    if !activity.is_idle {
                        let new_session = Session::start(
                            activity.app_name,
                            activity.window_title,
                            category,
                            activity.url,
                        );
                        self.persist_new_session(&new_session);
                        self.current_session = Some(new_session);
                    }
                }
            }
            None => {
                if !activity.is_idle {
                    debug!(app = %activity.app_name, category = %category, "New session");
                    let new_session = Session::start(
                        activity.app_name,
                        activity.window_title,
                        category,
                        activity.url,
                    );
                    self.persist_new_session(&new_session);
                    self.current_session = Some(new_session);
                }
            }
        }
    }

    fn persist_new_session(&self, session: &Session) {
        if let Ok(db) = self.db.lock() {
            if let Err(e) = db.insert_session(session) {
                error!("Failed to insert session: {}", e);
            }
        }
    }

    /// Periodic flush — update session in DB and incrementally add time
    /// to the rollup. Does NOT bump session_count (only finalize does that).
    fn flush_current_session(&mut self) {
        if let Some(session) = &self.current_session {
            if let Ok(db) = self.db.lock() {
                if let Err(e) = db.update_session(session) {
                    error!("Failed to flush session: {}", e);
                }

                let current_secs = session.duration().num_seconds();
                let delta = current_secs - self.last_rollup_seconds;
                if delta > 0 {
                    let date = session.start_time.date_naive();
                    if let Err(e) = db.upsert_daily_rollup_seconds(date, &session.category, delta) {
                        error!("Failed to update rollup during flush: {}", e);
                    }
                    self.last_rollup_seconds = current_secs;
                }
            }
        }
    }

    /// Close and finalize the current session.
    fn finalize_current_session(&mut self) {
        let min_duration = {
            let s = self.settings.lock().unwrap();
            s.min_session_duration_secs as i64
        };

        if let Some(mut session) = self.current_session.take() {
            session.finish();
            let duration_secs = session.duration().num_seconds();
            let category = session.category.clone();

            if duration_secs >= min_duration {
                if let Ok(db) = self.db.lock() {
                    if let Err(e) = db.update_session(&session) {
                        error!("Failed to finalize session: {}", e);
                    }

                    let remaining = duration_secs - self.last_rollup_seconds;
                    let date = session.start_time.date_naive();
                    if remaining > 0 {
                        if let Err(e) = db.upsert_daily_rollup_seconds(date, &category, remaining) {
                            error!("Failed to update daily rollup: {}", e);
                        }
                    }
                    if let Err(e) = db.increment_daily_rollup_session_count(date, &category) {
                        error!("Failed to increment rollup session count: {}", e);
                    }
                }

                info!(
                    app = %session.app_name, category = %session.category,
                    duration_secs = duration_secs, "Session finalized"
                );
            } else {
                // Close the short session in DB but don't add to rollups
                if let Ok(db) = self.db.lock() {
                    let _ = db.update_session(&session);
                }
                debug!(
                    app = %session.app_name, duration_secs = duration_secs,
                    "Session too short, closed but not rolled up"
                );
            }

            self.last_rollup_seconds = 0;
        }
    }

    /// Check if an app name or window title matches any pattern in a list.
    /// Matching is case-insensitive substring on both app_name and window_title.
    fn matches_list(app_name: &str, window_title: &str, patterns: &[String]) -> bool {
        let app_lower = app_name.to_lowercase();
        let title_lower = window_title.to_lowercase();
        patterns.iter().any(|pattern| {
            let p = pattern.to_lowercase();
            app_lower.contains(&p) || title_lower.contains(&p)
        })
    }
}
