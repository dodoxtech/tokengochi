---
type: knowledge
status: active
created: 2026-07-11
updated: 2026-07-22
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
- Count only post-launch usage (tasks 0023 → 0026): **every launch** seeds each existing log file's offset to its current size, so tokens logged before the app opened — whether on the first-ever run or because the app was closed for a while — are **never** counted as food. Only usage appended after the app opens is tailed live and converted to food. There is no history "catch-up": past tokens are neither fed to the economy nor shown. See `seed_all_to_end`/`seed_offset_to_end` in `src-tauri/src/watcher/claude_code.rs` and the startup seed loop in `run_poll_loop` (`codex_cli.rs`). (This supersedes the earlier first-run-only skip and the backfill-crediting design; `TokenEvent` no longer carries a `backfill` flag.)
- Empty food queue on launch (task 0026): startup also calls `EconomyState::clear_pending_food` (in `lib.rs` `run()` setup), zeroing the uneaten Food inventory so opening the app never shows leftover food on the floor — even Food earned and left uneaten in a previous session does not carry across launches. XP, level, streak, and other progression are preserved; only the Food queue resets. Combined with the seed-to-end above, a fresh launch starts with an empty screen and zero queue, and only post-launch usage produces falling Food.
- No falling food on restore (task 0025): the overlay's `spawnFood(id, landed)` only lets a **live** reward fall from the top of the screen (that drop is the reward feedback). The restore path (`ensurePendingFoodVisible`, used on reopen and after each eat) spawns already-earned-but-uneaten pending food already on the ground (`landed = true`), and caps on-screen pieces at `MAX_VISIBLE_FOOD` (`ui/overlay/src/state.ts`) so a reload never re-rains pending inventory; the true count stays visible in the meter.
- Caveats: format is undocumented/unstable — parser must ignore unknown fields and tolerate schema drift; verify field names against the installed Claude Code version during implementation. Multiple concurrent sessions are normal.
- Fallback: OpenTelemetry — Claude Code can export usage metrics via OTLP; a local OTLP listener is a cleaner but heavier alternative if log parsing breaks.

## OpenAI / Codex CLI (v2 plugin)

- Codex CLI: local session logs (`~/.codex/sessions/`) — same tailing approach. Verified on 2026-07-12 against local Codex Desktop/CLI JSONL: token usage appears as `event_msg` records with `payload.type = "token_count"` and `payload.info.last_token_usage`. The parser reads only `input_tokens`, `cached_input_tokens`, `output_tokens`, `reasoning_output_tokens`, timestamp, and optional ids/model.
- Raw OpenAI API: Usage API polling (opt-in). API key is stored in the OS keychain helper, never SQLite/config. Usage API buckets are delayed, so Tokengochi converts them using the bucket `start_time` day rather than the polling day.

## Manual / Demo (v1)

- User enters tokens or clicks a "simulate session" button. Earns at ×0.25 rate, capped progression ([[game-economy|Game Economy]] §7).

## Agent Status Events (turn completed / needs approval)

Separate from token-usage tracking above: task 0017 adds a second, much
smaller event type - `AgentStatusEvent` (`provider`, `session_id`, `status`,
`ts`) - so the pet can react with a cute badge when an agent's turn finishes
or needs approval. It is sourced from Claude Code **hooks**, not JSONL log
parsing (the JSONL schema doesn't reliably carry either signal - see Open
Questions below), and is deliberately decoupled from the economy engine: it
never touches fullness/XP/food. Full detail, the hook JSON to install, and
the local file bridge: [[agent-status-notifications|Agent Status
Notifications]].

## Privacy Rules

- Read only `usage` numeric fields and message ids; never read or store message content.
- All data stays in local SQLite; no telemetry.

## Log Locations & Env Overrides

Both CLIs are home-directory rooted, and both honor an env var to relocate
their data dir. As of task 0020 the watchers honor these too (they no longer
hardcode `~/.claude` / `~/.codex`):

- Claude Code: `CLAUDE_CONFIG_DIR` (config root; watcher tails
  `<CLAUDE_CONFIG_DIR>/projects/**/*.jsonl`), default `~/.claude`. See
  `resolve_claude_root` in `src-tauri/src/watcher/claude_code.rs`.
- Codex: `CODEX_HOME` (default `~/.codex`; watcher tails
  `<CODEX_HOME>/sessions/**/*.jsonl`). See `resolve_codex_root` in
  `src-tauri/src/watcher/codex_cli.rs`.

Known limitation: a native Windows build does not bridge into a WSL
filesystem, so logs written inside WSL aren't seen (out of scope for 0020).

## Open Questions

- ~~Exact JSONL field set varies by Claude Code version~~ **Verified against a
  live Claude Code install on 2026-07-22** (task 0020): assistant lines carry
  `message.id`, `message.model`, and
  `usage.{input,output,cache_read_input,cache_creation_input}_tokens` — all the
  fields `parse_usage_line` (`src-tauri/src/watcher/claude_code.rs`) reads. The
  fixtures under `src-tauri/src/watcher/fixtures/claude_code/` match. Re-verify
  after a major Claude Code upgrade.
- Should cache_creation tokens count? (Currently: yes, at input weight.) Implemented as decided: the parser folds `cache_creation_input_tokens` into `TokenEvent.input_tokens` at parse time (see `parse_usage_line`), so the economy engine only ever sees three token buckets (input, output, cache_read), not four.
- **Claude Code dedup key**: message ids are present on assistant lines
  (`message.id`, confirmed 2026-07-22); if a line ever omits one, the watcher
  falls back to a synthesized `"<file>:<offset>"` key so dedup-by-offset still
  holds, but true dedup-by-message-id degrades for that line.
- **Codex model** (verified 2026-07-22): `token_count` records carry **no**
  `model` field (0/122 lines sampled), and `session_meta` only has
  `model_provider` (e.g. `"openai"`), not the model id. The real model lives in
  `turn_context` records as `payload.model` (e.g. `"gpt-5.4"`), emitted before
  the turn's `token_count` events. The watcher tracks the most-recent
  `turn_context` model per file and stamps it onto following usage events
  (task 0020); the hardcoded `"gpt-5"` is a last-resort fallback only.
- **Codex dedup** (verified 2026-07-22): `token_count` payloads carry no
  `id`/`session_id` (0/122 sampled), so Codex dedup always uses the synthesized
  `"<file>:<offset>"` key — dedup-by-message-id never runs for Codex. This is
  safe: byte offsets are unique within a file and the persisted per-file offset
  only advances, and no duplicate emission of the same turn's
  `last_token_usage` on two lines was observed in the sample.
