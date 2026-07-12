# ILLUSTRATION — reference for the `asset-illustration` specialist

The job is a **reusable style system**, not a single picture. Define the system once, then every illustration inherits it.

## STYLE SYSTEM (lock once per product)
- **Palette:** 4–6 colors max, one accent. Reuse across all art.
- **Line:** none (flat) / uniform / tapered — pick one.
- **Shading:** flat / two-tone / soft-gradient — pick one.
- **Geometry:** rounded-organic vs. sharp-geometric.
- **Proportions:** if characters appear, fix head:body ratio and stick to it.
- **Perspective:** flat-front / slight-isometric — pick one.

## CANVAS & OUTPUT
| Use | Master | Exports |
|---|---|---|
| Hero | 2400×1600 | 1600, 1200, 800 webp + png, transparent or scene bg |
| Spot | 1024×1024 | 512, 256 webp+png, transparent |
| Empty state | 1024×768 | 768, 512 webp, transparent |

## PROMPT TEMPLATE
> "{style} illustration of {scene}, palette {hexes}, {shading} shading, {line} linework, {perspective} perspective, generous negative space, no text, no UI chrome, on a solid chroma-green #00B140 background with no green used in the artwork, cohesive with a {brand-vibe} product."

Applies only when the illustration ships with a transparent/cut-out background (spot, empty state, or a hero with no scene bg). Generate on the chroma plate, then key `#00B140` to alpha in post (see **CHROMA-KEY BACKGROUND** in the main SKILL.md). Never request "transparent" directly. **Keep the green family (`{hexes}`) out of the artwork**, or keying will eat matching regions. If the scene needs green (foliage, landscapes), either ship it as a full scene-bg hero (no keying) or use a chroma-magenta `#FF00FF` plate.

**GOOD:** a person at a desk in warm neutrals + one orange accent on a `#00B140` plate → clean cut-out.
**BAD:** a garden scene full of green plants on a `#00B140` plate → keying shreds the foliage; keep it as a scene-bg hero or plate it magenta.

## CHECKS
- [ ] If cut-out: chroma plate fully keyed, no interior holes; artwork avoids the plate color.
- [ ] Same palette + line + shading as the rest of the set.
- [ ] No embedded text (text belongs in code/HTML, not the raster).
- [ ] Composition leaves room for headline overlay if it's a hero.
- [ ] Sidecar `docs/assets/<slug>.yaml` written, with a `composition` note (where the negative space is) so it's placed without opening the image.

## OUTPUT & FINISH

These specialize the main SKILL.md's OUTPUT TARGET detection and VERIFY gate for illustrations:

1. **Where to write.** Append the `illustrations/` subfolder to the framework-detected target (e.g. `public/assets/illustrations/`). An explicit output dir from the user always wins. Descriptors and style snapshots always go under `docs/`, regardless of framework.

2. **Post-process needs `sharp`.** Keying the chroma plate to alpha, resizing, and webp/png export all need it. If it's missing, recommend `npm install sharp` and wait for the user — never ship a cut-out that still has its chroma plate.

3. **Cut-out only: key the plate by tolerance, then re-check.** For spot/empty-state/cut-out heroes generated on the `#00B140` plate: key by color **distance / hue band**, not exact match (the plate isn't flat); suppress edge spill; then scan opaque pixels to confirm **no plate-family color survives** and corners read alpha 0. Residue → widen tolerance and re-key. Full-scene heroes keep their background — skip keying.

4. **VERIFY before calling it done.** Confirm on the files on disk: naming `<slug>-<variant>-<WxH>.<ext>`; real pixels = the `WxH` in the name; if cut-out, background fully cut with no interior holes; palette in budget; every requested format/size emitted; sidecars `docs/assets/<slug>.yaml` **and** `docs/assets/styles/style-profile-<slug>.yaml` exist. Report `✓ PASS` / `✗ FAIL: <reason>` per asset and fix fails before reporting. (Universal gate; the checklist above runs *on top of* it.)

> The per-asset **style snapshot** (`docs/assets/styles/style-profile-<slug>.yaml`) is the resolved style recipe that produced the illustration — freeze it on write so a future variant reproduces it.
