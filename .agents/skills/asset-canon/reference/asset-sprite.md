# SPRITE — reference for the `asset-sprite` specialist

Game art needs **strict consistency** in scale, palette, and pixel grid so sprites sit together in one world.

## STYLE SYSTEM (lock once per game)
- **Style:** pixel-art (fixed px resolution, e.g. 32×32 native) OR clean vector.
- **Pixel density (PPU):** one pixels-per-unit for the whole set, so characters, items, and tiles share scale.
- **Palette + ramps:** locked indexed palette (e.g. 16 colors) **and** a shadow→midtone→highlight ramp per material, reused for every sprite.
- **Lighting:** single consistent light direction (e.g. top-left).
- **Outline:** none / full / selective (selout) + color — pick one.
- **Camera:** top-down / side / 3-4 isometric — pick one for the whole set.
- **Proportions:** lock head:body ratio and limb scale so every character belongs to one cast.

See **SPRITE CRAFT** below for the principles behind each lock.

## SPRITE CRAFT — the fundamentals a standard sprite respects

This is the craft real game artists work by. Pixels here come from an image model, so each principle lands in one of three places: **(P)** bake it into the prompt / style lock, **(Q)** enforce it as a VERIFY/reject-and-regenerate criterion, **(F)** fix it in post-process. The model will *not* hand-place pixels — your job is to constrain it up front and reject what fails.

### 1. Readability before detail
- **Silhouette test.** A sprite must be recognizable as a solid black shape at game size — gesture-forward, distinct, no mushy blob. *(P: "strong readable silhouette, distinct recognizable shape" · Q: fill the alpha black at native size — still identifiable?)*
- **Judge at game size, generate large.** Detail that vanishes at 32–64 px is noise. Render big for clean edges, but accept/reject readability at the native cell. *(Q)*
- **Value carries the read.** The subject separates by **value** (light/dark), not hue alone; reserve the darkest darks for separation. *(P/Q)*
- **No tangents.** Edges that merely kiss (a limb grazing the body outline) flatten depth — overlap clearly or separate. *(Q)*

### 2. Pixel & resolution discipline
- **One pixel density (PPU) for the whole set.** A 32 px character and the tiles it stands on must agree on what "1 pixel" is. Mixed density is the #1 tell of an amateur set. *(P/Q)*
- **Native resolution, integer scale, nearest-neighbor.** Author at the cell size; scale only by whole-number factors with nearest-neighbor (never bilinear). Never upscale a small gen to fake resolution. *(F/Q)*
- **No stray pixels, no broken lines, no soft mush.** Crisp consistent edges; the model loves to add anti-aliased blur or noise — reject and regenerate, or clean in post. *(Q/F)*

### 3. Color — limited palette, ramps, hue-shift
- **Indexed palette + shared ramps.** Lock a small palette (≈16–32). Build a **color ramp** per material (shadow → midtone → highlight) and reuse the *same* ramps across every sprite — shared ramps are what unify a set. *(P/Q)*
- **Hue-shift, don't just darken.** Shadows shift hue (toward cool / the complement) and drop value+saturation; highlights shift warm. A ramp that only changes lightness reads dull and 3D-render-y. *(P)*
- **3–4 values per material; reserve pure black/white.** More reads muddy at sprite scale. Use a dark tinted color and an off-white instead of `#000`/`#fff`. *(P/Q)*

### 4. Light & form
- **One locked light direction** for the whole set (e.g. top-left) or sprites won't sit in one world. *(P/Q)*
- **Form shadow vs cast shadow vs AO.** Model the form across curved surfaces, add ambient occlusion where shapes meet, and keep any **contact/cast shadow as a separate cell** — engines usually want it as its own sprite, not baked under the feet. *(P)*
- **No pillow shading.** Light is directional, so shading is *not* symmetric concentric rings inward from every edge — that's the classic beginner mistake. *(P/Q)*

### 5. Outlines
- Pick **one** treatment for the set: **none** (flat/modern), **full** (classic, readable — dark tinted, not pure black), or **selective (selout)** (outline only where it aids separation, lighter on lit edges). State it in the style lock. *(P/Q)*

