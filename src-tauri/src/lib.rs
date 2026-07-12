mod economy;
mod pet;
mod store;
mod tray;
mod watcher;

use economy::EconomyConfig;
use std::sync::Mutex;
use tauri::Manager;

/// Managed state holding the economy balance constants loaded from
/// `economy.toml` at startup.
struct EconomyState(Mutex<EconomyConfig>);

/// Returns the currently loaded economy balance constants. See
/// `docs/knowledge/game-economy.md` §8.
#[tauri::command]
fn get_config(state: tauri::State<EconomyState>) -> EconomyConfig {
    state.0.lock().expect("economy state mutex poisoned").clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config = economy::load_economy_config(app.handle())
                .expect("failed to load economy.toml");
            app.manage(EconomyState(Mutex::new(config)));
            tray::setup(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
