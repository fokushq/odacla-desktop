//! Tauri commands — Bridge between Rust backend and Svelte UI.

use chrono::{DateTime, NaiveDate, Utc};
use tauri::State;
use uuid::Uuid;

use fokus_domain::{Category, CustomCategory, Rule, Session, Settings, rule::MatchTarget};
use fokus_platform::ActivityDetector;
use fokus_storage::queries::rollups::DailyRollup;

#[cfg(target_os = "windows")]
use fokus_platform_windows::WindowsActivityDetector as PlatformDetector;

#[cfg(target_os = "linux")]
use fokus_platform_linux::LinuxActivityDetector as PlatformDetector;

#[cfg(target_os = "macos")]
use fokus_platform_macos::MacosActivityDetector as PlatformDetector;

use crate::state::AppState;

/// After any rule/category mutation: re-run history through the current
/// rules so sessions, charts and rollups immediately reflect the change.
fn resync_history(
    db: &fokus_storage::Database,
    classifier: &fokus_classifier::Classifier,
    state: &State<AppState>,
) {
    let min_secs = state
        .settings
        .lock()
        .map(|s| s.min_session_duration_secs as i64)
        .unwrap_or(10);
    let changed = crate::reclassify::resync_sessions(db, classifier, min_secs);
    if changed > 0 {
        tracing::info!("Resynced {} session(s) after rule change", changed);
    }
}

/// Get sessions within an absolute UTC datetime range.
/// The frontend computes the range from *local* midnight boundaries, so
/// "today" means the user's local day — not the UTC day. (A user at UTC+3
/// would otherwise lose sessions started between 00:00 and 03:00.)
#[tauri::command]
pub fn get_sessions_in_range(start: String, end: String, state: State<AppState>) -> Result<Vec<Session>, String> {
    let start_dt = DateTime::parse_from_rfc3339(&start)
        .map_err(|e| format!("Invalid start datetime: {}", e))?
        .with_timezone(&Utc);
    let end_dt = DateTime::parse_from_rfc3339(&end)
        .map_err(|e| format!("Invalid end datetime: {}", e))?
        .with_timezone(&Utc);
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_sessions_in_range(start_dt, end_dt)
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
    resync_history(&db, &classifier, &state);

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
    resync_history(&db, &classifier, &state);

    Ok(())
}

/// Delete a rule by ID.
#[tauri::command]
pub fn delete_rule(rule_id: String, state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_rule(&rule_id).map_err(|e| e.to_string())?;

    // Refresh the classifier
    let rules = db.get_all_rules().map_err(|e| e.to_string())?;
    let mut classifier = state.classifier.lock().map_err(|e| e.to_string())?;
    classifier.update_rules(rules);
    resync_history(&db, &classifier, &state);

    Ok(())
}

// ─── Data export ────────────────────────────────────────────────────────────

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn sessions_to_csv(sessions: &[Session]) -> String {
    let mut out = String::from(
        "start_time,end_time,app_name,window_title,category,url,duration_seconds,idle_seconds_total\n",
    );
    for s in sessions {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            s.start_time.to_rfc3339(),
            s.end_time.map(|e| e.to_rfc3339()).unwrap_or_default(),
            csv_escape(&s.app_name),
            csv_escape(&s.window_title),
            csv_escape(s.category.display_name()),
            csv_escape(s.url.as_deref().unwrap_or("")),
            s.duration().num_seconds(),
            s.idle_seconds_total,
        ));
    }
    out
}

