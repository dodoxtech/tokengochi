//! System tray lifecycle controls (task 0007).

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{menu::MenuBuilder, tray::TrayIconBuilder, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::storage_paths;

const SHOW_PET: &str = "show_pet";
const HIDE_PET: &str = "hide_pet";
const OPEN_DASHBOARD: &str = "open_dashboard";
const TOGGLE_TRACKING: &str = "toggle_tracking";
const UNINSTALL: &str = "uninstall";
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
        .text(UNINSTALL, "Delete all data & quit…")
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
            UNINSTALL => confirm_and_wipe(app),
            QUIT => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

/// Prompts for confirmation, then permanently deletes all persisted Tokengochi
/// data (token history, pet/economy state, settings, and watcher bookkeeping)
/// and quits. A later launch recreates an empty database, so this doubles as an
/// in-app uninstall on every platform - including macOS and AppImage, which
/// have no OS-level uninstall hook. The dialog is shown non-blocking so the tray
/// event loop is never stalled while the user decides.
fn confirm_and_wipe(app: &tauri::AppHandle) {
    let app = app.clone();
    app.dialog()
        .message(
            "This permanently deletes all Tokengochi data on this computer - \
             token history, your pet and its progress, and settings. \
             This cannot be undone.\n\nThe app will quit. Reopening it (or \
             reinstalling) starts from a brand-new, empty database.",
        )
        .title("Delete all Tokengochi data?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Delete & quit".to_string(),
            "Cancel".to_string(),
        ))
        .show(move |confirmed| {
            if !confirmed {
                return;
            }
            if let Err(err) = storage_paths::wipe_all_app_data() {
                eprintln!("failed to delete app data on uninstall: {err}");
            }
            app.exit(0);
        });
}
