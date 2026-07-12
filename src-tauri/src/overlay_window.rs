//! Pet overlay window setup (task 0002 spike).
//!
//! The window's transparent/borderless/always-on-top/skip-taskbar flags are
//! declarative, in `tauri.conf.json` (label `"overlay"`). What can't be
//! declared statically is *which monitor* to cover - this module resizes and
//! repositions the overlay to the primary monitor's work area at startup.
//!
//! `tao`/`tauri`'s `Monitor` only reports the full display bounds, not the
//! work area macOS carves out for the Dock and menu bar. If the overlay
//! covers the full bounds, the frontend's "floor" (`groundY` in
//! `ui/overlay/src/main.ts`) sits at the physical bottom edge of the screen,
//! which is *underneath* the Dock in z-order (the Dock runs at a window
//! level above ordinary app windows), so the pet visually walks behind it.
//! On macOS we correct for this after the initial monitor-based placement by
//! reading `NSScreen.visibleFrame`, which excludes the Dock and menu bar.
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
pub fn fit_to_monitor(app: &AppHandle, monitor_index: u32, wayland_fallback: bool) {
    let Some(window) = app.get_webview_window("overlay") else {
        eprintln!("overlay_window: no window labeled \"overlay\" found");
        return;
    };

    if wayland_fallback {
        if let Err(e) = window.set_size(PhysicalSize::new(720, 480)) {
            eprintln!("overlay_window: failed to set fallback size: {e}");
        }
        if let Err(e) = window.set_position(PhysicalPosition::new(80, 80)) {
            eprintln!("overlay_window: failed to set fallback position: {e}");
        }
        return;
    }

    let monitors = match window.available_monitors() {
        Ok(monitors) => monitors,
        Err(e) => {
            eprintln!("overlay_window: failed to query monitors: {e}");
            return;
        }
    };
    if monitors.is_empty() {
        eprintln!("overlay_window: no monitors reported, keeping configured size");
        return;
    }
    let index = (monitor_index as usize).min(monitors.len() - 1);
    let monitor = &monitors[index];

    let size = monitor.size();
    let position = monitor.position();

    if let Err(e) = window.set_size(PhysicalSize::new(size.width, size.height)) {
        eprintln!("overlay_window: failed to set size: {e}");
    }
    if let Err(e) = window.set_position(PhysicalPosition::new(position.x, position.y)) {
        eprintln!("overlay_window: failed to set position: {e}");
    }

    #[cfg(target_os = "macos")]
    {
        match macos::visible_frame_physical(&window) {
            Some((position, size)) => {
                if let Err(e) = window.set_size(size) {
                    eprintln!("overlay_window: failed to set macOS visible-frame size: {e}");
                }
                if let Err(e) = window.set_position(position) {
                    eprintln!("overlay_window: failed to set macOS visible-frame position: {e}");
                }
            }
            None => {
                eprintln!(
                    "overlay_window: could not read NSScreen.visibleFrame, \
                     falling back to full monitor bounds (pet may render behind the Dock)"
                );
            }
        }
    }
}

/// macOS-only: resolves the overlay window's screen's visible frame (the
/// monitor bounds minus the Dock and menu bar) into the physical, top-left,
/// y-down pixel coordinates that `tauri`/`tao` expect for `set_position`.
#[cfg(target_os = "macos")]
mod macos {
    use core_graphics::display::CGDisplay;
    use std::ptr::NonNull;
    use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};

    pub(super) fn visible_frame_physical(
        window: &WebviewWindow,
    ) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
        let raw = window.ns_window().ok()?;
        let ptr = NonNull::new(raw as *mut objc2_app_kit::NSWindow)?;
        // SAFETY: `ns_window()` returns an autoreleased, valid `NSWindow*` for
        // the lifetime of this call, matching tauri's own internal usage of
        // this pointer (see `WebviewWindow::ns_window`).
        let ns_window: &objc2_app_kit::NSWindow = unsafe { ptr.as_ref() };
        let screen = ns_window.screen()?;

        let visible = screen.visibleFrame();
        let scale = screen.backingScaleFactor();

        // `NSScreen` frames use Cocoa's bottom-left, y-up coordinate space
        // shared across all displays, anchored at the primary display's
        // origin. `tao`'s `bottom_left_to_top_left` conversion (used for its
        // own window/monitor positioning) flips that into the top-left,
        // y-down space `set_position` expects, using the main display's
        // point height as the flip axis.
        let main_height_points = CGDisplay::main().pixels_high() as f64;
        let top_left_y = main_height_points - (visible.origin.y + visible.size.height);

        let position = PhysicalPosition::new(
            (visible.origin.x * scale).round() as i32,
            (top_left_y * scale).round() as i32,
        );
        let size = PhysicalSize::new(
            (visible.size.width * scale).round() as u32,
            (visible.size.height * scale).round() as u32,
        );

        Some((position, size))
    }
}
