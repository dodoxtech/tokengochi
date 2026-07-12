use super::TokenProvider;

/// Tails `~/.claude/projects/**/*.jsonl` for per-message `usage` numbers.
///
/// Only token counts are read, never message content (see
/// `docs/architecture.md` §Important Constraints). Implementation tracked in
/// `docs/tasks/backlog/0003-claude-code-token-watcher.md`.
pub struct ClaudeCodeProvider;

impl TokenProvider for ClaudeCodeProvider {
    fn name(&self) -> &'static str {
        "claude_code"
    }
}
