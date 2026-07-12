//! Pet overlay window setup (task 0002 spike).
//!
//! The window's transparent/borderless/always-on-top/skip-taskbar flags are
//! declarative, in `tauri.conf.json` (label `"overlay"`). What can't be
//! declared statically is *which monitor* to cover - this module resizes and
//! repositions the overlay to the primary monitor's work area at startup.
//!
//! Click-through hit-testing and dragging are *not* handled here: they're
//! driven from the frontend (`ui/overlay/src/main.ts`) via
//! `setIgnoreCursorEvents`/`startDragging`, called directly from mouse
//! events, per the scope of `docs/tasks/active/0002-pet-overlay-window-spike.md`.
//!
//! See `docs/knowledge/overlay-platform-notes.md` for per-OS findings and the
//! Wayland fallback plan - Wayland sessions don't get true global
//! positioning, so this best-effort resize/position is expected to be a
//! no-op or approximate there.

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize};

/// Resizes and positions the `"overlay"` window to cover the primary
/// monitor's work area. Logs (rather than panics) on any failure, since a
/// spike-stage overlay should degrade to "wrong size" rather than crash the
/// whole app if monitor info is unavailable (e.g. headless CI, or Wayland
/// without the right protocol support).
pub fn fit_to_primary_monitor(app: &AppHandle) {
    let Some(window) = app.get_webview_window("overlay") else {
        eprintln!("overlay_window: no window labeled \"overlay\" found");
        return;
    };

    let monitor = match window.primary_monitor() {
        Ok(Some(m)) => m,
        Ok(None) => {
            eprintln!("overlay_window: no primary monitor reported, keeping configured size");
            return;
        }
        Err(e) => {
            eprintln!("overlay_window: failed to query primary monitor: {e}");
            return;
        }
    };

    let size = monitor.size();
    let position = monitor.position();

    if let Err(e) = window.set_size(PhysicalSize::new(size.width, size.height)) {
        eprintln!("overlay_window: failed to set size: {e}");
    }
    if let Err(e) = window.set_position(PhysicalPosition::new(position.x, position.y)) {
        eprintln!("overlay_window: failed to set position: {e}");
    }
}
