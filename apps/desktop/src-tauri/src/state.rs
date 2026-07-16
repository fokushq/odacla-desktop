//! Application state — Shared resources for Tauri commands.

use std::sync::{Arc, Mutex};
use tokio::sync::watch;

use odacla_classifier::Classifier;
use odacla_domain::Settings;
use odacla_storage::Database;

/// The shared application state for Tauri commands.
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub classifier: Arc<Mutex<Classifier>>,
    pub settings: Arc<Mutex<Settings>>,
}

/// Handle to send the shutdown signal to the collector.
pub struct ShutdownHandle(pub watch::Sender<bool>);

/// Start instant of the running manual timer, if any. Mirrors the open
/// manual session in the database; kept in memory so the tray ticker can
/// update the menu-bar clock every second without hitting SQLite.
pub struct TimerState(pub Mutex<Option<chrono::DateTime<chrono::Utc>>>);
