use super::TokenProvider;

/// Manual/demo mode: lets a user without a supported provider still feed the
/// pet, at a reduced Food rate and capped progression (see
/// `docs/knowledge/game-economy.md` §7).
pub struct ManualProvider;

impl TokenProvider for ManualProvider {
    fn name(&self) -> &'static str {
        "manual"
    }
}
