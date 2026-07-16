//! Integration-style tests against an in-memory SQLite database.

use chrono::{Duration, Utc};
use uuid::Uuid;

use odacla_domain::{rule::MatchTarget, Category, CustomCategory, Rule, Session, Settings};

use crate::Database;

fn db() -> Database {
    Database::open_in_memory().expect("in-memory db")
}

fn make_session(app: &str) -> Session {
    Session::start(
        app.to_string(),
        format!("{app} — window"),
        Category::Coding,
        None,
    )
}

// ─── Schema & seeding ───────────────────────────────────────────────────────

#[test]
fn fresh_db_seeds_default_rules_and_settings() {
    let db = db();
    let rules = db.get_all_rules().unwrap();
    assert!(rules.len() > 50, "expected default rules, got {}", rules.len());

    // Settings row exists and round-trips to the defaults
    let settings = db.get_settings().unwrap();
    assert_eq!(settings.polling_interval_secs, Settings::default().polling_interval_secs);
}

#[test]
fn rules_seed_version_is_current_and_v2_rules_present() {
    let db = db();
    let version: String = db
        .conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'rules_seed_version'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, "2");

    let rules = db.get_all_rules().unwrap();
    assert!(rules.iter().any(|r| r.pattern == "safari"));
    assert!(rules.iter().any(|r| r.pattern == "xcode"));
}

#[test]
fn legacy_tables_are_dropped() {
    let db = db();
    let count: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('raw_events','categories')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

// ─── Sessions ───────────────────────────────────────────────────────────────

#[test]
fn session_insert_update_and_range_query() {
    let db = db();
    let mut session = make_session("Code");
    db.insert_session(&session).unwrap();

    // Extend and update
    session.extend("lib.rs".to_string(), 2, 5);
    session.finish();
    db.update_session(&session).unwrap();

    let start = Utc::now() - Duration::hours(1);
    let end = Utc::now() + Duration::hours(1);
    let found = db.get_sessions_in_range(start, end).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].app_name, "Code");
    assert_eq!(found[0].window_title, "lib.rs");
    assert_eq!(found[0].activity_count, 2);
    assert_eq!(found[0].idle_seconds_total, 2);
    assert!(found[0].end_time.is_some());

    // A range in the past finds nothing
    let past = db
        .get_sessions_in_range(start - Duration::days(2), start - Duration::days(1))
        .unwrap();
    assert!(past.is_empty());
}

#[test]
fn delete_session_removes_row() {
    let db = db();
    let session = make_session("Code");
    db.insert_session(&session).unwrap();
    db.delete_session(&session.id).unwrap();

    let found = db
        .get_sessions_in_range(Utc::now() - Duration::hours(1), Utc::now() + Duration::hours(1))
        .unwrap();
    assert!(found.is_empty());
}

#[test]
fn active_session_is_returned_and_stale_sessions_get_closed() {
    let db = db();
    let session = make_session("Code"); // end_time = None
    db.insert_session(&session).unwrap();

    let active = db.get_active_session().unwrap();
    assert!(active.is_some());
    assert_eq!(active.unwrap().id, session.id);

    // Closing stale sessions estimates end = start + activity_count * interval
    let closed = db.close_stale_sessions(5).unwrap();
    assert_eq!(closed, 1);
    assert!(db.get_active_session().unwrap().is_none());
}

#[test]
fn detected_apps_are_ordered_by_usage() {
    let db = db();
    db.insert_session(&make_session("Code")).unwrap();
    db.insert_session(&make_session("Code")).unwrap();
    db.insert_session(&make_session("Firefox")).unwrap();

    let apps = db.get_detected_apps().unwrap();
    assert_eq!(apps, vec!["Code".to_string(), "Firefox".to_string()]);
}

// ─── Manual sessions ────────────────────────────────────────────────────────

#[test]
fn manual_session_source_roundtrips() {
    let db = db();
    let start = Utc::now() - Duration::hours(2);
    let manual = Session::manual(
        "Sprint planning".to_string(),
        Category::Productive,
        start,
        Some(start + Duration::hours(1)),
    );
    db.insert_session(&manual).unwrap();

    let found = db
        .get_sessions_in_range(Utc::now() - Duration::hours(3), Utc::now())
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].source, odacla_domain::SessionSource::Manual);
    assert_eq!(found[0].app_name, "Sprint planning");
}

