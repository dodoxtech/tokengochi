use super::TokenProvider;

/// Placeholder for a future OpenAI usage log provider. See
/// `docs/tasks/backlog/0011-multi-provider-plugins.md`.
pub struct OpenAiProvider;

impl TokenProvider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "openai"
    }
}
