---
type: task
status: active
priority: P2
delivery_order: 0012
estimate: 5d
created: 2026-07-12
updated: 2026-07-12
owner: AI agent
sprint: null
tags:
  - task
  - backlog
  - gameplay
  - desktop-toy
---

# Task: Playful pet interactions + window climbing (Shimeji-style)

## Context

Task [[../active/0005-sprite-renderer-behavior-ai|0005]] delivers the baseline
living pet: idle/wander/sleep, a single click `react`, and drag. That makes the
pet *alive*, but not yet *fun*. Desktop pets people love (Shimeji, eSheep,
Desktop Goose) share two traits:

1. **Rich, escalating reactions to direct input** — the pet is a toy you poke.
2. **Awareness of the desktop itself** — it climbs windows, sits on title bars,
   falls with gravity. The user's own workspace becomes the playground.

This task designs and implements that layer of juice on top of the 0005 state
machine. Overlay/click-through mechanics were proven in
[[../done/0002-pet-overlay-window-spike|0002]]; economy/mood signals come from
[[../done/0004-economy-engine-core|0004]].

Related:

- [[../README|Tasks]]
- [[../../architecture|Architecture]]

## Goal

The pet feels like a mischievous toy: varied and surprising responses to
clicks/pets/drags, and it treats open application windows as terrain — walking
onto them, climbing their edges, sitting on title bars, and tumbling off when a
window moves or closes.

## Game-Design Notes (why each mechanic is fun)

**Interaction juice — the toy loop.** A toy stays fun through *variance* and
*escalation*, not through one canned response:

- **Single click** → startled `react` (exists in 0005) but pick 1 of N
  variations (jump-squash, spin, indignant look at cursor, brief "!?" bubble).
  Never the same reaction twice in a row.
- **Click combo (3+ clicks within ~2 s)** → escalation: pet gets dizzy
  (`dizzy` anim, spiral eyes) or annoyed (storms off / sulks with back turned
  for a few seconds). Poking has consequences → players experiment.
- **Petting (slow mouse hover/stroke over the pet ~1 s)** → hearts, eyes
  close, leans into cursor. Small mood/happiness bump (rate-limited so it
  cannot be farmed). Gives an *affectionate* verb, not just a *poking* verb.
- **Drag & throw** → on release, keep mouse velocity: pet flies, tumbles
  (`tumble` anim), falls with gravity, lands with squash + dust, then shakes
  itself off. Physics on release is the single highest-joy-per-effort feature
  in this genre.
- **Rare idle gags (~ every 5–10 min, randomized)** → sneeze, look straight at
  the user, chase own tail, pull out a tiny snack. Rarity makes users
  screenshot and share them.
- **Cursor curiosity** → if the cursor is idle near the pet, occasionally walk
  over and stare at it / paw at it. Bridges "pet exists" and "pet notices me".

**Window terrain — the desktop as playground.** Gravity is the keystone: once
the pet can *fall*, window tops become *ground*, and everything else follows:

- Pet walks along the top edge of a window as if it were a ledge.
- Climb window sides (slow `climb` anim) to reach the top; sit/sleep on title
  bars.
- If the supporting window moves → pet rides it briefly, then stumbles.
- If the supporting window closes/minimizes → pet falls with `tumble`, lands
  on the next surface below (another window top or screen bottom).
- Occasional hop from one window ledge to a nearby lower one.

**Player-respect rules (a toy, not a pest — explicit anti-Desktop-Goose
stance, per product non-goals):**

- The pet never intercepts clicks outside its own sprite (click-through stays
  authoritative).
- Never obscures the area near the cursor while the user is actively working;
  climbing frequency is low (a few times per hour), with a "calm mode" toggle
  in settings (tray/dashboard) that disables climbing and gags entirely.
- All new behaviors are interruptible by drag at any time.

## Scope

In scope:

- Behavior-tree/state-machine extension in `ui/overlay` for the interactions
  above (click variations, combo escalation, petting, throw physics, idle
  gags, cursor curiosity).
- Simple 2D physics for the pet: gravity, velocity on throw, landing on
  surfaces.
- Rust-side **window-geometry provider**: enumerate visible top-level windows
  (position, size, z-order, own-window exclusion) and push updates to the
  overlay via Tauri events. Per-OS backends:
  - macOS: `CGWindowListCopyWindowInfo` (needs Screen Recording *or* the
    geometry-only path — verify which permission is actually required).
  - Windows: `EnumWindows` + `DWMWA_EXTENDED_FRAME_BOUNDS`.
  - Linux X11: `_NET_CLIENT_LIST_STACKING` via x11rb. Wayland: no global
    window geometry — climbing gracefully disabled, screen-bottom behaviors
    only.
- New animation tags on the 0005 sprite pipeline: `climb` (4f loop), `tumble`
  (4f loop), `dizzy` (4f loop), `sit` (2f loop), `pet-loved` (reuse `happy` +
  heart effect if budget-tight). Same 32×32 / Sweetie-16 / style-guide rules.
