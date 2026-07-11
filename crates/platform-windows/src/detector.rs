//! Windows-specific implementation of activity detection.

#[cfg(windows)]
use fokus_platform::{extract_app_name, normalize_app_name};
use fokus_platform::{ActivityDetector, PlatformError, WindowInfo};

#[cfg(windows)]
mod win32 {
    use super::*;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible,
    };

    // ─── Free helper functions ──────────────────────────────────────────────
    // These are free functions (not associated methods) so they can be called
    // from the EnumWindows callback, which is a bare `extern "system" fn`.

    /// Read the window title from a window handle.
    pub(super) unsafe fn get_window_title(hwnd: HWND) -> Result<String, PlatformError> {
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

    /// Get the executable path for the process that owns a window.
    pub(super) unsafe fn get_process_path(hwnd: HWND) -> Result<Option<String>, PlatformError> {
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

    // ─── EnumWindows callback ───────────────────────────────────────────────

    /// Callback for `EnumWindows`. Collects visible top-level windows with
    /// non-empty titles into a `Vec<WindowInfo>` passed via `lparam`.
    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // Safety: lparam is a valid *mut Vec<WindowInfo> for the duration of
        // the EnumWindows call in get_visible_windows.
        let list = &mut *(lparam.0 as *mut Vec<WindowInfo>);

        // Filter: must be a visible window.
        if !IsWindowVisible(hwnd).as_bool() {
            return TRUE;
        }

        // Filter: must have a non-empty title.
        let title = match get_window_title(hwnd) {
            Ok(t) if !t.is_empty() => t,
            _ => return TRUE,
        };

        // Filter: must have a valid process path (skips system/protected processes).
        let process_path = match get_process_path(hwnd) {
            Ok(Some(p)) => p,
            _ => return TRUE,
        };

        let app_name = normalize_app_name(&extract_app_name(&process_path));

        list.push(WindowInfo {
            app_name,
            window_title: title,
            process_path: Some(process_path),
        });

        TRUE // continue enumeration
    }

    // ─── Detector implementation ────────────────────────────────────────────

    /// The Windows implementation of ActivityDetector.
    pub struct WindowsActivityDetectorImpl;

    impl WindowsActivityDetectorImpl {
        pub fn new() -> Self {
            Self
        }
    }

    impl ActivityDetector for WindowsActivityDetectorImpl {
        fn get_active_window(&self) -> Result<Option<WindowInfo>, PlatformError> {
            unsafe {
                let hwnd = GetForegroundWindow();

                if hwnd.0 == std::ptr::null_mut() {
                    return Ok(None);
                }

                let title = get_window_title(hwnd)?;
                let process_path = get_process_path(hwnd)?;

                let app_name = process_path
                    .as_ref()
                    .map(|p| normalize_app_name(&extract_app_name(p)))
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

        fn get_visible_windows(&self) -> Vec<WindowInfo> {
            let mut list: Vec<WindowInfo> = Vec::new();
            unsafe {
                if let Err(e) = EnumWindows(
                    Some(enum_windows_callback),
                    LPARAM(&mut list as *mut Vec<WindowInfo> as isize),
                ) {
                    tracing::warn!("EnumWindows returned error: {}", e);
                }
            }
            list
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

    fn get_visible_windows(&self) -> Vec<WindowInfo> {
        #[cfg(windows)]
        {
            self.inner.get_visible_windows()
        }
        #[cfg(not(windows))]
        {
            vec![
                WindowInfo {
                    app_name: "MockBrowser".to_string(),
                    window_title: "Mock Browser Window".to_string(),
                    process_path: Some("/usr/bin/mock-browser".to_string()),
                },
                WindowInfo {
                    app_name: "MockEditor".to_string(),
                    window_title: "Mock Editor — Development Mode".to_string(),
                    process_path: Some("/usr/bin/mock-editor".to_string()),
                },
            ]
        }
    }
}
