mod economy;
mod overlay_window;
mod pet;
mod store;
mod tray;
mod watcher;
mod window_geometry;

use chrono::{Datelike, Duration, Local, TimeZone, Timelike};
use economy::{
    level_for_xp, mood_from_fullness, DailyQuestState, EconomyConfig, EconomyState, ShopError,
};
use pet::{
    EvolutionBranch, EvolutionEvent, EvolutionStage, FurniturePlacement, ShopItem,
    UsagePatternSample, SHOP_CATALOG,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use store::{AppSettings, FoodStats, GameStateStore, Ledger, TokenTotals};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use watcher::{ClaudeCodeProvider, CodexCliProvider, OpenAiProvider, TokenEvent, TokenProvider};

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
    settings: AppSettings,
    /// Unix timestamp of the last accepted `pet_petted` call. Not persisted:
    /// resetting the cooldown across app restarts is an acceptable tradeoff
    /// for a rate limit whose only purpose is deterring rapid-fire farming.
    last_petted_unix: i64,
}

struct GameRuntimeState(Arc<Mutex<GameRuntime>>);
struct TrackingState(Arc<AtomicBool>);
struct CalmModeState(Arc<AtomicBool>);

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PetStatePayload {
    fullness: f64,
    mood: pet::Mood,
    xp: f64,
    level: u32,
    evolution_stage: EvolutionStage,
    evolution_branch: EvolutionBranch,
    sparks: u32,
    streak_days: u32,
    streak_freezes: u32,
    daily_quest: DailyQuestState,
    weekly_food_earned: u32,
    weekly_target: u32,
    weekly_milestone_claimed: bool,
    album: Vec<String>,
    album_records: Vec<pet::AlbumRecord>,
    owned_items: Vec<String>,
    equipped_cosmetic: Option<String>,
    equipped_food_skin: Option<String>,
    furniture: Vec<FurniturePlacement>,
    prestige_count: u32,
    xp_bonus_multiplier: f64,
    pending_evolution: Option<EvolutionEvent>,
    pending_food: u32,
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

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderStatusPayload {
    claude_code_detected: bool,
    codex_cli_detected: bool,
    openai_key_configured: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsPayload {
    food: FoodStats,
    today_tokens: TokenTotals,
    week_tokens: TokenTotals,
    streak_days: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct OverlaySettingsPayload {
    pet_size: u32,
    calm_mode: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardPayload {
    pet: PetStatePayload,
    settings: AppSettings,
    providers: ProviderStatusPayload,
    stats: StatsPayload,
    monitor_count: u32,
    shop_catalog: &'static [ShopItem],
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
fn get_dashboard_state(
    app: AppHandle,
    state: tauri::State<GameRuntimeState>,
) -> Result<DashboardPayload, String> {
    let mut runtime = state
        .0
        .lock()
        .map_err(|_| "game runtime mutex poisoned".to_string())?;
    reconcile_and_persist(&mut runtime).map_err(|err| err.to_string())?;
    dashboard_payload(&app, &runtime)
}

#[tauri::command]
fn update_settings(
    app: AppHandle,
    settings: AppSettings,
    state: tauri::State<GameRuntimeState>,
    tracking: tauri::State<TrackingState>,
    calm_mode: tauri::State<CalmModeState>,
) -> Result<AppSettings, String> {
    let settings = validate_settings(settings)?;
    let mut runtime = state
        .0
        .lock()
        .map_err(|_| "game runtime mutex poisoned".to_string())?;
    runtime.settings = settings;
    runtime
        .state_store
        .save_app_settings(&runtime.settings, Local::now().timestamp())
        .map_err(|err| err.to_string())?;
    tracking
        .0
        .store(runtime.settings.tracking_paused, Ordering::SeqCst);
    calm_mode
        .0
        .store(runtime.settings.calm_mode, Ordering::SeqCst);
    apply_overlay_settings(&app, &runtime.settings);
    let _ = app.emit("settings_changed", runtime.settings.clone());
    Ok(runtime.settings.clone())
}

#[tauri::command]
fn complete_onboarding(
    app: AppHandle,
    starter_egg: String,
    state: tauri::State<GameRuntimeState>,
    tracking: tauri::State<TrackingState>,
) -> Result<AppSettings, String> {
    let mut runtime = state
        .0
        .lock()
        .map_err(|_| "game runtime mutex poisoned".to_string())?;
    runtime.settings.starter_egg = normalize_starter_egg(&starter_egg);
    runtime.settings.onboarding_complete = true;
    runtime
        .state_store
        .save_app_settings(&runtime.settings, Local::now().timestamp())
        .map_err(|err| err.to_string())?;
    tracking
        .0
        .store(runtime.settings.tracking_paused, Ordering::SeqCst);
    apply_overlay_settings(&app, &runtime.settings);
    let _ = app.emit("settings_changed", runtime.settings.clone());
    Ok(runtime.settings.clone())
}

#[tauri::command]
fn set_tracking_paused(
    paused: bool,
    state: tauri::State<GameRuntimeState>,
    tracking: tauri::State<TrackingState>,
) -> Result<(), String> {
    let mut runtime = state
        .0
        .lock()
        .map_err(|_| "game runtime mutex poisoned".to_string())?;
    runtime.settings.tracking_paused = paused;
    runtime
        .state_store
        .save_app_settings(&runtime.settings, Local::now().timestamp())
        .map_err(|err| err.to_string())?;
    tracking.0.store(paused, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn is_tracking_paused(state: tauri::State<TrackingState>) -> bool {
    state.0.load(Ordering::SeqCst)
}

#[tauri::command]
fn set_autostart(enabled: bool, app: AppHandle) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|err| err.to_string())?;
    manager.is_enabled().map_err(|err| err.to_string())
}

#[tauri::command]
fn set_openai_api_key(api_key: String) -> Result<(), String> {
    OpenAiProvider::set_api_key(api_key.trim()).map_err(|err| err.to_string())
}

#[tauri::command]
fn clear_openai_api_key() -> Result<(), String> {
    OpenAiProvider::clear_api_key().map_err(|err| err.to_string())
}

#[tauri::command]
fn is_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|err| err.to_string())
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

/// Petting is rate-limited server-side (task 0012) so a player can't farm
/// happiness by hovering forever; the overlay debounces on its own ~1s
/// stroke gesture, but this is the authoritative guard.
const PET_BUMP_COOLDOWN_SECS: i64 = 60;

#[tauri::command]
fn pet_petted(
    app: AppHandle,
    state: tauri::State<GameRuntimeState>,
) -> Result<PetStatePayload, String> {
    let mut runtime = state
        .0
        .lock()
        .map_err(|_| "game runtime mutex poisoned".to_string())?;
    reconcile_and_persist(&mut runtime).map_err(|err| err.to_string())?;

    let now_unix = Local::now().timestamp();
    if now_unix - runtime.last_petted_unix >= PET_BUMP_COOLDOWN_SECS {
        runtime.economy.pet_bump();
        runtime.last_petted_unix = now_unix;
        runtime
            .state_store
            .save_economy_state(&runtime.economy, now_unix)
            .map_err(|err| err.to_string())?;
    }

    let payload = pet_state_payload(&runtime);
    let _ = app.emit("pet_state_changed", payload.clone());
    Ok(payload)
}

#[tauri::command]
fn buy_shop_item(
    app: AppHandle,
    item_id: String,
    state: tauri::State<GameRuntimeState>,
) -> Result<PetStatePayload, String> {
    mutate_pet_state(app, state, |runtime| {
        runtime
            .economy
            .buy_item(&item_id)
            .map_err(shop_error_message)
    })
}

#[tauri::command]
fn equip_shop_item(
    app: AppHandle,
    item_id: String,
    state: tauri::State<GameRuntimeState>,
) -> Result<PetStatePayload, String> {
    mutate_pet_state(app, state, |runtime| {
        runtime
            .economy
            .equip_item(&item_id)
            .map_err(shop_error_message)
    })
}

#[tauri::command]
fn place_furniture(
    app: AppHandle,
    item_id: String,
    x: f64,
    state: tauri::State<GameRuntimeState>,
) -> Result<PetStatePayload, String> {
    mutate_pet_state(app, state, |runtime| {
        runtime
            .economy
            .place_furniture(&item_id, x)
            .map_err(shop_error_message)
    })
}

#[tauri::command]
fn prestige_pet(
    app: AppHandle,
    state: tauri::State<GameRuntimeState>,
) -> Result<PetStatePayload, String> {
    mutate_pet_state(app, state, |runtime| {
        runtime
            .economy
            .prestige(Local::now().date_naive())
            .map_err(shop_error_message)
    })
}

fn mutate_pet_state<F>(
    app: AppHandle,
    state: tauri::State<GameRuntimeState>,
    mutate: F,
) -> Result<PetStatePayload, String>
where
    F: FnOnce(&mut GameRuntime) -> Result<(), String>,
{
    let mut runtime = state
        .0
        .lock()
        .map_err(|_| "game runtime mutex poisoned".to_string())?;
    reconcile_and_persist(&mut runtime).map_err(|err| err.to_string())?;
    mutate(&mut runtime)?;
    let now_unix = Local::now().timestamp();
    runtime
        .state_store
        .save_economy_state(&runtime.economy, now_unix)
        .map_err(|err| err.to_string())?;
    let payload = pet_state_payload(&runtime);
    let _ = app.emit("pet_state_changed", payload.clone());
    Ok(payload)
}

fn shop_error_message(error: ShopError) -> String {
    match error {
        ShopError::UnknownItem => "unknown shop item".to_string(),
        ShopError::AlreadyOwned => "item already owned".to_string(),
        ShopError::NotEnoughSparks => "not enough Sparks".to_string(),
        ShopError::NotOwned => "item is not owned".to_string(),
        ShopError::WrongItemKind => "item cannot be used that way".to_string(),
        ShopError::PrestigeRequiresElder => "prestige requires an Elder pet".to_string(),
    }
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
        evolution_stage: runtime.economy.evolution_stage,
        evolution_branch: runtime.economy.evolution_branch,
        sparks: runtime.economy.sparks,
        streak_days: runtime.economy.streak_days,
        streak_freezes: runtime.economy.streak_freezes,
        daily_quest: runtime.economy.daily_quest.clone(),
        weekly_food_earned: runtime.economy.weekly_food_earned,
        weekly_target: runtime.economy.weekly_target,
        weekly_milestone_claimed: runtime.economy.weekly_milestone_claimed,
        album: runtime.economy.album.clone(),
        album_records: runtime.economy.album_records.clone(),
        owned_items: runtime.economy.owned_items.clone(),
        equipped_cosmetic: runtime.economy.equipped_cosmetic.clone(),
        equipped_food_skin: runtime.economy.equipped_food_skin.clone(),
        furniture: runtime.economy.furniture.clone(),
        prestige_count: runtime.economy.prestige_count,
        xp_bonus_multiplier: runtime.economy.xp_bonus_multiplier,
        pending_evolution: runtime.economy.pending_evolution.clone(),
        pending_food: runtime.economy.food_inventory,
        food_earned_today: runtime.economy.food_earned_today,
        banked_tokens_today: runtime.economy.banked_tokens_today,
        tokens_per_food: runtime.config.tokens_per_food,
        meter_progress: next_food_progress,
    }
}

fn dashboard_payload(app: &AppHandle, runtime: &GameRuntime) -> Result<DashboardPayload, String> {
    let today = Local::now().date_naive();
    let tomorrow = today + Duration::days(1);
    let week_start = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let today_start_unix = local_midnight_unix(today)?;
    let tomorrow_start_unix = local_midnight_unix(tomorrow)?;
    let week_start_unix = local_midnight_unix(week_start)?;
    let mut food = runtime
        .state_store
        .food_stats_since(today, week_start)
        .map_err(|err| err.to_string())?;
    food.today = food.today.max(runtime.economy.food_earned_today);
    food.week = food.week.max(food.today);
    Ok(DashboardPayload {
        pet: pet_state_payload(runtime),
        settings: runtime.settings.clone(),
        providers: ProviderStatusPayload {
            claude_code_detected: ClaudeCodeProvider::default().detect(),
            codex_cli_detected: CodexCliProvider::default().detect(),
            openai_key_configured: OpenAiProvider::has_api_key(),
        },
        stats: StatsPayload {
            food,
            today_tokens: runtime
                .ledger
                .token_totals_between(today_start_unix, tomorrow_start_unix)
                .map_err(|err| err.to_string())?,
            week_tokens: runtime
                .ledger
                .token_totals_between(week_start_unix, tomorrow_start_unix)
                .map_err(|err| err.to_string())?,
            streak_days: runtime.economy.streak_days,
        },
        monitor_count: available_monitor_count(app),
        shop_catalog: SHOP_CATALOG,
    })
}

fn local_midnight_unix(day: chrono::NaiveDate) -> Result<i64, String> {
    let naive = day
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| "invalid local date".to_string())?;
    let local = Local
        .from_local_datetime(&naive)
        .earliest()
        .or_else(|| Local.from_local_datetime(&naive).latest())
        .ok_or_else(|| "local midnight is unavailable".to_string())?;
    Ok(local.timestamp())
}

fn validate_settings(mut settings: AppSettings) -> Result<AppSettings, String> {
    settings.pet_size = settings.pet_size.clamp(70, 160);
    settings.starter_egg = normalize_starter_egg(&settings.starter_egg);
    Ok(settings)
}

fn normalize_starter_egg(value: &str) -> String {
    match value {
        "ember" | "sprout" | "bubble" => value.to_string(),
        _ => "sprout".to_string(),
    }
}

fn available_monitor_count(app: &AppHandle) -> u32 {
    app.get_webview_window("overlay")
        .and_then(|window| window.available_monitors().ok())
        .map(|monitors| monitors.len() as u32)
        .filter(|count| *count > 0)
        .unwrap_or(1)
}

fn apply_overlay_settings(app: &AppHandle, settings: &AppSettings) {
    overlay_window::fit_to_monitor(app, settings.monitor_index, settings.wayland_fallback);
    let _ = app.emit(
        "overlay_settings_changed",
        OverlaySettingsPayload {
            pet_size: settings.pet_size,
            calm_mode: settings.calm_mode,
        },
    );
}

/// Polls the platform window-geometry provider at a low, fixed rate (2 Hz,
/// per the 0012 CPU budget) and pushes the segment list to the overlay.
/// Skips work entirely while calm mode is on, since climbing is disabled
/// and there's no reason to pay the enumeration cost.
fn start_window_geometry_watcher(app: AppHandle, calm_mode: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if calm_mode.load(Ordering::SeqCst) {
            continue;
        }
        let segments = window_geometry::enumerate_windows();
        let _ = app.emit("window_segments_changed", segments);
    });
}

