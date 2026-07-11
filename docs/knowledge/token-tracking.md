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

- Codex CLI: local session logs (`~/.codex/sessions/`) — same tailing approach; verify format at implementation time.
- Raw OpenAI API: no local logs; options are the Usage API (needs org API key, minutes of delay) or a local proxy mode. Ship as opt-in plugin; delay is acceptable since food just arrives late.

## Manual / Demo (v1)

- User enters tokens or clicks a "simulate session" button. Earns at ×0.25 rate, capped progression ([[game-economy|Game Economy]] §7).

## Privacy Rules

- Read only `usage` numeric fields and message ids; never read or store message content.
- All data stays in local SQLite; no telemetry.

## Open Questions

- Exact JSONL field set varies by Claude Code version — pin against current version in task 0003 and add a fixture corpus.
- Should cache_creation tokens count? (Currently: yes, at input weight.)
