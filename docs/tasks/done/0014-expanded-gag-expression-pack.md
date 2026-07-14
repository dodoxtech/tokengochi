---
type: task
status: done
priority: P2
delivery_order: 0014
estimate: 3d
created: 2026-07-13
updated: 2026-07-13
owner: AI agent
sprint: null
tags:
  - task
  - done
  - art
---

# Task: Expanded gag/expression pack (sneeze pose, yawn, dance, drink-break)

## Status

- State: done
- Created: 2026-07-13
- Owner: AI agent
- Priority: P2
- Delivery order: 0014
- Estimate: 3d
- Sprint: TBD

## Context

The pet currently has 3 idle "gag" asides (`GAG_VARIANTS` in
`ui/overlay/src/constants.ts`: `sneeze`, `stare`, `chase-tail`), but none of
them have dedicated body art — `pet.mode === "gag"` always plays the plain
`idle` tag, with the variant only distinguished by a procedural/effect
overlay (`drawGagEffect()` in `ui/overlay/src/render.ts`). The pet reads as
less alive than it could during these random idle moments.

The user wants a richer, more playful set of asides — sneezing (with real
body motion this time), yawning, a little dance, and pulling a drink out
and taking a swig — so idle moments feel more varied and fun. This task is
the asset/spec side of that: it defines each new gag's frames, timing, and
required effect/prop tags, and adds them to the [[../../knowledge/pet-action-pack-spec|Action Pack contract]]
so every current and future pet form implements the same set.

No code changes are in scope for this task — see
[[../../knowledge/pet-action-pack-spec|Pet Action Pack Spec]] for what code
wiring a future implementation task will need (new `GAG_VARIANTS` entries,
`MODE_ANIMATION_TAG`/`drawGagEffect` branches, optional new effect tags,
optional new item sprite).

Design decision: `drink-break` keeps the Hatchling's task-0005 no-arms
silhouette. The drink prop floats/pops beside the mouth rather than being
held by newly added arms, so the gag fits the current pet style and does
not create a character-design exception.

Related:

- [[../README|Tasks]]
- [[../../agile|Agile and Scrum Workflow]]
- [[../../knowledge/pet-action-pack-spec|Pet Action Pack Spec]]
- [[../active/0005-sprite-renderer-behavior-ai|0005 — sprite renderer + art style guide]]
- [[../active/0012-pet-playful-interactions-window-climbing|0012 — idle gags, click reactions]]

## Goal

Specify four gag/expression assets — a dedicated **sneeze** pose, **yawn**,
**dance**, and **drink-break** — precise enough that an artist or
image-model pipeline can produce conforming frames without further
clarification, and register them as required tags in the Action Pack
contract.

## Scope

In scope:

- Frame-by-frame spec (pose, frame count, duration, loop behavior) for the
  4 new/updated gag tags below.
- New effect tags needed to support them (music notes for `dance`, sweat
  droplet for `sneeze`).
