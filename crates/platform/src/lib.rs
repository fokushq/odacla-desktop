//! Shared platform abstraction for activity and idle detection.
//!
//! This crate defines the platform-neutral trait and types that
//! platform-specific implementations (Windows, Linux) must provide.

mod error;

pub use error::PlatformError;

/// Trait abstracting platform-specific activity detection.
pub trait ActivityDetector: Send + Sync {
    /// Get information about the currently active window.
    /// Returns None if no window is in the foreground (e.g., desktop is shown).
    fn get_active_window(&self) -> Result<Option<WindowInfo>, PlatformError>;

    /// Get the number of seconds since the user last moved the mouse
    /// or pressed a key.
    fn get_idle_seconds(&self) -> Result<u32, PlatformError>;

    /// Enumerate all currently visible top-level windows.
    /// Used by the running-apps picker in the UI.
    fn get_visible_windows(&self) -> Vec<WindowInfo>;
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

/// Map raw process name stems to human-readable canonical names.
/// Shared across platforms — matching is case-insensitive.
/// Apps not in this map are returned unchanged.
pub fn normalize_app_name(raw: &str) -> String {
    match raw.to_lowercase().as_str() {
        // ── JetBrains IDEs ──────────────────────────────────────────
        "pycharm64" | "pycharm"               => "PyCharm".to_string(),
        "idea64"    | "idea"                  => "IntelliJ IDEA".to_string(),
        "webstorm64"| "webstorm"              => "WebStorm".to_string(),
        "clion64"   | "clion"                 => "CLion".to_string(),
        "goland64"  | "goland"                => "GoLand".to_string(),
        "rider64"   | "rider"                 => "Rider".to_string(),
        "datagrip64"| "datagrip"              => "DataGrip".to_string(),
        "phpstorm64"| "phpstorm"              => "PhpStorm".to_string(),
        "rubymine64"| "rubymine"              => "RubyMine".to_string(),
        "androidstudio" | "studio64"          => "Android Studio".to_string(),
        // ── Other common apps with non-obvious exe names ────────────
        "devenv"                              => "Visual Studio".to_string(),
        "obs64"                               => "OBS Studio".to_string(),
        "powerpnt"                            => "PowerPoint".to_string(),
        "winword"                             => "Word".to_string(),
        // ── Linux-specific binary names ─────────────────────────────
        "gnome-terminal-server" | "gnome-terminal" => "Terminal".to_string(),
        "konsole"                             => "Konsole".to_string(),
        "xfce4-terminal"                      => "XFCE Terminal".to_string(),
        "alacritty"                           => "Alacritty".to_string(),
        "kitty"                               => "Kitty".to_string(),
        "nautilus"                             => "Files".to_string(),
        "thunar"                              => "Thunar".to_string(),
        "dolphin"                             => "Dolphin".to_string(),
        "eog"                                 => "Image Viewer".to_string(),
        "evince"                              => "Document Viewer".to_string(),
        "totem"                               => "Videos".to_string(),
        "rhythmbox"                           => "Rhythmbox".to_string(),
        "libreoffice" | "soffice"             => "LibreOffice".to_string(),
        // ── Pass through unchanged ──────────────────────────────────
        _ => raw.to_string(),
    }
}

/// Extract the application name stem from a full executable path.
/// Works with both Windows (`\`) and Unix (`/`) separators.
/// `"C:\...\pycharm64.exe"` → `"pycharm64"`
/// `"/usr/bin/code"` → `"code"`
pub fn extract_app_name(process_path: &str) -> String {
    std::path::Path::new(process_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string()
}
