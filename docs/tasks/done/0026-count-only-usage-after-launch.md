---
type: task
status: done
priority: P1
delivery_order: 0026
estimate: S
created: 2026-07-23
updated: 2026-07-23
owner: AI agent
sprint: null
tags:
  - task
  - done
---

# Task: Count only token usage produced after app launch

## Status

- State: done
- Created: 2026-07-23
- Owner: AI agent
- Priority: P1
- Delivery order: 0026
- Estimate: S
- Sprint: TBD

## Context

Supersedes [[0023-skip-token-history-on-first-run]] and
[[0024-decouple-food-animation-from-backfill]]. Those iterations still credited
usage logged while the app was closed to the economy (silently, without the
falling animation). The user's actual intent: tokens spent while the app was
**not running** must not be counted as food at all. Each launch should start
counting fresh — only tokens the user spends **after** opening the app get
converted to food and fall.

Related:

- [[../../knowledge/token-tracking|Token Tracking]]
- [[0025-no-falling-food-from-history]]

## Goal

On every launch the watchers ignore all pre-existing log content and only
convert usage appended after startup into food.

## Scope

In scope:

- Both watchers seed every existing log file's offset to its current end on
  **every** startup (not just the first run); no catch-up read.
- Remove the `TokenEvent.backfill` flag and its handling (0024), now moot.

Out of scope:

- Overlay restore behavior (`landed` spawn + `MAX_VISIBLE_FOOD` cap) stays as
  landed in [[0025-no-falling-food-from-history]]; it applies to legitimately
  earned-but-uneaten pending food on reload.

## Acceptance Criteria

- [x] Startup seeds all existing files to their current end every launch.
- [x] Tokens logged while the app was closed are not counted (no food, not fed
  to the economy, not shown).
- [x] Tokens appended after launch are tailed live and become falling food.
- [x] `TokenEvent` no longer carries `backfill`; `apply_token_event` emits
  `food_spawned` for every live event again.
- [x] Opening the app shows **zero** food on the floor and a zero queue: the
  uneaten Food inventory is cleared on launch (previous sessions' pending Food
  does not carry over), while XP/level/streak are preserved.

## Implementation Notes

- `src-tauri/src/watcher/claude_code.rs`: `run_watch_loop` calls
  `seed_all_to_end` (renamed from the first-run-only `initial_scan`) on every
  launch; `tail_file` dropped its `backfill` param and is used only by the
  live `notify` loop.
- `src-tauri/src/watcher/codex_cli.rs`: `run_poll_loop` always seeds every file
  to end before the poll loop; removed the `backfill_pass` logic.
- `src-tauri/src/watcher/mod.rs`: removed `backfill` from `TokenEvent`.
- `src-tauri/src/lib.rs`: reverted the `food_spawned` gate — every live event
  animates again. Also calls `economy.clear_pending_food()` once during `run()`
  setup so the uneaten Food queue starts empty on every launch.
- `src-tauri/src/economy/state.rs`: added `EconomyState::clear_pending_food`
  (zeroes `food_inventory`, leaves progression intact).
- Removed `backfill: false` from all other `TokenEvent` construction sites.

## Verification Results

- `cargo test` — 71 passed, incl. rewritten
  `claude_code::startup_seeds_offset_to_end_and_only_counts_post_launch`, new
  `claude_code::every_startup_skips_usage_logged_while_closed`, and
  `economy::state::clear_pending_food_empties_queue_but_keeps_progression`.
- `cargo clippy` — clean.

## Completion Notes

- Completed: 2026-07-23
- Changed files:
  - `src-tauri/src/watcher/mod.rs`
  - `src-tauri/src/economy/state.rs`
  - `src-tauri/src/watcher/claude_code.rs`
  - `src-tauri/src/watcher/codex_cli.rs`
  - `src-tauri/src/watcher/manual.rs`
  - `src-tauri/src/watcher/openai.rs`
  - `src-tauri/src/economy/conversion.rs`
  - `src-tauri/src/economy/state.rs`
  - `src-tauri/src/store/ledger.rs`
  - `src-tauri/src/lib.rs`
  - `docs/knowledge/token-tracking.md`
- Follow-ups: none.
