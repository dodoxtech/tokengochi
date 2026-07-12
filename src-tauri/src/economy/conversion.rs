//! Weighted token -> Food conversion with daily soft/hard cap escalation.
//!
//! See `docs/knowledge/game-economy.md` §2 and §8.

use super::EconomyConfig;
use crate::watcher::TokenEvent;

/// Weighted "effort" tokens for one event - the common currency the rest of
/// the economy engine operates on. Output tokens are weighted higher than
/// input, cache-read lowest, per `docs/knowledge/game-economy.md` §2.
pub fn weighted_tokens(event: &TokenEvent, config: &EconomyConfig) -> f64 {
    event.input_tokens as f64 * config.weight_input
        + event.output_tokens as f64 * config.weight_output
        + event.cache_read_tokens as f64 * config.weight_cache_read
}

/// Cost, in weighted tokens, of earning the `n`th Food of the day
/// (1-indexed, continuing past the hard cap - see `state.rs` for what a
/// Food earned past the hard cap actually does with it).
///
/// Flat at `tokens_per_food` for the first `daily_soft_cap` Food, then
/// escalates geometrically by `soft_cap_escalation` per Food beyond that -
/// e.g. with the reference constants (`tokens_per_food` = 20 000,
/// `soft_cap_escalation` = 1.5), food #11 costs 30 000, #12 costs 45 000 -
/// the same 1x / 1.5x / 2.25x progression as the illustrative 10 -> 15 ->
/// 22.5 example in `docs/knowledge/game-economy.md` §2, just at the real
/// token scale rather than the doc's simplified round numbers.
pub fn cost_of_nth_food(n: u32, config: &EconomyConfig) -> f64 {
    if n <= config.daily_soft_cap {
        config.tokens_per_food
    } else {
        let steps_beyond_cap = (n - config.daily_soft_cap) as i32;
        config.tokens_per_food * config.soft_cap_escalation.powi(steps_beyond_cap)
    }
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
            daily_soft_cap: 10,
            soft_cap_escalation: 1.5,
            daily_hard_cap: 20,
            pantry_max: 5,
            fullness_per_food: 20,
            fullness_decay_per_24h: 25,
            xp_per_food: 10,
            xp_curve_base: 50.0,
            xp_curve_exponent: 1.6,
        }
    }

    fn event(input: u64, output: u64, cache_read: u64) -> TokenEvent {
        TokenEvent {
            provider: "claude_code".to_string(),
            message_id: "msg".to_string(),
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
    fn cost_flat_at_soft_cap_and_below() {
        let config = test_config();
        for n in 1..=10 {
            assert_eq!(cost_of_nth_food(n, &config), 20_000.0, "n={n}");
        }
    }

    #[test]
    fn cost_escalates_geometrically_beyond_soft_cap() {
        let config = test_config();
        assert_eq!(cost_of_nth_food(11, &config), 30_000.0);
        assert_eq!(cost_of_nth_food(12, &config), 45_000.0);
        assert_eq!(cost_of_nth_food(13, &config), 67_500.0);
    }

    #[test]
    fn changing_a_constant_changes_behavior_with_no_code_change() {
        let mut config = test_config();
        let base_cost = cost_of_nth_food(1, &config);
        config.tokens_per_food = 40_000.0; // e.g. a balance-patch release
        let new_cost = cost_of_nth_food(1, &config);
        assert_eq!(new_cost, base_cost * 2.0);
    }
}
