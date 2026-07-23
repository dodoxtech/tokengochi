---
type: task
status: done
priority: P2
delivery_order: 0025
estimate: XS
created: 2026-07-23
updated: 2026-07-23
owner: AI agent
sprint: null
tags:
  - task
  - done
---

# Task: No falling food from history

## Status

- State: done
- Created: 2026-07-23
- Owner: AI agent
- Priority: P2
- Delivery order: 0025
- Estimate: XS
- Sprint: TBD

## Context

Follow-up to [[0024-decouple-food-animation-from-backfill]]. Task 0024 stopped
backfill events from emitting `food_spawned`, but the overlay's restore path
(`ensurePendingFoodVisible`) still materialized up to `MAX_VISIBLE_FOOD` pending
pieces by **dropping them from the top of the screen** — so history-credited
inventory still rained food on reopen. The user asked to stop falling food from
history entirely.

Related:

- [[../../knowledge/token-tracking|Token Tracking]]
- [[0024-decouple-food-animation-from-backfill]]

## Goal

Only live token events produce falling food. Any food materialized from
existing/pending inventory (including usage backfilled from log history)
appears already on the ground, never falling.

## Scope

In scope:

- Add a `landed` option to `spawnFood`; the restore path spawns landed food.

Out of scope:

- Removing food from inventory for history (history still credits inventory so
  it can be eaten for XP, per task 0024).

## Acceptance Criteria

- [x] Live `food_spawned` events still fall from the top (reward feedback).
- [x] `ensurePendingFoodVisible` spawns food already at ground level — no fall,
  no landing bounce.
- [x] Pre-landed food is still sought and eaten by the pet (`y >= targetY`).

## Implementation Notes

- `ui/overlay/src/state.ts`: `spawnFood(id, landed = false)` — when `landed`,
  start at `targetY` instead of `-FOOD_SIZE`. `ensurePendingFoodVisible` passes
  `true`. `landedAt` stays `-Infinity` so a restored piece shows no landing
  bounce, and `updateFood` skips the fall physics because `y === targetY`.

## Verification Results

- `ui/overlay` `npm run check` (tsc --noEmit) — clean.
- Reasoned through render (`sinceLanding = Infinity` → no bounce) and behavior
  (`food.y < targetY` false → no fall; `hasWaitingFood`/seek `y >= targetY`
  true → still eaten).

## Completion Notes

- Completed: 2026-07-23
- Changed files:
  - `ui/overlay/src/state.ts`
  - `docs/knowledge/token-tracking.md`
- Follow-ups: none.
