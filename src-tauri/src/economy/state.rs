//! Ties conversion, fullness, and XP together into the day-boundary-aware
//! [`EconomyState`] a running app would persist.
//!
//! Pure logic - no I/O, no Tauri, no SQLite - so it's cheaply unit-testable.
//! Persisting this state and wiring it up to the watcher/ledger/UI is
//! follow-up work (tasks 0006+); this module defines the state shape and
//! the functions that mutate it correctly, per
//! `docs/tasks/active/0004-economy-engine-core.md`.
//!
//! Day boundaries are local-calendar-day based (`NaiveDate`, no time-of-day
//! or timezone inside this module - the caller resolves "what's today,
//! locally" via `chrono::Local` and passes in a plain date). Decay uses raw
//! elapsed unix-seconds, which is DST-immune by construction: a clock change
//! shifts wall-clock *labels*, not the number of seconds that actually
//! passed.

use super::conversion::{cost_of_nth_food, weighted_tokens};
use super::fullness::{mood_from_fullness, mood_multiplier};
use super::EconomyConfig;
use crate::watcher::TokenEvent;
use chrono::NaiveDate;

/// One Food-conversion outcome, returned by [`EconomyState::apply_token_event`]
/// for observability/testing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConversionOutcome {
    /// Whole Food added to today's inventory (subject to the daily hard cap).
    pub food_earned: u32,
    /// Whole Food that overflowed into the Pantry instead (hard cap already
    /// reached today, Pantry had room).
    pub food_to_pantry: u32,
    /// Weighted tokens discarded because both the hard cap and the Pantry
    /// were full - see `docs/knowledge/game-economy.md` §2/§7 ("token
    /// burning strictly irrational" past this point).
    pub tokens_wasted: f64,
}

/// The economy engine's mutable state for one pet.
#[derive(Debug, Clone, PartialEq)]
pub struct EconomyState {
    /// Local calendar day the daily counters below apply to.
    pub current_day: NaiveDate,
    pub food_earned_today: u32,
    /// Weighted tokens banked toward the next Food, at the current cost
    /// tier; resets to 0 at each day boundary (unspent tokens don't carry
    /// across days - only whole Food/Pantry stock does).
    pub banked_tokens_today: f64,
    /// Persists across days; capped at `config.pantry_max`.
    pub pantry: u32,
    /// Food earned but not yet eaten. Eating (fullness/XP effects) is
    /// triggered separately - see [`EconomyState::eat_from_inventory`].
    pub food_inventory: u32,
    pub fullness: f64,
    pub xp: f64,
    /// Unix seconds of the last time decay/day-rollover was reconciled.
    pub last_reconciled_unix: i64,
}

impl EconomyState {
    /// A fresh pet: full, zero XP, "today" pinned to `day`.
    pub fn new(day: NaiveDate, now_unix: i64) -> Self {
        Self {
            current_day: day,
            food_earned_today: 0,
            banked_tokens_today: 0.0,
            pantry: 0,
            food_inventory: 0,
            fullness: 100.0,
            xp: 0.0,
            last_reconciled_unix: now_unix,
        }
    }

    /// Applies one token usage event: converts weighted tokens to Food,
    /// respecting the soft/hard cap escalation and Pantry overflow. Callers
    /// should call [`EconomyState::reconcile_elapsed_time`] first if real
    /// time has passed, so `current_day` reflects the event's day.
    pub fn apply_token_event(
        &mut self,
        event: &TokenEvent,
        config: &EconomyConfig,
    ) -> ConversionOutcome {
        self.banked_tokens_today += weighted_tokens(event, config);

        let mut outcome = ConversionOutcome {
            food_earned: 0,
            food_to_pantry: 0,
            tokens_wasted: 0.0,
        };

        loop {
            let today_count_so_far = self.food_earned_today + outcome.food_earned;
            let under_hard_cap = today_count_so_far < config.daily_hard_cap;
            let pantry_has_room = self.pantry + outcome.food_to_pantry < config.pantry_max;

            // Check "is there anywhere for tokens to go" *before* checking
            // affordability. The escalating cost of the next food can
            // easily exceed whatever's left banked, which would otherwise
            // make the loop exit via the affordability check below without
            // ever noticing both the hard cap and the Pantry are already
            // full - silently leaving leftover tokens sitting in
            // `banked_tokens_today` instead of correctly discarding them
            // (docs/knowledge/game-economy.md §2/§7: once both are full,
            // nothing more can happen with today's tokens, regardless of
            // amount).
            if !under_hard_cap && !pantry_has_room {
                outcome.tokens_wasted += self.banked_tokens_today;
                self.banked_tokens_today = 0.0;
                break;
            }

            let next_food_index = today_count_so_far + outcome.food_to_pantry + 1;
            let cost = cost_of_nth_food(next_food_index, config);

            if self.banked_tokens_today < cost {
                // Not enough for the next food yet, but there's still a
                // valid destination for it - carry it over as progress
                // toward the next event (or tomorrow, once the day rolls).
                break;
            }

            self.banked_tokens_today -= cost;
            if under_hard_cap {
                outcome.food_earned += 1;
            } else {
                outcome.food_to_pantry += 1;
            }
        }

        self.food_earned_today += outcome.food_earned;
        self.pantry = (self.pantry + outcome.food_to_pantry).min(config.pantry_max);
        self.food_inventory += outcome.food_earned;

        outcome
    }

