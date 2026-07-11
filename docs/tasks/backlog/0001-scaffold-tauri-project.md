---
type: task
status: backlog
priority: P0
delivery_order: 0001
estimate: 1d
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
sprint: null
tags:
  - task
  - backlog
---

# Task: Scaffold Tauri v2 project

## Context

Foundation for everything. Stack decided in [[../../decisions/0001-tauri-stack|ADR-0001]]; target layout in [[../../architecture|Architecture]] §Project Structure.

## Goal

A running Tauri v2 app with the planned module layout, Svelte dashboard shell, and CI checks (fmt, clippy, tests) on all three OS targets.

## Scope

In scope: `create-tauri-app` (Svelte + TS), `src-tauri` module skeleton (`watcher/`, `economy/`, `pet/`, `store/`), `economy.toml` with constants from [[../../knowledge/game-economy|Game Economy]] §8, GitHub Actions build matrix.

Out of scope: any game logic, overlay window, packaging/signing.

## Acceptance Criteria

- [ ] `cargo tauri dev` opens the dashboard shell on the dev machine.
- [ ] CI builds pass on Windows, macOS, Ubuntu.
- [ ] `economy.toml` is loaded and exposed via a `get_config` command.

## Dependencies

None.

## Implementation Notes

- Pin Tauri v2 stable; enable `tray-icon` and `devtools` features.

## References

- [[../../architecture|Architecture]]

## Verification Plan

- [ ] `cargo tauri dev`, `cargo test`, CI green; record results below.

## Verification Results

TBD