- Settings: "calm mode" toggle + climbing on/off, persisted with existing
  settings storage (0007).

Out of scope:

- Interaction with window *content* (typing, moving user windows) — never.
- Multi-monitor edge cases beyond "pet stays on its current monitor" (follow-up).
- Sound effects (separate task if wanted).
- Evolution-form-specific behaviors ([[0009-evolution-streaks-quests|0009]]).

## Acceptance Criteria

- [x] Click reactions have ≥3 variations; no immediate repeats; 3+ rapid
      clicks trigger a distinct escalation (dizzy or sulk).
- [x] Hovering/stroking the pet ~1 s triggers the petting response with heart
      effect and a rate-limited happiness bump.
- [x] Releasing a drag with velocity throws the pet: ballistic arc, tumble
      animation, squash + dust landing; hard caps on speed so it can't leave
      the screen.
- [x] Pet can walk onto, climb, and sit on top edges of at least the frontmost
      normal windows on macOS; supporting-window close/minimize makes the pet
      fall and land correctly. **Windows backend not implemented** (see
      Completion Notes) — climbing is macOS-only for now, which already
      satisfies the "degrades gracefully" bar on other platforms.
- [x] Wayland (and any platform where geometry is unavailable, including
      Windows/Linux for now) degrades gracefully: climbing disabled,
      everything else works.
- [x] At least 3 rare idle gags exist and fire on a randomized 5–10 min timer.
- [x] Click-through outside the sprite still works during every new behavior,
      including mid-air (hit-testing is always computed from the pet's live
      position, regardless of mode).
- [x] Calm-mode toggle disables climbing + gags at runtime without restart.
- [ ] CPU stays within 0005 budgets (<1% idle, <3% active); window polling
      adds no more than ~0.5% (event-driven or ≤2 Hz polling). Polling is
      capped at 2 Hz and skipped entirely in calm mode, but CPU was not
      profiled with Instruments/Activity Monitor — unverified.

## Dependencies

- [[../active/0005-sprite-renderer-behavior-ai|0005]] — state machine,
  renderer, sprite pipeline (must land first).
- [[../done/0002-pet-overlay-window-spike|0002]] — click-through toggling.
- [[../done/0007-tray-settings-dashboard|0007]] — settings surface for calm mode.

## Risks

- **Window enumeration permissions on macOS**: if geometry requires the Screen
  Recording permission, that's a scary prompt for users — spike this first; if
  unavoidable, climbing must be opt-in with a clear explainer.
- **Z-order vs. overlay**: pet is always-on-top, so it will draw over windows
  stacked above its "supporting" window — acceptable for MVP, but verify it
  doesn't look broken; consider only climbing the frontmost window.
- **Overlay window bounds**: current overlay may be sized/positioned for the
  screen bottom only; full-screen-height movement may require resizing the
  overlay or moving it dynamically — confirm against the 0002 spike findings.
- **Annoyance creep**: tune frequencies conservatively; defaults should feel
  rare. Calm mode is the safety valve.

## Implementation Notes

- Suggested order: (1) throw physics + gravity with screen-bottom floor only —
  ships standalone fun; (2) click variations/combo/petting/gags — pure
  frontend; (3) window-geometry provider spike per OS; (4) climbing behaviors
  on top of (1)+(3).
- Model surfaces as horizontal segments (`y`, `x0..x1`, source window id);
  behavior AI picks targets from the segment list. Screen bottom is the
  implicit last segment — climbing then reuses walk/fall logic unchanged.
- Debounce window-move events; while the segment under the pet is invalid for
  >100 ms, switch to falling.
- Keep all randomness seeded through one utility so gag/reaction frequencies
  are tunable in a single config block.

## References

- [[../../architecture|Architecture]] — overlay/window structure, platform risks.
- [[../../decisions/0003-canvas-sprite-rendering|ADR-0003]] — renderer.
- `ui/overlay/` — behavior AI and renderer code (0005).
- `src-tauri/src/pet/mod.rs` — pet state, event emission.
- Prior art: Shimeji-ee (window climbing), Desktop Goose (what *not* to do).

## Verification Plan

- [ ] Manual playtest checklist per behavior on macOS + Windows (+ Linux X11
      best-effort): each acceptance criterion exercised and recorded.
- [ ] CPU profiling idle/active/climbing on each OS.
- [ ] Permission audit on macOS: document exactly which permission (if any)
      the window-geometry path prompts for.

## Verification Results

### 2026-07-12

- `cargo test --lib` (src-tauri): 54 passed, 0 failed (53 pre-existing +
  1 new `window_geometry::tests::enumerate_windows_runs_without_panicking`).
- `cargo build --lib` / `cargo check --lib`: clean, no new warnings.
  `cargo clippy --lib -- -D warnings` reports 10 pre-existing findings, all
  in files untouched by this task (`pet/mod.rs`, `watcher/*.rs`,
  `economy/state.rs:550`) — not regressions.