#[test]
fn manual_sessions_are_invisible_to_auto_queries() {
    let db = db();
    let start = Utc::now() - Duration::hours(1);
    // Closed manual entry + running manual timer
    let entry = Session::manual(
        "Meeting".to_string(),
        Category::Productive,
        start,
        Some(start + Duration::minutes(30)),
    );
    let timer = Session::manual("Reading".to_string(), Category::Study, Utc::now(), None);
    db.insert_session(&entry).unwrap();
    db.insert_session(&timer).unwrap();

    // Resync input must skip manual history — rules never touch it
    assert!(db.get_closed_sessions().unwrap().is_empty());
    // The collector's active-session view must not see the timer
    assert!(db.get_active_session().unwrap().is_none());
    // Manual labels are not applications
    assert!(db.get_detected_apps().unwrap().is_empty());
    // ...but the timer query finds it
    let active = db.get_active_manual_session().unwrap().unwrap();
    assert_eq!(active.id, timer.id);
}

#[test]
fn stale_cleanup_discards_orphaned_manual_timer_but_closes_auto() {
    let db = db();
    db.insert_session(&make_session("Code")).unwrap(); // open auto
    let timer = Session::manual("Reading".to_string(), Category::Study, Utc::now(), None);
    db.insert_session(&timer).unwrap(); // open manual (crash leftover)

    let closed = db.close_stale_sessions(5).unwrap();
    assert_eq!(closed, 1, "only the auto session gets an estimated end");

    assert!(db.get_active_manual_session().unwrap().is_none());
    assert!(db.get_session_by_id(&timer.id).unwrap().is_none(), "orphaned timer is deleted");

    let all = db
        .get_sessions_in_range(Utc::now() - Duration::hours(1), Utc::now() + Duration::hours(1))
        .unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].end_time.is_some());
}

// ─── Rollups ────────────────────────────────────────────────────────────────

#[test]
fn rollup_seconds_accumulate_without_touching_session_count() {
    let db = db();
    let date = Utc::now().date_naive();

    db.upsert_daily_rollup_seconds(date, &Category::Coding, 60).unwrap();
    db.upsert_daily_rollup_seconds(date, &Category::Coding, 30).unwrap();

    let rollups = db.get_rollups_for_date(date).unwrap();
    assert_eq!(rollups.len(), 1);
    assert_eq!(rollups[0].total_seconds, 90);
    assert_eq!(rollups[0].session_count, 0);
}

#[test]
fn rollup_session_count_increments_independently() {
    let db = db();
    let date = Utc::now().date_naive();

    db.increment_daily_rollup_session_count(date, &Category::Coding).unwrap();
    db.increment_daily_rollup_session_count(date, &Category::Coding).unwrap();

    let rollups = db.get_rollups_for_date(date).unwrap();
    assert_eq!(rollups[0].session_count, 2);
    assert_eq!(rollups[0].total_seconds, 0);
}

#[test]
fn rollup_negative_delta_reverts_discarded_time() {
    let db = db();
    let date = Utc::now().date_naive();

    db.upsert_daily_rollup_seconds(date, &Category::Coding, 45).unwrap();
    db.upsert_daily_rollup_seconds(date, &Category::Coding, -45).unwrap();

    let rollups = db.get_rollups_for_date(date).unwrap();
    assert_eq!(rollups[0].total_seconds, 0);
}

#[test]
fn rollups_in_range_are_date_filtered_and_ordered() {
    let db = db();
    let today = Utc::now().date_naive();
    let yesterday = today.pred_opt().unwrap();
    let long_ago = today - Duration::days(30);

    db.upsert_daily_rollup_seconds(yesterday, &Category::Study, 10).unwrap();
    db.upsert_daily_rollup_seconds(today, &Category::Coding, 20).unwrap();
    db.upsert_daily_rollup_seconds(long_ago, &Category::Entertainment, 30).unwrap();

    let rollups = db.get_rollups_in_range(yesterday, today).unwrap();
    assert_eq!(rollups.len(), 2);
    assert!(rollups[0].date <= rollups[1].date);
}

// ─── Rules ──────────────────────────────────────────────────────────────────

#[test]
fn rule_crud_roundtrip() {
    let db = db();
    let mut rule = Rule::new(
        "Test rule".to_string(),
        "myapp".to_string(),
        MatchTarget::AppName,
        Category::Custom("Deep Work".to_string()),
    );
    db.insert_rule(&rule).unwrap();

    let fetched = db
        .get_all_rules()
        .unwrap()
        .into_iter()
        .find(|r| r.id == rule.id)
        .expect("inserted rule present");
    assert_eq!(fetched.pattern, "myapp");
    assert_eq!(fetched.category, Category::Custom("Deep Work".to_string()));
    assert!(fetched.enabled);

    // Update
    rule.enabled = false;
    rule.priority = 5;
    db.update_rule(&rule).unwrap();
    let fetched = db
        .get_all_rules()
        .unwrap()
        .into_iter()
        .find(|r| r.id == rule.id)
        .unwrap();
    assert!(!fetched.enabled);
    assert_eq!(fetched.priority, 5);

    // Delete
    db.delete_rule(&rule.id).unwrap();
    assert!(db.get_all_rules().unwrap().iter().all(|r| r.id != rule.id));
}

