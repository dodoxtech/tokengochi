---
type: task
status: active
priority: P0
delivery_order: 0003
estimate: 3d
created: 2026-07-11
updated: 2026-07-12
owner: AI agent
sprint: null
tags:
  - task
  - active
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

- [ ] Live Claude Code session produces events within 5s; restart does not double-count (offsets + dedup verified by test). **Offset+dedup covered by an automated test** (`restart_does_not_reprocess_lines_before_the_persisted_offset`); the "within 5s of a live session" half is not verified - needs a human with a real Claude Code install running `cargo test` and then watching a live session.
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

- [ ] Unit tests on fixtures; manual live-session test; record results below.

## Verification Results

**What was actually run (this sandbox still has no Rust toolchain - see task 0001's Verification Results for why):**

- Manual read-through of every function, including tracing through both the initial-scan and `notify`-event-driven code paths sharing `tail_file` so they can't drift apart.
- Hand-verified the fixtures' expected parse results against the test assertions line-by-line (e.g. `120 = 100 input + 20 cache_creation`, exactly 1 of 7 lines in `malformed.jsonl` should parse).
- Cross-checked the hand-rolled RFC3339-to-unix-seconds date math (`days_from_civil`) against Python's `datetime` for several dates including a leap day and the epoch boundary - all matched - since I got the manually-computed expected value in the `rfc3339_parses_known_timestamp` test wrong on the first pass and caught it this way rather than leaving an assertion that would fail on first `cargo test`.
- `Cargo.toml`, all new JSON fixtures are plain `.jsonl` (not validated as JSON since they intentionally contain malformed lines) - the two "clean" fixtures (`valid_session`, `unknown_fields`) were checked line-by-line against `parse_usage_line`'s logic by hand.
- **Not run: `cargo test` itself.** No Rust toolchain in this sandbox. This task leans on unit tests more than any task so far (`split_complete_lines`, `parse_usage_line`, dedup, restart-offset behavior) - all logic-level and should be fast to run. Please run `cargo test --manifest-path src-tauri/Cargo.toml` and let me know what fails, if anything; I can also inspect `src-tauri/target/` again afterward the way I did for task 0001.
- **Not verified - needs a human with a real Claude Code install:** whether the actual JSONL schema matches what's assumed here (field names, whether `message.id` is ever absent, whether `timestamp` is always RFC3339-with-`Z`), and the "events within 5s of live usage" timing claim.

**Conclusion:** code + tests are complete for the task's scope; staying in `docs/tasks/active/` until `cargo test` has actually run once and (ideally) someone points a real Claude Code session at this.