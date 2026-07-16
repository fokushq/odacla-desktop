//! Linux/X11 integration layer for activity and idle detection.

#[cfg(target_os = "linux")]
mod x11;

#[cfg(target_os = "linux")]
pub use x11::LinuxActivityDetector;

// Re-export shared types so consumers can use them through this crate.
pub use odacla_platform::{ActivityDetector, PlatformError, WindowInfo};

/// Development stub for non-Linux platforms (e.g. building on Windows).
#[cfg(not(target_os = "linux"))]
pub struct LinuxActivityDetector;

#[cfg(not(target_os = "linux"))]
impl LinuxActivityDetector {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_os = "linux"))]
impl ActivityDetector for LinuxActivityDetector {
    fn get_active_window(&self) -> Result<Option<WindowInfo>, PlatformError> {
        Ok(Some(WindowInfo {
            app_name: "MockApp".to_string(),
            window_title: "Mock Window — Linux Stub".to_string(),
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
                window_title: "Mock Browser — Linux Stub".to_string(),
                process_path: Some("/usr/bin/mock-browser".to_string()),
            },
            WindowInfo {
                app_name: "MockEditor".to_string(),
                window_title: "Mock Editor — Linux Stub".to_string(),
                process_path: Some("/usr/bin/mock-editor".to_string()),
            },
        ]
    }
}
