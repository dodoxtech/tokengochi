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
use super::{level_for_xp, EconomyConfig};
use crate::pet::{
    album_key, shop_item, stage_for_level, AlbumRecord, EvolutionBranch, EvolutionEvent,
    EvolutionStage, FurniturePlacement, ShopItemKind, UsagePatternSample, UsagePatternStats,
};
use crate::watcher::TokenEvent;
use chrono::{Datelike, NaiveDate};
use std::collections::BTreeMap;

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
    pub food_earned_by_day: BTreeMap<String, u32>,
    /// Weighted tokens banked toward the next Food, at the current cost
    /// tier; resets to 0 at each day boundary (unspent tokens don't carry
    /// across days - only whole Food/Pantry stock does).
    pub banked_tokens_today: f64,
    pub banked_tokens_by_day: BTreeMap<String, f64>,
    /// Persists across days; capped at `config.pantry_max`.
    pub pantry: u32,
    /// Food earned but not yet eaten. Eating (fullness/XP effects) is
    /// triggered separately - see [`EconomyState::eat_from_inventory`].
    pub food_inventory: u32,
    pub fullness: f64,
    pub xp: f64,
    pub sparks: u32,
    pub streak_days: u32,
    pub streak_freezes: u32,
    pub last_activity_day: Option<NaiveDate>,
    pub weekly_food_earned: u32,
    pub weekly_target: u32,
    pub weekly_milestone_claimed: bool,
    pub daily_quest: DailyQuestState,
    pub usage_stats: UsagePatternStats,
    pub providers_by_day: BTreeMap<String, Vec<String>>,
    pub evolution_stage: EvolutionStage,
    pub evolution_branch: EvolutionBranch,
    pub album: Vec<String>,
    pub album_records: Vec<AlbumRecord>,
    pub owned_items: Vec<String>,
    pub equipped_cosmetic: Option<String>,
    pub equipped_food_skin: Option<String>,
    pub furniture: Vec<FurniturePlacement>,
    pub prestige_count: u32,
    pub xp_bonus_multiplier: f64,
    pub pending_evolution: Option<EvolutionEvent>,
    /// Unix seconds of the last time decay/day-rollover was reconciled.
    pub last_reconciled_unix: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DailyQuestKind {
    EarnOneFood,
    EarnThreeFood,
    UseAtNight,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyQuestState {
    pub day: NaiveDate,
    pub kind: DailyQuestKind,
    pub progress: u32,
    pub target: u32,
    pub reward_sparks: u32,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShopError {
    UnknownItem,
    AlreadyOwned,
    NotEnoughSparks,
    NotOwned,
    WrongItemKind,
    PrestigeRequiresElder,
}

impl DailyQuestState {
    pub fn for_day(day: NaiveDate) -> Self {
        match day.num_days_from_ce().rem_euclid(3) {
            0 => Self {
                day,
                kind: DailyQuestKind::EarnOneFood,
                progress: 0,
                target: 1,
                reward_sparks: 1,
                completed: false,
            },
            1 => Self {
                day,
                kind: DailyQuestKind::EarnThreeFood,
                progress: 0,
                target: 3,
                reward_sparks: 2,
                completed: false,
            },
            _ => Self {
                day,
                kind: DailyQuestKind::UseAtNight,
                progress: 0,
                target: 1,
                reward_sparks: 1,
                completed: false,
            },
        }
    }
}

impl EconomyState {
    /// A fresh pet: full, zero XP, "today" pinned to `day`.
    pub fn new(day: NaiveDate, now_unix: i64) -> Self {
        Self {
            current_day: day,
            food_earned_today: 0,
            food_earned_by_day: BTreeMap::new(),
            banked_tokens_today: 0.0,
            banked_tokens_by_day: BTreeMap::new(),
            pantry: 0,
            food_inventory: 0,
            fullness: 100.0,
            xp: 0.0,
            sparks: 0,
            streak_days: 0,
            streak_freezes: 0,
            last_activity_day: None,
            weekly_food_earned: 0,
            weekly_target: 7,
            weekly_milestone_claimed: false,
            daily_quest: DailyQuestState::for_day(day),
            usage_stats: UsagePatternStats::default(),
            providers_by_day: BTreeMap::new(),
            evolution_stage: EvolutionStage::Egg,
            evolution_branch: EvolutionBranch::Sprout,
            album: vec![album_key(EvolutionStage::Egg, EvolutionBranch::Sprout, 0)],
            album_records: vec![AlbumRecord {
                key: album_key(EvolutionStage::Egg, EvolutionBranch::Sprout, 0),
                stage: EvolutionStage::Egg,
                branch: EvolutionBranch::Sprout,
                reached_day: day.to_string(),
                level: 0,
                xp: 0.0,
                sparks: 0,
                prestige_count: 0,
            }],
            owned_items: Vec::new(),
            equipped_cosmetic: None,
            equipped_food_skin: None,
            furniture: Vec::new(),
            prestige_count: 0,
            xp_bonus_multiplier: 1.0,
            pending_evolution: None,
            last_reconciled_unix: now_unix,
        }
    }

    pub fn buy_item(&mut self, item_id: &str) -> Result<(), ShopError> {
        let item = shop_item(item_id).ok_or(ShopError::UnknownItem)?;
        if self.owned_items.iter().any(|owned| owned == item.id) {
            return Err(ShopError::AlreadyOwned);
        }
        if self.sparks < item.price_sparks {
            return Err(ShopError::NotEnoughSparks);
        }
        self.sparks -= item.price_sparks;
        self.owned_items.push(item.id.to_string());
        if item.kind == ShopItemKind::Furniture {
            self.place_furniture(item.id, default_furniture_x(item.id))?;
        }
        Ok(())
    }

    pub fn equip_item(&mut self, item_id: &str) -> Result<(), ShopError> {
        let item = shop_item(item_id).ok_or(ShopError::UnknownItem)?;
        if !self.owned_items.iter().any(|owned| owned == item.id) {
            return Err(ShopError::NotOwned);
        }

        match item.kind {
            ShopItemKind::Cosmetic | ShopItemKind::Heirloom => {
                self.equipped_cosmetic = Some(item.id.to_string());
                Ok(())
            }
            ShopItemKind::FoodSkin => {
                self.equipped_food_skin = Some(item.id.to_string());
                Ok(())
            }
            ShopItemKind::Furniture => Err(ShopError::WrongItemKind),
        }
    }

    pub fn place_furniture(&mut self, item_id: &str, x: f64) -> Result<(), ShopError> {
        let item = shop_item(item_id).ok_or(ShopError::UnknownItem)?;
        if item.kind != ShopItemKind::Furniture {
            return Err(ShopError::WrongItemKind);
        }
        if !self.owned_items.iter().any(|owned| owned == item.id) {
            return Err(ShopError::NotOwned);
        }
        if let Some(placement) = self
            .furniture
            .iter_mut()
            .find(|placement| placement.item_id == item.id)
        {
            placement.x = x.clamp(0.05, 0.95);
        } else {
            self.furniture.push(FurniturePlacement {
                item_id: item.id.to_string(),
                x: x.clamp(0.05, 0.95),
            });
        }
        Ok(())
    }

    pub fn prestige(&mut self, today: NaiveDate) -> Result<(), ShopError> {
        if self.evolution_stage != EvolutionStage::Elder {
            return Err(ShopError::PrestigeRequiresElder);
        }

        let retained_sparks = self.sparks;
        let retained_owned_items = self.owned_items.clone();
        let retained_furniture = self.furniture.clone();
        let retained_album = self.album.clone();
        let retained_album_records = self.album_records.clone();
        let next_prestige = self.prestige_count + 1;
        let next_xp_bonus = 1.0 + (next_prestige as f64 * 0.10);

        *self = Self::new(today, self.last_reconciled_unix);
        self.sparks = retained_sparks;
        self.owned_items = retained_owned_items;
        if !self
            .owned_items
            .iter()
            .any(|owned| owned == "halo-heirloom")
        {
            self.owned_items.push("halo-heirloom".to_string());
        }
        self.equipped_cosmetic = Some("halo-heirloom".to_string());
        self.furniture = retained_furniture;
        self.album = retained_album;
        self.album_records = retained_album_records;
        self.prestige_count = next_prestige;
        self.xp_bonus_multiplier = next_xp_bonus;
        self.record_album_entry(0);
        Ok(())
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
        self.apply_token_event_on_day(event, self.current_day, config)
    }

    pub fn apply_token_event_on_day(
        &mut self,
        event: &TokenEvent,
        event_day: NaiveDate,
        config: &EconomyConfig,
    ) -> ConversionOutcome {
        let day_key = event_day.to_string();
        let mut banked_tokens = if event_day == self.current_day {
            self.banked_tokens_today
        } else {
            self.banked_tokens_by_day
                .get(&day_key)
                .copied()
                .unwrap_or_default()
        };
        let mut food_earned_for_day = if event_day == self.current_day {
            self.food_earned_today
        } else {
            self.food_earned_by_day
                .get(&day_key)
                .copied()
                .unwrap_or_default()
        };

        banked_tokens += weighted_tokens(event, config);

        let mut outcome = ConversionOutcome {
            food_earned: 0,
            food_to_pantry: 0,
            tokens_wasted: 0.0,
        };

        loop {
            let today_count_so_far = food_earned_for_day + outcome.food_earned;
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
                outcome.tokens_wasted += banked_tokens;
                banked_tokens = 0.0;
                break;
            }

            let next_food_index = today_count_so_far + outcome.food_to_pantry + 1;
            let cost = cost_of_nth_food(next_food_index, config);

            if banked_tokens < cost {
                // Not enough for the next food yet, but there's still a
                // valid destination for it - carry it over as progress
                // toward the next event (or tomorrow, once the day rolls).
                break;
            }

            banked_tokens -= cost;
            if under_hard_cap {
                outcome.food_earned += 1;
            } else {
                outcome.food_to_pantry += 1;
            }
        }

        food_earned_for_day += outcome.food_earned;
        if event_day == self.current_day {
            self.food_earned_today = food_earned_for_day;
            self.banked_tokens_today = banked_tokens;
        } else {
            self.food_earned_by_day
                .insert(day_key.clone(), food_earned_for_day);
            self.banked_tokens_by_day.insert(day_key, banked_tokens);
        }
        self.pantry = (self.pantry + outcome.food_to_pantry).min(config.pantry_max);
        self.food_inventory += outcome.food_earned;
        if outcome.food_earned > 0 {
            if event_day >= self.last_activity_day.unwrap_or(event_day) {
                self.record_activity_day(event_day);
            }
            if event_day == self.current_day {
                self.weekly_food_earned += outcome.food_earned;
                self.advance_daily_quest_for_food(outcome.food_earned);
                self.check_weekly_milestone();
            }
        }

        outcome
    }

    pub fn record_usage_pattern(&mut self, sample: UsagePatternSample) {
        self.usage_stats.record_day(sample);
        if sample.night_events > 0 {
            self.advance_daily_quest_for_night_usage();
        }
    }

    pub fn record_provider_usage(&mut self, provider: &str, sample: UsagePatternSample) {
        self.record_provider_usage_on_day(provider, sample, self.current_day);
    }

    pub fn record_provider_usage_on_day(
        &mut self,
        provider: &str,
        mut sample: UsagePatternSample,
        day: NaiveDate,
    ) {
        sample.provider_count = 1;
        self.usage_stats.record_day(sample);
        if day == self.current_day && sample.night_events > 0 {
            self.advance_daily_quest_for_night_usage();
        }

        let key = day.to_string();
        let providers = self.providers_by_day.entry(key).or_default();
        let was_multi = providers.len() > 1;
        if !providers.iter().any(|seen| seen == provider) {
            providers.push(provider.to_string());
        }
        if !was_multi && providers.len() > 1 {
            self.usage_stats.multi_provider_days += 1;
        }
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
        self.check_evolution(config);
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
            self.food_earned_by_day
                .insert(self.current_day.to_string(), self.food_earned_today);
            self.food_earned_today = 0;
            self.banked_tokens_today = 0.0;
            self.roll_daily_quest();
            if self.current_day.weekday().number_from_monday() == 7 {
                self.roll_week();
            }
            self.current_day = self
                .current_day
                .succ_opt()
                .expect("date overflow while rolling economy day");
        }
    }

    fn auto_feed_from_pantry(&mut self, config: &EconomyConfig) {
        if self.pantry == 0 {
            return;
        }
        self.pantry -= 1;
        self.record_activity_day(self.current_day);
        self.eat_one_food(config);
        self.check_evolution(config);
    }

    /// Shared fullness/XP math for "the pet ate one Food," regardless of
    /// whether it came from `food_inventory` or an auto-feed from the
    /// Pantry. Mood is evaluated from fullness *before* this meal.
    fn eat_one_food(&mut self, config: &EconomyConfig) {
        let mood = mood_from_fullness(self.fullness);
        let xp_gain = config.xp_per_food as f64 * mood_multiplier(mood) * self.xp_bonus_multiplier;
        self.xp += xp_gain;
        self.fullness = (self.fullness + config.fullness_per_food as f64).min(100.0);
    }

    fn record_activity_day(&mut self, day: NaiveDate) {
        if self.last_activity_day == Some(day) {
            return;
        }

        let continued = self
            .last_activity_day
            .map(|last| day == last.succ_opt().unwrap_or(last))
            .unwrap_or(false);
        let gap_days = self
            .last_activity_day
            .map(|last| (day - last).num_days())
            .unwrap_or(0);

        if self.last_activity_day.is_none() || continued {
            self.streak_days += 1;
        } else if gap_days == 2 && self.streak_freezes > 0 {
            self.streak_freezes -= 1;
            self.streak_days += 1;
        } else {
            self.streak_days = 1;
        }

        self.last_activity_day = Some(day);
        self.apply_streak_rewards();
    }

    fn apply_streak_rewards(&mut self) {
        let sparks = match self.streak_days {
            3 => 1,
            7 => 3,
            14 => 5,
            30 => 10,
            100 => 30,
            _ => 0,
        };
        self.sparks += sparks;

        if self.streak_days > 0 && self.streak_days % 7 == 0 {
            self.streak_freezes = (self.streak_freezes + 1).min(2);
        }
    }

    fn advance_daily_quest_for_food(&mut self, food: u32) {
        if matches!(
            self.daily_quest.kind,
            DailyQuestKind::EarnOneFood | DailyQuestKind::EarnThreeFood
        ) {
            self.daily_quest.progress =
                (self.daily_quest.progress + food).min(self.daily_quest.target);
            self.complete_daily_quest_if_ready();
        }
    }

    fn advance_daily_quest_for_night_usage(&mut self) {
        if self.daily_quest.kind == DailyQuestKind::UseAtNight {
            self.daily_quest.progress = self.daily_quest.target;
            self.complete_daily_quest_if_ready();
        }
    }

    fn complete_daily_quest_if_ready(&mut self) {
        if !self.daily_quest.completed && self.daily_quest.progress >= self.daily_quest.target {
            self.daily_quest.completed = true;
            self.sparks += self.daily_quest.reward_sparks;
        }
    }

    fn check_weekly_milestone(&mut self) {
        if !self.weekly_milestone_claimed && self.weekly_food_earned >= self.weekly_target {
            self.weekly_milestone_claimed = true;
            self.sparks += 4;
        }
    }

    fn roll_daily_quest(&mut self) {
        let next_day = self
            .current_day
            .succ_opt()
            .expect("date overflow while rolling daily quest");
        self.daily_quest = DailyQuestState::for_day(next_day);
    }

    fn roll_week(&mut self) {
        let completed_food = self.weekly_food_earned;
        self.weekly_target = ((completed_food as f64 * 0.85).round() as u32).clamp(3, 35);
        self.weekly_food_earned = 0;
        self.weekly_milestone_claimed = false;
    }

    fn check_evolution(&mut self, config: &EconomyConfig) {
        let level = level_for_xp(self.xp, config);
        let next_stage = stage_for_level(level);
        if next_stage == self.evolution_stage {
            return;
        }

        let branch = if matches!(next_stage, EvolutionStage::Juvenile | EvolutionStage::Adult) {
            self.usage_stats.selected_branch()
        } else {
            self.evolution_branch
        };

        self.evolution_stage = next_stage;
        self.evolution_branch = branch;
        self.sparks += evolution_reward_sparks(next_stage);

        let album_key = self.record_album_entry(level);
        self.pending_evolution = Some(EvolutionEvent {
            stage: next_stage,
            branch,
            level,
            album_key,
        });
    }

    fn record_album_entry(&mut self, level: u32) -> String {
        let key = album_key(
            self.evolution_stage,
            self.evolution_branch,
            self.prestige_count,
        );
        if !self.album.contains(&key) {
            self.album.push(key.clone());
        }
        if !self.album_records.iter().any(|record| record.key == key) {
            self.album_records.push(AlbumRecord {
                key: key.clone(),
                stage: self.evolution_stage,
                branch: self.evolution_branch,
                reached_day: self.current_day.to_string(),
                level,
                xp: self.xp,
                sparks: self.sparks,
                prestige_count: self.prestige_count,
            });
        }
        key
    }
}