fn apply_token_event(
    app: &AppHandle,
    shared: &Arc<Mutex<GameRuntime>>,
    tracking_paused: &Arc<AtomicBool>,
    event: &TokenEvent,
) -> Result<(), String> {
    if tracking_paused.load(Ordering::SeqCst) {
        return Ok(());
    }
    let mut runtime = shared
        .lock()
        .map_err(|_| "game runtime mutex poisoned".to_string())?;
    if !provider_enabled(&runtime.settings, &event.provider) {
        return Ok(());
    }
    reconcile_and_persist(&mut runtime).map_err(|err| err.to_string())?;

    let inserted = runtime
        .ledger
        .record_event(event)
        .map_err(|err| err.to_string())?;
    if !inserted {
        return Ok(());
    }

    let config = runtime.config.clone();
    let event_day = Local
        .timestamp_opt(event.timestamp, 0)
        .single()
        .map(|dt| dt.date_naive())
        .unwrap_or_else(|| Local::now().date_naive());
    runtime.economy.record_provider_usage_on_day(
        &event.provider,
        usage_sample_from_event(event),
        event_day,
    );
    let outcome = runtime
        .economy
        .apply_token_event_on_day(event, event_day, &config);
    let now_unix = Local::now().timestamp();
    runtime
        .state_store
        .save_economy_state(&runtime.economy, now_unix)
        .map_err(|err| err.to_string())?;
    runtime
        .state_store
        .increment_daily_food(event_day, outcome.food_earned)
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

fn provider_enabled(settings: &AppSettings, provider: &str) -> bool {
    match provider {
        "claude_code" => settings.claude_code_enabled,
        "codex_cli" => settings.codex_cli_enabled,
        "openai" => settings.openai_enabled,
        "manual" => true,
        _ => false,
    }
}

fn usage_sample_from_event(event: &TokenEvent) -> UsagePatternSample {
    let hour = Local
        .timestamp_opt(event.timestamp, 0)
        .single()
        .map(|dt| dt.hour())
        .unwrap_or_else(|| Local::now().hour());
    UsagePatternSample::single_event(hour, 1)
}

fn start_claude_code_watcher(
    app: AppHandle,
    shared: Arc<Mutex<GameRuntime>>,
    tracking_paused: Arc<AtomicBool>,
) {
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
            if let Err(err) = apply_token_event(&app, &shared, &tracking_paused, &event) {
                eprintln!("failed to apply token event {}: {err}", event.message_id);
            }
        }
    });
}

