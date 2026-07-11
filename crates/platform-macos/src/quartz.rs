//! macOS (Quartz/CoreGraphics) implementation of activity detection.
//!
//! Uses `CGWindowListCopyWindowInfo` to enumerate on-screen windows,
//! `CGEventSourceSecondsSinceLastEventType` for idle time, and
//! `proc_pidpath` to resolve the owning executable.
//!
//! PERMISSIONS: window titles (`kCGWindowName`) are only populated when the
//! app has Screen Recording permission (System Settings → Privacy & Security).
//! Without it we gracefully fall back to the owner application's name, so
//! app-level tracking keeps working out of the box.

use fokus_platform::{
    extract_app_name, normalize_app_name, ActivityDetector, PlatformError, WindowInfo,
};

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;

// ─── CoreGraphics FFI ───────────────────────────────────────────────────────

type CGWindowListOption = u32;
type CGWindowID = u32;

const K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: CGWindowListOption = 1 << 0;
const K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS: CGWindowListOption = 1 << 4;
const K_CG_NULL_WINDOW_ID: CGWindowID = 0;

/// kCGEventSourceStateHIDSystemState — hardware-level input events.
const K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: u32 = 1;
/// kCGAnyInputEventType — any keyboard/mouse/tablet input.
const K_CG_ANY_INPUT_EVENT_TYPE: u32 = u32::MAX;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(
        option: CGWindowListOption,
        relative_to_window: CGWindowID,
    ) -> core_foundation::array::CFArrayRef;

    fn CGEventSourceSecondsSinceLastEventType(state: u32, event_type: u32) -> f64;
}

extern "C" {
    // From libproc (part of libSystem, no explicit link needed).
    fn proc_pidpath(pid: i32, buffer: *mut std::ffi::c_void, buffersize: u32) -> i32;
}

/// Maximum path length accepted by proc_pidpath (PROC_PIDPATHINFO_MAXSIZE).
const PROC_PIDPATH_MAX: usize = 4096;

// ─── Window info helpers ────────────────────────────────────────────────────

/// A single entry parsed out of the CGWindowList dictionaries.
struct RawWindow {
    layer: i32,
    owner_pid: i32,
    owner_name: Option<String>,
    title: Option<String>,
}

/// Copy the on-screen window list, front-to-back.
fn copy_window_list() -> Vec<RawWindow> {
    let array_ref = unsafe {
        CGWindowListCopyWindowInfo(
            K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS,
            K_CG_NULL_WINDOW_ID,
        )
    };
    if array_ref.is_null() {
        return Vec::new();
    }

    // Safety: CGWindowListCopyWindowInfo follows the Copy rule — we own the
    // returned array and wrap it so CF releases it when dropped.
    let array: CFArray<CFDictionary<CFString, CFType>> =
        unsafe { CFArray::wrap_under_create_rule(array_ref) };

    array
        .iter()
        .map(|dict| RawWindow {
            layer: get_i32(&dict, "kCGWindowLayer").unwrap_or(-1),
            owner_pid: get_i32(&dict, "kCGWindowOwnerPID").unwrap_or(0),
            owner_name: get_string(&dict, "kCGWindowOwnerName"),
            title: get_string(&dict, "kCGWindowName"),
        })
        .collect()
}

fn get_i32(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<i32> {
    let value = dict.find(CFString::new(key))?;
    value.downcast::<CFNumber>()?.to_i32()
}

fn get_string(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<String> {
    let value = dict.find(CFString::new(key))?;
    Some(value.downcast::<CFString>()?.to_string())
}

/// Resolve a PID to its executable path via proc_pidpath.
fn process_path_for_pid(pid: i32) -> Option<String> {
    if pid <= 0 {
        return None;
    }
    let mut buffer = vec![0u8; PROC_PIDPATH_MAX];
    let len = unsafe {
        proc_pidpath(
            pid,
            buffer.as_mut_ptr() as *mut std::ffi::c_void,
            buffer.len() as u32,
        )
    };
    if len <= 0 {
        return None;
    }
    buffer.truncate(len as usize);
    String::from_utf8(buffer).ok()
}

/// Build a WindowInfo from a raw window entry.
/// Title falls back to the owner app name when kCGWindowName is unavailable
/// (i.e. no Screen Recording permission).
fn to_window_info(raw: &RawWindow) -> Option<WindowInfo> {
    let process_path = process_path_for_pid(raw.owner_pid);

    let app_name = process_path
        .as_ref()
        .map(|p| normalize_app_name(&extract_app_name(p)))
        .or_else(|| raw.owner_name.clone())?;

    let window_title = raw
        .title
        .clone()
        .filter(|t| !t.is_empty())
        .or_else(|| raw.owner_name.clone())
        .unwrap_or_else(|| app_name.clone());

    Some(WindowInfo {
        app_name,
        window_title,
        process_path,
    })
}

/// True for entries that represent normal application windows.
/// Layer 0 is the standard window level; menu bar items, the Dock, and
/// overlays live on other layers.
fn is_app_window(raw: &RawWindow) -> bool {
    raw.layer == 0 && raw.owner_pid > 0
}

// ─── Detector ───────────────────────────────────────────────────────────────

/// The macOS implementation of ActivityDetector, backed by CoreGraphics.
pub struct MacosActivityDetector;

impl MacosActivityDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacosActivityDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityDetector for MacosActivityDetector {
    fn get_active_window(&self) -> Result<Option<WindowInfo>, PlatformError> {
        // The window list is ordered front-to-back, so the first standard
        // (layer 0) window belongs to the frontmost application.
        let front = copy_window_list().into_iter().find(is_app_window);
        Ok(front.as_ref().and_then(to_window_info))
    }

    fn get_idle_seconds(&self) -> Result<u32, PlatformError> {
        let seconds = unsafe {
            CGEventSourceSecondsSinceLastEventType(
                K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE,
                K_CG_ANY_INPUT_EVENT_TYPE,
            )
        };
        if seconds.is_nan() || seconds < 0.0 {
            return Err(PlatformError::IdleTime(
                "CGEventSourceSecondsSinceLastEventType returned invalid value".to_string(),
            ));
        }
        Ok(seconds as u32)
    }

    fn get_visible_windows(&self) -> Vec<WindowInfo> {
        copy_window_list()
            .iter()
            .filter(|raw| is_app_window(raw))
            .filter_map(to_window_info)
            .collect()
    }
}