- Resolving the no-arms prop question for `drink-break` (see
  [[../../knowledge/pet-action-pack-spec#Prop sprites|Prop sprites]]) and
  specifying the resulting bottle/can prop sprite.
- Updating `docs/knowledge/pet-action-pack-spec.md`'s gag-tag table once
  these are decided.

Out of scope:

- Implementation: editing `constants.ts`, `atlas.ts`, `behavior.ts`,
  `render.ts`, or generating the actual PNG/JSON/aseprite files. Follow-up
  implementation task, opened once this spec is reviewed.
- Any gag beyond the 4 named here (more can follow the same pattern later).
- Rebalancing gag frequency/selection weighting (`MIN_GAG_INTERVAL_MS`
  etc.) — purely additive to the existing random-pick pool.

## Asset Specification

All frames: 32×32, left-facing, Sweetie-16 palette only, 1px `#1a1c2c`
outline, binary alpha, flat 2-tone shading — same style guide as task 0005.

### 1. `sneeze` (upgrade from procedural-only to real body art)

Replaces the current `dust`-effect-only treatment with a real 4-frame
body animation, once, non-looping, returns to `idle`:

| Frame | Duration | Pose |
|---|---|---|
| 1 | 120 ms | Head pulls back slightly, eyes squint shut |
| 2 | 90 ms | Anticipation hold — head tilted down, body slightly compressed |
| 3 | 80 ms | Sneeze release — head snaps forward, mouth open, 1px body squash |
| 4 | 150 ms | Recovery — eyes reopen, small shake (1px side-to-side blur optional) |

Effect: keep `dust` puff in front of the face on frame 3 (already exists),
add a small `sparkle`-style droplet burst as an alternative/additional
effect tag if the dust puff alone reads too subtle at 32×32 — artist's
call during implementation, not a hard requirement.

### 2. `yawn` (new gag variant)

4 frames, ~200 ms average, once, loops back to `idle`:

| Frame | Duration | Pose |
|---|---|---|
| 1 | 150 ms | Mouth begins to open, eyes start to squint |
| 2 | 250 ms | Mouth wide open (largest silhouette change of the set — reads clearly even at 1× scale), eyes fully closed, body stretched 1px taller |
| 3 | 200 ms | Hold, tiny shake |
| 4 | 200 ms | Mouth closes, body settles back to base height |

No new effect tag required. Optional: a tiny `#94b0c2` motion-blur line
above the head on frame 2 if the mouth-open silhouette isn't legible
enough on its own — check against the light/dark wallpaper visual pass
from task 0005 before adding it.

### 3. `dance` (new gag variant)

6 frames, 110 ms each, loops **exactly twice** then returns to `idle` (same
loop-count pattern as the existing `happy` tag), a bouncy 2-step sway:

| Frame | Duration | Pose |
|---|---|---|
| 1 | 110 ms | Lean left, 1px hop up |
| 2 | 110 ms | Center, feet down |
| 3 | 110 ms | Lean right, 1px hop up |
| 4 | 110 ms | Center, feet down |
| 5 | 110 ms | Lean left, 1px hop up (repeat of 1, kept as a distinct frame so the JSON tag is self-contained) |
| 6 | 110 ms | Center, feet down (repeat of 2) |

New effect tag: `notes` — 2 frames @200ms, loop, a small pixel music-note
pair, anchored above the head the same way `heart` anchors for `petted`.

### 4. `drink-break` (new gag variant, prop-dependent)

The no-arms question is resolved for this gag: the bottle appears to float
or pop into frame beside the pet's mouth rather than being visibly "held,"
since the Hatchling has no arms to grip it. This avoids a body-design
change just for one gag and matches the playful desktop-pet read of small
objects being summoned near the character.

6 frames, 120 ms each, once, returns to `idle`:

| Frame | Duration | Pose |
|---|---|---|
| 1 | 120 ms | Bottle prop fades/pops in beside the mouth, pet notices (head turns slightly) |
| 2 | 120 ms | Head tilts back, mouth to bottle |
| 3 | 130 ms | Drinking — small throat-bob squash, bottle tilted |
| 4 | 130 ms | Drinking, repeat with slight variation (2-gulp read) |
| 5 | 110 ms | Head returns upright, satisfied close-mouth expression |
| 6 | 110 ms | Bottle prop fades/pops out |

New prop sprite: `prop-drink-bottle`, 16×16 (matches effects-sheet scale,
not the 32×32 body scale), small rounded bottle in `#41a6f6` (or
`#38b764` for a canteen read) with a `#f4f4f4` highlight and `#1a1c2c`
outline/cap. Lives in `ui/assets/sprites/items/` per the existing
prop/item pattern (see [[../../knowledge/pet-action-pack-spec#Prop sprites|Prop sprites]]),
not baked into the pet body sheet or the effects sheet.

## Acceptance Criteria

- [x] `docs/knowledge/pet-action-pack-spec.md`'s gag-tag section lists
      `sneeze`, `yawn`, `dance`, `drink-break` as required tags with a
      pointer to this task's frame tables.
- [x] The no-arms/prop-handling question for `drink-break` has a recorded
      decision (either here or as a new ADR if it turns out to affect the
      character design beyond this one gag).
- [x] Each of the 4 gags above has a complete frame table (pose, duration,
      loop behavior) precise enough to hand to an artist or an image-model
      prompt without follow-up questions.
- [x] New effect tags (`notes`) and the new prop sprite (`prop-drink-bottle`)
      are specified with palette/size, matching the existing effects-sheet
      (16×16) and item-sprite conventions respectively.

## Dependencies

- [[../active/0005-sprite-renderer-behavior-ai|0005]] (style guide, sheet format)
- [[../active/0012-pet-playful-interactions-window-climbing|0012]] (existing gag system this extends)

## Risks

- No-arms character design may make `drink-break` read awkwardly no matter
  which prop-handling option is chosen; if the "floating prop" default
  looks wrong once drafted, this may need to become a character-design ADR
  rather than a one-gag workaround.
- `dance`'s 2px lean silhouette is the biggest departure from the existing
  idle/walk poses — worth a quick visual check against both wallpaper
  backgrounds (per task 0005's verification plan) before committing frame
  counts.

## Implementation Notes

- Updated [[../../knowledge/pet-action-pack-spec|Pet Action Pack Spec]] so
  the gag-tag table now includes the four required authored tags:
  `sneeze`, `yawn`, `dance`, and `drink-break`.
- Registered new shared effect tag `notes`: 2 frames, 200 ms each, looped,
  anchored above the pet like `heart`.
- Registered shared prop sprite `prop-drink-bottle`: 16×16, Sweetie-16,
  `#1a1c2c` outline/cap, `#41a6f6` blue bottle body or `#38b764` green
  canteen body, `#f4f4f4` highlight.
- Follow-up implementation task will need: `GAG_VARIANTS` additions in
  `constants.ts`, new `MODE_ANIMATION_TAG` entries in `atlas.ts`, new
  `drawGagEffect()`/prop branches in `render.ts`, sheet/JSON/source asset
  generation, and descriptor/style sidecars for any new prop artwork (see
  [[../../knowledge/sprite-asset-pipeline|Sprite Asset Pipeline]]).

## References

- [[../../README|Documentation]]
- [[../../index|Project Map]]
- [[../../knowledge/pet-action-pack-spec|Pet Action Pack Spec]]
- `ui/overlay/src/constants.ts` (`GAG_VARIANTS`)
- `ui/overlay/src/render.ts` (`drawGagEffect`, `drawOverlayEffects`)
- `ui/overlay/src/atlas.ts` (`MODE_ANIMATION_TAG`)

## Verification Plan

- [x] Doc review: frame tables are unambiguous enough to hand off without
      the reviewer needing to ask clarifying questions.
- [x] Cross-check against `docs/tasks/active/0005-sprite-renderer-behavior-ai.md`'s
      Art Style Guide for palette/canvas/outline consistency.

## Verification Results

- 2026-07-13: Cross-checked frame specs against task 0005's Hatchling art
  style: 32×32 pet frames, left-facing, Sweetie-16-only, 1px `#1a1c2c`
  outline, binary alpha, flat two-tone shading, top-left light, chunky
  1×-legible silhouette, and no arms.
- 2026-07-13: Confirmed `notes` and `prop-drink-bottle` specs match the
  existing shared-effect/item split: effects are 16×16 and shared across
  forms; props live under `ui/assets/sprites/items/` and are not baked
  into the body sheet.
- 2026-07-13: No code or PNG generation was run because implementation and
  asset production are explicitly out of scope for task 0014.

## Completion Notes

- Completed: 2026-07-13
- Changed files:
  - `docs/knowledge/pet-action-pack-spec.md`
  - `docs/tasks/done/0014-expanded-gag-expression-pack.md`
- Follow-ups:
  - [[../active/0016-implement-expanded-gag-expression-pack|0016]]:
    generate the body/effect/prop assets and wire `GAG_VARIANTS`,
    animation tags, and render branches.
