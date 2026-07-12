---
type: knowledge
status: active
created: 2026-07-12
updated: 2026-07-12
tags:
  - knowledge
  - overlay
  - platform
  - ai-context
owner: AI agent
---

# Overlay Platform Notes

Findings from task [[../tasks/active/0002-pet-overlay-window-spike|0002 - pet overlay window spike]]. Related: [[../architecture|Architecture]] §Important Constraints, [[../decisions/0003-canvas-sprite-rendering|ADR-0003]].

**Caveat up front:** this implementation was written and code-reviewed in a sandbox with no display server and no Windows/macOS machines - none of the per-OS behavior below has been hands-on tested yet. It's the documented, best-available-knowledge basis for the fallback decision, not a confirmed test matrix. See the task's Verification Results for what still needs a human on each OS.

## What the spike implements

- `overlay` window (`src-tauri/tauri.conf.json`): transparent, undecorated, always-on-top, skip-taskbar, non-resizable, non-focusable at creation (`focus: false`), `acceptFirstMouse: true` for macOS so the first click on the pet isn't swallowed while the window is unfocused.
- Click-through: `ui/overlay/src/main.ts` hit-tests the cursor against the test square on every `mousemove` and calls `Window.setIgnoreCursorEvents(!hit)` only when the hit state flips - ignored (click-through) everywhere else, captured over the square.
- Dragging: `mousedown` over the square calls `Window.startDragging()`.
- Multi-monitor: `src-tauri/src/overlay_window.rs` resizes/repositions the window to the primary monitor's work area on startup via `window.primary_monitor()`.
- Idle CPU: the square alternates every ~6s between an "active" tick (~30fps, bouncing) and an "idle" tick (~2fps, stationary), matching the tick budget in `docs/architecture.md` §Runtime and Deployment.

## Per-OS expectations (from documented Tauri/OS behavior)

### Windows

`setIgnoreCursorEvents`/transparent/always-on-top/skip-taskbar are all well-trodden WebView2 + Win32 layered-window paths in Tauri; expected to work with no major surprises. Watch for: DPI scaling on multi-monitor setups with different scale factors (physical vs. logical pixel mismatches).

### macOS

Maps to `NSWindow` borderless + `setIgnoresMouseEvents:` + window level for always-on-top + `collectionBehavior`/`NSWindow.isExcludedFromWindowsMenu`-style flags for skip-taskbar. This is the same mechanism classic macOS desktop pets use; expected to work. Watch for: Retina (HiDPI) coordinate handling, and Spaces/Mission Control interaction with always-on-top windows (may need `canJoinAllSpaces` behavior later).

### Linux X11

Override-redirect/always-on-top/transparent windows work via webkit2gtk + GTK, given a compositing window manager (most modern DEs ship one). Click-through maps to an input-shape region. Watch for: transparency renders as opaque/black on a non-compositing WM - worth a startup check + a documented "enable compositing" note for users on minimal X11 setups.

### Linux Wayland

**This is the real risk, and where a fallback is needed.** Wayland's security model deliberately prevents a normal `xdg-toplevel` surface from knowing or setting its absolute screen position, and there's no standard protocol for a positioned, click-through, always-on-top overlay outside of `wlr-layer-shell` - supported by wlroots-based compositors (Sway, Hyprland) but **not** by GNOME (Mutter) or KDE (KWin) without non-default extensions. Tauri/webkit2gtk does not implement layer-shell today. So on GNOME/KDE Wayland (the common case), true desktop-wide click-through overlay positioning is not expected to work.

**Fallback decision:** detect Wayland at startup (`WAYLAND_DISPLAY` env var present, or `XDG_SESSION_TYPE=wayland`) and, when detected, don't attempt the primary-monitor-covering overlay. Instead run a **docked corner window**: a small, normally-positioned window (whatever position the compositor grants) sized to just the pet, always-on-top where the compositor allows it, and *not* click-through - it behaves like an ordinary small widget window the user can move/minimize, trading "wanders your whole desktop" for "lives in a corner." This matches the already-flagged open question in `docs/architecture.md`.

This fallback is a documented decision based on how Wayland/wlr-layer-shell work, not something confirmed by hands-on testing on an actual GNOME/KDE Wayland session - flagging that explicitly rather than claiming it's verified. The detection check itself is not yet wired into the code (out of scope for this spike per its "prove or disprove the approach" goal - implementing the actual fallback window is follow-up work once someone confirms the primary approach fails on a real Wayland session).

## Open follow-ups

- Wire up the `WAYLAND_DISPLAY`/`XDG_SESSION_TYPE` detection and docked-corner fallback window for real (currently just this decision doc).
- Confirm DPI/HiDPI coordinate handling once tested on a real multi-monitor Windows/macOS setup.
- Confirm X11 compositor requirement and write a user-facing message if compositing is off.
