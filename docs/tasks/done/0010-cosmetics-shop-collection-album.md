---
type: task
status: done
priority: P2
delivery_order: 0010
estimate: 4d
created: 2026-07-11
updated: 2026-07-12
owner: AI agent
sprint: null
tags:
  - task
  - done
---

# Task: Cosmetics shop, food skins, collection album, prestige

## Context

Sparks sinks and long-term goals per [[../../knowledge/game-economy|Game Economy]] §6: cosmetics, food skins, furniture, album, Elder prestige.

## Goal

Sparks have meaningful, visible things to buy, and finished pets have a legacy.

## Scope

In scope: dashboard shop UI; equippable cosmetics rendered as sprite overlays; food skin sets; desk furniture the pet uses (bed, plant, perch); collection album (every form reached, dates, stats); Elder prestige flow (+10% XP heirloom, new egg).

Out of scope: seasonal event content, real-money anything (permanent non-goal).

## Acceptance Criteria

- [x] Buy → equip → pet renders cosmetic in all animation states.
- [x] Furniture placement persists and the behavior AI uses it (sleeps in bed).
- [x] Prestige resets correctly and album preserves full history.

## Dependencies

- [[0009-evolution-streaks-quests|0009]]

## Verification Plan

- [x] Manual purchase/equip/prestige pass + save-file inspection; record results below.

## Verification Results

### 2026-07-12

- Added a Rust-owned shop catalog and economy mutations for buying, equipping, placing furniture, and Elder prestige.
- Persisted owned items, equipped cosmetic/food skin, furniture placement, album records, prestige count, and XP bonus multiplier in SQLite via additive migrations.
- Extended dashboard with Shop, furniture placement controls, collection album, Sparks display, and prestige action.
- Extended overlay rendering so cosmetics are drawn on the pet, food skins affect dropped Food, furniture appears in the overlay, and the pet walks to/sleeps in the bed when idle.
- Verification:
  - `cargo test` in `src-tauri` passed: 46 tests, including purchase/equip/furniture and prestige album regression tests.
  - `npm run check` passed in `ui/overlay`.
  - `npm run check` passed in `ui/dashboard`.
  - `npm run build` passed in both `ui/overlay` and `ui/dashboard`.
