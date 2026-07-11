---
type: product
status: active
created: 2026-07-10
updated: 2026-07-11
tags:
  - product
  - ai-context
---

# Product Context

## Summary

**Tokengochi** is a cross-platform desktop pet (Windows, macOS, Ubuntu). A pixel-art creature lives on top of the user's screen and is fed by the user's real AI token consumption. Every fixed amount of tokens spent (primarily via Claude Code; other LLM providers via plugins) converts into food that drops onto the screen. The pet runs over, eats it, and grows.

The core insight: developers already burn tokens all day. Tokengochi turns that invisible metric into an ambient, emotional companion — a Tamagotchi powered by your AI usage.

## Users

- Primary: developers who use Claude Code daily and keep an eye on token usage.
- Secondary: power users of other LLM CLIs/tools (Codex CLI, OpenAI API, Cursor) via provider plugins.
- Tertiary: anyone who wants a desktop pet and can use manual/demo feeding mode.

## Goals

- Make token usage visible and fun instead of anxiety-inducing.
- A pet that feels alive: wanders the screen edge/taskbar, idles, sleeps, reacts to food.
- A long-term economy that rewards consistent daily usage without incentivizing wasteful token burn (see [[knowledge/game-economy|Game Economy]]).
- Near-zero footprint: <100 MB RAM, negligible CPU when idle, no network calls required for the core loop.
- Ship the same experience on Windows, macOS, and Ubuntu (X11 first; Wayland best-effort).

## Non-Goals

- Not a token cost dashboard or analytics tool (a small stats panel is enough).
- No multiplayer/online features in v1 (no accounts, no server).
- No real-money purchases; the economy is fully earned in-app.
- Not encouraging users to waste tokens to feed the pet — economy uses diminishing returns and daily caps to prevent this.
- No 3D rendering.

## Core Workflows

1. **Passive loop (the product):** user works in Claude Code → token watcher tails local session logs → tokens accumulate in a food meter → meter hits threshold → food drops on screen → pet walks to it, eats, gains XP/fullness → pet grows and evolves over days/weeks.
2. **Interaction:** click/drag pet, pet reacts; hover shows mood + fullness; tray icon opens stats and settings.
3. **Setup:** install → pick a starter egg → app auto-detects Claude Code logs → zero-config start. Optional: enable other providers or manual mode in settings.
4. **Progression:** daily streaks, evolution stages, unlockable cosmetics and food skins; collection album of evolved forms (see [[knowledge/game-economy|Game Economy]]).

## Open Questions

- Wayland: global overlay positioning is restricted; may need per-compositor fallbacks or a "windowed sandbox" mode.
- Should multiple pets be supported in v2 (one per project/provider)?
- Do we surface token *cost* (USD) at all, or keep it purely playful?
- Distribution channels: GitHub Releases only, or also Homebrew/winget/Snap?
