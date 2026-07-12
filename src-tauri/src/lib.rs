mod economy;
mod overlay_window;
mod pet;
mod store;
mod tray;
mod watcher;

use chrono::Local;
use economy::{level_for_xp, mood_from_fullness, EconomyConfig, EconomyState};
use std::sync::{mpsc, Arc, Mutex};
use store::{GameStateStore, Ledger};
use tauri::{AppHandle, Emitter, Manager};
use watcher::{ClaudeCodeProvider, TokenEvent, TokenProvider};

/// Managed state holding the economy balance constants loaded from
/// `economy.toml` at startup. Named distinctly from `economy::EconomyState`
/// (the actual mutable pet/game state, task 0004) to avoid confusing the
/// two - this one just wraps the read-mostly config.
struct EconomyConfigState(Mutex<EconomyConfig>);

struct GameRuntime {
    config: EconomyConfig,
    economy: EconomyState,
    ledger: Ledger,
    state_store: GameStateStore,
}

struct GameRuntimeState(Arc<Mutex<GameRuntime>>);

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PetStatePayload {
    fullness: f64,
    mood: pet::Mood,
    xp: f64,
    level: u32,
    pending_food: u32,
    pantry: u32,
    food_earned_today: u32,
    banked_tokens_today: f64,
    tokens_per_food: f64,
    meter_progress: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FoodSpawnedPayload {
    id: String,
    pending_food: u32,
}

/// Returns the currently loaded economy balance constants. See
/// `docs/knowledge/game-economy.md` §8.
#[tauri::command]
fn get_config(state: tauri::State<EconomyConfigState>) -> EconomyConfig {
    state
        .0
        .lock()
        .expect("economy state mutex poisoned")
        .clone()
}

#[tauri::command]
fn get_pet_state(state: tauri::State<GameRuntimeState>) -> Result<PetStatePayload, String> {
    let mut runtime = state
        .0
        .lock()
        .map_err(|_| "game runtime mutex poisoned".to_string())?;
    reconcile_and_persist(&mut runtime).map_err(|err| err.to_string())?;
    Ok(pet_state_payload(&runtime))
}

#[tauri::command]
fn pet_ate(
    app: AppHandle,
    state: tauri::State<GameRuntimeState>,
) -> Result<PetStatePayload, String> {
    let mut runtime = state
        .0
        .lock()
        .map_err(|_| "game runtime mutex poisoned".to_string())?;
    reconcile_and_persist(&mut runtime).map_err(|err| err.to_string())?;

    let config = runtime.config.clone();
    if !runtime.economy.eat_from_inventory(&config) {
        return Err("no pending food to eat".to_string());
    }

    let now_unix = Local::now().timestamp();
    runtime
        .state_store
        .save_economy_state(&runtime.economy, now_unix)
        .map_err(|err| err.to_string())?;

    let payload = pet_state_payload(&runtime);
    let _ = app.emit("pet_state_changed", payload.clone());
    Ok(payload)
}

fn now_parts() -> (i64, chrono::NaiveDate) {
    let now = Local::now();
    (now.timestamp(), now.date_naive())
}

fn reconcile_and_persist(runtime: &mut GameRuntime) -> rusqlite::Result<()> {
    let (now_unix, today) = now_parts();
    runtime
        .economy
        .reconcile_elapsed_time(now_unix, today, &runtime.config);
    runtime
        .state_store
        .save_economy_state(&runtime.economy, now_unix)
}

fn pet_state_payload(runtime: &GameRuntime) -> PetStatePayload {
    let next_food_progress =
        (runtime.economy.banked_tokens_today / runtime.config.tokens_per_food).clamp(0.0, 1.0);

    PetStatePayload {
        fullness: runtime.economy.fullness,
        mood: mood_from_fullness(runtime.economy.fullness),
        xp: runtime.economy.xp,
        level: level_for_xp(runtime.economy.xp, &runtime.config),
        pending_food: runtime.economy.food_inventory,
        pantry: runtime.economy.pantry,
        food_earned_today: runtime.economy.food_earned_today,
        banked_tokens_today: runtime.economy.banked_tokens_today,
        tokens_per_food: runtime.config.tokens_per_food,
        meter_progress: next_food_progress,
    }
}

fn apply_token_event(
    app: &AppHandle,
    shared: &Arc<Mutex<GameRuntime>>,
    event: &TokenEvent,
) -> Result<(), String> {
    let mut runtime = shared
        .lock()
        .map_err(|_| "game runtime mutex poisoned".to_string())?;
    reconcile_and_persist(&mut runtime).map_err(|err| err.to_string())?;

    let inserted = runtime
        .ledger
        .record_event(event)
        .map_err(|err| err.to_string())?;
    if !inserted {
        return Ok(());
    }

    let config = runtime.config.clone();
    let outcome = runtime.economy.apply_token_event(event, &config);
    let now_unix = Local::now().timestamp();
    runtime
        .state_store
        .save_economy_state(&runtime.economy, now_unix)
        .map_err(|err| err.to_string())?;

    for index in 0..outcome.food_earned {
        let payload = FoodSpawnedPayload {
            id: format!("{}:{index}", event.message_id),
            pending_food: runtime.economy.food_inventory,
        };
        let _ = app.emit("food_spawned", payload);
    }

    let _ = app.emit("pet_state_changed", pet_state_payload(&runtime));
    Ok(())
}

fn start_claude_code_watcher(app: AppHandle, shared: Arc<Mutex<GameRuntime>>) {
    let provider = ClaudeCodeProvider::default();
    if !provider.detect() {
        return;
    }

    let (tx, rx) = mpsc::channel();
    if let Err(err) = provider.start(tx) {
        eprintln!("failed to start Claude Code watcher: {err}");
        return;
    }

    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            if let Err(err) = apply_token_event(&app, &shared, &event) {
                eprintln!("failed to apply token event {}: {err}", event.message_id);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config =
                economy::load_economy_config(app.handle()).expect("failed to load economy.toml");
            let (now_unix, today) = now_parts();
            let app_data_dir = app.path().app_data_dir()?;
            let db_path = app_data_dir.join("tokengochi.sqlite3");
            let ledger = Ledger::open(&db_path)?;
            let state_store = GameStateStore::open(&db_path)?;
            let mut economy = state_store
                .load_economy_state()?
                .unwrap_or_else(|| EconomyState::new(today, now_unix));
            economy.reconcile_elapsed_time(now_unix, today, &config);
            state_store.save_economy_state(&economy, now_unix)?;

            let shared = Arc::new(Mutex::new(GameRuntime {
                config: config.clone(),
                economy,
                ledger,
                state_store,
            }));

            app.manage(EconomyConfigState(Mutex::new(config)));
            app.manage(GameRuntimeState(shared.clone()));
            tray::setup(app.handle());
            overlay_window::fit_to_primary_monitor(app.handle());
            start_claude_code_watcher(app.handle().clone(), shared);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_config, get_pet_state, pet_ate])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
