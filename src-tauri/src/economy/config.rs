use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tauri::Manager;

/// Balance constants tuned via `economy.toml`. Field names match
/// `docs/knowledge/game-economy.md` §8 (Balance Reference).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyConfig {
    /// Weighted tokens required to earn one Food. Flat rate, no caps -
    /// heavier usage always earns proportionally more Food.
    pub tokens_per_food: f64,
    /// Weight applied to output tokens.
    pub weight_output: f64,
    /// Weight applied to input tokens.
    pub weight_input: f64,
    /// Weight applied to cache-read tokens.
    pub weight_cache_read: f64,
    /// Per-model-tier token value multipliers, keyed by a case-insensitive
    /// *substring* of the model id (e.g. `"opus"` matches
    /// `"claude-opus-4-8"`). Checked in sorted key order; first match wins.
    /// `BTreeMap` (not `HashMap`) so that order is deterministic.
    pub model_weights: BTreeMap<String, f64>,
    /// Multiplier for models matching no `model_weights` key (and for
    /// events with no model recorded at all).
    pub model_weight_default: f64,
    /// Fullness gained per Food eaten.
    pub fullness_per_food: u32,
    /// Food/day the pet needs to hold fullness steady - decay is derived
    /// from it (`daily_food_need * fullness_per_food` fullness lost per
    /// 24h), so "how much must the pet eat per day" is tuned directly
    /// instead of implied by a separate decay constant.
    pub daily_food_need: f64,
    /// Base XP gained per Food eaten (before mood multiplier).
    pub xp_per_food: u32,
    /// `XP(n) = xp_curve_base * n ^ xp_curve_exponent`.
    pub xp_curve_base: f64,
    pub xp_curve_exponent: f64,
}

impl EconomyConfig {
    /// Fullness lost per 24h of real time, derived from the daily food
    /// need: a pet eating exactly `daily_food_need` Food/day holds steady.
    pub fn fullness_decay_per_24h(&self) -> f64 {
        self.daily_food_need * self.fullness_per_food as f64
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_economy_toml_parses_into_the_config_struct() {
        // Guards against the struct and the shipped file drifting apart
        // (e.g. a renamed field) - the failure would otherwise only show up
        // at app startup.
        let raw = include_str!("../../economy.toml");
        let config: EconomyConfig =
            toml::from_str(raw).expect("economy.toml must match EconomyConfig");
        assert!(config.tokens_per_food > 0.0);
        assert!(config.daily_food_need > 0.0);
        assert!(config.model_weight_default > 0.0);
        assert!(!config.model_weights.is_empty());
    }
}
