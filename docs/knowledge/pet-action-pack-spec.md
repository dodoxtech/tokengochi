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
  - behavior
owner: engineering
---

# Pet Action Pack Spec (character-swap contract)

Defines the fixed set of animation tags, effect overlays, and prop sprites
every pet form must ship so it can be dropped in as a drag-and-drop
character swap — no code change to `behavior.ts`, `atlas.ts`, or
`render.ts` required. This formalizes the "future forms add sibling
folders" note in [[../tasks/done/0009-evolution-streaks-quests]] and the
folder layout in [[../tasks/active/0005-sprite-renderer-behavior-ai]].

Related: [[code-map|Code Map]], [[sprite-asset-pipeline|Sprite Asset Pipeline]], [[../decisions/0003-canvas-sprite-rendering|ADR-0003]].

## Two tiers

There are two different things people mean by "add more expressions," and
they have very different cost:

1. **Swap the character** (new body art for the same behavior set) — art
   only, zero code. This is the **Action Pack**: a fixed list of tags. Any
   folder under `ui/assets/sprites/<form-name>/` that contains every tag in
   the [Required Tags](#required-tags) table below is a complete,
   swappable pet form.
2. **Add a new kind of expression** (a gag/react the game doesn't have at
   all yet, e.g. "dance") — this is a **one-time code change**
   (`GAG_VARIANTS`/`REACT_VARIANTS` in `constants.ts`, wiring in
   `behavior.ts`/`render.ts`) that *extends* the Action Pack contract for
   every pet form going forward. After that change lands, the new tag is
   just another required row in the table below, and every existing form
   (including old ones) needs the matching frames added before its next
   release. The first gag expansion is specified in
   [[../tasks/done/0014-expanded-gag-expression-pack]]: yawn, dance,
   drink-break, plus a dedicated sneeze pose.

Do not skip tier 1's zero-code guarantee by hand-tuning `MODE_ANIMATION_TAG`
per character — the whole point of the contract is that every form maps
through the exact same table.

## Required tags

Canvas size, palette, outline, and facing rules are unchanged from the
[Art Style Guide in task 0005](../tasks/active/0005-sprite-renderer-behavior-ai.md#art-style-guide)
— every tag below is a 32×32 frame, left-facing, Sweetie-16-only, binary
alpha.

### Body tags (pet sheet, drawn through `MODE_ANIMATION_TAG`)

| Tag | Used for `PetMode` | Loop | Notes |
|---|---|---|---|
| `idle` | `idle`, `gag`, `landing` | yes | Base resting pose; also the fallback for override modes with no dedicated art |
| `walk` | `seek`, `climb`, `sulk` | yes | Must read correctly at both normal and 0.5–0.8× speed scaling (sulk/climb throttle it) |
| `sleep` | `sleep` | yes | Paired with the `zzz` effect |
| `eat` | `eat` | once | Returns to `idle` when done |
| `happy` | `happy`, `petted` | ×2 | Paired with the `heart` effect for `petted` |
| `drag` | `dragged`, `tumble` | yes | Dangling pose doubles as the airborne/fall pose |
| `react` | `react`, `dizzy` | once | `dizzy` overlays spiral-eyes on top of this pose (procedural, not art) |

A pet form is **not required** to author a distinct pose for every
`PetMode` — `MODE_ANIMATION_TAG` in `ui/overlay/src/atlas.ts` is the single
source of truth for which tag plays for which mode, and several modes
intentionally reuse a body tag paired with a different effect overlay
(documented inline there). What's required is that all 7 tags above exist;
new *modes* reusing an existing tag is a code decision, not a per-form art
decision.

### Gag tags (idle-flavored asides, `pet.mode === "gag"`)

One tag per entry in `GAG_VARIANTS` (`ui/overlay/src/constants.ts`). Gag
tags use the same 32x32, left-facing, Sweetie-16 style as the body tags:
chunky silhouette, 1px `#1a1c2c` outline, binary alpha, flat two-tone
shading, top-left light, and no anti-aliasing. Hatchling-derived forms keep
the no-arms character read unless a later character-design ADR changes the
whole family.

The baseline implementation currently ships `sneeze`, `stare`, and
`chase-tail` as procedural overlays on the plain `idle` tag. Task
[[../tasks/done/0014-expanded-gag-expression-pack]] expands the contract:
`sneeze`, `yawn`, `dance`, and `drink-break` are required authored gag
tags for the next art pass, while `stare` and `chase-tail` may remain
`idle`+effect fallbacks until a follow-up spec gives them dedicated poses.
For existing forms, new gag tags may ship in a supplemental atlas such as
`hatchling-gag-expressions.png`/`.json` instead of being appended to the
base body sheet. Do not overwrite a proven base sheet just to add optional
gags; the renderer should load the supplemental atlas when those variants
are enabled.

| Tag | Frames | Duration | Loop | Effects/props | Notes |
|---|---:|---|---|---|---|
| `sneeze` | 4 | 120, 90, 80, 150 ms | once | `dust` on release; optional droplet burst | Real body art replacing the current procedural-only gag: pullback, compressed anticipation, forward release, recovery |
| `yawn` | 4 | 150, 250, 200, 200 ms | once | none | Wide-mouth silhouette is the key read; optional tiny motion line only if 1x legibility fails |
| `dance` | 6 | 110 ms each | exactly x2 | `notes` above head | Bouncy two-step sway: left hop, center, right hop, center, left hop, center |
| `drink-break` | 6 | 120, 120, 130, 130, 110, 110 ms | once | `prop-drink-bottle` | Floating bottle pops beside the mouth; pet tilts back for two gulps, then bottle pops out |

Implementation detail: `drink-break` deliberately uses a floating/summoned
prop rather than adding hands to the Hatchling. This preserves the current
no-arms silhouette from task 0005 and keeps props shared across forms
instead of forcing a per-gag body redesign.

### Effect tags (effects sheet, shared across all pet forms — author once, not per form)

| Tag | Paired with | Loop |
|---|---|---|
| `zzz` | `sleep` | yes |
| `heart` | `petted` | yes |
| `exclaim` | `react` (`reactVariant === "exclaim"`) | once |
| `dust` | landing squash, `sneeze`/`chase-tail` gag | once |
| `notes` | `dance` gag | yes |

Effects live in `ui/assets/sprites/effects/` and are **not** duplicated per
pet form — one effects sheet serves every character, since they're drawn
in screen space at an anchor offset from the pet, not baked into the body
sheet. A new gag/react variant that needs a new visual flourish adds one
tag here, once. `notes` is specified as 2 frames at 200 ms each: a small
Sweetie-16 music-note pair anchored above the head, following the same
anchor behavior as `heart` for `petted`. Like gag body frames, new effects
may ship as a supplemental atlas such as `effects-notes.png`/`.json` when
changing the base `effects.png` would risk regressing existing overlays.

### Prop sprites (held/produced items, optional)

Some expressions need an item the pet interacts with (e.g. pulling a drink
bottle out for a `drink-break` gag). These are **not** part of the pet
body sheet or the effects sheet — they follow the existing item-sprite
pattern already used for cosmetics/food/furniture: a standalone PNG in
`ui/assets/sprites/items/`, registered in `ITEM_SPRITE_PATHS` in
`render.ts`, and drawn at a hand-picked offset the same way
`drawCosmetic()`/`drawFoodSkin()` already do. A pet form does not need to
author its own prop variant — props are shared, like effects.

The no-arms prop decision is resolved by
[[../tasks/done/0014-expanded-gag-expression-pack]]: props for
Hatchling-style forms may appear, float, or pop into place near the body
without being visibly held. Do not add temporary arms for a single gag.
`drink-break` uses shared prop sprite `prop-drink-bottle`: 16x16,
Sweetie-16-only, rounded bottle/canteen silhouette, `#41a6f6` blue body
(or `#38b764` green if the bottle read needs it), `#f4f4f4` highlight,
and 1px `#1a1c2c` outline/cap.

## Folder layout for a new pet form

```
ui/assets/sprites/<form-name>/
├── <form-name>.aseprite      # editable source
├── <form-name>.png           # base packed sheet: 7 body tags
├── <form-name>.json          # Aseprite JSON (array) with frameTags
├── <form-name>-gag-expressions.png   # optional supplemental gag atlas
└── <form-name>-gag-expressions.json  # gag frameTags, same schema
```

Match `hatchling/` exactly. No new effects/props folder — those are shared
per [Effect tags](#effect-tags) and [Prop sprites](#prop-sprites) above.

## Definition of "swappable"

A pet form is a valid drop-in replacement when:

- [ ] Its base sheet has every required body tag, and either the base sheet
      or supplemental gag atlas has every current authored `GAG_VARIANTS`
      tag, each with a `meta.frameTags` entry matching the schema in task
      0005.
- [ ] PNG is Sweetie-16-only opaque pixels, binary alpha (same lint as
      task 0005's acceptance criteria).
- [ ] Loops/terminates correctly for every tag in a manual pass (`once`
      tags return to `idle` cleanly).
- [ ] No changes needed to `behavior.ts`, `atlas.ts`, or `render.ts` —
      only the sprite folder is added and the base URL passed to
      `loadAtlas()` is swapped (or made selectable, if forms are meant to
      coexist rather than replace).

If any of those requires touching game logic, the form isn't following the
contract and the gap belongs in this doc, not worked around per-form.
