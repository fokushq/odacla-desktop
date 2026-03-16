//! Fokus Desktop Application — Main Entry Point

// Prevents console window on Windows when running the app
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod state;

use std::sync::{Arc, Mutex};

use tauri::Manager;
use tokio::sync::watch;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use fokus_classifier::Classifier;
use fokus_collector::Collector;
use fokus_storage::Database;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,fokus=debug")),
        )
        .init();

    info!("Starting Fokus v{}", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            let db_path = app_data_dir.join("fokus.db");
            info!("Database path: {}", db_path.display());

            let db = Database::open(&db_path)
                .expect("Failed to open database");

            match db.close_stale_sessions() {
                Ok(0) => info!("No stale sessions to clean up"),
                Ok(n) => warn!("Closed {} stale session(s) from previous run", n),
                Err(e) => warn!("Failed to close stale sessions: {}", e),
            }

            let settings = db.get_settings()
                .unwrap_or_default();

            let rules = db.get_all_rules()
                .unwrap_or_default();
            info!("Loaded {} classification rules", rules.len());

            let classifier = Classifier::new(rules);

            let db = Arc::new(Mutex::new(db));
            let classifier = Arc::new(Mutex::new(classifier));
            let settings = Arc::new(Mutex::new(settings));

            let app_state = state::AppState {
                db: db.clone(),
                classifier: classifier.clone(),
                settings: settings.clone(),
            };
            app.manage(app_state);

            let (shutdown_tx, shutdown_rx) = watch::channel(false);
            app.manage(state::ShutdownHandle(shutdown_tx));

            let collector_db = db.clone();
            let collector_classifier = classifier.clone();
            let collector_settings = settings.clone();

            tauri::async_runtime::spawn(async move {
                let mut collector = Collector::new(
                    collector_db,
                    collector_classifier,
                    collector_settings,
                );
                collector.run(shutdown_rx).await;
            });

            info!("Fokus initialized — collector running in background");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_today_sessions,
            commands::get_today_rollups,
            commands::get_sessions_for_date,
            commands::get_rollups_for_date,
            commands::get_rollups_in_range,
            commands::get_all_rules,
            commands::create_rule,
            commands::update_rule,
            commands::delete_rule,
            commands::get_settings,
            commands::save_settings,
            commands::get_active_session,
            commands::get_detected_apps,
        ])
        .run(tauri::generate_context!())
        .expect("Failed to run Fokus");
}
