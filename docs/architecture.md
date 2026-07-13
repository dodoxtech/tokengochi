---
type: architecture
status: active
created: 2026-07-10
updated: 2026-07-11
tags:
  - architecture
  - ai-context
---

# Architecture

## System Overview

Tokengochi is a **Tauri v2** app (Rust core + web frontend) with two windows:

1. **Pet overlay window** — transparent, borderless, always-on-top, click-through except on the pet/food sprites. Renders the pixel-art pet on an HTML canvas.
2. **Dashboard window** (hidden during normal startup; opened from tray, or shown for first-run onboarding) — stats, settings, collection album, cosmetics shop.

```
┌────────────────────── Rust core (Tauri) ──────────────────────┐
│  Token Watcher ──► Economy Engine ──► Pet State Machine       │
│   (provider        (conversion,        (mood, hunger,          │
│    plugins,         caps, streaks,      evolution)             │
│    file tailing)    XP, sparks)             │                  │
│        │                 │                  │                  │
│        └───── SQLite (rusqlite) ◄───────────┘                  │
│                          │ events (tauri emit)                 │
└──────────────────────────┼─────────────────────────────────────┘
                           ▼
        ┌─ Overlay window ─────────┐   ┌─ Dashboard window ─┐
        │ Canvas sprite renderer,  │   │ Stats, settings,   │
        │ behavior AI (wander,     │   │ album, shop        │
        │ seek food, eat, sleep)   │   │ (Svelte)           │
        └──────────────────────────┘   └────────────────────┘
```

Decisions: [[decisions/0001-tauri-stack|ADR-0001]], [[decisions/0002-token-source-local-logs|ADR-0002]], [[decisions/0003-canvas-sprite-rendering|ADR-0003]].

## Project Structure

Planned layout (not yet scaffolded):

```
src-tauri/
  src/
    watcher/        # TokenProvider trait + claude_code.rs, openai.rs, manual.rs
    economy/        # conversion, caps, streaks, xp (pure functions, heavily unit-tested)
    pet/            # state machine: mood, hunger decay, evolution
    store/          # SQLite persistence, migrations
    tray.rs         # system tray, autostart
  economy.toml      # balance constants (see game-economy §8)
ui/
  overlay/          # canvas renderer + behavior AI (TypeScript)
  dashboard/        # Svelte app
  assets/sprites/   # pixel sprite sheets (aseprite exports)
```

## Key Dependencies

- **Tauri v2** — windowing, tray, autostart, IPC, updater.
- **rusqlite** — local state (pet, ledger, inventory).
- **notify** (Rust) — filesystem watching of provider log dirs.
- **Svelte + Vite** — dashboard UI (small bundle, no runtime).
- Plain **Canvas 2D** for the pet renderer — no game engine needed for one sprite (ADR-0003).

## Data Flow

1. `TokenProvider` (trait) implementations tail their sources and emit `TokenEvent { provider, input, output, cache_read, ts }`. Claude Code provider tails `~/.claude/projects/**/*.jsonl` (see [[knowledge/token-tracking|Token Tracking]]).
2. Economy engine applies weights, caps, and diminishing returns ([[knowledge/game-economy|Game Economy]] §2), appends to a token ledger in SQLite, and emits `FoodSpawned` when the meter crosses the threshold.
3. Overlay window receives `FoodSpawned` via Tauri event, drops a food sprite at a random reachable point; behavior AI switches pet to seek → eat.
4. On eat, frontend calls `pet_ate` command → Rust updates fullness/XP/streak, persists, emits `PetStateChanged` back to both windows.
5. Hunger decay + Pantry auto-feed run on a Rust-side timer (also reconciled on app launch using elapsed wall time, so state is correct after the machine sleeps or the app is closed).

All game-truth lives in Rust; the frontend is presentation + behavior animation only. This keeps the economy tamper-resistant enough and testable.

## Runtime and Deployment

- Targets: Windows 10+ (WebView2), macOS 12+ (WKWebView), Ubuntu 22.04+ (webkit2gtk; X11 fully, Wayland best-effort).
- Single instance, launches at login (opt-in), lives in system tray.
- Bundles: `.msi`/NSIS, `.dmg`, `.deb` + AppImage via `tauri bundle`; CI on GitHub Actions matrix; auto-update via `tauri-plugin-updater` (GitHub Releases).
- Footprint budget: <100 MB RAM, <1% CPU idle (behavior AI ticks at 30 fps only while pet is visible and moving; drops to 2 fps when idle/sleeping).

## Important Constraints

- **No network required** for the core loop; the only network use is the auto-updater.
- **Privacy:** only per-message `usage` numbers are read from Claude Code logs — never message content. State stays local.
- **Wayland:** global always-on-top transparent overlays are compositor-dependent; fallback is a docked corner window ([[../product|Product Context]] open question).
- Click-through with interactive sprite regions requires per-OS window flags (`set_ignore_cursor_events` toggled on sprite hit-testing) — riskiest platform code, prototype first ([[tasks/backlog/0002-pet-overlay-window-spike|task 0002]]).
- Log formats of providers are unstable; parsers must be defensive and versioned per provider.
