//! Token usage watcher: tails provider log files and emits [`TokenEvent`]s.
//!
//! See `docs/architecture.md` §Data Flow and `docs/knowledge/token-tracking.md`.
//! [`ClaudeCodeProvider`] (task 0003) is the first real implementation;
//! nothing here is wired into `lib.rs` / app startup yet since there's no
//! consumer for the events until the economy engine (task 0004) lands -
//! hence the blanket allow below. Remove it once `run()` actually starts a
//! provider and consumes its events.
#![allow(dead_code, unused_imports)]

mod claude_code;
mod manual;
mod openai;

pub use claude_code::ClaudeCodeProvider;
pub use manual::ManualProvider;
pub use openai::OpenAiProvider;

/// Raw usage numbers observed from a single provider log entry, deduplicated
/// by `message_id`.
///
/// `input_tokens` already includes `cache_creation_input_tokens` (counted at
/// input weight, per `docs/knowledge/token-tracking.md` Open Questions -
/// "yes, at input weight"), so the economy engine only needs to know about
/// three token buckets, not four.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TokenEvent {
    pub provider: String,
    /// Provider-native message id, used for dedup. Falls back to a
    /// synthesized `"<file>:<offset>"` key if the source line has no id -
    /// see `docs/knowledge/token-tracking.md` Open Questions (schema is
    /// undocumented/unverified against a live install).
    pub message_id: String,
    /// Provider-native model id (e.g. `"claude-opus-4-8"`), used for
    /// per-model token weighting (`docs/knowledge/game-economy.md` §2).
    /// Empty string when the source line has no model field - the economy
    /// engine falls back to `model_weight_default` for it.
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    /// Unix timestamp (seconds) the event was observed.
    pub timestamp: i64,
}

/// A source of token usage events (Claude Code logs, OpenAI logs, manual/demo
/// mode, ...). Implementations tail their source and push [`TokenEvent`]s to
/// the economy engine.
pub trait TokenProvider: Send + Sync {
    /// Stable identifier used in [`TokenEvent::provider`] and persistence.
    fn name(&self) -> &'static str;

    /// Best-effort check for whether this provider's data source exists on
    /// this machine (e.g. its log directory exists).
    fn detect(&self) -> bool;

    /// Begins watching/tailing in the background and sends events to `tx` as
    /// they're observed. Returns once the watcher is set up; it keeps
    /// running on its own thread. Returns `Err` if setup fails (e.g. the
    /// directory can't be watched).
    fn start(&self, tx: std::sync::mpsc::Sender<TokenEvent>) -> std::io::Result<()>;
}