/// Export sessions in a local-date range to the Downloads folder as CSV
/// or JSON. Returns the written file path. No dialogs, no network — the
/// file lands in a predictable place the user already knows.
#[tauri::command]
pub fn export_data(
    app: tauri::AppHandle,
    start: String,
    end: String,
    format: String,
    state: State<AppState>,
) -> Result<String, String> {
    use chrono::{Local, TimeZone};
    use tauri::Manager;

    let start_date = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start date: {}", e))?;
    let end_date = NaiveDate::parse_from_str(&end, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end date: {}", e))?;
    if end_date < start_date {
        return Err("End date is before start date".to_string());
    }

    // Local midnights → UTC instants, end exclusive (day after end_date)
    let start_utc = Local
        .from_local_datetime(&start_date.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .ok_or("Ambiguous start date")?
        .with_timezone(&Utc);
    let end_utc = Local
        .from_local_datetime(
            &end_date
                .succ_opt()
                .ok_or("Invalid end date")?
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        )
        .single()
        .ok_or("Ambiguous end date")?
        .with_timezone(&Utc);

    let sessions = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_sessions_in_range(start_utc, end_utc)
            .map_err(|e| e.to_string())?
    };

    let (content, ext) = match format.as_str() {
        "csv" => (sessions_to_csv(&sessions), "csv"),
        "json" => (
            serde_json::to_string_pretty(&sessions).map_err(|e| e.to_string())?,
            "json",
        ),
        _ => return Err("Invalid format (expected csv or json)".to_string()),
    };

    let dir = app.path().download_dir().map_err(|e| e.to_string())?;
    let path = dir.join(format!("odacla-sessions-{start}_{end}.{ext}"));
    std::fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(path.display().to_string())
}

// ─── Custom categories ──────────────────────────────────────────────────────

/// Get all user-defined categories.
#[tauri::command]
pub fn get_custom_categories(state: State<AppState>) -> Result<Vec<CustomCategory>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_custom_categories().map_err(|e| e.to_string())
}

/// Create a new custom category (name must be unique).
#[tauri::command]
pub fn create_custom_category(
    name: String,
    color: String,
    state: State<AppState>,
) -> Result<CustomCategory, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Category name cannot be empty".to_string());
    }

    let category = CustomCategory::new(name, color);
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.insert_custom_category(&category)
        .map_err(|e| e.to_string())?;
    Ok(category)
}

/// Update a custom category's name/color. Renames cascade to rules,
/// sessions and rollups; the classifier is refreshed so renamed rules
/// keep matching.
#[tauri::command]
pub fn update_custom_category(
    category: CustomCategory,
    state: State<AppState>,
) -> Result<(), String> {
    if category.name.trim().is_empty() {
        return Err("Category name cannot be empty".to_string());
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_custom_category(&category)
        .map_err(|e| e.to_string())?;

    let rules = db.get_all_rules().map_err(|e| e.to_string())?;
    let mut classifier = state.classifier.lock().map_err(|e| e.to_string())?;
    classifier.update_rules(rules);
    resync_history(&db, &classifier, &state);

    Ok(())
}

/// Delete a custom category. Fails with a friendly message while rules
/// still reference it.
#[tauri::command]
pub fn delete_custom_category(id: String, state: State<AppState>) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_custom_category(&uuid).map_err(|e| e.to_string())
}

/// Get the current application settings.
#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_settings().map_err(|e| e.to_string())
}

/// Save updated settings.
/// Updates both SQLite and the shared settings so changes take effect immediately.
/// Also applies side effects that live outside the collector: tray icon
/// visibility and the OS autostart entry.
#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    settings: Settings,
    state: State<AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.save_settings(&settings).map_err(|e| e.to_string())?;

    // Apply tray visibility immediately
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_visible(settings.show_tray_icon);
    }

    // Apply autostart immediately
    {
        use tauri_plugin_autostart::ManagerExt;
        let autolaunch = app.autolaunch();
        let result = if settings.start_on_boot {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        if let Err(e) = result {
            tracing::warn!("Failed to update autostart: {}", e);
        }
    }

    // Hot-reload: update the shared settings so the collector sees the change
    let mut shared = state.settings.lock().map_err(|e| e.to_string())?;
    *shared = settings;

    Ok(())
}

/// Get all currently visible application windows from the OS.
/// Enumerates live top-level windows — not limited to the last-focused app.
/// Returns deduplicated, sorted names with Odacla itself filtered out.
/// Powers the "Currently running" picker in Settings.
#[tauri::command]
pub fn get_running_apps() -> Result<Vec<String>, String> {
    let detector = PlatformDetector::new();
    let windows = detector.get_visible_windows();

    let mut seen = std::collections::HashSet::new();
    let mut apps: Vec<String> = windows
        .into_iter()
        .map(|w| w.app_name)
        .filter(|n| {
            let lower = n.to_lowercase();
            !lower.contains("odacla") && !lower.contains("fokus")
        })
        .filter(|n| seen.insert(n.clone()))
        .collect();

    apps.sort();
    Ok(apps)
}
