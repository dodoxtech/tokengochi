---
type: knowledge
status: active
created: 2026-07-11
updated: 2026-07-11
tags:
  - knowledge
  - integration
  - ai-context
owner: AI agent
---

# Token Tracking

How Tokengochi measures real token usage per provider. Related: [[../architecture|Architecture]], [[game-economy|Game Economy]], [[../decisions/0002-token-source-local-logs|ADR-0002]].

## Provider Abstraction

Rust trait; each provider is a plugin registered in settings:

```rust
trait TokenProvider {
    fn id(&self) -> &str;                       // "claude-code", "openai", "manual"
    fn detect(&self) -> DetectResult;           // auto-detect availability on this machine
    fn start(&self, tx: Sender<TokenEvent>);    // begin tailing/polling
}

struct TokenEvent { provider: String, input: u64, output: u64, cache_read: u64, ts: DateTime }
```

Events are deduplicated by `(provider, message_id)` in the ledger so re-reads never double-feed.

## Claude Code (primary, v1)

- Source: session logs at `~/.claude/projects/<project-slug>/*.jsonl` — one JSON object per line; assistant messages carry `message.usage` with `input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`.
- Method: watch the directory tree with `notify`, tail appended lines, parse `usage`, emit events. Persist per-file byte offsets so restarts don't re-count.
- Caveats: format is undocumented/unstable — parser must ignore unknown fields and tolerate schema drift; verify field names against the installed Claude Code version during implementation. Multiple concurrent sessions are normal.
- Fallback: OpenTelemetry — Claude Code can export usage metrics via OTLP; a local OTLP listener is a cleaner but heavier alternative if log parsing breaks.

## OpenAI / Codex CLI (v2 plugin)

- Codex CLI: local session logs (`~/.codex/sessions/`) — same tailing approach. Verified on 2026-07-12 against local Codex Desktop/CLI JSONL: token usage appears as `event_msg` records with `payload.type = "token_count"` and `payload.info.last_token_usage`. The parser reads only `input_tokens`, `cached_input_tokens`, `output_tokens`, `reasoning_output_tokens`, timestamp, and optional ids/model.
- Raw OpenAI API: Usage API polling (opt-in). API key is stored in the OS keychain helper, never SQLite/config. Usage API buckets are delayed, so Tokengochi converts them using the bucket `start_time` day rather than the polling day.

## Manual / Demo (v1)

- User enters tokens or clicks a "simulate session" button. Earns at ×0.25 rate, capped progression ([[game-economy|Game Economy]] §7).

## Privacy Rules

- Read only `usage` numeric fields and message ids; never read or store message content.
- All data stays in local SQLite; no telemetry.

## Open Questions

- Exact JSONL field set varies by Claude Code version — pin against current version in task 0003 and add a fixture corpus. **Still open**: task 0003's implementation (`src-tauri/src/watcher/claude_code.rs`) was written and fixture-tested against the schema documented above, but has not been verified against a real, currently-installed Claude Code version (no live install was available in the implementing environment). Re-verify field names against `~/.claude/projects/**/*.jsonl` on a real machine before relying on this in production, and update the fixtures under `src-tauri/src/watcher/fixtures/claude_code/` if anything differs.
- Should cache_creation tokens count? (Currently: yes, at input weight.) Implemented as decided: the parser folds `cache_creation_input_tokens` into `TokenEvent.input_tokens` at parse time (see `parse_usage_line`), so the economy engine only ever sees three token buckets (input, output, cache_read), not four.
- Dedup key: message ids are assumed present on assistant lines (`message.id`); if a line is missing one, the watcher falls back to a synthesized `"<file>:<offset>"` key so dedup-by-offset still holds, but true dedup-by-message-id degrades for that line. Unconfirmed whether real Claude Code lines ever omit `message.id`.
