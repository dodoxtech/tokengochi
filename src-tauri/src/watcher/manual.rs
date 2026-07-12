use super::{TokenEvent, TokenProvider};

/// Manual/demo mode: lets a user without a supported provider still feed the
/// pet, at a reduced Food rate and capped progression (see
/// `docs/knowledge/game-economy.md` §7). Unlike the other providers, nothing
/// is tailed automatically - the dashboard calls a command (wired up
/// alongside the economy engine, task 0004+) that builds an event via
/// [`ManualProvider::build_event`] on demand.
pub struct ManualProvider;

impl ManualProvider {
    /// Builds a [`TokenEvent`] for a manually-entered/simulated session.
    /// `weighted_tokens` is already the caller's chosen "pretend I used N
    /// tokens" amount - the ×0.25 manual-mode rate and progression cap are
    /// applied by the economy engine (`docs/knowledge/game-economy.md` §7),
    /// not here. `message_id` should be unique per manual action (e.g. a
    /// UUID or timestamp-based id from the caller) since there's no
    /// provider-native id to dedup by.
    pub fn build_event(message_id: String, weighted_tokens: u64, timestamp: i64) -> TokenEvent {
        TokenEvent {
            provider: "manual".to_string(),
            message_id,
            // No real model behind manual mode - empty string takes the
            // economy engine's `model_weight_default` path.
            model: String::new(),
            input_tokens: weighted_tokens,
            output_tokens: 0,
            cache_read_tokens: 0,
            timestamp,
        }
    }
}

impl TokenProvider for ManualProvider {
    fn name(&self) -> &'static str {
        "manual"
    }

    fn detect(&self) -> bool {
        // Always available - it's the no-dependencies fallback.
        true
    }

    fn start(&self, _tx: std::sync::mpsc::Sender<TokenEvent>) -> std::io::Result<()> {
        // Nothing to tail; events are pushed on demand via `build_event`
        // through a Tauri command, not through this background-watcher path.
        Ok(())
    }
}
