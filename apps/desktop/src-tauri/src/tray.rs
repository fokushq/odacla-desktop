//! System tray menu — built here so both startup and later refreshes
//! (focus-mode toggles from the tray or the Settings page) share one
//! definition of the menu.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Wry};

/// Build the tray menu. `focus_active` drives the checkmark on the
/// Focus Mode item (focus mode == whitelist/IncludeList tracking).
pub fn build_menu(app: &AppHandle, focus_active: bool) -> tauri::Result<Menu<Wry>> {
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
    Menu::with_items(app, &[&show, &focus, &separator, &quit])
}

/// Swap the tray menu so the Focus Mode checkmark matches reality.
/// Called after any tracking-mode change, wherever it originated.
pub fn refresh(app: &AppHandle, focus_active: bool) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_menu(app, focus_active) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}
