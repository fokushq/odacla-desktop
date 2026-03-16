//! Windows OS integration layer for activity and idle detection.

pub mod detector;
pub mod error;

pub use detector::WindowsActivityDetector;
pub use error::PlatformError;

/// Trait abstracting platform-specific activity detection.
pub trait ActivityDetector: Send + Sync {
    /// Get information about the currently active window.
    /// Returns None if no window is in the foreground (e.g., desktop is shown).
    fn get_active_window(&self) -> Result<Option<WindowInfo>, PlatformError>;

    /// Get the number of seconds since the user last moved the mouse
    /// or pressed a key.
    fn get_idle_seconds(&self) -> Result<u32, PlatformError>;
}

/// Information about the currently focused window.
/// This is the platform-neutral representation — each platform fills it in
/// using its own APIs.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    /// The application name (extracted from process path)
    pub app_name: String,
    /// The window title text
    pub window_title: String,
    /// The full path to the executable (useful for exclusion rules)
    pub process_path: Option<String>,
}