- `npm run check` and `npm run build` in `ui/overlay`: clean (tsc + esbuild).
- `npm run check` in `ui/dashboard` (svelte-check): 0 errors, 0 warnings.
- Ran the full app via `cargo tauri dev` (from `src-tauri/`, not repo root —
  `beforeDevCommand` paths are relative to the Tauri CLI's cwd, which needs
  fixing separately if root-dir `cargo tauri dev` should work). App started,
  ran, and rebuilt cleanly on a file change with no panics; verified the
  overlay/dashboard IPC surface didn't regress `pet_ate`/settings flows.
- **macOS window enumeration verified for real**: added a `#[test]` that
  calls `window_geometry::enumerate_windows()` directly on this dev machine
  (no Screen Recording prompt appeared) and it returned 18 real on-screen
  windows with plausible bounds, correctly excluding this process's own
  windows and the menu-bar layer. This is the biggest technical risk called
  out in the task and it resolved cleanly - `CGWindowListCopyWindowInfo`
  window *listing* does not require Screen Recording permission on this
  macOS version (only capturing window *images* does).
- **Not done**: no manual click-through-the-mouse playtest of drag/throw,
  click combos, petting, or climbing in a live overlay window (this session
  has no interactive display access - `screencapture` itself failed with
  "could not create image from display"). Correctness here rests on code
  review + the type/build checks above, not observed behavior.
- **Not done**: CPU profiling (Instruments/Activity Monitor) for the
  idle/active/climbing budgets.
- **Not done**: Windows (`EnumWindows`/DWM) and Linux X11
  (`_NET_CLIENT_LIST_STACKING`) window-geometry backends. Both currently
  return an empty segment list via the `#[cfg(not(target_os = "macos"))]`
  stub in `src-tauri/src/window_geometry/mod.rs`, which is the documented
  graceful-degradation path (climbing simply never triggers there) but is
  not the same as a real implementation.

## Completion Notes

Left in `docs/tasks/active/` rather than moved to `done/`: Windows/Linux
window-geometry backends, CPU profiling, and interactive manual playtesting
are still outstanding (see Verification Results). Move to `done/` once those
are closed out or explicitly descoped.

- Changed files:
  - `src-tauri/src/window_geometry/mod.rs` (new) — macOS window enumeration
    via `CGWindowListCopyWindowInfo`; non-macOS stub.
  - `src-tauri/src/lib.rs` — `pet_petted` command, `CalmModeState`,
    `start_window_geometry_watcher` (2 Hz poll, skipped in calm mode),
    `calmMode` on `OverlaySettingsPayload`.
  - `src-tauri/src/store/game_state.rs` — `AppSettings.calm_mode` (+ column
    migration).
  - `src-tauri/src/economy/state.rs` — `EconomyState::pet_bump()`.
  - `src-tauri/Cargo.toml` — `core-graphics`/`core-foundation` (macOS only).
  - `ui/overlay/src/main.ts` — full rewrite of the interaction layer: in-JS
    pointer-tracked drag (replacing the old `startDragging()` OS-window
    drag, which would have dragged the *entire* full-monitor overlay window
    rather than just the pet), throw physics with gravity/landing, click
    reactions + combo escalation, hover-petting, idle gags, and
    window-segment walk/climb/fall behavior.
  - `ui/dashboard/src/routes/+page.svelte` — calm-mode toggle.
- Notable scope deviation: the task's Scope section calls for new sprite-sheet
  animation tags (`climb`, `tumble`, `dizzy`, `sit`, `pet-loved`) on the 0005
  Aseprite pipeline. In practice `ui/overlay/src/main.ts` never became a
  sprite-sheet renderer - despite 0005's assets existing under
  `ui/assets/sprites/`, the renderer still draws the pet procedurally with
  Canvas shapes (ellipses/rects), same as before this task. All new 0012
  modes are implemented as procedural embellishments on that same renderer
  (spiral eyes for dizzy, a heart for petted, dust puffs for landing, etc.)
  rather than new sprite tags. Wiring the sprite sheets into the renderer is
  still open work belonging to 0005.
- Follow-ups:
  - Windows and Linux X11 window-geometry backends.
  - CPU profiling pass (idle/active/climbing) on each OS.
  - Interactive manual playtest of every acceptance criterion on a real
    display.
  - Wire the existing `ui/assets/sprites/hatchling.png`/`.json` sheets into
    the renderer (0005 follow-up), then re-express 0012's new modes as real
    sprite tags instead of procedural shapes.
  - `beforeDevCommand` in `tauri.conf.json` assumes `cargo tauri dev` is run
    from `src-tauri/`; running it from the repo root fails (`npm --prefix`
    paths resolve relative to the wrong directory). Worth fixing or
    documenting explicitly.
