//! System tray icon, menu, and autostart wiring.
//!
//! Full behavior (menu items, autostart toggle, open-dashboard action) is
//! tracked in `docs/tasks/backlog/0007-tray-settings-dashboard.md`. This is a
//! setup hook called from `run()` so the wiring point is in place.

use tauri::AppHandle;

/// Called once during app `setup()`. Currently a no-op placeholder.
pub fn setup(_app: &AppHandle) {
    // TODO(0007): system tray icon + menu + autostart.
}
