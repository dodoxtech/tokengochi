//! Mood bands (driven by Fullness) and their XP multiplier.
//!
//! See `docs/knowledge/game-economy.md` §3. Reuses [`crate::pet::Mood`]
//! rather than defining a second mood enum.

use crate::pet::Mood;

/// Fullness (0-100) -> discrete mood band. Thresholds per
/// `docs/knowledge/game-economy.md` §3: Full 75+, Content 40-74, Peckish
/// 15-39, Hungry 5-14, Starving <5 (hibernation - the deep-neglect state;
/// still no death, no level loss).
pub fn mood_from_fullness(fullness: f64) -> Mood {
    if fullness >= 75.0 {
        Mood::Full
    } else if fullness >= 40.0 {
        Mood::Content
    } else if fullness >= 15.0 {
        Mood::Peckish
    } else if fullness >= 5.0 {
        Mood::Hungry
    } else {
        Mood::Starving
    }
}

/// XP multiplier for a given mood. Per `docs/knowledge/game-economy.md` §3.
/// Starving is x0: a hibernating pet gains no XP at all (including from the
/// meal that wakes it - mood is evaluated *before* eating), which is the
/// whole penalty; nothing is ever taken away.
pub fn mood_multiplier(mood: Mood) -> f64 {
    match mood {
        Mood::Full => 1.2,
        Mood::Content => 1.0,
        Mood::Peckish => 0.8,
        Mood::Hungry => 0.5,
        Mood::Starving => 0.0,
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
        assert_eq!(mood_from_fullness(5.0), Mood::Hungry);
        assert_eq!(mood_from_fullness(4.9), Mood::Starving);
        assert_eq!(mood_from_fullness(0.0), Mood::Starving);
    }

    #[test]
    fn mood_multipliers_match_spec() {
        assert_eq!(mood_multiplier(Mood::Full), 1.2);
        assert_eq!(mood_multiplier(Mood::Content), 1.0);
        assert_eq!(mood_multiplier(Mood::Peckish), 0.8);
        assert_eq!(mood_multiplier(Mood::Hungry), 0.5);
        assert_eq!(mood_multiplier(Mood::Starving), 0.0);
    }
}
