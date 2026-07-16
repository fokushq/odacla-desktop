//! Category resync — keeps session history consistent with the rules.
//!
//! Whenever the rule set changes (rule created/edited/deleted, category
//! renamed) and once at startup, every CLOSED session is re-run through
//! the classifier. Sessions whose category changes are updated, and
//! their time is moved between the matching daily rollups. The mental
//! model for the user: rules are the single source of truth — history
//! always reflects the current rules.

use tracing::warn;

use odacla_classifier::Classifier;
use odacla_domain::{Activity, ActivityKind};
use odacla_storage::Database;

/// Re-run the classifier over all closed sessions; returns how many changed.
/// Rollups are only adjusted for sessions long enough to have been rolled
/// up in the first place (`min_session_secs`).
pub fn resync_sessions(db: &Database, classifier: &Classifier, min_session_secs: i64) -> u32 {
    let sessions = match db.get_closed_sessions() {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to load sessions for resync: {}", e);
            return 0;
        }
    };

    let mut changed = 0;
    for mut session in sessions {
        let activity = Activity::new(
            session.app_name.clone(),
            session.window_title.clone(),
            session.url.clone(),
            ActivityKind::Desktop,
            false,
            0,
        );
        let new_category = classifier.classify(&activity);
        if new_category == session.category {
            continue;
        }

        let old_category = session.category.clone();
        session.category = new_category.clone();
        if let Err(e) = db.update_session(&session) {
            warn!("Failed to resync session category: {}", e);
            continue;
        }

        let duration_secs = session.duration().num_seconds();
        if duration_secs >= min_session_secs {
            let date = session
                .start_time
                .with_timezone(&chrono::Local)
                .date_naive();
            let _ = db.adjust_daily_rollup(date, &old_category, -duration_secs, -1);
            let _ = db.adjust_daily_rollup(date, &new_category, duration_secs, 1);
        }
        changed += 1;
    }
    changed
}