    /// Eats one Food from `food_inventory`, applying fullness/XP effects.
    /// Returns `true` if a Food was actually available and eaten. This is
    /// what a future `pet_ate` command (task 0006) will call.
    pub fn eat_from_inventory(&mut self, config: &EconomyConfig) -> bool {
        if self.food_inventory == 0 {
            return false;
        }
        self.food_inventory -= 1;
        self.eat_one_food(config);
        true
    }

    /// Call on app launch (and periodically while running) with the current
    /// wall-clock unix time and local calendar date. Applies fullness decay
    /// proportional to real elapsed seconds and rolls day boundaries
    /// (Pantry auto-feed) for any days that passed while the app was
    /// closed.
    pub fn reconcile_elapsed_time(
        &mut self,
        now_unix: i64,
        today: NaiveDate,
        config: &EconomyConfig,
    ) {
        let elapsed_secs = (now_unix - self.last_reconciled_unix).max(0) as f64;
        let decay = elapsed_secs / 86_400.0 * config.fullness_decay_per_24h();
        self.fullness = (self.fullness - decay).max(0.0);
        self.last_reconciled_unix = now_unix;

        self.roll_day_if_needed(today, config);
    }

    /// Advances `current_day` to `today`, one day at a time, applying
    /// Pantry auto-feed for each zero-usage day passed along the way. A
    /// no-op if `today` is not after `current_day`.
    fn roll_day_if_needed(&mut self, today: NaiveDate, config: &EconomyConfig) {
        if today <= self.current_day {
            return;
        }

        let days_to_advance = (today - self.current_day).num_days().max(0);
        for _ in 0..days_to_advance {
            if self.food_earned_today == 0 && self.pantry > 0 {
                self.auto_feed_from_pantry(config);
            }
            self.food_earned_today = 0;
            self.banked_tokens_today = 0.0;
        }

        self.current_day = today;
    }

    fn auto_feed_from_pantry(&mut self, config: &EconomyConfig) {
        if self.pantry == 0 {
            return;
        }
        self.pantry -= 1;
        self.eat_one_food(config);
    }

