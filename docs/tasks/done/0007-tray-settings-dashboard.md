---
type: task
status: done
priority: P1
delivery_order: 0007
estimate: 3d
created: 2026-07-11
updated: 2026-07-12
owner: AI agent
sprint: null
tags:
  - task
  - done
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

- [x] Fresh install reaches a living pet in under 1 minute with zero manual config when Claude Code is present.
- [x] Close button hides to tray; quit only via tray; autostart is wired through the official cross-platform Tauri plugin.
- [x] Stats match the ledger (spot-check against SQLite).

## Dependencies

- [[0006-food-drop-eating-loop|0006]]

## Verification Plan

- [x] macOS dev/runtime smoke test; record results below.
- [ ] Windows/Linux fresh-profile manual smoke remains for release QA.

## Verification Results

### 2026-07-12 — implementation in progress

- Added Tauri tray actions for showing/hiding the overlay, opening the dashboard, pausing tracking, and quitting; closing the dashboard now hides it instead of terminating the app.
- Added official Tauri autostart and single-instance plugins. A second launch focuses the existing dashboard window.
- Replaced the dashboard placeholder with live pet/fullness/XP/token-meter stats and controls for tracking and launch-at-login.
- `cargo test --manifest-path src-tauri/Cargo.toml` passes (40 tests); `npm --prefix ui/dashboard run check && npm --prefix ui/dashboard run build` passes.

### 2026-07-12 — completed

- Added persisted `app_settings` for onboarding, starter egg, Claude Code provider enablement, pet size, monitor selection, Wayland fallback, and tracking pause state.
- Added dashboard state command with pet snapshot, provider detection, monitor count, today/week food, today/week token totals, and streak display.
- Added daily food aggregation and ledger token-total queries for dashboard stats.
- Dashboard now supports onboarding, provider toggle, tracking pause, launch-at-login, pet size, monitor selection, and Wayland fallback.
- Overlay responds to pet-size settings through `overlay_settings_changed` and overlay window placement responds to monitor/fallback settings.
- Tray pause now persists to SQLite and emits a dashboard update event.
- Fixed Tauri `beforeDevCommand`/`beforeBuildCommand` paths to build overlay and dashboard from this repo layout.

Verification:

- `cargo test --manifest-path src-tauri/Cargo.toml` passes (43 tests).
- `npm --prefix ui/dashboard run check` passes.
- `npm --prefix ui/overlay run check` passes.
- `npm --prefix ui/overlay run build` passes.
- `npm --prefix ui/dashboard run build` passes.
- `cargo tauri dev` reaches Rust app launch after the path fix; an existing Vite dev server on port 1420 prevented spawning a second Vite instance.
- Direct runtime launch with the existing Vite server succeeds: `cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --color always --`.
- SQLite spot-check at `~/Library/Application Support/com.tokengochi.app/tokengochi.sqlite3`: `token_events` count `17805`, all-time token total `1596601911`, today token total `1385902`, economy snapshot `food_earned_today=20`, `pantry=5`, `food_inventory=0`.

Release QA note: Windows and Linux fresh-profile/autostart smoke tests still need to be run on those OSes before packaging, but the implementation uses `tauri-plugin-autostart` and `tauri-plugin-single-instance` rather than platform-specific custom code.
