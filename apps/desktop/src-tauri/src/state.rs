//! Application state — Shared resources for Tauri commands.

use std::sync::{Arc, Mutex};
use tokio::sync::watch;

use fokus_classifier::Classifier;
use fokus_domain::Settings;
use fokus_storage::Database;

/// The shared application state for Tauri commands.
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub classifier: Arc<Mutex<Classifier>>,
    pub settings: Arc<Mutex<Settings>>,
}

/// Handle to send the shutdown signal to the collector.
pub struct ShutdownHandle(pub watch::Sender<bool>);
