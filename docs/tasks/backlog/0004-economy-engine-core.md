---
type: task
status: backlog
priority: P0
delivery_order: 0004
estimate: 3d
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
sprint: null
tags:
  - task
  - backlog
---

# Task: Economy engine core (token → food conversion, caps, XP, fullness)

## Context

Implements [[../../knowledge/game-economy|Game Economy]] §1–3 and §8. Pure Rust module; all constants from `economy.toml`.

## Goal

Deterministic, unit-tested economy functions: weighted conversion, daily soft/hard caps with escalation, Pantry overflow, fullness decay, mood multipliers, XP accrual.

## Scope

In scope: `economy/` module (pure functions over a ledger), SQLite ledger schema (`store/`), day-boundary handling in local time, elapsed-time reconciliation on launch (decay while app was closed).

Out of scope: evolution branching, streaks/quests (task 0009), UI.

## Acceptance Criteria

- [ ] Property/unit tests cover: weighting, soft-cap escalation exactly per spec, hard cap, Pantry fill/auto-feed, decay across app-closed gaps and DST changes.
- [ ] All constants read from `economy.toml`; changing a constant requires no code change.
- [ ] Ledger dedup: replaying the same TokenEvents is idempotent.

## Dependencies

- [[0003-claude-code-token-watcher|0003]]

## Risks

- Time handling (sleep, timezone, clock changes) — test explicitly.

## Verification Plan

- [ ] `cargo test` economy suite; simulated 30-day usage script; record results below.

## Verification Results

TBD
