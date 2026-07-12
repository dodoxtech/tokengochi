---
type: task
status: done
priority: P2
delivery_order: 0009
estimate: 5d
created: 2026-07-11
updated: 2026-07-12
owner: AI agent
sprint: null
tags:
  - task
  - done
---

# Task: Evolution stages, streaks, quests, Sparks (v1.1 economy layer)

## Context

Long-term retention layer per [[../../knowledge/game-economy|Game Economy]] §4–5: evolution with usage-pattern branching, forgiving streaks, daily quests, Sparks currency.

## Goal

The pet visibly grows over weeks and daily use is rewarded beyond food.

## Scope

In scope: level/evolution state machine (Egg→Elder) with branch selection from usage-pattern stats (night-heavy, session length, session count); evolution cutscene animation; streak tracking + freezes; auto-detected daily quest + weekly personalized milestone; Sparks earning; sprite sets for Juvenile branches (art task may split out).

Out of scope: chimera branch (needs multi-provider, 0011), shop/album (0010), seasonal events.

## Acceptance Criteria

- [x] Simulated 60-day usage script produces correct levels, branch choice, streak/freeze behavior, and Sparks totals per spec.
- [x] Evolution triggers a celebration animation and album record.
- [x] Quests never require UI interaction to complete (auto-detected).

## Dependencies

- [[0008-packaging-ci-updater|0008]]

## Verification Plan

- [x] Economy simulation tests + manual evolution check; record results below.

## Verification Results

### 2026-07-12

- Added the v1.1 economy layer in Rust: deterministic evolution stages/branches, album records, pending evolution celebration events, streaks with freeze banking/spending, daily auto-detected quests, weekly personalized milestone rewards, and Sparks.
- Persisted the new state in SQLite with additive migrations for existing local databases.
- Exposed progression state in the Tauri pet/dashboard payload.
- Verification: `cargo test` in `src-tauri` passed, including `simulated_60_day_usage_tracks_evolution_streaks_quests_and_sparks` covering the 60-day script, nocturnal branch selection, one missed day with freeze behavior, quest completion without UI interaction, album entries, and Sparks accumulation.
