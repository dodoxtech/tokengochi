//! Weighted token -> Food conversion, flat rate, no caps.
//!
//! See `docs/knowledge/game-economy.md` §2 and §8.

use super::EconomyConfig;
use crate::watcher::TokenEvent;

/// Per-model-tier token value multiplier. `config.model_weights` keys are
/// case-insensitive substrings matched against the event's model id
/// (e.g. `"opus"` matches `"claude-opus-4-8"`), checked in sorted key order
/// with the first match winning; unmatched (or empty) model ids fall back
/// to `config.model_weight_default`. See `docs/knowledge/game-economy.md` §2.
pub fn model_multiplier(model: &str, config: &EconomyConfig) -> f64 {
    let model_lower = model.to_lowercase();
    config
        .model_weights
        .iter()
        .find(|(pattern, _)| model_lower.contains(pattern.to_lowercase().as_str()))
        .map(|(_, weight)| *weight)
        .unwrap_or(config.model_weight_default)
}

/// Weighted "effort" tokens for one event - the common currency the rest of
/// the economy engine operates on. Output tokens are weighted higher than
/// input, cache-read lowest, then the whole event is scaled by its model's
/// tier multiplier ([`model_multiplier`]), per
/// `docs/knowledge/game-economy.md` §2.
pub fn weighted_tokens(event: &TokenEvent, config: &EconomyConfig) -> f64 {
    let base = event.input_tokens as f64 * config.weight_input
        + event.output_tokens as f64 * config.weight_output
        + event.cache_read_tokens as f64 * config.weight_cache_read;
    base * model_multiplier(&event.model, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> EconomyConfig {
        // Mirrors src-tauri/economy.toml exactly - if these constants ever
        // drift apart, that itself is a signal worth noticing.
        EconomyConfig {
            tokens_per_food: 20_000.0,
            weight_output: 1.0,
            weight_input: 0.25,
            weight_cache_read: 0.05,
            model_weights: [
                ("opus".to_string(), 2.0),
                ("sonnet".to_string(), 1.0),
                ("haiku".to_string(), 0.4),
            ]
            .into_iter()
            .collect(),
            model_weight_default: 1.0,
            fullness_per_food: 20,
            daily_food_need: 1.5,
            xp_per_food: 10,
            xp_curve_base: 50.0,
            xp_curve_exponent: 1.6,
        }
    }

    fn event(input: u64, output: u64, cache_read: u64) -> TokenEvent {
        event_for_model("claude-sonnet-5", input, output, cache_read)
    }

    fn event_for_model(model: &str, input: u64, output: u64, cache_read: u64) -> TokenEvent {
        TokenEvent {
            provider: "claude_code".to_string(),
            message_id: "msg".to_string(),
            model: model.to_string(),
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: cache_read,
            timestamp: 0,
        }
    }

    #[test]
    fn weighted_tokens_applies_configured_weights() {
        let config = test_config();
        // 100 output (x1.0) + 400 input (x0.25) + 1000 cache_read (x0.05)
        // = 100 + 100 + 50 = 250
        let e = event(400, 100, 1000);
        assert_eq!(weighted_tokens(&e, &config), 250.0);
    }

    #[test]
    fn model_multiplier_matches_tier_by_substring_case_insensitively() {
        let config = test_config();
        assert_eq!(model_multiplier("claude-opus-4-8", &config), 2.0);
        assert_eq!(model_multiplier("claude-sonnet-5", &config), 1.0);
        assert_eq!(model_multiplier("claude-haiku-4-5-20251001", &config), 0.4);
        assert_eq!(model_multiplier("Claude-OPUS-4-8", &config), 2.0);
    }

    #[test]
    fn unknown_or_missing_model_falls_back_to_default_weight() {
        let config = test_config();
        assert_eq!(
            model_multiplier("gpt-5o-mega", &config),
            config.model_weight_default
        );
        assert_eq!(model_multiplier("", &config), config.model_weight_default);
    }

    #[test]
    fn weighted_tokens_scales_by_model_tier() {
        let config = test_config();
        // Identical usage, different model tiers: Opus tokens are worth 2x
        // a Sonnet's, Haiku's 0.4x, per [model_weights] in economy.toml.
        let sonnet = weighted_tokens(&event_for_model("claude-sonnet-5", 400, 100, 1000), &config);
        let opus = weighted_tokens(&event_for_model("claude-opus-4-8", 400, 100, 1000), &config);
        let haiku = weighted_tokens(
            &event_for_model("claude-haiku-4-5", 400, 100, 1000),
            &config,
        );
        assert_eq!(sonnet, 250.0);
        assert_eq!(opus, 500.0);
        assert_eq!(haiku, 100.0);
    }
}