### 6. Animation craft (motion, not packing)
The FRAME GRID STANDARD below handles *packing*; these are the *motion* rules:
- **Key → breakdown → in-between.** Establish the extreme key poses first — the action reads from the keys. *(P, per-frame prompts)*
- **Timing & spacing.** Spacing (distance moved per frame) sets speed and weight; cluster frames at the extremes to ease in/out. Don't ship an evenly-spaced linear flipbook.
- **Squash & stretch** (preserve volume) for weight/impact; **anticipation** before a big move; **follow-through / overlap** after (hair, cloth, tails lag).
- **Smear frames** — 1–2 motion-blur-as-shape frames — read fast motion better than a crisp in-between.
- **Anchor stability.** Every frame shares the same pivot pixel (bottom-center for grounded characters) so playback doesn't jitter. *(Q — also the packing rule below)*
- **Loop seamlessly** for cyclic actions; the last frame must flow into the first — test the wrap. *(Q)*
- **Frame economy.** Mirror/reuse symmetric frames; a readable run is often 6–8 well-spaced frames, not 24 muddy ones.
- **Canonical cycles:** *walk* = contact → down → passing → up (×2 sides); *idle* = a subtle breathing loop, never frozen; *attack* = anticipation → strike (with a smear) → recovery. *(P)*

**How many frames + how fast — the established defaults.** These are the shipped-game conventions; pick a count, then carry it as `count`/`fps` in the atlas and the per-asset `animation` block. *The golden rule: timing beats count — 4 well-timed frames (uneven hold times) beat 12 evenly-spaced ones.* Don't pad frames to chase smoothness.

| Action | Frames (typical) | Playback FPS | Notes |
|---|---|---|---|
| **Idle** | **2** (range 2–6) | 4–6 (slow) | breathe in / breathe out, ~200–800 ms hold each; never a single static frame |
| **Walk** | **4** small (16×16) → **6–8** at 32×32+ | 8 casual, 10–12 brisk | 4 key poses: contact → down(recoil) → passing → up, mirrored for both legs |
| **Run** | **4–8** (6 is a safe default) | 10–15 | faster spacing than walk; real refs: Celeste run = 4, Shovel Knight walk = 6 |
| **Attack** | **3–5** | 10–15 (snappy) | anticipation → strike(+smear) → recovery; hold the impact frame |
| **Jump** | **1–3 per phase** | per phase | split into rise / peak / fall, not one cycle |

Guidance baked in:
- **8–12 FPS is the character-animation sweet spot.** Idle slower (4–6), attacks faster (10–15). **Above ~15 FPS a pixel sprite starts to look *too* smooth and loses the aesthetic** — more frames is not "better."
- **Animation FPS ≠ game render FPS.** The game renders at 60; the sprite *plays back* at 8–12 — that's the `fps` in the atlas, the rate frames advance, not the engine's frame rate.
- **More frames = more model generations = more cost + more drift.** Each frame is a separate image-model call here, so a 24-frame run is 24 chances for the subject to drift off-model. Prefer the low end (4–8) and lean on timing.

