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

# Decision: Render the pet with Canvas 2D sprite sheets; game truth lives in Rust

## Status

Accepted

## Date

2026-07-11

## Context

The overlay must draw one pixel-art pet plus occasional food items at 30 fps, over a transparent window, identically on three OS webviews, with near-zero idle cost. Options ranged from plain Canvas 2D to WebGL/PixiJS to rendering natively in Rust.

## Decision

- **Canvas 2D** with pre-rendered sprite sheets (Aseprite exports, nearest-neighbor scaling, `image-rendering: pixelated`).
- Behavior AI (wander/seek/eat/sleep state machine) runs in the overlay's TypeScript at a variable tick (30 fps active, 2 fps idle).
- All *game truth* (XP, fullness, streaks, inventory) lives in the Rust core; the frontend only animates and reports interactions (`pet_ate`, `pet_clicked`).

## Consequences

Positive:

- Simplest possible pipeline for a handful of sprites; no engine or WebGL context cost on a transparent window.
- Consistent output across WebView2/WKWebView/webkit2gtk.
- Artist workflow = Aseprite → PNG sheet + JSON, hot-swappable for cosmetics/food skins.
- Rust-side truth makes the economy unit-testable and resistant to frontend tampering.

Negative or tradeoffs:

- No engine niceties (particles, tweening) — hand-rolled, acceptable at this scope.
- If v2 wants many simultaneous entities (multi-pet), revisit with PixiJS.

## Alternatives Considered

- **PixiJS/WebGL:** overkill for ~2 sprites; WebGL on transparent webviews has flaky vendor behavior.
- **DOM/CSS sprite animation:** fine for the pet, awkward for hit-testing driven click-through toggling.
- **Native Rust rendering (wgpu/softbuffer):** best perf, but loses webview and doubles UI stacks.

## References

- [[../architecture|Architecture]]
- [[0001-tauri-stack|ADR-0001]]