fn evolution_reward_sparks(stage: EvolutionStage) -> u32 {
    match stage {
        EvolutionStage::Egg => 0,
        EvolutionStage::Hatchling => 2,
        EvolutionStage::Juvenile => 5,
        EvolutionStage::Adult => 10,
        EvolutionStage::Elder => 20,
    }
}

fn default_furniture_x(item_id: &str) -> f64 {
    match item_id {
        "furniture-bed" => 0.18,
        "furniture-plant" => 0.78,
        "furniture-perch" => 0.5,
        _ => 0.5,
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

    fn event_for_food_count(id: &str, food_count: u32, config: &EconomyConfig) -> TokenEvent {
        let tokens = (1..=food_count)
            .map(|index| cost_of_nth_food(index, config))
            .sum::<f64>()
            .ceil() as u64;
        small_event(id, tokens)
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

    #[test]
    fn simulated_60_day_usage_tracks_evolution_streaks_quests_and_sparks() {
        let config = test_config();
        let start = day(2026, 1, 1);
        let mut state = EconomyState::new(start, 0);

        for offset in 0..60 {
            let current_day = start + chrono::Duration::days(offset);
            state.reconcile_elapsed_time(offset * 86_400, current_day, &config);

            // One missed day: no Pantry stock in this simulation, so the
            // following active day must spend a streak freeze.
            if offset == 14 {
                continue;
            }

            state.record_usage_pattern(UsagePatternSample {
                night_events: 1,
                session_count: 1,
                short_sessions: 0,
                long_sessions: 1,
                provider_count: 1,
            });
            let outcome = state.apply_token_event(
                &event_for_food_count(&format!("day-{offset}"), 20, &config),
                &config,
            );
            assert_eq!(outcome.food_earned, 20);
            assert_eq!(outcome.food_to_pantry, 0);

            while state.eat_from_inventory(&config) {}
        }

        assert_eq!(state.streak_days, 59);
        assert_eq!(
            state.last_activity_day,
            Some(start + chrono::Duration::days(59))
        );
        assert_eq!(state.evolution_stage, EvolutionStage::Adult);
        assert_eq!(state.evolution_branch, EvolutionBranch::Nocturnal);
        assert!(state.album.contains(&"hatchling:sprout:p0".to_string()));
        assert!(state.album.contains(&"juvenile:nocturnal:p0".to_string()));
        assert!(state.album.contains(&"adult:nocturnal:p0".to_string()));
        assert_eq!(
            state.pending_evolution.as_ref().unwrap().stage,
            EvolutionStage::Adult
        );
        assert!(
            state.sparks >= 140,
            "streak, daily quest, weekly milestone, and evolution rewards should all contribute"
        );
        assert!(
            state.daily_quest.completed,
            "quests are auto-detected from food/night usage without UI interaction"
        );
    }

    #[test]
    fn buying_equipping_and_placing_shop_items_updates_persistent_state() {
        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        state.sparks = 40;

        state.buy_item("hat-leaf").unwrap();
        state.equip_item("hat-leaf").unwrap();
        state.buy_item("food-sushi").unwrap();
        state.equip_item("food-sushi").unwrap();
        state.buy_item("furniture-bed").unwrap();
        state.place_furniture("furniture-bed", 0.9).unwrap();

        assert_eq!(state.sparks, 10);
        assert_eq!(state.equipped_cosmetic.as_deref(), Some("hat-leaf"));
        assert_eq!(state.equipped_food_skin.as_deref(), Some("food-sushi"));
        assert_eq!(
            state
                .furniture
                .iter()
                .find(|item| item.item_id == "furniture-bed")
                .unwrap()
                .x,
            0.9
        );
        assert_eq!(state.buy_item("hat-leaf"), Err(ShopError::AlreadyOwned));
    }

    #[test]
    fn prestige_requires_elder_resets_pet_and_preserves_album_legacy() {
        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        state.sparks = 25;
        state.xp = 100_000.0;
        state.evolution_stage = EvolutionStage::Elder;
        state.evolution_branch = EvolutionBranch::Scholar;
        state.record_album_entry(45);
        let album_before = state.album_records.clone();

        state.prestige(day(2026, 3, 1)).unwrap();

        assert_eq!(state.evolution_stage, EvolutionStage::Egg);
        assert_eq!(state.xp, 0.0);
        assert_eq!(state.sparks, 25);
        assert_eq!(state.prestige_count, 1);
        assert_eq!(state.xp_bonus_multiplier, 1.1);
        assert!(state.owned_items.contains(&"halo-heirloom".to_string()));
        assert_eq!(state.equipped_cosmetic.as_deref(), Some("halo-heirloom"));
        assert!(state.album_records.len() > album_before.len());
        assert!(state
            .album_records
            .iter()
            .any(|record| record.stage == EvolutionStage::Elder));
    }

    #[test]
    fn delayed_provider_usage_uses_caps_for_the_day_it_occurred() {
        let config = test_config();
        let mut state = EconomyState::new(day(2026, 1, 2), 86_400);
        state.food_earned_today = config.daily_hard_cap;

        let delayed_day = day(2026, 1, 1);
        let outcome = state.apply_token_event_on_day(
            &event_for_food_count("openai-late", 2, &config),
            delayed_day,
            &config,
        );

        assert_eq!(outcome.food_earned, 2);
        assert_eq!(
            state.food_earned_today, config.daily_hard_cap,
            "today's exhausted cap must not block a delayed prior-day event"
        );
        assert_eq!(
            state
                .food_earned_by_day
                .get(&delayed_day.to_string())
                .copied(),
            Some(2)
        );
    }

    #[test]
    fn provider_mix_counts_once_per_multi_provider_day_for_chimera() {
        let mut state = EconomyState::new(day(2026, 1, 1), 0);
        let sample = UsagePatternSample::single_event(12, 1);

        state.record_provider_usage("claude_code", sample);
        state.record_provider_usage("claude_code", sample);
        assert_eq!(state.usage_stats.multi_provider_days, 0);

        state.record_provider_usage("codex_cli", sample);
        state.record_provider_usage("openai", sample);
        assert_eq!(state.usage_stats.multi_provider_days, 1);
        assert_eq!(
            state
                .providers_by_day
                .get("2026-01-01")
                .map(|providers| providers.len()),
            Some(3)
        );
    }
}
