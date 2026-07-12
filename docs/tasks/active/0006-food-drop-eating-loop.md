---
type: task
status: active
priority: P1
delivery_order: 0006
estimate: 2d
created: 2026-07-11
updated: 2026-07-12
owner: AI agent
sprint: null
tags:
  - task
  - active
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

- [ ] A real Claude Code session crossing `TOKENS_PER_FOOD` produces the full drop→seek→eat→reward sequence. Implemented via the Claude Code watcher → economy → `food_spawned` event path; still needs a live app run against real Claude Code logs for acceptance evidence.
- [x] Multiple queued foods are eaten one by one; none lost on app restart (persisted pending queue). Pending Food is stored in the `economy_state` SQLite snapshot (`food_inventory`) and restored into the overlay queue on startup.
- [x] Hover tooltip shows fullness, mood, and meter progress.

## Dependencies

- [[0004-economy-engine-core|0004]], [[0005-sprite-renderer-behavior-ai|0005]]

## Verification Plan

- [ ] Live end-to-end test + restart-with-pending-food test; record results below.

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

Not yet complete for `done`: a live `cargo tauri dev` run with a real Claude Code session crossing `tokens_per_food` still needs to be observed, and a restart-with-pending-food manual test should be recorded. `cargo fmt --check --manifest-path src-tauri/Cargo.toml` was not used as final evidence because it reports pre-existing formatting drift in files outside this task's edit set.
