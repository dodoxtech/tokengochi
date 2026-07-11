---
type: task
status: backlog
priority: P2
delivery_order: 0009
estimate: 5d
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
sprint: null
tags:
  - task
  - backlog
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

- [ ] Simulated 60-day usage script produces correct levels, branch choice, streak/freeze behavior, and Sparks totals per spec.
- [ ] Evolution triggers a celebration animation and album record.
- [ ] Quests never require UI interaction to complete (auto-detected).

## Dependencies

- [[0008-packaging-ci-updater|0008]]

## Verification Plan

- [ ] Economy simulation tests + manual evolution check; record results below.

## Verification Results

TBD
