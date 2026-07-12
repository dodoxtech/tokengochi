//! System tray lifecycle controls (task 0007).

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{menu::MenuBuilder, tray::TrayIconBuilder, Emitter, Manager};

const SHOW_PET: &str = "show_pet";
const HIDE_PET: &str = "hide_pet";
const OPEN_DASHBOARD: &str = "open_dashboard";
const TOGGLE_TRACKING: &str = "toggle_tracking";
const QUIT: &str = "quit";

pub fn show_dashboard(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn setup(
    app: &tauri::AppHandle,
    tracking_paused: Arc<AtomicBool>,
    on_tracking_changed: Arc<dyn Fn(bool) + Send + Sync>,
) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text(SHOW_PET, "Show pet")
        .text(HIDE_PET, "Hide pet")
        .text(OPEN_DASHBOARD, "Open dashboard")
        .separator()
        .text(TOGGLE_TRACKING, "Pause tracking")
        .separator()
        .text(QUIT, "Quit Tokengochi")
        .build()?;
    let icon = app
        .default_window_icon()
        .cloned()
        .expect("missing app icon");

    TrayIconBuilder::with_id("tokengochi")
        .menu(&menu)
        .icon(icon)
        .tooltip("Tokengochi")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            SHOW_PET => {
                if let Some(window) = app.get_webview_window("overlay") {
                    let _ = window.show();
                }
            }
            HIDE_PET => {
                if let Some(window) = app.get_webview_window("overlay") {
                    let _ = window.hide();
                }
            }
            OPEN_DASHBOARD => show_dashboard(app),
            TOGGLE_TRACKING => {
                let paused = !tracking_paused.fetch_xor(true, Ordering::SeqCst);
                on_tracking_changed(paused);
                let _ = app.emit("tracking_changed", paused);
            }
            QUIT => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
