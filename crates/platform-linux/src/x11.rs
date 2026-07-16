//! X11-based implementation of activity detection for Linux.
//!
//! Uses xcb (via x11rb) to query the active window, window titles,
//! process IDs, and idle time. Falls back gracefully when properties
//! are missing (e.g. some tiling WMs omit _NET_WM_PID).
//!
//! If no X11 display is reachable (pure Wayland without XWayland, or a
//! headless session), the detector degrades to a no-op instead of
//! panicking: the app stays alive, tracking simply reports nothing.

use odacla_platform::{
    extract_app_name, normalize_app_name, ActivityDetector, PlatformError, WindowInfo,
};
use tracing::warn;

use x11rb::connection::Connection;
use x11rb::protocol::screensaver;
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt, Window};
use x11rb::rust_connection::RustConnection;

/// The Linux implementation of ActivityDetector, backed by X11/XCB.
/// `inner` is None when no X server is reachable.
pub struct LinuxActivityDetector {
    inner: Option<X11Connection>,
}

struct X11Connection {
    conn: RustConnection,
    root: Window,
    // Cached atoms — these never change for a given X server.
    atom_net_active_window: Atom,
    atom_net_wm_name: Atom,
    atom_net_wm_pid: Atom,
    atom_net_client_list: Atom,
    atom_utf8_string: Atom,
    atom_wm_name: Atom,
}

impl LinuxActivityDetector {
    pub fn new() -> Self {
        match X11Connection::connect() {
            Some(inner) => Self { inner: Some(inner) },
            None => {
                warn!(
                    "No X11 display available (Wayland without XWayland?) — \
                     activity detection is disabled for this session"
                );
                Self { inner: None }
            }
        }
    }
}

impl Default for LinuxActivityDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl X11Connection {
    /// Connect and intern the atoms we need. Any failure yields None —
    /// callers treat that as "no detection available".
    fn connect() -> Option<Self> {
        let (conn, screen_num) = RustConnection::connect(None).ok()?;
        let root = conn.setup().roots.get(screen_num)?.root;

        // Must resolve all cookies before moving conn into Self.
        let c1 = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW").ok()?;
        let c2 = conn.intern_atom(false, b"_NET_WM_NAME").ok()?;
        let c3 = conn.intern_atom(false, b"_NET_WM_PID").ok()?;
        let c4 = conn.intern_atom(false, b"_NET_CLIENT_LIST").ok()?;
        let c5 = conn.intern_atom(false, b"UTF8_STRING").ok()?;

        let atom_net_active_window = c1.reply().ok()?.atom;
        let atom_net_wm_name = c2.reply().ok()?.atom;
        let atom_net_wm_pid = c3.reply().ok()?.atom;
        let atom_net_client_list = c4.reply().ok()?.atom;
        let atom_utf8_string = c5.reply().ok()?.atom;

        Some(Self {
            atom_net_active_window,
            atom_net_wm_name,
            atom_net_wm_pid,
            atom_net_client_list,
            atom_utf8_string,
            atom_wm_name: AtomEnum::WM_NAME.into(),
            conn,
            root,
        })
    }

    /// Get the _NET_ACTIVE_WINDOW from the root.
    fn active_window_id(&self) -> Result<Option<Window>, PlatformError> {
        let reply = self
            .conn
            .get_property(
                false,
                self.root,
                self.atom_net_active_window,
                AtomEnum::WINDOW,
                0,
                1,
            )
            .map_err(|e| PlatformError::ForegroundWindow(e.to_string()))?
            .reply()
            .map_err(|e| PlatformError::ForegroundWindow(e.to_string()))?;

        if reply.format == 32 && reply.length == 1 {
            let win = u32::from_ne_bytes(
                reply.value[..4]
                    .try_into()
                    .map_err(|_| PlatformError::ForegroundWindow("bad property".into()))?,
            );
            if win == 0 {
                return Ok(None);
            }
            Ok(Some(win))
        } else {
            Ok(None)
        }
    }

