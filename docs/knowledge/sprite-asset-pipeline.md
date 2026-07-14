---
type: reference
status: active
created: 2026-07-13
updated: 2026-07-13
tags:
  - knowledge
  - ai-context
  - sprites
  - assets
owner: engineering
---

# Sprite Asset Pipeline

All Node scripts that regenerate sprite PNGs from image-model master
artwork live in `ui/assets/sprites/scripts/`. They are not part of the
`ui/overlay` or `ui/dashboard` app builds — they are one-off/rerunnable
tooling, kept next to the sprite assets they produce rather than under
`ui/overlay/`.

## Scripts

- **`scripts/generate-from-masters.mjs`** — rebuilds the hatchling and
  effects runtime sheets (`hatchling/hatchling.png` + `.json`,
  `effects/effects.png` + `.json`) from `source/hatchling-master.png` and
  `source/effects-master.png`. Uses a hard chroma-key threshold (no alpha
  blending) because the source antialiased edges don't follow a clean
  linear blend against the magenta plate. See the file's header comment
  for why frame crop boxes are hand-derived bounding boxes, not a uniform
  grid.
- **`scripts/generate-shop-items-from-master.mjs`** — rebuilds all Sparks
  sink item sprites (`items/*.png`) from the single 3x2 grid master
  `source/shop-items-master.png`. Keys the chroma-magenta plate, trims each
  cell's subject, nearest-neighbor resizes into the item's native canvas,
  quantizes to the Sweetie 16 subset, and writes both sidecars
  (`docs/assets/<id>.yaml`, `docs/assets/styles/style-profile-<id>.yaml`).
- **`scripts/generate-hat-mushroom-from-master.mjs`** — same pattern as
  above but for the single-item `source/hat-mushroom-master.png` master
  (Mushroom Cap hat).
- **`scripts/generate-food-default-from-master.mjs`** — rebuilds the
  default unequipped food drop sprite
  `items/food-default-sprite-32x32.png` from
  `source/food-default-master.png`. The renderer uses this before falling
  back to the old procedural default food drawing.
- **`scripts/generate-gag-expression-pack.mjs`** — builds the task 0016
  gag expansion from `source/gag-expression-pack-master.png` as standalone
  supplemental files: `hatchling/hatchling-gag-expressions.png`/`.json`,
  `effects/effects-notes.png`/`.json`, and the 16×16
  `items/prop-drink-bottle-sprite-16x16.png`. It deliberately does not
  overwrite the base `hatchling.png` or `effects.png` runtime sheets.

Run any of them with plain `node`, e.g.:

```
node ui/assets/sprites/scripts/generate-shop-items-from-master.mjs
```

They resolve `sharp` from `ui/overlay/node_modules/sharp` (relative import,
three levels up from `scripts/`), so `npm install` must have run in
`ui/overlay` first.

## Chroma-key plate threshold

All three scripts key a chroma-magenta (`#FF00FF`) plate to alpha by
Euclidean RGB distance. Antialiased edge pixels between dark artwork and
the plate can land just outside a too-tight threshold and get treated as
opaque, then palette-quantized to whatever palette color happens to be
closest — this showed up as a pink (`#b13e53`) fringe under a bed sprite's
frame (`plateThreshold` was 145; raised to 200 to key out the residual
blend pixels, ~148–183 distance from the plate). If a similar color fringe
shows up on a future asset, check the plate distance of the offending
pixels in the source master before assuming it's an intentional palette
color — it's very likely bleed that needs a higher threshold, not new art.

## Adding a new item script

Follow `generate-shop-items-from-master.mjs` as the template: it already
writes both the descriptor and style-profile sidecars per item, which is
required by [[../README|the asset workflow]] (see also the `asset-canon`
skill's ASSET DESCRIPTOR / STYLE PROFILE conventions). Prefer adding a new
item to an existing master's item list over creating a new master +
script, if the item fits on that master's grid.
