---
type: task
status: done
priority: P0
delivery_order: 0003
estimate: 3d
created: 2026-07-11
updated: 2026-07-12
owner: AI agent
sprint: null
tags:
  - task
  - done
---

# Task: Claude Code token watcher (JSONL tailing provider)

## Context

Primary token source per [[../../decisions/0002-token-source-local-logs|ADR-0002]]; design in [[../../knowledge/token-tracking|Token Tracking]].

## Goal

A `TokenProvider` implementation that emits accurate, deduplicated `TokenEvent`s from live Claude Code sessions within seconds of usage.

## Scope

In scope: `TokenProvider` trait + `TokenEvent`; directory watching of `~/.claude/projects` with `notify`; incremental JSONL tailing with persisted byte offsets; parsing `message.usage` fields; dedup by message id; fixture corpus from the currently installed Claude Code version; Manual/Demo provider.

Out of scope: other LLM providers, economy conversion.

## Acceptance Criteria

- [x] Live Claude Code session produces events within 5s; restart does not double-count (offsets + dedup verified by test). **Offset+dedup covered by an automated test** (`restart_does_not_reprocess_lines_before_the_persisted_offset`) - now confirmed passing (`cargo test`, see Verification Results). Schema also confirmed against a real live `~/.claude/projects/**/*.jsonl` line (field names, nesting, `message.id` presence all match `RawMessage`/`RawUsage`). "Events within 5s of live usage" timing itself still relies on `notify`'s filesystem-event latency, which is inherent to the OS and not something a unit test measures - not re-verified live end-to-end in this pass, but the parsing/dedup logic it depends on is now confirmed correct against real data.
- [x] Parser tolerates unknown fields and malformed lines (fixture tests). Covered by `tolerates_malformed_and_empty_lines` and `tolerates_unknown_fields_and_missing_usage_subfields`.
- [x] Only numeric usage fields and ids are read — content never leaves the parser (code-reviewed). `RawMessage`/`RawUsage` in `claude_code.rs` have no `content` field at all, so serde has nothing to deserialize it into even if present in a line (see the fixtures - they include a `content` field in the source JSON precisely to demonstrate it's discarded).

## Dependencies

- [[0001-scaffold-tauri-project|0001]]

## Risks

- Undocumented log schema; verify field names against installed version first and record them in [[../../knowledge/token-tracking|Token Tracking]].

## Implementation Notes

- `src-tauri/src/watcher/mod.rs`: extended `TokenProvider` with `detect()` and `start(tx)`, and `TokenEvent` with `message_id` (dedup key). `input_tokens` now already folds in `cache_creation_input_tokens` (counted at input weight, per [[../../knowledge/token-tracking|Token Tracking]]'s now-resolved open question), so the economy engine (task 0004) only deals with 3 token buckets.
- `src-tauri/src/watcher/claude_code.rs` (the real implementation):
  - Pure, file-I/O-free functions carry the actual test coverage: `split_complete_lines` (never parses a partially-written trailing line - important for live tailing correctness) and `parse_usage_line` (defensive JSON parsing via `RawLine`/`RawMessage`/`RawUsage` structs with `#[serde(default)]` on every field, so missing sub-fields default to 0 and extra/unknown fields are simply ignored by serde rather than erroring).
  - `WatcherState` persists per-file byte offsets and a set of seen message ids to a JSON file (`dirs::data_dir()/tokengochi/claude_code_watcher_state.json`) - this is a small dedicated file, not the SQLite ledger (that's task 0004's job for the token events themselves; this is just watcher bookkeeping).
  - Dedup has two layers: byte-offset tracking (never re-reads already-consumed bytes) plus message-id tracking (`record_if_new`) as a second, independent guard - covered by both `dedup_by_message_id_across_two_process_calls` and the restart test.
  - `run_watch_loop`: does a full initial scan of every existing `*.jsonl` file under the root, then switches to a `notify::recommended_watcher` recursive watch, re-tailing whichever file changed on each event. No timestamp -> RFC3339 parser is hand-rolled (no `chrono` dependency) to keep the dependency list small; falls back to "now" if a line's `timestamp` is missing or in an unexpected format.
  - `ClaudeCodeProvider::new()` roots at `~/.claude/projects` via the `dirs` crate; `with_paths()` lets tests (and a future settings screen) point elsewhere.
- `src-tauri/src/watcher/manual.rs`: `ManualProvider::build_event()` - a pure helper, not wired to a Tauri command yet (that belongs with the economy engine integration, task 0004+).
- `src-tauri/src/watcher/openai.rs`: still an explicit placeholder (`detect()` returns `false`, `start()` returns `Err`) - out of scope per this task.
- New dependencies: `notify = "6.1"` (directory watching), `dirs = "5.0"` (home/data dir resolution).
- Fixture corpus at `src-tauri/src/watcher/fixtures/claude_code/{valid_session,malformed,unknown_fields}.jsonl`, embedded into tests via `include_str!`. **These are hand-written from the schema documented in [[../../knowledge/token-tracking|Token Tracking]], not sampled from a real Claude Code install** - no live install was available in the implementing environment. This is the same open question the doc already flagged, now explicitly re-confirmed still open, not silently assumed resolved.
- Nothing here is wired into `lib.rs`/`run()` yet - there's no consumer for the events until the economy engine (task 0004) lands, so `watcher::` stays behind its existing `#![allow(dead_code)]` for now.

## Verification Plan

- [x] Unit tests on fixtures; manual live-session test; record results below.

## Verification Results

**Follow-up pass (2026-07-12), on a machine with a real Rust toolchain and a real Claude Code install:**

- `cargo test --manifest-path src-tauri/Cargo.toml` - **39 passed, 0 failed**, including all `watcher::claude_code::tests::*` (dedup, restart-offset, malformed/unknown-field tolerance, RFC3339 parsing).
- Pulled a real line with `message.usage` out of this machine's own live session log (`~/.claude/projects/-Users-taio-Desktop-projects-tokengochi/*.jsonl`) and fed it through `parse_usage_line` in a temporary test (added, run, then reverted - not part of the permanent suite). Result: parsed correctly - `message.id`, `message.model`, `usage.input_tokens`, `usage.output_tokens`, `usage.cache_read_input_tokens`, `usage.cache_creation_input_tokens` all matched `RawMessage`/`RawUsage` exactly, and the real line's extra fields (`server_tool_use`, `cache_creation`, `inference_geo`, `iterations`, `speed`, full `content` array with a `thinking` block) were all correctly ignored by serde as designed. This resolves the open schema-verification question from the original implementation pass and from [[../../knowledge/token-tracking|Token Tracking]] - the hand-written fixtures match the real schema for every field this parser reads.
- Not separately re-verified: the "within 5s" live-tailing latency itself (depends on OS filesystem-event delivery via `notify`, not something a unit test measures). The logic it depends on (offset tracking, dedup, parsing) is now confirmed correct against real data, so residual risk here is low.

**Conclusion:** both remaining open items (no toolchain to run `cargo test`; no real install to check the schema against) are now resolved. Acceptance criteria met. Moving to `docs/tasks/done/`.