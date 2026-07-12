use serde::{Deserialize, Serialize};
use tauri::Manager;

/// Balance constants tuned via `economy.toml`. Field names match
/// `docs/knowledge/game-economy.md` §8 (Balance Reference).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyConfig {
    /// Weighted tokens required to earn one Food, before caps/escalation.
    pub tokens_per_food: f64,
    /// Weight applied to output tokens.
    pub weight_output: f64,
    /// Weight applied to input tokens.
    pub weight_input: f64,
    /// Weight applied to cache-read tokens.
    pub weight_cache_read: f64,
    /// Food/day earned at full rate before diminishing returns kick in.
    pub daily_soft_cap: u32,
    /// Cost multiplier applied per Food beyond the soft cap (e.g. 1.5).
    pub soft_cap_escalation: f64,
    /// Absolute Food/day ceiling, regardless of tokens spent.
    pub daily_hard_cap: u32,
    /// Max Food banked in the Pantry for zero-usage days.
    pub pantry_max: u32,
    /// Fullness gained per Food eaten.
    pub fullness_per_food: u32,
    /// Fullness lost per 24h of real time.
    pub fullness_decay_per_24h: u32,
    /// Base XP gained per Food eaten (before mood multiplier).
    pub xp_per_food: u32,
    /// `XP(n) = xp_curve_base * n ^ xp_curve_exponent`.
    pub xp_curve_base: f64,
    pub xp_curve_exponent: f64,
}

/// Loads `economy.toml`, bundled as a Tauri resource (see `tauri.conf.json`
/// `bundle.resources`), from the app's resolved resource directory.
pub fn load_economy_config(app: &tauri::AppHandle) -> Result<EconomyConfig, String> {
    let path = app
        .path()
        .resolve("economy.toml", tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("could not resolve economy.toml path: {e}"))?;

    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;

    toml::from_str(&raw).map_err(|e| format!("could not parse economy.toml: {e}"))
}