fn start_codex_cli_watcher(
    app: AppHandle,
    shared: Arc<Mutex<GameRuntime>>,
    tracking_paused: Arc<AtomicBool>,
) {
    let provider = CodexCliProvider::default();
    if !provider.detect() {
        return;
    }
    let (tx, rx) = mpsc::channel();
    if let Err(err) = provider.start(tx) {
        eprintln!("failed to start Codex CLI watcher: {err}");
        return;
    }
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            if let Err(err) = apply_token_event(&app, &shared, &tracking_paused, &event) {
                eprintln!("failed to apply token event {}: {err}", event.message_id);
            }
        }
    });
}

fn start_openai_watcher(
    app: AppHandle,
    shared: Arc<Mutex<GameRuntime>>,
    tracking_paused: Arc<AtomicBool>,
) {
    let provider = OpenAiProvider::default();
    if !provider.detect() {
        return;
    }
    let (tx, rx) = mpsc::channel();
    if let Err(err) = provider.start(tx) {
        eprintln!("failed to start OpenAI usage watcher: {err}");
        return;
    }
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            if let Err(err) = apply_token_event(&app, &shared, &tracking_paused, &event) {
                eprintln!("failed to apply token event {}: {err}", event.message_id);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_dashboard(app)
        }))
        .setup(|app| {
            let config =
                economy::load_economy_config(app.handle()).expect("failed to load economy.toml");
            let (now_unix, today) = now_parts();
            let app_data_dir = app.path().app_data_dir()?;
            let db_path = app_data_dir.join("tokengochi.sqlite3");
            let ledger = Ledger::open(&db_path)?;
            let state_store = GameStateStore::open(&db_path)?;
            let settings = validate_settings(state_store.load_app_settings()?)?;
            let mut economy = state_store
                .load_economy_state()?
                .unwrap_or_else(|| EconomyState::new(today, now_unix));
            economy.reconcile_elapsed_time(now_unix, today, &config);
            state_store.save_economy_state(&economy, now_unix)?;
            state_store.save_app_settings(&settings, now_unix)?;

            let shared = Arc::new(Mutex::new(GameRuntime {
                config: config.clone(),
                economy,
                ledger,
                state_store,
                settings: settings.clone(),
                last_petted_unix: 0,
            }));
            let tracking_paused = Arc::new(AtomicBool::new(settings.tracking_paused));
            let calm_mode = Arc::new(AtomicBool::new(settings.calm_mode));

            app.manage(EconomyConfigState(Mutex::new(config)));
            app.manage(GameRuntimeState(shared.clone()));
            app.manage(TrackingState(tracking_paused.clone()));
            app.manage(CalmModeState(calm_mode.clone()));
            start_window_geometry_watcher(app.handle().clone(), calm_mode);
            let settings_shared = shared.clone();
            let persist_tracking_change = Arc::new(move |paused: bool| {
                let Ok(mut runtime) = settings_shared.lock() else {
                    eprintln!("failed to persist tracking pause: game runtime mutex poisoned");
                    return;
                };
                runtime.settings.tracking_paused = paused;
                if let Err(err) = runtime
                    .state_store
                    .save_app_settings(&runtime.settings, Local::now().timestamp())
                {
                    eprintln!("failed to persist tracking pause: {err}");
                }
            });
            tray::setup(
                app.handle(),
                tracking_paused.clone(),
                persist_tracking_change,
            )?;
            apply_overlay_settings(app.handle(), &settings);
            start_claude_code_watcher(
                app.handle().clone(),
                shared.clone(),
                tracking_paused.clone(),
            );
            start_codex_cli_watcher(
                app.handle().clone(),
                shared.clone(),
                tracking_paused.clone(),
            );
            start_openai_watcher(app.handle().clone(), shared, tracking_paused);
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_pet_state,
            get_dashboard_state,
            pet_ate,
            pet_petted,
            buy_shop_item,
            equip_shop_item,
            place_furniture,
            prestige_pet,
            update_settings,
            complete_onboarding,
            set_tracking_paused,
            is_tracking_paused,
            set_autostart,
            is_autostart_enabled,
            set_openai_api_key,
            clear_openai_api_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
