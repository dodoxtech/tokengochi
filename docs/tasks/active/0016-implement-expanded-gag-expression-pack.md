---
type: task
status: active
priority: P2
delivery_order: 0016
estimate: TBD
created: 2026-07-13
updated: 2026-07-13
owner: AI agent
sprint: null
tags:
  - task
  - active
  - art
  - frontend
---

# Task: Implement expanded gag/expression pack

## Status

- State: active
- Created: 2026-07-13
- Owner: AI agent
- Priority: P2
- Delivery order: 0016
- Estimate: TBD
- Sprint: TBD

## Context

Task [[../done/0014-expanded-gag-expression-pack|0014]] completed the
asset/spec side of the expanded gag pack: dedicated `sneeze` body art,
`yawn`, `dance`, `drink-break`, shared effect tag `notes`, and shared prop
sprite `prop-drink-bottle`. The current overlay still only picks
`sneeze`, `stare`, and `chase-tail` in `GAG_VARIANTS`, and `pet.mode ===
"gag"` still falls back to the plain `idle` animation plus procedural
effects.

This task is the implementation pass: generate or author the specified
assets in the Hatchling style and wire the overlay so the new gag variants
actually play.

Related:

- [[../README|Tasks]]
- [[../../agile|Agile and Scrum Workflow]]
- [[../../knowledge/pet-action-pack-spec|Pet Action Pack Spec]]
- [[../../knowledge/sprite-asset-pipeline|Sprite Asset Pipeline]]
- [[../done/0014-expanded-gag-expression-pack|0014 — expanded gag/expression spec]]

## Goal

Ship the expanded gag pack in the running overlay: `sneeze`, `yawn`,
`dance`, and `drink-break` have authored Hatchling-style frames, required
effect/prop artwork exists, and random idle gags can select and render the
new variants.

## Scope

In scope:

- Add `sneeze`, `yawn`, `dance`, and `drink-break` frames/tags as a
  supplemental Hatchling gag-expression sheet and JSON according to task
  0014, without overwriting the base Hatchling sheet.
- Add shared `notes` effect frames/tag as a supplemental effects sheet,
  without overwriting the base effects sheet.
- Add `prop-drink-bottle` under `ui/assets/sprites/items/` with descriptor
  and style sidecars following the item-sprite conventions.
- Update overlay code paths for gag selection, animation tag mapping, and
  effect/prop rendering.
- Verify palette, binary alpha, frame counts, and renderer behavior.

Out of scope:

- New gags beyond the task-0014 set.
- Redesigning the Hatchling with arms.
- Rebalancing idle gag frequency beyond adding the new variants to the
  selection pool.

## Acceptance Criteria

- [x] `GAG_VARIANTS` includes `sneeze`, `stare`, `chase-tail`, `yawn`,
      `dance`, and `drink-break` without breaking calm-mode gag disabling.
- [x] `sneeze`, `yawn`, `dance`, and `drink-break` play from the authored
      supplemental body tags with frame counts/durations matching task
      0014.
- [x] `dance` renders the shared `notes` effect anchored above the pet.
- [x] `drink-break` renders `prop-drink-bottle` beside the mouth without
      adding arms or baking the prop into the body sheet.
- [x] New/updated PNGs contain only Sweetie-16 opaque colors plus binary
      alpha. (Verified in the 2026-07-13 asset pass, unchanged by this
      code-wiring pass.)
- [x] Overlay build/check passes.
- [ ] A visual smoke pass in the running overlay confirms each gag reads
      at 1x and normal display scale. **Not done from this environment** -
      no way to launch the Tauri overlay window here. Needs a manual pass
      before this task moves to done.

## Dependencies

- [[../done/0014-expanded-gag-expression-pack|0014]]
- [[../active/0005-sprite-renderer-behavior-ai|0005]]
- [[../../knowledge/pet-action-pack-spec|Pet Action Pack Spec]]

## Risks

- Adding variable gag-specific tags may expose assumptions that `gag`
  always maps to `idle`.
- The floating drink prop may need small offset tuning to read clearly on
  both left-facing and mirrored right-facing movement.
- Repacking source masters can accidentally introduce palette or alpha
  regressions in existing frames.

## Implementation Notes

- 2026-07-13 asset pass:
  - Generated image-model master art at
    `ui/assets/sprites/source/gag-expression-pack-master.png`, using the
    existing Hatchling sprite sheet as the style reference.
  - Added `ui/assets/sprites/scripts/generate-gag-expression-pack.mjs` to
    crop, chroma-key, quantize to Sweetie16, and pack the generated master
    without overwriting the base runtime sheets.
  - Added standalone authored `sneeze`, `yawn`, `dance`, and `drink-break`
    body atlas at `ui/assets/sprites/hatchling/hatchling-gag-expressions.png`
    and `.json`.
  - Added standalone shared `notes` effect atlas at
    `ui/assets/sprites/effects/effects-notes.png` and `.json`.
  - Added shared prop sprite
    `ui/assets/sprites/items/prop-drink-bottle-sprite-16x16.png` with
    descriptor/style sidecars under `docs/assets/`.
  - Updated `ui/overlay/package.json` so the build copies supplemental
    hatchling/effects atlases into `ui/dashboard/static/overlay/sprites/`.
