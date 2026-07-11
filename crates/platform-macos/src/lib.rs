//! macOS integration layer for activity and idle detection.

#[cfg(target_os = "macos")]
mod quartz;

#[cfg(target_os = "macos")]
pub use quartz::MacosActivityDetector;

// Re-export shared types so consumers can use them through this crate.
pub use fokus_platform::{ActivityDetector, PlatformError, WindowInfo};

/// Development stub for non-macOS platforms (e.g. building on Windows).
#[cfg(not(target_os = "macos"))]
pub struct MacosActivityDetector;

#[cfg(not(target_os = "macos"))]
impl MacosActivityDetector {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_os = "macos"))]
impl ActivityDetector for MacosActivityDetector {
    fn get_active_window(&self) -> Result<Option<WindowInfo>, PlatformError> {
        Ok(Some(WindowInfo {
            app_name: "MockApp".to_string(),
            window_title: "Mock Window — macOS Stub".to_string(),
            process_path: Some("/usr/bin/mock-app".to_string()),
        }))
    }

    fn get_idle_seconds(&self) -> Result<u32, PlatformError> {
        Ok(0)
    }

    fn get_visible_windows(&self) -> Vec<WindowInfo> {
        vec![
            WindowInfo {
                app_name: "MockBrowser".to_string(),
                window_title: "Mock Browser — macOS Stub".to_string(),
                process_path: Some("/usr/bin/mock-browser".to_string()),
            },
            WindowInfo {
                app_name: "MockEditor".to_string(),
                window_title: "Mock Editor — macOS Stub".to_string(),
                process_path: Some("/usr/bin/mock-editor".to_string()),
            },
        ]
    }
}
