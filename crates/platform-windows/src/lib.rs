//! Windows OS integration layer for activity and idle detection.

pub mod detector;

pub use detector::WindowsActivityDetector;

// Re-export shared types so existing consumers (collector, tauri app)
// continue to compile without changes to their import paths.
pub use fokus_platform::{ActivityDetector, PlatformError, WindowInfo};
