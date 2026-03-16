//! Rule-based activity classification engine.

use fokus_domain::{Activity, Category, Rule};
use tracing::trace;

/// The classification engine.
pub struct Classifier {
    rules: Vec<Rule>,
}

impl Classifier {
    /// Create a new classifier with the given rules (sorted by priority).
    pub fn new(mut rules: Vec<Rule>) -> Self {
        rules.sort_by_key(|r| r.priority);
        Self { rules }
    }

    /// Classify an activity by evaluating rules in priority order.
    /// Returns the category from the first matching rule, or Uncategorized.
    /// Idle status takes precedence over rules.
    pub fn classify(&self, activity: &Activity) -> Category {
        if activity.is_idle {
            return Category::Idle;
        }

        for rule in &self.rules {
            if rule.matches(
                &activity.app_name,
                &activity.window_title,
                activity.url.as_deref(),
            ) {
                trace!(
                    rule = %rule.name,
                    app = %activity.app_name,
                    category = %rule.category,
                    "Rule matched"
                );
                return rule.category.clone();
            }
        }

        Category::Uncategorized
    }

    /// Replace the entire rule set (called when user edits rules in the UI).
    pub fn update_rules(&mut self, mut rules: Vec<Rule>) {
        rules.sort_by_key(|r| r.priority);
        self.rules = rules;
    }

    /// Get the current number of active rules.
    pub fn rule_count(&self) -> usize {
        self.rules.iter().filter(|r| r.enabled).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fokus_domain::{Activity, ActivityKind, rule::MatchTarget};

    /// Helper to create a test activity
    fn make_activity(app: &str, title: &str, url: Option<&str>) -> Activity {
        Activity::new(
            app.to_string(),
            title.to_string(),
            url.map(|u| u.to_string()),
            ActivityKind::Desktop,
            false,
            0,
        )
    }

    /// Helper to create a test rule
    fn make_rule(pattern: &str, target: MatchTarget, category: Category, priority: i32) -> Rule {
        let mut rule = Rule::new(
            format!("Test: {}", pattern),
            pattern.to_string(),
            target,
            category,
        );
        rule.priority = priority;
        rule
    }

    #[test]
    fn test_basic_classification() {
        let rules = vec![
            make_rule("Code", MatchTarget::AppName, Category::Coding, 10),
            make_rule("youtube", MatchTarget::Url, Category::Entertainment, 50),
        ];
        let classifier = Classifier::new(rules);

        let activity = make_activity("Code", "main.rs — my-project", None);
        assert_eq!(classifier.classify(&activity), Category::Coding);
    }

    #[test]
    fn test_priority_ordering() {
        // Higher priority (lower number) should win
        let rules = vec![
            make_rule("youtube.com/education", MatchTarget::Url, Category::Study, 10),
            make_rule("youtube.com", MatchTarget::Url, Category::Entertainment, 50),
        ];
        let classifier = Classifier::new(rules);

        // Educational YouTube → Study (priority 10 wins)
        let edu = make_activity("Chrome", "Learn Rust", Some("https://youtube.com/education/rust"));
        assert_eq!(classifier.classify(&edu), Category::Study);

        // Regular YouTube → Entertainment (priority 50 matches)
        let regular = make_activity("Chrome", "Cat Videos", Some("https://youtube.com/watch?v=cats"));
        assert_eq!(classifier.classify(&regular), Category::Entertainment);
    }

    #[test]
    fn test_idle_overrides_rules() {
        let rules = vec![
            make_rule("Code", MatchTarget::AppName, Category::Coding, 10),
        ];
        let classifier = Classifier::new(rules);

        // Even though the app is VS Code, idle should override
        let mut activity = make_activity("Code", "main.rs", None);
        activity.is_idle = true;
        assert_eq!(classifier.classify(&activity), Category::Idle);
    }

    #[test]
    fn test_no_match_returns_uncategorized() {
        let rules = vec![
            make_rule("Code", MatchTarget::AppName, Category::Coding, 10),
        ];
        let classifier = Classifier::new(rules);

        let activity = make_activity("Calculator", "Calculator", None);
        assert_eq!(classifier.classify(&activity), Category::Uncategorized);
    }
}
