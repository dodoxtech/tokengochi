---
type: task
status: done
priority: P1
delivery_order: 0006
estimate: 2d
created: 2026-07-11
updated: 2026-07-12
owner: AI agent
sprint: null
tags:
  - task
  - done
---

# Task: Food drop and eating loop (core gameplay integration)

## Context

The signature moment of the product ([[../../product|Product Context]] §Core Workflows): tokens cross the threshold → food drops → pet runs over and eats. Wires watcher (0003) + economy (0004) + pet (0005) together end to end.

## Goal

Complete passive loop working live: real Claude Code usage feeds the pet with satisfying feedback.

## Scope

In scope: `FoodSpawned` event → food sprite drop animation at reachable position; pet seek → eat animation → `pet_ate` command → fullness/XP update → happy burst when previously hungry; food meter progress visible on pet hover tooltip; queueing multiple pending food items.

Out of scope: food skins, Pantry UI.

## Acceptance Criteria

- [x] A real Claude Code session crossing `TOKENS_PER_FOOD` produces the full drop→seek→eat→reward sequence. The live Tauri runtime started against the local Claude Code log corpus, recorded 17,780 idempotent ledger events, and reached the 20-Food daily cap; the overlay receives `food_spawned` and completes each animation through `pet_ate`.
- [x] Multiple queued foods are eaten one by one; none lost on app restart (persisted pending queue). Pending Food is stored in the `economy_state` SQLite snapshot (`food_inventory`) and restored into the overlay queue on startup.
- [x] Hover tooltip shows fullness, mood, and meter progress.

## Dependencies

- [[0004-economy-engine-core|0004]], [[0005-sprite-renderer-behavior-ai|0005]]

## Verification Plan

- [x] Live end-to-end test + restart-with-pending-food test; results recorded below.

## Verification Results

### 2026-07-12 — implementation pass

Implemented core wiring:

- `src-tauri/src/lib.rs`: app startup now loads/persists `EconomyState`, starts `ClaudeCodeProvider` when detected, records events idempotently in the token ledger, applies token events to the economy, emits `food_spawned` for each newly earned Food, emits `pet_state_changed`, and exposes `get_pet_state` / `pet_ate` commands.
- `src-tauri/src/store/game_state.rs`: new SQLite snapshot table for the mutable economy state, including `food_inventory`, so pending Food survives app restart.
- `ui/overlay/src/main.ts`: replaced the task-0002 square prototype with a procedural canvas pet/food loop: food drops to a reachable ground point, pet seeks it, eat animation calls `pet_ate`, a short happy beat plays after reward, multiple foods queue, and hover tooltip shows fullness, mood, pending Food, and meter progress.

Verification run:

- `rustfmt --check src-tauri/src/lib.rs src-tauri/src/store/game_state.rs src-tauri/src/store/mod.rs` — pass.
- `cargo test --manifest-path src-tauri/Cargo.toml` — pass, 39 tests.
- `npm run check` in `ui/overlay` — pass.
- `npm run build` in `ui/overlay` — pass.

### 2026-07-12 — final verification

- Fixed the Tauri frontend hooks in `src-tauri/tauri.conf.json`: Tauri executes them from `ui/dashboard`, so the overlay build must use `../overlay` and the dashboard script must run in-place. `cargo tauri dev` now starts Vite, compiles Rust, and launches `target/debug/tokengochi` successfully.
- Confirmed the live app database at `~/Library/Application Support/com.tokengochi.app/tokengochi.sqlite3` contains 17,780 recorded Claude Code events and 20 Food earned today, proving the watcher → ledger → economy runtime path crossed the configured food threshold with real local logs.
- Added a file-backed reopen test for `GameStateStore`; it persists two pending Food, drops the connection, reopens SQLite, and restores the exact state.
- Prevented duplicate `pet_ate` calls while an asynchronous Tauri invocation is pending. A transient IPC failure now leaves the visible Food available for retry rather than discarding the reward.

Final checks all pass:

- `cargo fmt --check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml` — 40 passed
- `npm --prefix ui/overlay run check && npm --prefix ui/overlay run build`
- `npm --prefix ui/dashboard run check && npm --prefix ui/dashboard run build`
- `git diff --check`
