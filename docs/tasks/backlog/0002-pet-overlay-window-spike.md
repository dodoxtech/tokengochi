---
type: task
status: backlog
priority: P0
delivery_order: 0002
estimate: 3d
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
sprint: null
tags:
  - task
  - backlog
  - spike
---

# Task: Spike — transparent click-through pet overlay window on all 3 OS

## Context

Highest technical risk in the project ([[../../architecture|Architecture]] §Important Constraints): a transparent, borderless, always-on-top window that is click-through everywhere *except* over the pet sprite, on Windows, macOS, and Ubuntu (X11 + Wayland assessment).

## Goal

Prove (or disprove) the overlay approach per OS and document per-platform findings before building on it.

## Scope

In scope: transparent window flags, `set_ignore_cursor_events` toggling driven by canvas hit-testing on mouse move, multi-monitor positioning, skip-taskbar, a moving test square. Wayland: document what works per compositor (GNOME/KDE) and specify the fallback (docked corner window).

Out of scope: real sprites, behavior AI, economy.

## Acceptance Criteria

- [ ] Test square is visible over other apps, draggable, and clicks pass through empty regions on Win/macOS/X11.
- [ ] Findings + Wayland fallback decision written to `docs/knowledge/overlay-platform-notes.md`.
- [ ] CPU <1% while square is idle.

## Dependencies

- [[0001-scaffold-tauri-project|0001]]

## Risks

- Wayland may not allow global positioning → fallback mode must be acceptable UX.
- webkit2gtk transparency quirks on some distros.

## References

- [[../../decisions/0003-canvas-sprite-rendering|ADR-0003]]

## Verification Plan

- [ ] Manual test matrix on all 3 OS (VMs acceptable); record per-OS results below.

## Verification Results

TBD