    /// Read _NET_WM_NAME (UTF-8) with fallback to WM_NAME (Latin-1).
    fn window_title(&self, win: Window) -> String {
        // Try _NET_WM_NAME first (UTF-8).
        if let Ok(cookie) = self.conn.get_property(
            false,
            win,
            self.atom_net_wm_name,
            self.atom_utf8_string,
            0,
            u32::MAX,
        ) {
            if let Ok(reply) = cookie.reply() {
                if !reply.value.is_empty() {
                    return String::from_utf8_lossy(&reply.value).into_owned();
                }
            }
        }
        // Fallback to WM_NAME.
        if let Ok(cookie) = self.conn.get_property(
            false,
            win,
            self.atom_wm_name,
            AtomEnum::STRING,
            0,
            u32::MAX,
        ) {
            if let Ok(reply) = cookie.reply() {
                if !reply.value.is_empty() {
                    return String::from_utf8_lossy(&reply.value).into_owned();
                }
            }
        }
        String::new()
    }

    /// Read _NET_WM_PID and resolve to an exe path via /proc.
    fn process_path(&self, win: Window) -> Option<String> {
        let cookie = self
            .conn
            .get_property(
                false,
                win,
                self.atom_net_wm_pid,
                AtomEnum::CARDINAL,
                0,
                1,
            )
            .ok()?;
        let reply = cookie.reply().ok()?;

        if reply.format == 32 && reply.length == 1 {
            let pid = u32::from_ne_bytes(reply.value[..4].try_into().ok()?);
            // Resolve via /proc/<pid>/exe symlink.
            std::fs::read_link(format!("/proc/{}/exe", pid))
                .ok()
                .and_then(|p| p.to_str().map(String::from))
        } else {
            None
        }
    }

    /// Build a WindowInfo for a given X11 window ID.
    fn window_info(&self, win: Window) -> Option<WindowInfo> {
        let title = self.window_title(win);
        if title.is_empty() {
            return None;
        }

        let process_path = self.process_path(win);
        let app_name = process_path
            .as_ref()
            .map(|p| normalize_app_name(&extract_app_name(p)))
            .unwrap_or_else(|| "Unknown".to_string());

        Some(WindowInfo {
            app_name,
            window_title: title,
            process_path,
        })
    }

    /// Read _NET_CLIENT_LIST from the root to get all managed windows.
    fn client_list(&self) -> Vec<Window> {
        let cookie = match self.conn.get_property(
            false,
            self.root,
            self.atom_net_client_list,
            AtomEnum::WINDOW,
            0,
            u32::MAX,
        ) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let reply = match cookie.reply() {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        if reply.format != 32 {
            return Vec::new();
        }

        reply
            .value
            .chunks_exact(4)
            .filter_map(|chunk| chunk.try_into().ok().map(u32::from_ne_bytes))
            .collect()
    }

    fn idle_seconds(&self) -> Result<u32, PlatformError> {
        let reply = screensaver::query_info(&self.conn, self.root)
            .map_err(|e| PlatformError::IdleTime(e.to_string()))?
            .reply()
            .map_err(|e| PlatformError::IdleTime(e.to_string()))?;

        Ok((reply.ms_since_user_input / 1000) as u32)
    }
}

impl ActivityDetector for LinuxActivityDetector {
    fn get_active_window(&self) -> Result<Option<WindowInfo>, PlatformError> {
        let Some(x11) = &self.inner else {
            return Ok(None);
        };
        let win = match x11.active_window_id()? {
            Some(w) => w,
            None => return Ok(None),
        };
        Ok(x11.window_info(win))
    }

    fn get_idle_seconds(&self) -> Result<u32, PlatformError> {
        match &self.inner {
            Some(x11) => x11.idle_seconds(),
            None => Ok(0),
        }
    }

    fn get_visible_windows(&self) -> Vec<WindowInfo> {
        let Some(x11) = &self.inner else {
            return Vec::new();
        };
        x11.client_list()
            .into_iter()
            .filter_map(|win| x11.window_info(win))
            .collect()
    }
}