- 2026-07-13 code-wiring pass:
  - `ui/overlay/src/constants.ts`: `GAG_VARIANTS` now includes `yawn`,
    `dance`, `drink-break`. Added `GAG_VARIANT_DURATION_MS` (one total
    on-screen duration per variant, derived from task 0014's frame tables:
    `sneeze` 440ms, `yawn` 800ms, `dance` 1320ms = 660ms x2 loops,
    `drink-break` 720ms; `stare`/`chase-tail` keep the original flat
    1800ms since they have no dedicated art).
  - `ui/overlay/src/atlas.ts`: loads two new atlases at startup,
    `gagExpressionsAtlas` (`./sprites/hatchling-gag-expressions`) and
    `notesAtlas` (`./sprites/effects-notes`). Added `GAG_BODY_SOURCE` +
    `resolveGagBody()` so `sneeze`/`yawn`/`dance`/`drink-break` resolve to
    the supplemental atlas's matching tag, while `stare`/`chase-tail` keep
    resolving to the base atlas's `idle` tag - `MODE_ANIMATION_TAG` itself
    is untouched (per the pet-action-pack spec's "don't hand-tune it per
    character" rule; this is a per-mode, not per-character, resolution).
  - `ui/overlay/src/render.ts`: `drawPet()` now calls `resolveGagBody()`
    when `pet.mode === "gag"` instead of always drawing from
    `hatchlingAtlas`. `drawGagEffect()` gained branches for `dance` (draws
    `notes` from `notesAtlas` above the head), `drink-break` (draws
    `prop-drink-bottle` beside the mouth, offset by `pet.facing`), and
    `yawn` (no effect, per spec). Registered `prop-drink-bottle` in
    `ITEM_SPRITE_PATHS`.
  - `ui/overlay/src/behavior.ts`: `maybeTriggerIdleGag()` now sets
    `pet.overrideUntil` from `GAG_VARIANT_DURATION_MS[pet.gagVariant]`
    instead of a flat 1800ms, so each variant's override window matches
    its actual animation length (dance runs its full 2x loop before the
    mode changes).
  - No changes needed to `ui/overlay/package.json` - its existing
    wildcard copy (`../assets/sprites/hatchling/*.png` etc.) already
    picked up the supplemental files from the asset pass.
- Use task 0014's frame tables as the source of truth for frame counts,
  durations, loops, and prop/effect behavior.
- Preserve the Hatchling style from task 0005: 32×32 body frames,
  left-facing, Sweetie-16-only, 1px `#1a1c2c` outline, binary alpha, flat
  two-tone shading, top-left light, and no arms.
- Prefer extending the existing scripts in `ui/assets/sprites/scripts/`
  over hand-editing packed PNGs.

## References

- [[../../README|Documentation]]
- [[../../index|Project Map]]
- [[../../knowledge/pet-action-pack-spec|Pet Action Pack Spec]]
- [[../../knowledge/sprite-asset-pipeline|Sprite Asset Pipeline]]
- `ui/overlay/src/constants.ts`
- `ui/overlay/src/atlas.ts`
- `ui/overlay/src/render.ts`
- `ui/assets/sprites/scripts/`

## Verification Plan

- [ ] Run overlay checks/builds relevant to changed code.
- [ ] Run or add a palette/alpha verification over changed sprite PNGs.
- [ ] Manually preview each gag on light and dark backgrounds.
- [ ] Record commands and results below.

## Verification Results

- 2026-07-13 code-wiring QA:
  - `npm --prefix ui/overlay run check`: PASS.
  - `npm --prefix ui/overlay run build`: PASS; confirmed via
    `overlay.js` string scan that the bundle references
    `sprites/hatchling-gag-expressions` and `sprites/effects-notes`, and
    that both files plus `prop-drink-bottle-sprite-16x16.png` land in
    `ui/dashboard/static/overlay/sprites/`.
  - Not done: an actual visual smoke pass of the running overlay (no way
    to launch the Tauri window from this environment). Before moving this
    task to done, run the overlay and manually trigger `sneeze`, `yawn`,
    `dance`, and `drink-break` (e.g. by shortening `MIN_GAG_INTERVAL_MS`/
    `MAX_GAG_INTERVAL_MS` locally, or setting `pet.gagVariant`/`pet.mode`
    from the console) to confirm framing, the `notes` anchor height, and
    the drink-bottle offset read correctly at both facings.
- 2026-07-13 asset QA:
  - Base `hatchling.png` was restored unchanged: 1472×736, 29 frames,
    original tags only.
  - Base `effects.png` was restored unchanged: 1300×580, 10 frames,
    original tags only.
  - `hatchling-gag-expressions.png`: 1472×552, 20 frames, tags include
    `sneeze`, `yawn`, `dance`, and `drink-break`; binary alpha; 16
    Sweetie16 colors; transparent corners.
  - `effects-notes.png`: 176×88, 2 frames, `notes` tag; binary alpha; 15
    Sweetie16 colors; transparent corners.
  - `prop-drink-bottle-sprite-16x16.png`: 16×16, binary alpha, 8
    Sweetie16 colors, transparent corners.
  - Visual preview pass completed for hatchling sheet, effects sheet, and
    bottle prop. A few generated gag frames intentionally include small
    motion/burst pixels, but no chroma plate or off-palette residue remains.
  - `npm --prefix ui/overlay run check`: PASS.
  - `npm --prefix ui/overlay run build`: PASS; supplemental atlas files are
    copied to dashboard static overlay assets.

## Completion Notes

Fill this in before moving the task to `docs/tasks/done/`.

- Completed: YYYY-MM-DD
- Changed files:
- Follow-ups:
