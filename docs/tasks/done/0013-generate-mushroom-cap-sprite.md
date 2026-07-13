---
type: task
status: done
priority: P3
delivery_order: 0013
estimate: 1h
created: 2026-07-13
updated: 2026-07-13
owner: AI agent
sprint: null
tags:
  - task
  - backlog
  - assets
---

# Task: Generate the Mushroom Cap cosmetic sprite

## Status

- State: done
- Created: 2026-07-13
- Owner: AI agent
- Priority: P3
- Delivery order: 0013
- Estimate: 1h
- Sprint: TBD

## Context

The **Sunset Scarf** cosmetic (`scarf-sunset`) was replaced by a new hat
cosmetic, **Mushroom Cap** (`hat-mushroom`) — a red dome cap with white
spots, keeping the item as a headwear piece instead of a neckwear piece, per
[[../../knowledge/game-economy|Game Economy §6]]. The code-side rename is
already done (shop catalog, overlay renderer, dashboard preview); this task
covers producing the actual pixel-art sprite, which was deliberately not
generated yet.

Related:

- [[../README|Tasks]]
- [[../../agile|Agile and Scrum Workflow]]

## Goal

Produce a 32x32 transparent-background sprite for `hat-mushroom` in the
project's existing Sweetie 16 pixel-art style, matching the other shop item
sprites (`hat-leaf`, `food-sushi`, etc.), so the placeholder procedural
fallback in `ui/overlay/src/render.ts` (`drawCosmetic`, case `"hat-mushroom"`)
is replaced by real art.

## Scope

In scope:

- Run the `asset-canon` skill to generate the `hat-mushroom` sprite: a red
  mushroom-dome cap with white spots, worn on the pet's head, matching the
  bounding box hat-leaf uses (roughly `-20,-43,40,40` in overlay coordinates).
- Write the asset brief `docs/assets/hat-mushroom.yaml` and style profile
  `docs/assets/styles/style-profile-hat-mushroom.yaml` (same shape as the
  removed `scarf-sunset` ones — see git history for the old scarf brief as a
  reference for field structure).
- Export the sprite to `ui/assets/sprites/items/hat-mushroom-sprite-32x32.png`
  and copy it into `ui/dashboard/static/overlay/sprites/items/` so the
  dashboard shop preview picks it up (mirrors how `hat-leaf` is placed in
  both locations).

Out of scope:

- Any further cosmetic rename/removal.
- Changing the item's price or shop placement.

## Acceptance Criteria

- [x] `ui/assets/sprites/items/hat-mushroom-sprite-32x32.png` exists, 32x32,
      transparent background, matches the project's Sweetie 16 palette.
- [x] Same file also present at
      `ui/dashboard/static/overlay/sprites/items/hat-mushroom-sprite-32x32.png`.
- [x] Dashboard shop preview renders the real sprite (no broken image icon)
      for the Mushroom Cap item.
- [x] Overlay renders the real sprite when `hat-mushroom` is equipped (the
      procedural fallback in `drawCosmetic` no longer triggers).
- [x] `docs/assets/hat-mushroom.yaml` and its style profile are added,
      following the existing brief format (see `docs/assets/hat-leaf.yaml`).

## Dependencies

- None — the code-side rename (item id, label, overlay/dashboard wiring) is
  already complete.

## Risks

- TBD

## Implementation Notes

- Reference sprites for style consistency: `docs/assets/hat-leaf.yaml`,
  `docs/assets/styles/style-profile-hat-leaf.yaml`.
- Procedural fallback shape already implemented in
  `ui/overlay/src/render.ts` (`drawCosmetic`, `"hat-mushroom"` case) — keep it
  in place as the loading-failure fallback; only add the real sprite file, do
  not remove the fallback code.

## References

- [[../../README|Documentation]]
- [[../../index|Project Map]]
- [[../../knowledge/game-economy|Game Economy Design]] §6
- `src-tauri/src/pet/mod.rs` (`SHOP_CATALOG`)
- `ui/overlay/src/render.ts` (`ITEM_SPRITE_PATHS`, `drawCosmetic`)
- `ui/dashboard/src/routes/+page.svelte` (`ITEM_PREVIEWS`)

## Verification Plan

- [ ] Manual: buy/equip Mushroom Cap in the dashboard, confirm sprite
      renders on the pet overlay and in the shop preview card.

## Verification Results

### 2026-07-13

- Generated `ui/assets/sprites/source/hat-mushroom-master.png` with image generation
  on a chroma-magenta plate.
- Added `ui/assets/sprites/generate-hat-mushroom-from-master.mjs` to key the
  plate, trim, nearest-neighbor resize, quantize to the Sweetie 16 subset, and
  write the descriptor/style sidecars.
- Exported `ui/assets/sprites/items/hat-mushroom-sprite-32x32.png` and copied it
  to `ui/dashboard/static/overlay/sprites/items/hat-mushroom-sprite-32x32.png`.
- Pixel QA passed: 32x32, transparent corners, no magenta residue, no off-palette
  opaque pixels.
- Verification commands:
  - `node ui/assets/sprites/generate-hat-mushroom-from-master.mjs`
  - Pixel QA one-off Node script using `sharp`.
  - `npm run check` in `ui/overlay`.
  - `npm run check` in `ui/dashboard`.
  - `npm run build` in `ui/overlay`.
  - `npm run build` in `ui/dashboard`.

## Completion Notes

Fill this in before moving the task to `docs/tasks/done/`.

- Completed: 2026-07-13
- Changed files:
  - `docs/assets/hat-mushroom.yaml`
  - `docs/assets/styles/style-profile-hat-mushroom.yaml`
  - `ui/assets/sprites/source/hat-mushroom-master.png`
  - `ui/assets/sprites/items/hat-mushroom-sprite-32x32.png`
  - `ui/assets/sprites/generate-hat-mushroom-from-master.mjs`
- Follow-ups: none
