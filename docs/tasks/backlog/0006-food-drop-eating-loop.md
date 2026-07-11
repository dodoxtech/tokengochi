---
type: task
status: backlog
priority: P1
delivery_order: 0006
estimate: 2d
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
sprint: null
tags:
  - task
  - backlog
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

- [ ] A real Claude Code session crossing `TOKENS_PER_FOOD` produces the full drop→seek→eat→reward sequence.
- [ ] Multiple queued foods are eaten one by one; none lost on app restart (persisted pending queue).
- [ ] Hover tooltip shows fullness, mood, and meter progress.

## Dependencies

- [[0004-economy-engine-core|0004]], [[0005-sprite-renderer-behavior-ai|0005]]

## Verification Plan

- [ ] Live end-to-end test + restart-with-pending-food test; record results below.

## Verification Results

TBD
