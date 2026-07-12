//! Mood bands (driven by Fullness) and their XP multiplier.
//!
//! See `docs/knowledge/game-economy.md` §3. Reuses [`crate::pet::Mood`]
//! rather than defining a second mood enum.

use crate::pet::Mood;

/// Fullness (0-100) -> discrete mood band. Thresholds per
/// `docs/knowledge/game-economy.md` §3: Full 75+, Content 40-74, Peckish
/// 15-39, Hungry <15.
pub fn mood_from_fullness(fullness: f64) -> Mood {
    if fullness >= 75.0 {
        Mood::Full
    } else if fullness >= 40.0 {
        Mood::Content
    } else if fullness >= 15.0 {
        Mood::Peckish
    } else {
        Mood::Hungry
    }
}

/// XP multiplier for a given mood. Per `docs/knowledge/game-economy.md` §3.
pub fn mood_multiplier(mood: Mood) -> f64 {
    match mood {
        Mood::Full => 1.2,
        Mood::Content => 1.0,
        Mood::Peckish => 0.8,
        Mood::Hungry => 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mood_thresholds_match_spec() {
        assert_eq!(mood_from_fullness(100.0), Mood::Full);
        assert_eq!(mood_from_fullness(75.0), Mood::Full);
        assert_eq!(mood_from_fullness(74.9), Mood::Content);
        assert_eq!(mood_from_fullness(40.0), Mood::Content);
        assert_eq!(mood_from_fullness(39.9), Mood::Peckish);
        assert_eq!(mood_from_fullness(15.0), Mood::Peckish);
        assert_eq!(mood_from_fullness(14.9), Mood::Hungry);
        assert_eq!(mood_from_fullness(0.0), Mood::Hungry);
    }

    #[test]
    fn mood_multipliers_match_spec() {
        assert_eq!(mood_multiplier(Mood::Full), 1.2);
        assert_eq!(mood_multiplier(Mood::Content), 1.0);
        assert_eq!(mood_multiplier(Mood::Peckish), 0.8);
        assert_eq!(mood_multiplier(Mood::Hungry), 0.5);
    }
}
