---
type: task
status: done
priority: P2
delivery_order: 0023
estimate: S
created: 2026-07-23
updated: 2026-07-23
owner: AI agent
sprint: null
tags:
  - task
  - done
---

# Task: Skip old token history on first run

## Status

- State: done
- Created: 2026-07-23
- Owner: AI agent
- Priority: P2
- Delivery order: 0023
- Estimate: S
- Sprint: TBD

## Context

On startup the token watchers scan every existing `*.jsonl` log and tail each
file from its persisted byte offset. On the first-ever run the persisted offset
is 0, so the watcher reads the entire Claude Code / Codex history at once and
emits a `TokenEvent` per historical message. The economy engine turns each into
falling food, so opening the app (notably `cargo tauri dev`, which uses its own
`tokengochi-dev` state namespace and therefore backfills fresh every first run)
produces a flood of food. Users also perceive this as "the app counting while
it was closed", because the same scan dumps everything appended while the app
was closed all at once.

Related:

- [[../../knowledge/token-tracking|Token Tracking]]
- [[../README|Tasks]]

## Goal

On the first-ever run, count only tokens produced after the app opens instead
of backfilling the whole log history.

## Scope

In scope:

- Seed each existing log file's offset to its current size on the first-ever
  run (detected by a missing watcher state file) for both the Claude Code and
  Codex watchers.

Out of scope:

- Changing the catch-up behavior on subsequent runs (usage logged while the app
  was closed is still counted on the next launch, from the persisted offset).
- Time-window based backfill limiting or decoupling food animation from
  historical events (the other options considered; may revisit later).

## Acceptance Criteria

- [x] First-ever run emits no events for pre-existing history; offsets are
  seeded to the current file size.
- [x] Tokens appended after the app opens are still counted.
- [x] Sessions created while the app is running are counted from the start
  (new files arrive via `notify` / a later poll pass with no seeded offset).
- [x] Subsequent runs behave as before (tail from persisted offset).

## Implementation Notes

- `src-tauri/src/watcher/claude_code.rs`: added `initial_scan` and
  `seed_offset_to_end`; `run_watch_loop` computes `first_run = !state_path.exists()`
  and seeds instead of tailing on the first run.
- `src-tauri/src/watcher/codex_cli.rs`: `run_poll_loop` computes the same
  `first_run` flag and seeds all existing files (priming the per-file model
  tracker via `scan_latest_model`) before the poll loop.
- First-run detection uses the absence of the per-provider watcher state file,
  so each provider is independent and a wiped/uninstalled data dir re-skips.

## Verification Results

- `cargo test watcher` — 28 passed, incl. new
  `claude_code::first_run_seeds_offset_to_end_and_skips_history`,
  `claude_code::non_first_run_catches_up_existing_history`, and
  `codex_cli::first_run_seed_skips_history_and_primes_model`.

## Completion Notes

- Completed: 2026-07-23
- Changed files:
  - `src-tauri/src/watcher/claude_code.rs`
  - `src-tauri/src/watcher/codex_cli.rs`
  - `docs/knowledge/token-tracking.md`
- Follow-ups: consider decoupling historical catch-up from the food animation
  so even the "usage while closed" catch-up doesn't arrive as a burst of food.

> **Superseded by [[0026-count-only-usage-after-launch]]** — the app now ignores all pre-launch usage on every startup, not just the first run, and no longer credits history to the economy.
