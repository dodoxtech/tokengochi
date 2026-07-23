---
type: task
status: done
priority: P2
delivery_order: 0024
estimate: S
created: 2026-07-23
updated: 2026-07-23
owner: AI agent
sprint: null
tags:
  - task
  - done
---

# Task: Decouple food animation from backfill events

## Status

- State: done
- Created: 2026-07-23
- Owner: AI agent
- Priority: P2
- Delivery order: 0024
- Estimate: S
- Sprint: TBD

## Context

Follow-up to [[0023-skip-token-history-on-first-run]]. Task 0023 stopped the
first-ever run from backfilling the whole log history. What remained: on a
**subsequent** launch the watcher catches up on all tokens logged while the app
was closed (from the persisted offset), and each caught-up event emitted a
`food_spawned` event, so reopening after heavy offline usage still rained a
burst of falling food. The desired behavior is to credit that usage to the
ledger / economy / XP (the tokens are not lost) but not animate a falling-food
drop per historical event - only real-time events should animate.

Related:

- [[../../knowledge/token-tracking|Token Tracking]]
- [[0023-skip-token-history-on-first-run]]

## Goal

Backfill (startup catch-up) usage feeds the ledger, economy, food inventory,
streaks, and XP exactly as before, but does not produce the falling-food
animation; only live events do.

## Scope

In scope:

- `TokenEvent.backfill: bool` flag; watchers mark startup catch-up reads as
  backfill and live reads as not.
- `apply_token_event` suppresses the `food_spawned` emit for backfill events
  (everything else unchanged).
- Overlay caps on-screen pending food so a large credited inventory doesn't
  materialize as a burst via the `pet_state_changed` restore path.

Out of scope:

- Batching the per-event `pet_state_changed` emit / DB writes during catch-up
  (pre-existing behavior; performance tuning left for later).

## Acceptance Criteria

- [x] Catch-up (non-first-run initial scan) events are marked `backfill: true`;
  live `notify`/poll events are `backfill: false`.
- [x] Backfill events still record to the ledger and update economy/food/XP.
- [x] No `food_spawned` (falling animation) is emitted for backfill events.
- [x] Overlay shows at most `MAX_VISIBLE_FOOD` pending pieces at once; the true
  count stays in the meter.

## Implementation Notes

- `src-tauri/src/watcher/mod.rs`: added `#[serde(default)] backfill: bool` to
  `TokenEvent`.
- `src-tauri/src/watcher/claude_code.rs`: `tail_file` takes a `backfill` flag;
  `initial_scan` non-first-run path passes `true`, the `notify` loop passes
  `false`.
- `src-tauri/src/watcher/codex_cli.rs`: `run_poll_loop` marks the first poll
  pass on a non-first run as backfill (`backfill_pass = !first_run`, reset to
  `false` after the first iteration).
- `src-tauri/src/lib.rs`: `apply_token_event` gates the `food_spawned` loop on
  `!event.backfill`.
- `ui/overlay/src/state.ts`: `ensurePendingFoodVisible` clamps to
  `MAX_VISIBLE_FOOD` (12).
- All other `TokenEvent` construction sites default `backfill: false`.

## Verification Results

- `cargo test` — 70 passed, incl. new backfill assertions in
  `claude_code::first_run_seeds_offset_to_end_and_skips_history`,
  `claude_code::non_first_run_catches_up_existing_history`, and
  `codex_cli::falls_back_to_default_model_before_any_turn_context`.
- `ui/overlay` `tsc --noEmit` (npm run check) — clean.

## Completion Notes

- Completed: 2026-07-23
- Changed files:
  - `src-tauri/src/watcher/mod.rs`
  - `src-tauri/src/watcher/claude_code.rs`
  - `src-tauri/src/watcher/codex_cli.rs`
  - `src-tauri/src/watcher/manual.rs`
  - `src-tauri/src/watcher/openai.rs`
  - `src-tauri/src/economy/conversion.rs`
  - `src-tauri/src/economy/state.rs`
  - `src-tauri/src/store/ledger.rs`
  - `src-tauri/src/lib.rs`
  - `ui/overlay/src/state.ts`
  - `docs/knowledge/token-tracking.md`
- Follow-ups: consider batching per-event `pet_state_changed`/DB writes during
  a large catch-up to reduce write churn.

> **Superseded by [[0026-count-only-usage-after-launch]]** — the app now ignores all pre-launch usage on every startup, not just the first run, and no longer credits history to the economy.
