//! Tauri commands — Bridge between Rust backend and Svelte UI.

use chrono::{NaiveDate, Utc};
use tauri::State;
use uuid::Uuid;

use fokus_domain::{Category, Rule, Session, Settings, rule::MatchTarget};
use fokus_platform::ActivityDetector;
use fokus_storage::queries::rollups::DailyRollup;

#[cfg(target_os = "windows")]
use fokus_platform_windows::WindowsActivityDetector as PlatformDetector;

#[cfg(target_os = "linux")]
use fokus_platform_linux::LinuxActivityDetector as PlatformDetector;

use crate::state::AppState;

/// Get all sessions for today or a specific date.
#[tauri::command]
pub fn get_today_sessions(date: Option<String>, state: State<AppState>) -> Result<Vec<Session>, String> {
    let today = match date {
        Some(d) => NaiveDate::parse_from_str(&d, "%Y-%m-%d")
            .map_err(|e| format!("Invalid date format: {}", e))?,
        None => Utc::now().date_naive(),
    };
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_sessions_for_date(today).map_err(|e| e.to_string())
}

/// Get sessions for a specific date (for browsing history).
#[tauri::command]
pub fn get_sessions_for_date(date: String, state: State<AppState>) -> Result<Vec<Session>, String> {
    let parsed_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_sessions_for_date(parsed_date)
        .map_err(|e| e.to_string())
}

/// Get the currently active (unclosed) session.
#[tauri::command]
pub fn get_active_session(state: State<AppState>) -> Result<Option<Session>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_active_session().map_err(|e| e.to_string())
}

/// Get distinct application names that Fokus has tracked, sorted by usage.
/// Powers the "detected apps" UI in Settings for whitelist mode.
#[tauri::command]
pub fn get_detected_apps(state: State<AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_detected_apps().map_err(|e| e.to_string())
}

/// Get today's daily rollups — total time per category.
#[tauri::command]
pub fn get_today_rollups(date: Option<String>, state: State<AppState>) -> Result<Vec<DailyRollup>, String> {
    let today = match date {
        Some(d) => NaiveDate::parse_from_str(&d, "%Y-%m-%d")
            .map_err(|e| format!("Invalid date format: {}", e))?,
        None => Utc::now().date_naive(),
    };
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_rollups_for_date(today).map_err(|e| e.to_string())
}

/// Get rollups for a specific date.
#[tauri::command]
pub fn get_rollups_for_date(date: String, state: State<AppState>) -> Result<Vec<DailyRollup>, String> {
    let parsed_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_rollups_for_date(parsed_date)
        .map_err(|e| e.to_string())
}

/// Get rollups for a date range (for weekly/monthly charts).
#[tauri::command]
pub fn get_rollups_in_range(start: String, end: String, state: State<AppState>) -> Result<Vec<DailyRollup>, String> {
    let start_date = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start date: {}", e))?;
    let end_date = NaiveDate::parse_from_str(&end, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end date: {}", e))?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_rollups_in_range(start_date, end_date)
        .map_err(|e| e.to_string())
}

/// Get all classification rules.
#[tauri::command]
pub fn get_all_rules(state: State<AppState>) -> Result<Vec<Rule>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_all_rules().map_err(|e| e.to_string())
}

/// Data structure for creating a new rule from the UI.
/// Uses a separate struct so the UI doesn't need to generate UUIDs.
#[derive(serde::Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub pattern: String,
    pub target: String,       // "app_name", "window_title", or "url"
    pub category: String,     // JSON-encoded Category
    pub priority: i32,
}

/// Create a new classification rule.
#[tauri::command]
pub fn create_rule(request: CreateRuleRequest, state: State<AppState>) -> Result<Rule, String> {
    let target = match request.target.as_str() {
        "app_name" => MatchTarget::AppName,
        "window_title" => MatchTarget::WindowTitle,
        "url" => MatchTarget::Url,
        _ => return Err("Invalid match target".to_string()),
    };

    let category: Category = serde_json::from_str(&request.category)
        .map_err(|e| format!("Invalid category: {}", e))?;

    let mut rule = Rule::new(request.name, request.pattern, target, category);
    rule.priority = request.priority;

    // Save to database
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.insert_rule(&rule).map_err(|e| e.to_string())?;

    // Update the classifier's rule set so new rules take effect immediately
    let rules = db.get_all_rules().map_err(|e| e.to_string())?;
    let mut classifier = state.classifier.lock().map_err(|e| e.to_string())?;
    classifier.update_rules(rules);

    Ok(rule)
}

/// Update an existing rule.
#[tauri::command]
pub fn update_rule(rule: Rule, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_rule(&rule).map_err(|e| e.to_string())?;

    // Refresh the classifier
    let rules = db.get_all_rules().map_err(|e| e.to_string())?;
    let mut classifier = state.classifier.lock().map_err(|e| e.to_string())?;
    classifier.update_rules(rules);

    Ok(())
}

/// Delete a rule by ID.
#[tauri::command]
pub fn delete_rule(rule_id: String, state: State<AppState>) -> Result<(), String> {
    let uuid = Uuid::parse_str(&rule_id)
        .map_err(|e| format!("Invalid UUID: {}", e))?;

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_rule(&uuid).map_err(|e| e.to_string())?;

    // Refresh the classifier
    let rules = db.get_all_rules().map_err(|e| e.to_string())?;
    let mut classifier = state.classifier.lock().map_err(|e| e.to_string())?;
    classifier.update_rules(rules);

    Ok(())
}

/// Get the current application settings.
#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_settings().map_err(|e| e.to_string())
}

/// Save updated settings.
/// Updates both SQLite and the shared settings so changes take effect immediately.
#[tauri::command]
pub fn save_settings(settings: Settings, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.save_settings(&settings).map_err(|e| e.to_string())?;

    // Hot-reload: update the shared settings so the collector sees the change
    let mut shared = state.settings.lock().map_err(|e| e.to_string())?;
    *shared = settings;

    Ok(())
}

/// Get all currently visible application windows from the OS.
/// Enumerates live top-level windows — not limited to the last-focused app.
/// Returns deduplicated, sorted, Fokus-filtered names.
/// Powers the "Currently running" picker in Settings.
#[tauri::command]
pub fn get_running_apps() -> Result<Vec<String>, String> {
    let detector = PlatformDetector::new();
    let windows = detector.get_visible_windows();

    let mut seen = std::collections::HashSet::new();
    let mut apps: Vec<String> = windows
        .into_iter()
        .map(|w| w.app_name)
        .filter(|n| !n.to_lowercase().contains("fokus"))
        .filter(|n| seen.insert(n.clone()))
        .collect();

    apps.sort();
    Ok(apps)
}
