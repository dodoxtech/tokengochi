//! Pet state machine: mood, hunger decay, evolution.
//!
//! See `docs/knowledge/game-economy.md` §3-4 and `docs/architecture.md`
//! §Data Flow. This module is currently a structural placeholder; behavior is
//! tracked in `docs/tasks/backlog/0004-economy-engine-core.md`,
//! `docs/tasks/backlog/0005-sprite-renderer-behavior-ai.md`, and
//! `docs/tasks/backlog/0009-evolution-streaks-quests.md`. Nothing here is
//! constructed yet, hence the blanket allow below - remove it once the state
//! machine actually uses these types.
#![allow(dead_code)]

/// Evolution stages, in order. See `docs/knowledge/game-economy.md` §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EvolutionStage {
    Egg,
    Hatchling,
    Juvenile,
    Adult,
    Elder,
}

/// Discrete mood bands, driven by Fullness. See
/// `docs/knowledge/game-economy.md` §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Mood {
    Full,
    Content,
    Peckish,
    Hungry,
    /// Fullness has (nearly) bottomed out: the pet hibernates - sleeps, sad
    /// animation, gains zero XP - but never dies and never loses levels
    /// (guilt-free by design, see `docs/knowledge/game-economy.md` §3).
    Starving,
}
