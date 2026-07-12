//! Economy engine: token→Food conversion, caps, streaks, XP.
//!
//! Balance constants live in `economy.toml` (see
//! `docs/knowledge/game-economy.md` §8) so tuning never requires a code
//! change. Streak/quest bookkeeping (task 0009) and wiring this into the
//! running app (consuming watcher events, persisting `EconomyState`, a
//! `pet_ate` command, ...) are follow-up work - see
//! `docs/tasks/active/0004-economy-engine-core.md` for exactly what's in
//! scope here vs. later.
#![allow(dead_code, unused_imports)]

mod config;
mod conversion;
mod fullness;
mod state;
mod xp;

pub use config::{load_economy_config, EconomyConfig};
pub use conversion::{cost_of_nth_food, model_multiplier, weighted_tokens};
pub use fullness::{mood_from_fullness, mood_multiplier};
pub use state::{ConversionOutcome, DailyQuestKind, DailyQuestState, EconomyState};
pub use xp::{level_for_xp, xp_required_for_level, MAX_LEVEL};
