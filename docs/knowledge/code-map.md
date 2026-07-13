---
type: reference
status: active
created: 2026-07-13
updated: 2026-07-13
tags:
  - knowledge
  - ai-context
  - code-map
owner: engineering
---

# Code Map: Pet Behavior, Movement, and Actions

Index of where pet logic actually lives, so an agent (or human) can jump
straight to the right function instead of grepping the whole repo. Line
numbers are a snapshot from 2026-07-13 and will drift as the file changes —
if a line number looks wrong, search for the **function name** anchor given
below (`rg "fn functionName"`), it is more stable than the line number.

Related: [[architecture|Architecture]] (data flow), [[game-economy|Game Economy]] (balance numbers), [[../decisions/0003-canvas-sprite-rendering|ADR-0003]].

## Quick Answer Table

| I want to change... | Go to |
|---|---|
| How fast/where the pet walks | `ui/overlay/src/behavior.ts` → `updatePet()` (walk-to-food, walk-to-bed) |
| Falling / being thrown / bouncing off edges (drag-release only) | `ui/overlay/src/behavior.ts` → `updateTumble()`, `beginDrop()` |
| Climbing onto other app windows / jumping back down to eat or sleep | `ui/overlay/src/behavior.ts` → `maybeStartClimb()`, `updateClimb()`, `beginDescend()` |
| Idle random gags (sneeze/stare/chase-tail) | `ui/overlay/src/behavior.ts` → `maybeTriggerIdleGag()` |
| Adding a new pet form, or a new gag/expression (yawn, dance, drink-break, …) | [[pet-action-pack-spec|Pet Action Pack Spec]] (required tag contract); planned batch in [[../tasks/backlog/0014-expanded-gag-expression-pack|0014]] |
| Click reactions / rage-quit combo | `ui/overlay/src/behavior.ts` → `triggerClickReaction()`, `updateSulk()` |
| Petting / stroke detection | `ui/overlay/src/behavior.ts` → `maybeTriggerPetting()` |
| Dragging the pet with the mouse | `ui/overlay/src/input.ts` → `mousedown`/`mousemove`/`mouseup` listeners inside `initInput()` |
| Eating food (walk-to-food + eat timer) | `ui/overlay/src/behavior.ts` → `updatePet()` tail (food branch), `pet_ate` IPC |
| What animation/sprite plays for a mode | `ui/overlay/src/atlas.ts` → `MODE_ANIMATION_TAG`; `ui/overlay/src/render.ts` → `drawPet()` |
| Speed/physics tuning constants | `ui/overlay/src/constants.ts` |
| Mood bands / hunger / evolution stage | `src-tauri/src/pet/mod.rs` (`Mood`, `stage_for_level`, `EvolutionBranch`) |
| Fullness decay, XP, streaks math | `src-tauri/src/economy/*.rs` (see [[game-economy]]) |
| Persisted pet state (SQLite row) | `src-tauri/src/store/game_state.rs` |
| Tauri commands callable from the overlay (`pet_ate`, `pet_petted`, ...) | `src-tauri/src/lib.rs` (~L124-373, `#[tauri::command]` fns) |
| Click-through / hit-testing / window flags | `ui/overlay/src/state.ts` → `isOverPet()`; `ui/overlay/src/input.ts` → `updateHitTest()`; `src-tauri/src/overlay_window.rs` |
| Climbable window segments (other app windows) | `src-tauri/src/window_geometry/mod.rs` (`enumerate_windows`), emitted as `window_segments_changed` |
| Sprite atlas / animation frame lookup | `ui/overlay/src/atlas.ts` → `loadAtlas()`, `frameForTag()` |

## Frontend: `ui/overlay/src/` (behavior AI + renderer)

Originally one ~1150-line `main.ts`; split by responsibility (2026-07-13) so
each file stays reviewable on its own. `main.ts` is now just wiring: the
`tick()` game loop, DOM/Tauri event listener registration, and startup
sequence. It owns no game logic itself — everything below is imported.

Shared mutable state (the `pet` object, live server `state`, `foods[]`,
`windowSegments`, `hover`/`pointerDown` flags, `PET_SIZE`) lives in
`state.ts` and is imported by the other modules; where a module can't
reassign an imported `let` directly (JS/TS restriction on live bindings),
`state.ts` exposes a setter (`setState()`, `setHover()`, `setPointerDown()`,
`setWindowSegments()`).

- **`dom.ts`** — `canvas`, `ctx` (2D context), `appWindow` (Tauri window
  handle). Everything else that touches the DOM/Tauri window imports these.
- **`types.ts`** — `PetMode` (union of `BaseMode | PhysicsMode |
  OverrideMode`), `Segment` (a climbable window edge), `Food`,
  `PetStatePayload` and the other Tauri event payload shapes.
