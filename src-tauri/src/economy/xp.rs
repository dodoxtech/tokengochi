//! XP curve and level lookup. See `docs/knowledge/game-economy.md` §4, §8.

use super::EconomyConfig;

/// Levels are capped at 50 per `docs/knowledge/game-economy.md` §4
/// ("Levels 1-50 on a gentle exponential XP curve").
pub const MAX_LEVEL: u32 = 50;

/// Total XP required to *reach* `level` (level 0 requires 0 XP).
/// `XP(n) = xp_curve_base * n ^ xp_curve_exponent`.
pub fn xp_required_for_level(level: u32, config: &EconomyConfig) -> f64 {
    if level == 0 {
        return 0.0;
    }
    config.xp_curve_base * (level as f64).powf(config.xp_curve_exponent)
}

/// The highest level whose XP requirement `total_xp` meets or exceeds,
/// capped at [`MAX_LEVEL`].
pub fn level_for_xp(total_xp: f64, config: &EconomyConfig) -> u32 {
    let mut level = 0;
    while level < MAX_LEVEL && total_xp >= xp_required_for_level(level + 1, config) {
        level += 1;
    }
    level
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> EconomyConfig {
        EconomyConfig {
            tokens_per_food: 20_000.0,
            weight_output: 1.0,
            weight_input: 0.25,
            weight_cache_read: 0.05,
            model_weights: Default::default(),
            model_weight_default: 1.0,
            fullness_per_food: 20,
            daily_food_need: 1.5,
            xp_per_food: 10,
            xp_curve_base: 50.0,
            xp_curve_exponent: 1.6,
        }
    }

    #[test]
    fn level_zero_requires_no_xp() {
        let config = test_config();
        assert_eq!(xp_required_for_level(0, &config), 0.0);
        assert_eq!(level_for_xp(0.0, &config), 0);
    }

    #[test]
    fn level_up_thresholds_match_curve() {
        let config = test_config();
        let level_1_xp = xp_required_for_level(1, &config); // 50 * 1^1.6 = 50
        assert_eq!(level_1_xp, 50.0);
        assert_eq!(level_for_xp(level_1_xp - 1.0, &config), 0);
        assert_eq!(level_for_xp(level_1_xp, &config), 1);
    }

    #[test]
    fn level_never_exceeds_max_level() {
        let config = test_config();
        assert_eq!(level_for_xp(1_000_000_000.0, &config), MAX_LEVEL);
    }

    #[test]
    fn level_is_monotonic_in_xp() {
        let config = test_config();
        let mut last_level = 0;
        let mut xp = 0.0;
        while xp < 200_000.0 {
            let level = level_for_xp(xp, &config);
            assert!(level >= last_level, "level regressed at xp={xp}");
            last_level = level;
            xp += 137.0; // odd step size, not aligned to the curve on purpose
        }
    }
}