#[test]
fn deleting_unknown_rule_is_a_noop() {
    let db = db();
    db.delete_rule(&Uuid::new_v4().to_string()).unwrap();
}

// ─── Custom categories ──────────────────────────────────────────────────────

#[test]
fn custom_category_crud_roundtrip() {
    let db = db();
    let mut cat = CustomCategory::new("Deep Work".to_string(), "#FF6B6B".to_string());
    db.insert_custom_category(&cat).unwrap();

    let all = db.get_custom_categories().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "Deep Work");
    assert_eq!(all[0].color, "#FF6B6B");

    // Duplicate name is rejected by the UNIQUE constraint
    let dup = CustomCategory::new("Deep Work".to_string(), "#000000".to_string());
    assert!(db.insert_custom_category(&dup).is_err());

    // Color update
    cat.color = "#00FF00".to_string();
    db.update_custom_category(&cat).unwrap();
    assert_eq!(db.get_custom_categories().unwrap()[0].color, "#00FF00");

    // Delete
    db.delete_custom_category(&cat.id).unwrap();
    assert!(db.get_custom_categories().unwrap().is_empty());
}

#[test]
fn custom_category_rename_cascades_to_rules_and_sessions() {
    let db = db();
    let mut cat = CustomCategory::new("Deep".to_string(), "#FF6B6B".to_string());
    db.insert_custom_category(&cat).unwrap();

    let rule = Rule::new(
        "Deep rule".to_string(),
        "focusapp".to_string(),
        MatchTarget::AppName,
        Category::Custom("Deep".to_string()),
    );
    db.insert_rule(&rule).unwrap();

    let mut session = make_session("FocusApp");
    session.category = Category::Custom("Deep".to_string());
    db.insert_session(&session).unwrap();

    cat.name = "Deep Work".to_string();
    db.update_custom_category(&cat).unwrap();

    let renamed = Category::Custom("Deep Work".to_string());
    let fetched_rule = db
        .get_all_rules()
        .unwrap()
        .into_iter()
        .find(|r| r.id == rule.id)
        .unwrap();
    assert_eq!(fetched_rule.category, renamed);

    let sessions = db
        .get_sessions_in_range(Utc::now() - Duration::hours(1), Utc::now() + Duration::hours(1))
        .unwrap();
    assert_eq!(sessions[0].category, renamed);
}

#[test]
fn custom_category_delete_blocked_while_rules_reference_it() {
    let db = db();
    let cat = CustomCategory::new("Deep".to_string(), "#FF6B6B".to_string());
    db.insert_custom_category(&cat).unwrap();

    let rule = Rule::new(
        "Deep rule".to_string(),
        "focusapp".to_string(),
        MatchTarget::AppName,
        Category::Custom("Deep".to_string()),
    );
    db.insert_rule(&rule).unwrap();

    // Blocked while a rule uses it
    assert!(db.delete_custom_category(&cat.id).is_err());

    // Free to delete once the rule is gone
    db.delete_rule(&rule.id).unwrap();
    db.delete_custom_category(&cat.id).unwrap();
}

// ─── Settings ───────────────────────────────────────────────────────────────

#[test]
fn settings_roundtrip_preserves_custom_values() {
    let db = db();
    let mut settings = Settings::default();
    settings.polling_interval_secs = 9;
    settings.excluded_apps.push("SecretApp".to_string());
    settings.daily_goals.push(odacla_domain::DailyGoal {
        category: Category::Coding,
        target_minutes: 240,
    });

    db.save_settings(&settings).unwrap();
    let loaded = db.get_settings().unwrap();
    assert_eq!(loaded.polling_interval_secs, 9);
    assert!(loaded.excluded_apps.contains(&"SecretApp".to_string()));
    assert_eq!(loaded.daily_goals.len(), 1);
    assert_eq!(loaded.daily_goals[0].target_minutes, 240);
}

#[test]
fn settings_without_goals_field_defaults_to_empty() {
    // Stored settings from older versions lack daily_goals — serde default
    let db = db();
    db.conn
        .execute(
            "UPDATE settings SET value = ?1 WHERE key = 'app_settings'",
            [r#"{"polling_interval_secs":5,"idle_threshold_secs":300,"excluded_apps":[],"track_browser_urls":true,"start_on_boot":false,"show_tray_icon":true,"min_session_duration_secs":10}"#],
        )
        .unwrap();
    let loaded = db.get_settings().unwrap();
    assert!(loaded.daily_goals.is_empty());
}
