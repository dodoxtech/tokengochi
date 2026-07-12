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
| How fast/where the pet walks | `ui/overlay/src/main.ts` → `updatePet()` (walk-to-food, walk-to-bed) |
| Falling / being thrown / bouncing off edges | `ui/overlay/src/main.ts` → `updateTumble()`, `beginDrop()` |
| Climbing onto other app windows | `ui/overlay/src/main.ts` → `maybeStartClimb()`, `updateClimb()` |
| Idle random gags (sneeze/stare/chase-tail) | `ui/overlay/src/main.ts` → `maybeTriggerIdleGag()` |
| Click reactions / rage-quit combo | `ui/overlay/src/main.ts` → `triggerClickReaction()`, `updateSulk()` |
| Petting / stroke detection | `ui/overlay/src/main.ts` → `maybeTriggerPetting()` |
| Dragging the pet with the mouse | `ui/overlay/src/main.ts` → `mousedown`/`mousemove`/`mouseup` listeners (~L1042-1107) |
| Eating food (walk-to-food + eat timer) | `ui/overlay/src/main.ts` → `updatePet()` tail (food branch), `pet_ate` IPC |
| What animation/sprite plays for a mode | `ui/overlay/src/main.ts` → `MODE_ANIMATION_TAG`, `drawPet()` |
| Speed/physics tuning constants | `ui/overlay/src/main.ts` → constants block (~L131-159) |
| Mood bands / hunger / evolution stage | `src-tauri/src/pet/mod.rs` (`Mood`, `stage_for_level`, `EvolutionBranch`) |
| Fullness decay, XP, streaks math | `src-tauri/src/economy/*.rs` (see [[game-economy]]) |
| Persisted pet state (SQLite row) | `src-tauri/src/store/game_state.rs` |
| Tauri commands callable from the overlay (`pet_ate`, `pet_petted`, ...) | `src-tauri/src/lib.rs` (~L124-373, `#[tauri::command]` fns) |
| Click-through / hit-testing / window flags | `ui/overlay/src/main.ts` → `isOverPet()`, `updateHitTest()`; `src-tauri/src/overlay_window.rs` |
| Climbable window segments (other app windows) | `src-tauri/src/window_geometry/mod.rs` (`enumerate_windows`), emitted as `window_segments_changed` |
| Sprite atlas / animation frame lookup | `ui/overlay/src/main.ts` → `loadAtlas()`, `frameForTag()` |

## Frontend: `ui/overlay/src/main.ts` (behavior AI + renderer, ~1150 lines)

This is a single-file canvas game loop. It owns the `pet` mutable object
(`const pet = {...}`, ~L233) and a `PetMode` state machine (~L165).

Layout, top to bottom:

1. **Sprite atlas loading** (~L31-113): `AtlasFrame`/`AtlasJson`/`SpriteAtlas`
   types, `loadImage()`, `loadAtlas()`, `frameForTag()`. Sprite sheets live in
   `ui/assets/sprites/` (aseprite exports).
2. **Tuning constants** (~L131-159): `WALK_SPEED`, `GRAVITY`,
   `MAX_THROW_SPEED`, `CLIMB_SPEED`, `CLICK_COMBO_COUNT`, gag/climb interval
   ranges, etc. Change pet speed/feel here first.
3. **Types** (~L165-231): `PetMode` (union of `BaseMode | PhysicsMode |
   OverrideMode`), `Segment` (a climbable window edge), `Food`.
4. **Mutable state** (~L233-300): `pet` object (`x`, `y`, `vx`, `vy`, `mode`,
   `supportId`, `climbPhase`, ...), `foods[]` array.
5. **Geometry/window helpers** (~L311-378): `resizeCanvas()`,
   `refreshWindowOffset()`, `groundY()`, `petMaxX()`, `currentSegments()`,
   `landingSurfaceAt()` (decides what surface the pet lands on when falling).
6. **Hit-testing / click-through** (~L399-451): `isOverPet()`,
   `updateHitTest()` (toggles OS click-through so clicks pass through except
   over the pet sprite), `pollCursorForHover()`.
7. **Physics/override modes** (~L461-642, comment-marked "Task 0012
   physics/override modes"):
   - `beginDrop(now, vx, vy)` — enters free-fall (`mode = "tumble"`).
   - `updateTumble()` — gravity integration, bounces off screen edges,
     detects landing via `landingSurfaceAt()`.
   - `maybeStartClimb()` / `updateClimb()` — picks a random other-app window
     edge from `windowSegments`, walks to it, climbs up (`CLIMB_SPEED`),
     sits for a random duration, then drops back off.
   - `maybeTriggerIdleGag()` — random sneeze/stare/chase-tail.
   - `triggerClickReaction()` / `updateSulk()` — click combo → dizzy/sulk
     escalation; otherwise squash/spin/look/exclaim reaction.
   - `maybeTriggerPetting()` — hover-and-hold detection, calls `pet_petted`
     IPC (rate-limited client-side by `PET_BUMP_COOLDOWN_MS`, mirrored
     server-side).
8. **Baseline behavior** (~L646-753, comment-marked "Baseline behavior
   0005/0006"): `updatePet(dtMs, now)` — the main state-machine dispatcher.
   Mode precedence: `dragged` → `tumble` → `climb` → time-boxed overrides
   (`dizzy`/`sulk`/`react`/`petted`/`gag`) → petting check → forced drop if
   not on the floor → seek nearest unclaimed food → walk to bed when idle →
   idle gag/climb rolls → `idle`. Food-eating sub-branch at the bottom calls
   `invoke("pet_ate")`.
9. **Rendering** (~L749-1021): `drawPet()`, `drawSpiralEyes()` (dizzy),
   `drawOverlayEffects()`/`drawEffect()`/`drawGagEffect()`, `drawCosmetic()`,
   `drawFood()`/`drawFoodSkin()`, `drawFurniture()`, `drawTooltip()`,
   `draw()` (composes the frame).
10. **Game loop** (~L1022-1041): `tick(now)` — computes `dtMs`, throttles to
    `ACTIVE_TICK_MS` (30fps moving) or `IDLE_TICK_MS` (2fps idle/sleeping),
    calls `updatePet()` + `updateFood()` + `draw()`.
11. **DOM input listeners** (~L1042-1119): `mousemove` (drag tracking,
    `DRAG_PROMOTE_PX` threshold turns a click into a drag → `mode =
    "dragged"`), `mousedown`, `mouseup` (releasing a drag calls
    `beginDrop()` with throw velocity, capped by `MAX_THROW_SPEED`), `blur`.
12. **Tauri event listeners** (~L1119-1152): `food_spawned` (push into
    `foods[]`), `pet_state_changed`, `overlay_settings_changed`,
    `window_segments_changed` (updates `windowSegments` used by climb logic),
    initial `get_pet_state` invoke on startup.

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
- **`lib.rs`** (812 lines) — Tauri command registration and app wiring.
  Commands the overlay calls: `get_pet_state` (~L134), `pet_ate` (~L262),
  `pet_petted` (~L294), `buy_shop_item`/`equip_shop_item`/`place_furniture`/
  `prestige_pet` (~L320-374). `apply_token_event()` (~L557) is where a
  watcher-reported token usage event turns into economy updates and
  eventually a `food_spawned` emit. `start_window_geometry_watcher()`
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