- **`constants.ts`** — `WALK_SPEED`, `GRAVITY`, `MAX_THROW_SPEED`,
  `CLIMB_SPEED` (slow ascent), `JUMP_UP_HEIGHT`/`JUMP_UP_SPEED` (anticipation
  hop before a deliberate jump-down), `JUMP_DOWN_SPEED` (fast, unlike the
  climb), `LANDING_PAUSE_MS` (recovery beat after landing), `CLICK_COMBO_COUNT`,
  gag/climb interval ranges, etc. Change pet speed/feel here first.
- **`atlas.ts`** — `AtlasFrame`/`AtlasJson`/`SpriteAtlas` types,
  `loadImage()`, `loadAtlas()`, `frameForTag()`, `MODE_ANIMATION_TAG`.
  Sprite sheets live in `ui/assets/sprites/` (aseprite exports).
  Sparks sink item sprites live in `ui/assets/sprites/items/` and are
  regenerated from `ui/assets/sprites/source/shop-items-master.png` via
  `ui/assets/sprites/scripts/generate-shop-items-from-master.mjs`. All
  sprite regeneration scripts live in `ui/assets/sprites/scripts/` — see
  [[sprite-asset-pipeline]].
- **`state.ts`** — the `pet` object (`x`, `y`, `vx`, `vy`, `mode`,
  `supportId`, `climbPhase`, `jumpPeakY`, ...), the live `state:
  PetStatePayload`, `foods[]`, `PET_SIZE`; geometry/window helpers
  `resizeCanvas()`, `applyOverlaySettings()`, `groundY()`, `petMaxX()`,
  `currentSegments()`, `landingSurfaceAt()` (decides what surface the pet
  lands on when falling — uses a strict `>` on `surfaceY` so the ledge the
  pet is falling *from* is never re-selected as its own landing target,
  which used to cause an infinite tumble/idle loop), `isOverPet()`,
  `pruneEatenFood()` (splices `eaten` entries out of `foods[]` — without it
  the array grew unbounded over a long-running session since eaten food was
  only ever flagged, never removed, and every tick/draw walked the whole
  array).