### 7. Tiles & tilesets
- **Seamless on the wrapped axes** — opposite edges line up (see TEXTURE reference's seam check). *(Q)*
- **Break visible repetition.** Avoid a unique high-contrast feature the eye latches onto when tiled; keep the field low-contrast and add variation tiles. *(P)*
- **Autotiling-ready for terrain.** Author the standard transition set (blob/47-tile or Wang 2-corner) so edges and corners connect; declare the scheme. *(P)*

### 8. One world — set consistency
Everything rolls up to: same **PPU/scale**, same locked **palette & ramps**, same **light direction**, same **outline** treatment, same **proportions** (lock head:body and limb scale). Break any one and the sprite reads as "from a different game." This is exactly what the shared **style profile** + per-sprite **style snapshot** exist to hold — write PPU, palette, ramps, light, and outline into the profile and reuse it for every sprite and every frame.

## CANVAS & OUTPUT
| Use | Native | Export |
|---|---|---|
| Character | 64×64 or 128×128 | @1x @2x png, transparent |
| Tile | 32×32 / 16×16 | seamless-tested png |
| Item | 64×64 | transparent png |
| Animation | N frames, one shared cell | packed sheet + atlas.json |

## PROMPT TEMPLATE
> "{style} game sprite of {subject}, {camera} view, {palette} palette, {light} lighting, {outline} outline, centered on a {NxN} grid, on a solid chroma-green #00B140 background with no green in the sprite, no drop shadow on the canvas, crisp edges."

**Always append these craft cues** (from SPRITE CRAFT above) so the model doesn't fall back to its slop defaults: *"strong readable silhouette, directional {light} light with hue-shifted shadows (cooler, not just darker), 3–4 values per material, no pillow shading, crisp pixels with no anti-aliased blur, consistent pixel density."*

Generate on the chroma plate, then key `#00B140` to alpha in post (see **CHROMA-KEY BACKGROUND** in the main SKILL.md). Never request "transparent" directly. **Reserve one slot of the locked palette as the chroma plate and exclude it from every sprite** so keying never eats sprite pixels. If sprites are predominantly green (forests, slimes), switch the plate to chroma-magenta `#FF00FF` for the whole set.

**GOOD:** a red-and-steel knight on a `#00B140` plate → keying yields clean alpha around the silhouette.
**BAD:** a green slime on a `#00B140` plate → keying punches a hole through the slime; use a `#FF00FF` plate instead.

## FRAME GRID STANDARD (animation-ready)

A sheet is "detectable" only if frame `i`'s rectangle is **computable from the sheet alone** — no per-frame lookup needed. Lock these so any packer/reader derives positions with pure arithmetic:

- **Uniform cell.** Every frame occupies the exact same `cellW × cellH` box. Power-of-two cells only: `16, 32, 64, 128, 256`. Never mix cell sizes in one sheet.
- **Zero gutter, zero margin.** Frames butt edge-to-edge — no padding between cells, no border around the sheet. (If bleed is unavoidable, declare a single fixed `gutter` and apply it uniformly; the reader subtracts it. Default 0.)
- **Row-major order.** Fill left→right, then top→bottom. Frame 0 is top-left. No gaps, no skipped cells; trailing unused cells (if any) sit at the bottom-right and are excluded by `count`.
- **Fixed column count.** Declare `columns`. Then `rows = ceil(count / columns)`, `sheet = (columns·cellW) × (rows·cellH)`. One action per row is a good convention (`columns` = longest action's frame count).
- **Shared registration/anchor.** Every frame's pivot lands on the same pixel inside its cell (e.g. bottom-center `anchor: [0.5, 1.0]`). This is what stops the sprite from jittering during playback — the subject must not drift cell-to-cell.
- **Sequential, zero-padded names** in playback order: `hero_run_00.png … hero_run_07.png`. The numeric suffix *is* the frame index.

**Reader math** — given the atlas `meta`, frame `i` is:
```
cols = sheet.w / (cell.w + gutter)
x = (i % cols) * (cell.w + gutter)
y = floor(i / cols) * (cell.h + gutter)
w = cell.w ,  h = cell.h
```

## SPRITESHEET PACKING

Lay frames out row-major on the grid above, then write the atlas yourself with the Write tool — you have the cell size, `columns`, and frame order, so every `x,y,w,h` is the reader math (no library needed for the data). Composing the actual sheet **image** does need an image library; use a script that's already present, or write a short visible helper into the user's project. Emit the atlas in whatever format(s) the target engine reads.

The atlas carries the playback contract (cell, columns, count, fps, loop, anchor) — not just per-frame boxes:
```json
{
  "meta": {
    "image": "hero_run-512x128.png",
    "sheet":   { "w": 512, "h": 128 },
    "cell":    { "w": 64,  "h": 64 },
    "gutter":  0,
    "columns": 8,
    "count":   8,
    "fps":     12,
    "loop":    true,
    "anchor":  [0.5, 1.0]
  },
  "frames": {
    "hero_run_00": { "x": 0,   "y": 0, "w": 64, "h": 64 },
    "hero_run_07": { "x": 448, "y": 0, "w": 64, "h": 64 }
  }
}
```
With `meta` alone a reader reconstructs every frame rect via the reader math; `frames` is a convenience/verification map. Multiple actions → either one row per action (with a `clips` map like `{ "run": [0,7], "jump": [8,11] }`) or one sheet+atlas per action.

> *Optional (repo/CI):* the packer composes the sheet and emits all formats in one go —
> ```bash
> node "${CLAUDE_PLUGIN_ROOT:-.}/scripts/pack-sprite.mjs" --in assets/generated/sprites/hero_run \
>   --name hero_run --columns 8 --fps 12 --formats json,xml,texturepacker --clips run:0-7,jump:8-11
> ```

**Atlas format options** (pick what the engine reads):
| Format | File | Reads natively |
|---|---|---|
| `json` | `<name>.json` | asset-canon native schema (full playback contract) |
| `xml` | `<name>.xml` | TexturePacker/Starling `<TextureAtlas>` — Cocos2d, Starling |
| `texturepacker` | `<name>.tp.json` | TexturePacker JSON-Hash — Phaser, PixiJS, many Godot/Unity importers |

All formats carry the same `x,y,w,h` per frame, so the sheet cuts identically whichever you choose.

## CHECKS
- [ ] **Silhouette reads** — recognizable as a solid black shape at native size; no tangents flattening depth.
- [ ] **Consistent pixel density (PPU)** with the rest of the set; native-res, integer-scaled, nearest-neighbor (no soft/bilinear blur, no stray pixels).
- [ ] **Shading is directional** (locked light dir) and **hue-shifted**, not pillow-shaded; 3–4 values per material from the shared ramps.
- [ ] **Animation craft** (if animated) — eased spacing not linear, stable anchor (no jitter), seamless loop for cyclic actions.
- [ ] Chroma plate fully keyed to alpha; sprite has no interior holes from keying.
- [ ] Sprite palette excludes the plate color (green family, or magenta if plated magenta).
- [ ] Uniform power-of-two cell; zero gutter/margin (or one declared gutter); row-major, no gaps.
- [ ] `sheet.w == columns·cell.w` and `sheet.h == ceil(count/columns)·cell.h` — reader math resolves.
- [ ] Every frame identical canvas + shared anchor pixel (no subject drift across frames).
- [ ] Frames zero-padded and named in playback order; atlas `meta` carries cell/columns/count/fps/loop/anchor.
- [ ] Atlas emitted in the format the target engine reads (json native / xml Starling / texturepacker JSON-Hash).
- [ ] Palette stays within the locked index set.
- [ ] Tiles pass the seamless-edge check (delegate to the TEXTURE reference's check).
- [ ] Sidecar `docs/assets/<slug>.yaml` written, including the `animation` block (cell/columns/count/fps/loop/anchor/clips) so motion is reconstructable without opening the sheet.

## OUTPUT & FINISH

These specialize the main SKILL.md's OUTPUT TARGET detection and VERIFY gate for sprites:

1. **Where to write.** Append the `sprites/` subfolder to the framework-detected target (e.g. `public/assets/sprites/`). An explicit output dir from the user always wins. Descriptors and style snapshots always live under `docs/`; the sheet PNG + atlas go to the framework target.

2. **Post-process needs `sharp`.** Keying the chroma plate to alpha, resizing, and composing the actual sheet PNG all need it. If it's missing, recommend `npm install sharp` and wait for the user — never ship a sprite that still has its chroma plate.

3. **Key the plate by tolerance, then re-check.** The `#00B140` plate doesn't come back flat. Key by color **distance / hue band**, not exact match; suppress edge spill; then scan opaque pixels to confirm **no plate-family color survives** and the cell margins read alpha 0. Residue → widen tolerance and re-key. (Predominantly-green sprites use the magenta plate instead.)

4. **VERIFY before calling it done.** Confirm on the files on disk: naming `<slug>-<variant>-<WxH>.<ext>`; real pixels = the `WxH` in the name; transparent background fully cut, no interior holes; frame count + cell size match the atlas; palette in budget; sidecars `docs/assets/<slug>.yaml` **and** `docs/assets/styles/style-profile-<slug>.yaml` exist. Report `✓ PASS` / `✗ FAIL: <reason>` per asset and fix fails before reporting. (Universal gate; the checklist above runs *on top of* it.)

> The per-asset **style snapshot** (`docs/assets/styles/style-profile-<slug>.yaml`) is the resolved style recipe that produced the sprite — freeze it on write so a future frame/variant reproduces it.
