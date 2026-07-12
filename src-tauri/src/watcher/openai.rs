use super::{TokenEvent, TokenProvider};

/// Placeholder for a future OpenAI usage log provider. See
/// `docs/tasks/backlog/0011-multi-provider-plugins.md`.
pub struct OpenAiProvider;

impl TokenProvider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn detect(&self) -> bool {
        false
    }

    fn start(&self, _tx: std::sync::mpsc::Sender<TokenEvent>) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "openai provider not implemented yet (task 0011)",
        ))
    }
}
