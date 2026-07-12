//! Token usage watcher: tails provider log files and emits [`TokenEvent`]s.
//!
//! See `docs/architecture.md` §Data Flow and `docs/knowledge/token-tracking.md`.
//! This module is a structural placeholder; the first real implementation
//! (Claude Code log tailing) is tracked in
//! `docs/tasks/backlog/0003-claude-code-token-watcher.md`. Nothing here is
//! wired up yet, hence the blanket allow below - remove it once providers are
//! actually constructed and used.
#![allow(dead_code)]

mod claude_code;
mod manual;
mod openai;

pub use claude_code::ClaudeCodeProvider;
pub use manual::ManualProvider;
pub use openai::OpenAiProvider;

/// Raw usage numbers observed from a single provider log entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenEvent {
    pub provider: String,
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
}
