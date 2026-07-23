---
type: task
status: done
priority: P2
delivery_order: 0020
estimate: M
created: 2026-07-22
updated: 2026-07-23
owner: AI agent
sprint: null
tags:
  - task
  - done
  - watcher
  - cross-platform
---

# Task: Token watcher path & schema hardening (Codex model, env overrides, cross-platform)

## Status

- State: done
- Created: 2026-07-22
- Owner: AI agent
- Priority: P2
- Delivery order: 0020
- Estimate: M
- Sprint: TBD

## Context

Verification of where the app counts tokens was done on 2026-07-22 against
real local data (macOS) and cross-checked against public docs for Windows/Linux
layouts. The provider log **locations** are correct on all three platforms
(both CLIs are home-directory rooted, so `dirs::home_dir()` join is right), but
several gaps surfaced between the parsers and real data:

1. **Codex `model` is never read** — real `token_count` records carry no
   `model` field (0/122 lines in a sampled session), so
   [`process_lines`](../../../src-tauri/src/watcher/codex_cli.rs) always falls
   back to the hardcoded `"gpt-5"`. Every Codex turn is therefore weighted as
   gpt-5 regardless of the actual model. The real model lives in the first
   `session_meta` record of each rollout file, which the parser never reads.
2. **Codex dedup degrades to offset-only** — `token_count` payloads carry no
   `id` or `session_id` (0/122), so `message_id` always falls back to the
   synthesized `"<file>:<offset>"` key. Dedup-by-message-id effectively never
   runs for Codex. This is safe within a single file but should be documented
   and its double-count risk (same turn emitted on two lines) confirmed.
3. **Env-var overrides ignored** — Codex honors `CODEX_HOME` and Claude Code
   honors `CLAUDE_CONFIG_DIR`; both watchers hardcode `~/.codex` / `~/.claude`
   and will silently see no logs when a user relocates those dirs.
4. **Windows/Linux never run-tested** — the `notify`-based Claude Code watcher
   and the Codex poll loop have only been exercised on macOS. Paths are correct
   on paper but the watcher behavior (fs events, path casing, WSL split) is
   unverified on Windows/Linux.
5. **Claude Code schema is now verified** — the open question in
   [[../../knowledge/token-tracking|Token Tracking]] (§Open Questions) about
   Claude Code JSONL field names was confirmed against a live install on
   2026-07-22: `message.id`, `message.model`, and
   `usage.{input,output,cache_read_input,cache_creation_input}_tokens` all
   present. Docs should be updated to close it.

Related:

- [[../README|Tasks]]
- [[../../agile|Agile and Scrum Workflow]]
- [[../../knowledge/token-tracking|Token Tracking]]
- [[../done/0003-claude-code-token-watcher|0003 Claude Code Token Watcher]]
- [[../done/0011-multi-provider-plugins|0011 Multi-provider Plugins]]

## Goal

Make token counting robust to real-world Codex data and to non-default log
locations, and gain confidence the watchers work on Windows and Linux.

## Scope

In scope:

- Read the actual Codex model from the `session_meta` record and apply it to
  that file's `token_count` events (replace the hardcoded `"gpt-5"` fallback).
- Honor `CODEX_HOME` (Codex) and `CLAUDE_CONFIG_DIR` (Claude Code) env vars,
  falling back to `~/.codex` / `~/.claude` when unset.
- Document the Codex offset-only dedup behavior and confirm whether Codex ever
  emits the same turn's `last_token_usage` on multiple lines (double-count
  risk).
- Update `docs/knowledge/token-tracking.md` to mark the Claude Code schema as
  verified against a live install (2026-07-22).
- Verify watcher startup + event flow on Windows and Linux (build + smoke run).

Out of scope:

- OpenAI Usage API changes (`openai.rs`).
- WSL bridging (detecting logs inside a WSL filesystem from a native Windows
  build) — note as a known limitation only.
- Any economy/weighting formula changes beyond feeding the correct model id.

## Acceptance Criteria

- [x] Codex events carry the real model id parsed from the file's
      `turn_context` records (see correction below — **not** `session_meta`);
      `"gpt-5"` is used only when no model can be determined.
- [x] `CODEX_HOME` and `CLAUDE_CONFIG_DIR` are honored; when set to a custom
      dir, the watcher tails logs there. Unset behavior is unchanged.
- [x] `token-tracking.md` Open Questions updated: Claude Code schema marked
      verified (2026-07-22); Codex `model`/`id` absence documented.
- [x] Codex offset-only dedup documented, with a note on whether duplicate
      turn emission is observed (none observed in the sampled rollout).
- [x] `cargo test` passes, including new/updated unit tests for the
      `turn_context` model extraction and env-override path resolution.
