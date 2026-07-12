//! Economy engine: token→Food conversion, caps, streaks, XP.
//!
//! Balance constants live in `economy.toml` (see
//! `docs/knowledge/game-economy.md` §8) so tuning never requires a code
//! change. This module currently only loads that config; conversion, caps,
//! streak, and XP logic are tracked in
//! `docs/tasks/backlog/0004-economy-engine-core.md`.

mod config;

pub use config::{load_economy_config, EconomyConfig};