- **`behavior.ts`** (comment-marked sections for "Task 0012 physics/override
  modes" and the 0005/0006 baseline):
  - `beginDrop(now, vx, vy)` — enters free-fall (`mode = "tumble"`); reserved
    for an actual throw/drag release (see `input.ts`), not the pet's own
    decision to come down.
  - `updateTumble()` — gravity integration, bounces off screen edges,
    detects landing via `landingSurfaceAt()`; every landing (any surface, not
    just the floor) goes through the same `mode = "landing"` /
    `LANDING_PAUSE_MS` recovery beat described below.
  - `beginDescend()` — the pet's own deliberate way down from a ledge
    (`mode = "climb"`, `climbPhase = "jump-up"` then `"jump-fall"`): a short
    anticipation hop up (`JUMP_UP_HEIGHT`/`JUMP_UP_SPEED`) followed by a fast
    fall to `groundY()` (`JUMP_DOWN_SPEED`), ending in `mode = "landing"` for
    a `LANDING_PAUSE_MS` recovery beat before the pet acts again.
  - `maybeStartClimb()` / `updateClimb()` — picks a random other-app window
    edge from `windowSegments`, walks to it, climbs up (`CLIMB_SPEED`), pauses
    in `climbPhase = "landed"` for `LANDING_PAUSE_MS` (same recovery beat as
    any other landing), then sits indefinitely (`climbPhase = "sit"`) — it
    only calls `beginDescend()` to jump back down once there's unclaimed food
    waiting on the floor (or the ledge itself becomes invalid/walked off).
    There is no idle timer that pulls the pet down on its own.
  - `maybeTriggerIdleGag()` — random sneeze/stare/chase-tail.
  - `triggerClickReaction()` / `updateSulk()` — click combo → dizzy/sulk
    escalation; otherwise squash/spin/look/exclaim reaction.
  - `maybeTriggerPetting()` — hover-and-hold detection, calls `pet_petted`
    IPC (rate-limited client-side by `PET_BUMP_COOLDOWN_MS`, mirrored
    server-side).
  - `updatePet(dtMs, now)` — the main state-machine dispatcher. Mode
    precedence: `dragged` → `tumble` → `climb` → time-boxed overrides
    (`dizzy`/`sulk`/`react`/`petted`/`gag`/`landing`) → petting check → if
    not on the floor, `beginDescend()` *only when* there's unclaimed food
    waiting (otherwise stays put on the ledge) → seek nearest unclaimed food
    → walk to bed when idle → idle gag/climb rolls → `idle`. Food-eating
    sub-branch at the bottom calls `invoke("pet_ate")`.
  - `updateFood(dtMs, now)` — animates queued food sprites dropping to the
    floor; stamps `food.landedAt = now` the frame each one reaches
    `targetY`, which `render.ts` reads to draw a brief landing bounce.
- **`render.ts`** — `drawPet()` (picks the idle animation tag instead of
  `MODE_ANIMATION_TAG.climb` while `climbPhase === "sit"`, since sitting on a
  ledge should read as idle, not mid-climb), `drawSpiralEyes()` (dizzy),
  `drawOverlayEffects()`/`drawEffect()`/`drawGagEffect()`, `drawCosmetic()`,
  `drawFood()`/`drawFoodSkin()`, `drawFurniture()`, `drawTooltip()`,
  `draw()` (composes the frame — called once per `tick()`).
- **`input.ts`** — `updateHitTest()` (toggles OS click-through so clicks
  pass through except over the pet sprite), `pollCursorForHover()` (breaks
  the click-through chicken-and-egg deadlock), `initInput()` registers
  `mousemove` (drag tracking, `DRAG_PROMOTE_PX` threshold turns a click into
  a drag → `mode = "dragged"`), `mousedown`, `mouseup` (releasing a drag
  calls `beginDrop()` with throw velocity, capped by `MAX_THROW_SPEED`),
  `blur`.
- **`main.ts`** — `tick(now)` (computes `dtMs`, throttles to
  `ACTIVE_TICK_MS` (30fps moving) or `IDLE_TICK_MS` (2fps idle/sleeping) —
  stays on the active rate through `LANDING_PAUSE_MS` after a landing so the
  jump-down/recovery beat doesn't stutter, calls `updatePet()` +
  `updateFood()` + `pruneEatenFood()` + `draw()`); Tauri event listeners
  `food_spawned` (push into `foods[]`), `pet_state_changed`,
  `overlay_settings_changed`, `window_segments_changed` (updates
  `windowSegments` used by climb logic); initial `get_pet_state` invoke and
  `initInput()` call on startup.

## Backend: Rust (`src-tauri/src/`)

- **`pet/mod.rs`** (228 lines) — pure data/enums, no I/O: `EvolutionStage`,
  `EvolutionBranch` (+ `selected_branch()` picks branch from
  `UsagePatternStats`), `Mood` (Full/Content/Peckish/Hungry/Starving),
  `stage_for_level()`, `SHOP_CATALOG`. This is *not* movement logic — it's
  the mood/evolution state that the frontend reads to decide sprite/animation
  choice via `MODE_ANIMATION_TAG`.
- **`economy/`** — fullness decay, XP curve, token→food conversion, streaks.
  See [[game-economy|Game Economy]] for the formulas; this is where "how
  much food does N tokens produce" lives, not pet movement.
- **`store/game_state.rs`** (639 lines) — `GameRuntime`, persistence
  (rusqlite), `reconcile_and_persist()` (catches up hunger decay after the
  app was closed/asleep), builds the `PetStatePayload` sent to the frontend.
- **`lib.rs`** (~826 lines) — Tauri command registration and app wiring.
  Commands the overlay calls: `get_pet_state` (~L134), `pet_ate` (~L262),
  `pet_petted` (~L294), `buy_shop_item`/`equip_shop_item`/`place_furniture`/
  `prestige_pet` (~L320-374), `debug_add_sparks` (~L376, dev-build-only —
  no-ops behind `cfg!(debug_assertions)`, used by the dashboard's dev-only
  Sparks button to manually test shop sinks without grinding). `apply_token_event()`
  (~L557) is where a watcher-reported token usage event turns into economy
  updates and eventually a `food_spawned` emit. `start_window_geometry_watcher()`
  (~L546) polls other-app window positions and emits
  `window_segments_changed`, which feeds the climb behavior above.
- **`overlay_window.rs`** (138 lines) — `fit_to_monitor()` (positions the
  transparent overlay window per monitor), `visible_frame_physical()`
  (click-through geometry helper).
- **`window_geometry/mod.rs`** — `WindowSegment` struct, `enumerate_windows()`
  (platform-specific: macOS via CoreGraphics `CGRectMakeWithDictionary...`,
  other platforms have separate impls in the same file). This is the source
  of the "ledges" the pet climbs onto.

## How to Keep This Doc Useful

- When you add/rename/move a function referenced above, update its entry in
  the same change — don't leave this doc pointing at dead names.
- When a change is large enough to move line numbers by a lot (new section,
  big refactor), it's fine to leave line numbers stale as long as the
  function-name anchors are still correct; re-grep to refresh numbers only
  when convenient.
- A `PostToolUse` hook (see `.claude/settings.json`) prints a reminder in the
  transcript when a big edit lands in one of the files listed above, so the
  agent making that edit sees a nudge to refresh this map before finishing.