- [ ] **Deferred**: Watcher start/first-event confirmation on Windows and
      Linux — no such environment available in this session. Paths are pure and
      unit-tested; the fs-event/poll behavior remains to be smoke-run on those
      platforms before the next cross-platform release.

## Correction to the original plan

The Implementation Notes assumed the Codex model lives in the `session_meta`
record. Verified against a real rollout on 2026-07-22: `session_meta` only
carries `model_provider` (e.g. `"openai"`), **not** the model id. The real
model is in `turn_context` records (`payload.model`, e.g. `"gpt-5.4"`), which
precede the turn's `token_count` events. Implementation tracks the most-recent
`turn_context` model per file instead, priming it from already-consumed bytes
on restart (`scan_latest_model`).

## Dependencies

- None blocking. Builds on 0003 (Claude Code watcher) and 0011 (multi-provider).

## Risks

- Codex `session_meta` schema is also undocumented/unstable — extraction must
  tolerate its absence and fall back gracefully.
- Windows/Linux verification may require environments not currently available;
  if so, mark those criteria as deferred rather than silently skipped.

## Implementation Notes

- Codex model: parse the first line of each rollout file where
  `type == "session_meta"`; the model lives under `payload` (fields observed:
  `model_provider`, plus version/cwd metadata — confirm the exact model key on
  a real file before relying on it). Cache per-file so it isn't re-read every
  poll.
- Env overrides: resolve root as
  `env::var("CODEX_HOME").map(PathBuf::from).unwrap_or_else(|| home/.codex)` and
  the analogous `CLAUDE_CONFIG_DIR` for Claude Code, keeping the existing
  `with_paths` test seam.
- Files: `src-tauri/src/watcher/codex_cli.rs`,
  `src-tauri/src/watcher/claude_code.rs`.

## References

- [[../../README|Documentation]]
- [[../../index|Project Map]]
- `src-tauri/src/watcher/codex_cli.rs`
- `src-tauri/src/watcher/claude_code.rs`
- `src-tauri/src/storage_paths.rs`
- Claude Code `.claude` directory docs: https://code.claude.com/docs/en/claude-directory
- Codex CLI config/paths (CODEX_HOME): https://inventivehq.com/knowledge-base/openai/where-configuration-files-are-stored

## Verification Plan

- [ ] `cargo test` in `src-tauri/`.
- [ ] Manual: set `CODEX_HOME` to a temp dir with a fixture rollout file and
      confirm the watcher detects/tails it.
- [ ] Manual: confirm a real Codex event now shows the correct model id.
- [ ] Smoke-run the app on Windows and Linux; confirm events flow.

## Verification Results

- `cargo test --lib`: 66 passed, 0 failed. New tests:
  `codex_cli::stamps_turn_context_model_onto_following_usage_events`,
  `codex_cli::falls_back_to_default_model_before_any_turn_context`,
  `codex_cli::codex_root_honors_env_then_home_then_relative`,
  `claude_code::claude_root_honors_env_then_home_then_relative`,
  `store::ledger::splits_token_totals_by_provider`.
- `cargo clippy --lib`: clean.
- `npm run check` (dashboard): 0 errors, 0 warnings.
- Real-data confirmation (macOS): the sampled rollout's last `turn_context`
  carries `model = "gpt-5.4"` while 0 `token_count` lines carry a `model`
  field, so the watcher now stamps `gpt-5.4` where it previously hardcoded
  `gpt-5`.
- Deferred: Windows/Linux smoke run (no environment available).

## Completion Notes

Beyond the task scope, the user requested the dashboard show Claude and Codex
token totals **separately** rather than merged. Added
`Ledger::token_totals_between_by_provider`, surfaced it as
`StatsPayload.{today,week}_tokens_by_provider`, and rendered per-provider token
cards in the dashboard (`TOKEN_PROVIDERS` / `providerTotals`). The combined
`today_tokens`/`week_tokens` fields remain for backward compatibility.

- Completed: 2026-07-23
- Changed files:
  - `src-tauri/src/watcher/codex_cli.rs` (turn_context model tracking,
    `CODEX_HOME` override, `CodexRecord` parser, `scan_latest_model`)
  - `src-tauri/src/watcher/claude_code.rs` (`CLAUDE_CONFIG_DIR` override,
    `resolve_claude_root`)
  - `src-tauri/src/store/ledger.rs` (`token_totals_between_by_provider`)
  - `src-tauri/src/lib.rs` (per-provider fields on `StatsPayload`)
  - `ui/dashboard/src/routes/+page.svelte` (per-provider token cards)
  - `docs/knowledge/token-tracking.md`, `docs/knowledge/code-map.md`
- Follow-ups:
  - Windows/Linux watcher smoke run (deferred acceptance criterion).
  - Consider a combined "all providers" card or a provider toggle if more than
    two providers ever emit token events (OpenAI/manual currently roll into the
    combined totals only).
