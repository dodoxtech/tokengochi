---
type: decision
status: accepted
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
tags:
  - decision
  - architecture
---

# Decision: Use Tauri v2 (Rust + web frontend) as the app framework

## Status

Accepted

## Date

2026-07-11

## Context

Tokengochi must run on Windows, macOS, and Ubuntu as a lightweight always-running desktop pet: transparent always-on-top overlay window, click-through, system tray, autostart, file watching, and a small footprint (<100 MB RAM). It also needs a normal dashboard UI.

## Decision

Build with **Tauri v2**: Rust core for token watching, economy, persistence, tray; web frontend (Canvas 2D overlay + Svelte dashboard).

## Consequences

Positive:

- Small binaries (~10 MB) and low RAM vs Electron (~150–300 MB).
- First-class transparent/always-on-top/skip-taskbar window flags and `set_ignore_cursor_events` for click-through.
- Rust is ideal for the log-tailing watcher and a tamper-resistant economy engine; web tech is ideal for the dashboard UI.
- Official bundlers + updater for all three OS targets.

Negative or tradeoffs:

- Three system webviews (WebView2/WKWebView/webkit2gtk) → minor rendering differences; canvas keeps overlay rendering consistent.
- Rust learning curve is higher than JS-only.
- Linux Wayland overlay behavior is compositor-dependent regardless of framework.

## Alternatives Considered

- **Electron:** easiest dev, but 10–20× the RAM for an app meant to run all day; unattractive for an ambient utility.
- **Godot:** great animation tooling, but transparent click-through desktop overlays and tray integration are awkward and per-OS hacks; overkill for one sprite.
- **Flutter Desktop:** decent, but weaker story for transparent click-through overlays and system-level integration on Linux.
- **Native per OS (Swift/C#/GTK):** best integration, triple maintenance cost — not viable.

## References

- [[../architecture|Architecture]]
- [[0003-canvas-sprite-rendering|ADR-0003]]
