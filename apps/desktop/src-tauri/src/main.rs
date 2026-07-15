//! Odacla Desktop Application — Main Entry Point

// Prevents console window on Windows when running the app
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod reclassify;
mod state;

use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
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

    info!("Starting Odacla v{}", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .on_window_event(|window, event| {
            // Closing the window minimizes to tray so tracking keeps running.
            // If the tray icon is disabled in settings, closing quits the app.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let hide_to_tray = app
                    .state::<state::AppState>()
                    .settings
                    .lock()
                    .map(|s| s.show_tray_icon)
                    .unwrap_or(true);

                if hide_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else if let Some(handle) = app.try_state::<state::ShutdownHandle>() {
                    let _ = handle.0.send(true);
                }
            }
        })
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            let db_path = app_data_dir.join("odacla.db");
            info!("Database path: {}", db_path.display());

            // One-time migration: copy the database from the legacy Fokus
            // install location (com.fokus.app/fokus.db) if this is a fresh
            // Odacla install. WAL sidecar files are copied too so no
            // un-checkpointed transactions are lost.
            if !db_path.exists() {
                if let Some(legacy_dir) = app_data_dir
                    .parent()
                    .map(|p| p.join("com.fokus.app"))
                {
                    let legacy_db = legacy_dir.join("fokus.db");
                    if legacy_db.exists() {
                        let _ = std::fs::create_dir_all(&app_data_dir);
                        match std::fs::copy(&legacy_db, &db_path) {
                            Ok(_) => {
                                for ext in ["-wal", "-shm"] {
                                    let src = legacy_dir.join(format!("fokus.db{ext}"));
                                    if src.exists() {
                                        let _ = std::fs::copy(
                                            &src,
                                            app_data_dir.join(format!("odacla.db{ext}")),
                                        );
                                    }
                                }
                                info!("Migrated legacy Fokus database from {}", legacy_db.display());
                            }
                            Err(e) => warn!("Failed to migrate legacy Fokus database: {}", e),
                        }
                    }
                }
            }

            let db = Database::open(&db_path)
                .expect("Failed to open database");

            let settings = db.get_settings()
                .unwrap_or_default();

            match db.close_stale_sessions(settings.polling_interval_secs) {
                Ok(0) => info!("No stale sessions to clean up"),
                Ok(n) => warn!("Closed {} stale session(s) from previous run", n),
                Err(e) => warn!("Failed to close stale sessions: {}", e),
            }

            let rules = db.get_all_rules()
                .unwrap_or_default();
            info!("Loaded {} classification rules", rules.len());

            let classifier = Classifier::new(rules);

            // Bring session history in line with the current rule set —
            // rules may have changed since the last run (new defaults,
            // migrations, edits from a previous session).
            let min_secs = settings.min_session_duration_secs as i64;
            match reclassify::resync_sessions(&db, &classifier, min_secs) {
                0 => {}
                n => info!("Resynced {} session(s) to the current rules", n),
            }

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

            // ─── System tray ────────────────────────────────────────
            let show_tray = settings.lock().map(|s| s.show_tray_icon).unwrap_or(true);

            let show_item = MenuItem::with_id(app, "show", "Show Odacla", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Odacla", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // Dedicated monochrome glyph (ring + dot) — macOS renders
            // template icons from the alpha channel, so the colored app
            // icon would show up as a solid blob in the menu bar.
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))
                .expect("embedded tray icon is valid PNG");

            let tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .icon_as_template(true)
                .tooltip("Odacla — Time Tracker")
                .menu(&tray_menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        // Signal the collector so it can finalize the current
                        // session, then exit shortly after.
                        if let Some(handle) = app.try_state::<state::ShutdownHandle>() {
                            let _ = handle.0.send(true);
                        }
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                            app.exit(0);
                        });
                    }
                    _ => {}
                })
                .build(app)?;
            let _ = tray.set_visible(show_tray);

            // ─── Autostart — sync OS state with the stored setting ──
            let start_on_boot = settings.lock().map(|s| s.start_on_boot).unwrap_or(false);
            let autolaunch = app.autolaunch();
            let result = if start_on_boot {
                autolaunch.enable()
            } else {
                autolaunch.disable()
            };
            if let Err(e) = result {
                warn!("Failed to sync autostart state: {}", e);
            }

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

            info!("Odacla initialized — collector running in background");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_sessions_in_range,
            commands::get_today_rollups,
            commands::get_rollups_for_date,
            commands::get_rollups_in_range,
            commands::get_all_rules,
            commands::create_rule,
            commands::update_rule,
            commands::delete_rule,
            commands::get_custom_categories,
            commands::create_custom_category,
            commands::update_custom_category,
            commands::delete_custom_category,
            commands::get_settings,
            commands::save_settings,
            commands::get_active_session,
            commands::get_detected_apps,
            commands::get_running_apps,
        ])
        .run(tauri::generate_context!())
        .expect("Failed to run Odacla");
}
