---
type: task
status: done
priority: P2
delivery_order: 0015
estimate: 0.25d
created: 2026-07-13
updated: 2026-07-13
owner: AI agent
sprint: null
tags:
  - task
  - done
---

# Task: Hide dashboard on normal startup

## Status

- State: done
- Created: 2026-07-13
- Owner: AI agent
- Priority: P2
- Delivery order: 0015
- Estimate: 0.25d
- Sprint: null

## Context

Tokengochi is intended to feel tray-first after setup: the pet overlay can stay active while the dashboard stays out of the way until the user opens it.

Related:

- [[../README|Tasks]]
- [[../../architecture|Architecture]]
- [[0007-tray-settings-dashboard|System tray, settings, and dashboard MVP]]

## Goal

Keep the dashboard hidden during normal app startup, while preserving first-run onboarding and tray/single-instance access.

## Scope

In scope:

- Start the `main` dashboard window hidden.
- Show the dashboard automatically only when onboarding has not been completed.
- Preserve tray "Open dashboard" and single-instance dashboard focus behavior.

Out of scope:

- Changing overlay visibility or tray menu labels.
- Redesigning dashboard UI.

## Acceptance Criteria

- [x] Returning users can launch the app without the dashboard appearing.
- [x] First-run users still see onboarding.
- [x] Opening the dashboard from tray or a second app launch still works.

## Dependencies

- [[0007-tray-settings-dashboard|System tray, settings, and dashboard MVP]]

## Risks

- Dashboard initialization now happens while the window is hidden for returning users; no app state should depend on immediate dashboard rendering.

## Implementation Notes

- `src-tauri/tauri.conf.json` sets the `main` window `visible` flag to `false`.
- `src-tauri/src/lib.rs` calls `tray::show_dashboard` during setup only when `settings.onboarding_complete` is false.
- Existing `tray::show_dashboard` remains the single path for tray and single-instance dashboard opening.

## References

- [[../../README|Documentation]]
- [[../../index|Project Map]]
- `src-tauri/tauri.conf.json`
- `src-tauri/src/lib.rs`
- `src-tauri/src/tray.rs`

## Verification Plan

- [x] Run Rust tests.
- [x] Run dashboard type/check command.
- [x] Run dashboard build.

## Verification Results

- `cargo test --manifest-path src-tauri/Cargo.toml` passed.
- `npm --prefix ui/dashboard run check` passed.
- `npm --prefix ui/dashboard run build` passed.

## Completion Notes

- Completed: 2026-07-13
- Changed files: `src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs`, `docs/architecture.md`, `docs/tasks/done/0015-hide-dashboard-on-normal-startup.md`
- Follow-ups: none.