    /// Shared fullness/XP math for "the pet ate one Food," regardless of
    /// whether it came from `food_inventory` or an auto-feed from the
    /// Pantry. Mood is evaluated from fullness *before* this meal.
    fn eat_one_food(&mut self, config: &EconomyConfig) {
        let mood = mood_from_fullness(self.fullness);
        let xp_gain = config.xp_per_food as f64 * mood_multiplier(mood);
        self.xp += xp_gain;
        self.fullness = (self.fullness + config.fullness_per_food as f64).min(100.0);
    }
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
            model_weights: [
                ("opus".to_string(), 2.0),
                ("sonnet".to_string(), 1.0),
                ("haiku".to_string(), 0.4),
            ]
            .into_iter()
            .collect(),
            model_weight_default: 1.0,
            daily_soft_cap: 10,
            soft_cap_escalation: 1.5,
            daily_hard_cap: 20,
            pantry_max: 5,
            fullness_per_food: 20,
            daily_food_need: 1.5,
            xp_per_food: 10,
            xp_curve_base: 50.0,
            xp_curve_exponent: 1.6,
        }
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("valid test date")
    }

    fn huge_event(id: &str) -> TokenEvent {
        // 30,000,000 weighted tokens - well past the ~26.4M needed to reach
        // the hard cap (20 food) *and* fully fill the Pantry (5 more food)
        // given the geometric escalation, with some left over to be wasted.
        // (Escalation compounds fast: reaching the hard cap alone costs
        // ~3.6M weighted tokens; the 21st food alone costs ~1.7M more.)
        TokenEvent {
            provider: "claude_code".to_string(),
            message_id: id.to_string(),
            // Sonnet tier = x1.0 model multiplier, so the arithmetic in the
            // comments above stays as written.
            model: "claude-sonnet-5".to_string(),
            input_tokens: 0,
            output_tokens: 30_000_000,
            cache_read_tokens: 0,
            timestamp: 0,
        }
    }

    fn small_event(id: &str, weighted_output_tokens: u64) -> TokenEvent {
        TokenEvent {
            provider: "claude_code".to_string(),
            message_id: id.to_string(),
            model: "claude-sonnet-5".to_string(),
            input_tokens: 0,
            output_tokens: weighted_output_tokens,
            cache_read_tokens: 0,
            timestamp: 0,
        }
    }

    #[test]
    fn small_event_below_one_food_cost_just_banks_tokens() {
        let config = test_config();
        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        let outcome = state.apply_token_event(&small_event("m1", 5_000), &config);
        assert_eq!(outcome.food_earned, 0);
        assert_eq!(state.banked_tokens_today, 5_000.0);
        assert_eq!(state.food_inventory, 0);
    }

    #[test]
    fn tokens_accumulate_across_events_toward_one_food() {
        let config = test_config();
        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        state.apply_token_event(&small_event("m1", 12_000), &config);
        let outcome = state.apply_token_event(&small_event("m2", 8_000), &config);
        // 12,000 + 8,000 = 20,000 = exactly one food's cost.
        assert_eq!(outcome.food_earned, 1);
        assert_eq!(state.food_inventory, 1);
        assert_eq!(state.banked_tokens_today, 0.0);
    }

    #[test]
    fn hard_cap_stops_daily_food_and_overflow_goes_to_pantry_then_is_wasted() {
        let config = test_config();
        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        let outcome = state.apply_token_event(&huge_event("m1"), &config);

        assert_eq!(state.food_earned_today, config.daily_hard_cap);
        assert_eq!(outcome.food_earned, config.daily_hard_cap);
        assert_eq!(state.pantry, config.pantry_max);
        assert_eq!(outcome.food_to_pantry, config.pantry_max);
        // A 5,000,000-output-token event is far more than even the hard
        // cap + full pantry can absorb - something must be wasted.
        assert!(outcome.tokens_wasted > 0.0);
        // food_inventory only reflects the day's hard-cap-bound food, not
        // Pantry stock.
        assert_eq!(state.food_inventory, config.daily_hard_cap);
    }

    #[test]
    fn zero_usage_day_triggers_pantry_auto_feed() {
        let config = test_config();
        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        // Day 1: heavy usage, fills the pantry (and hits the hard cap, so
        // day 1 itself is *not* a zero-usage day - no auto-feed for it).
        state.apply_token_event(&huge_event("m1"), &config);
        assert_eq!(state.pantry, config.pantry_max);

        let xp_before = state.xp;

        // Jump straight from day 1 to day 3, with no events recorded on day
        // 2 in between - day 2 is therefore a zero-usage day, and rolling
        // past it should fire exactly one Pantry auto-feed.
        state.reconcile_elapsed_time(2 * 86_400, day(2026, 1, 3), &config);

        assert_eq!(
            state.pantry,
            config.pantry_max - 1,
            "exactly one auto-feed for day 2 (day 1 had usage, so it doesn't auto-feed)"
        );
        assert!(
            state.xp > xp_before,
            "auto-feed should grant XP same as any other meal"
        );
    }

    #[test]
    fn nonzero_usage_day_does_not_trigger_pantry_auto_feed() {
        let config = test_config();
        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        state.apply_token_event(&small_event("m1", 20_000), &config); // 1 food earned today
        state.pantry = 3; // pretend the pantry already has stock from a prior overflow

        state.reconcile_elapsed_time(86_400, day(2026, 1, 2), &config);

        // Day 1 had usage (1 food earned), so no auto-feed for day 1.
        assert_eq!(state.pantry, 3);
    }

    #[test]
    fn decay_across_multi_day_gap_is_proportional_and_floors_at_zero() {
        let config = test_config();
        let mut state = EconomyState::new(day(2026, 1, 1), 0);

        // 2 days closed, no pantry stock -> pure decay, no auto-feed noise.
        state.reconcile_elapsed_time(2 * 86_400, day(2026, 1, 3), &config);
        assert_eq!(
            state.fullness,
            100.0 - 2.0 * config.fullness_decay_per_24h()
        );

        // A much longer gap floors at 0 rather than going negative.
        let mut long_gap_state = EconomyState::new(day(2026, 1, 1), 0);
        long_gap_state.reconcile_elapsed_time(30 * 86_400, day(2026, 1, 31), &config);
        assert_eq!(long_gap_state.fullness, 0.0);
    }

    #[test]
    fn decay_depends_only_on_elapsed_seconds_not_calendar_dates() {
        // Simulates "closed over a DST transition": two scenarios with the
        // identical real elapsed time (25h) but different local dates
        // reported for "today" must decay identically, since decay is
        // computed from unix-second deltas, never from date arithmetic.
        let config = test_config();

        let mut state_a = EconomyState::new(day(2026, 3, 8), 1_000_000_000);
        state_a.reconcile_elapsed_time(1_000_000_000 + 25 * 3600, day(2026, 3, 9), &config);

        let mut state_b = EconomyState::new(day(2026, 11, 1), 1_000_000_000);
        state_b.reconcile_elapsed_time(1_000_000_000 + 25 * 3600, day(2026, 11, 2), &config);

        assert_eq!(state_a.fullness, state_b.fullness);
    }

    #[test]
    fn eating_from_inventory_applies_mood_multiplier_and_caps_fullness() {
        let config = test_config();
        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        state.fullness = 90.0; // Full mood -> x1.2
        state.food_inventory = 1;

        let ate = state.eat_from_inventory(&config);
        assert!(ate);
        assert_eq!(state.xp, config.xp_per_food as f64 * 1.2);
        assert_eq!(state.fullness, 100.0); // 90 + 20 = 110, capped at 100

        // No more food in inventory - second call is a no-op.
        assert!(!state.eat_from_inventory(&config));
    }

    #[test]
    fn decay_rate_is_derived_from_daily_food_need() {
        // daily_food_need = 1.5 and fullness_per_food = 20 -> the pet needs
        // 30 fullness/day, i.e. exactly daily_food_need Food/day to hold
        // steady - that's the "mỗi ngày cần ăn" contract.
        let config = test_config();
        assert_eq!(config.fullness_decay_per_24h(), 30.0);

        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        state.reconcile_elapsed_time(86_400, day(2026, 1, 2), &config);
        assert_eq!(state.fullness, 70.0);
    }

    #[test]
    fn starving_pet_hibernates_gaining_zero_xp_until_fed_out_of_it() {
        let config = test_config();
        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        state.fullness = 0.0; // deep neglect: Starving band (<5)
        state.food_inventory = 2;

        // First meal: mood evaluated before eating -> Starving -> x0 XP.
        // The meal still restores fullness (0 -> 20), waking the pet up.
        assert!(state.eat_from_inventory(&config));
        assert_eq!(
            state.xp, 0.0,
            "a hibernating pet gains no XP, even from the waking meal"
        );
        assert_eq!(state.fullness, 20.0);

        // Second meal: now Peckish (15-39) -> x0.8, XP flows again. The
        // pet never lost anything while starving - XP only ever goes up.
        assert!(state.eat_from_inventory(&config));
        assert_eq!(state.xp, config.xp_per_food as f64 * 0.8);
    }

    #[test]
    fn replaying_the_same_event_twice_at_the_state_layer_double_counts() {
        // EconomyState itself has no dedup - that's the ledger's job
        // (store::Ledger, keyed by message_id). This test documents that
        // boundary: callers MUST check the ledger before calling
        // apply_token_event, or replay will double-count here.
        let config = test_config();
        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        state.apply_token_event(&small_event("m1", 20_000), &config);
        state.apply_token_event(&small_event("m1", 20_000), &config);
        assert_eq!(
            state.food_inventory, 2,
            "EconomyState alone is not idempotent by design"
        );
    }
}
