//! System tray menu — built here so both startup and later refreshes
//! (focus-mode toggles, manual timer start/stop) share one definition
//! of the menu.

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Manager, Wry};

/// The tray glyph for the current tracking state: the plain ring
/// normally, a bullseye (filled center) while Focus Mode narrows
/// tracking to the whitelist — visible at a glance even with the
/// window closed, since focus mode silently drops other apps.
/// Both are monochrome templates; macOS recolors them per theme.
pub fn icon(focus_active: bool) -> Image<'static> {
    let bytes: &[u8] = if focus_active {
        include_bytes!("../icons/tray-focus.png")
    } else {
        include_bytes!("../icons/tray.png")
    };
    Image::from_bytes(bytes).expect("embedded tray icon is valid PNG")
}

/// Build the tray menu. `focus_active` drives the checkmark on the
/// Focus Mode item (focus mode == whitelist/IncludeList tracking);
/// `timer_running` adds a Stop Timer item while a manual timer is live.
pub fn build_menu(
    app: &AppHandle,
    focus_active: bool,
    timer_running: bool,
) -> tauri::Result<Menu<Wry>> {
    let show = MenuItem::with_id(app, "show", "Show Odacla", true, None::<&str>)?;
    let focus = CheckMenuItem::with_id(
        app,
        "focus",
        "Focus Mode",
        true,
        focus_active,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Odacla", true, None::<&str>)?;

    if timer_running {
        let stop_timer =
            MenuItem::with_id(app, "stop-timer", "Stop Timer", true, None::<&str>)?;
        Menu::with_items(app, &[&show, &focus, &stop_timer, &separator, &quit])
    } else {
        Menu::with_items(app, &[&show, &focus, &separator, &quit])
    }
}

/// Swap the tray menu and glyph so they match reality (focus checkmark
/// + bullseye icon, timer item). Called after any tracking-mode or
/// timer change, wherever it originated.
pub fn refresh(app: &AppHandle, focus_active: bool, timer_running: bool) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_menu(app, focus_active, timer_running) {
            let _ = tray.set_menu(Some(menu));
        }
        let _ = tray.set_icon(Some(icon(focus_active)));
        // set_icon resets the template flag — reassert it so macOS keeps
        // recoloring the glyph for light/dark menu bars.
        let _ = tray.set_icon_as_template(true);
    }
}

/// Convenience: refresh the tray menu reading both states from the
/// managed app state.
pub fn refresh_from_state(app: &AppHandle) {
    let focus_active = app
        .try_state::<crate::state::AppState>()
        .and_then(|s| {
            s.settings
                .lock()
                .ok()
                .map(|s| s.tracking_mode == odacla_domain::TrackingMode::IncludeList)
        })
        .unwrap_or(false);
    let timer_running = app
        .try_state::<crate::state::TimerState>()
        .and_then(|t| t.0.lock().ok().map(|v| v.is_some()))
        .unwrap_or(false);
    refresh(app, focus_active, timer_running);
}
