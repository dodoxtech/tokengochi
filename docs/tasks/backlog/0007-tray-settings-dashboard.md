---
type: task
status: backlog
priority: P1
delivery_order: 0007
estimate: 3d
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
sprint: null
tags:
  - task
  - backlog
---

# Task: System tray, settings, and dashboard MVP

## Context

The pet needs an app around it: launch at login, tray control, onboarding, and a small stats view ([[../../product|Product Context]] §Core Workflows 2–3).

## Goal

Tray-first app lifecycle plus a Svelte dashboard with onboarding, stats, and settings.

## Scope

In scope: tray icon + menu (show/hide pet, dashboard, pause tracking, quit); autostart (opt-in, `tauri-plugin-autostart`); onboarding flow (pick starter egg → auto-detect Claude Code → done); settings (providers on/off, pet size, monitor selection, Wayland fallback mode); stats page (today/week food, level, streak, token totals); single-instance guard.

Out of scope: shop/album (0009–0010).

## Acceptance Criteria

- [ ] Fresh install reaches a living pet in under 1 minute with zero manual config when Claude Code is present.
- [ ] Close button hides to tray; quit only via tray; autostart works on all 3 OS.
- [ ] Stats match the ledger (spot-check against SQLite).

## Dependencies

- [[0006-food-drop-eating-loop|0006]]

## Verification Plan

- [ ] Fresh-profile install test per OS; record results below.

## Verification Results

TBD
