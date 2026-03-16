//! Windows-specific implementation of activity detection.

use crate::{ActivityDetector, PlatformError, WindowInfo};

#[cfg(windows)]
mod win32 {
    use super::*;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId,
    };

    /// The Windows implementation of ActivityDetector.
    pub struct WindowsActivityDetectorImpl;

    impl WindowsActivityDetectorImpl {
        pub fn new() -> Self {
            Self
        }

        /// Read the window title from a window handle.
        fn get_window_title(hwnd: HWND) -> Result<String, PlatformError> {
            unsafe {
                let length = GetWindowTextLengthW(hwnd);
                if length == 0 {
                    return Ok(String::new());
                }

                // +1 for the null terminator that Windows expects
                let mut buffer: Vec<u16> = vec![0; (length + 1) as usize];
                let copied = GetWindowTextW(hwnd, &mut buffer);

                if copied == 0 {
                    return Ok(String::new());
                }

                Ok(String::from_utf16_lossy(&buffer[..copied as usize]))
            }
        }

        /// Get the executable path for the process that owns a window.
        fn get_process_path(hwnd: HWND) -> Result<Option<String>, PlatformError> {
            unsafe {
                let mut process_id: u32 = 0;
                GetWindowThreadProcessId(hwnd, Some(&mut process_id));

                if process_id == 0 {
                    return Ok(None);
                }

                let process_handle = OpenProcess(
                    PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                    false,
                    process_id,
                );

                match process_handle {
                    Ok(handle) => {
                        let mut buffer: Vec<u16> = vec![0; 1024];
                        let mut size = buffer.len() as u32;

                        let result = QueryFullProcessImageNameW(
                            handle,
                            PROCESS_NAME_FORMAT(0),
                            windows::core::PWSTR(buffer.as_mut_ptr()),
                            &mut size,
                        );

                        if result.is_ok() {
                            let path = String::from_utf16_lossy(&buffer[..size as usize]);
                            Ok(Some(path))
                        } else {
                            Ok(None)
                        }
                    }
                    Err(_) => Ok(None),
                }
            }
        }

        /// Extract the application name from a full exe path.
        fn extract_app_name(process_path: &str) -> String {
            std::path::Path::new(process_path)
                .file_stem()                    // "Code.exe" → "Code"
                .and_then(|s| s.to_str())       // OsStr → &str
                .unwrap_or("Unknown")
                .to_string()
        }
    }

    impl ActivityDetector for WindowsActivityDetectorImpl {
        fn get_active_window(&self) -> Result<Option<WindowInfo>, PlatformError> {
            unsafe {
                let hwnd = GetForegroundWindow();

                if hwnd.0 == std::ptr::null_mut() {
                    return Ok(None);
                }

                let title = Self::get_window_title(hwnd)?;
                let process_path = Self::get_process_path(hwnd)?;

                let app_name = process_path
                    .as_ref()
                    .map(|p| Self::extract_app_name(p))
                    .unwrap_or_else(|| "Unknown".to_string());

                Ok(Some(WindowInfo {
                    app_name,
                    window_title: title,
                    process_path,
                }))
            }
        }

        fn get_idle_seconds(&self) -> Result<u32, PlatformError> {
            unsafe {
                let mut info = LASTINPUTINFO {
                    cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                    dwTime: 0,
                };

                let success = GetLastInputInfo(&mut info);
                if !success.as_bool() {
                    return Err(PlatformError::IdleTime(
                        "GetLastInputInfo failed".to_string(),
                    ));
                }

                let current_tick = windows::Win32::System::SystemInformation::GetTickCount();
                let idle_ms = current_tick.saturating_sub(info.dwTime);

                Ok(idle_ms / 1000)
            }
        }
    }
}

/// Cross-platform wrapper.
pub struct WindowsActivityDetector {
    #[cfg(windows)]
    inner: win32::WindowsActivityDetectorImpl,
}

impl WindowsActivityDetector {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            inner: win32::WindowsActivityDetectorImpl::new(),
        }
    }
}

impl ActivityDetector for WindowsActivityDetector {
    fn get_active_window(&self) -> Result<Option<WindowInfo>, PlatformError> {
        #[cfg(windows)]
        {
            self.inner.get_active_window()
        }
        #[cfg(not(windows))]
        {
            // Development stub for non-Windows platforms
            Ok(Some(WindowInfo {
                app_name: "MockApp".to_string(),
                window_title: "Mock Window Title — Development Mode".to_string(),
                process_path: Some("/usr/bin/mock-app".to_string()),
            }))
        }
    }

    fn get_idle_seconds(&self) -> Result<u32, PlatformError> {
        #[cfg(windows)]
        {
            self.inner.get_idle_seconds()
        }
        #[cfg(not(windows))]
        {
            Ok(0)
        }
    }
}
