//! Core data models for the Fokus time tracker.

pub mod activity;
pub mod category;
pub mod rule;
pub mod session;
pub mod settings;

// Re-export commonly used types at the crate root.
pub use activity::{Activity, ActivityKind};
pub use category::{Category, CustomCategory};
pub use rule::Rule;
pub use session::{Session, SessionSource};
pub use settings::{DailyGoal, Settings, TrackingMode};
