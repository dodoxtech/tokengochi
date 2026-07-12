# ICON — reference for the `asset-icon` specialist

Generate icon **families** — not one-off pictures. Consistency across the set is the whole job.

## DESIGN SYSTEM (lock these once per set)
- **Grid:** 24px live area on a 32px canvas (or 1024 master for app icons).
- **Stroke:** one weight for the whole set (e.g. 2px @ 24px). Never mix.
- **Corner radius:** one value. Terminals rounded or square — pick one.
- **Optical balance:** circular glyphs slightly larger than square ones so they read equal.
- **Style:** line / solid / duotone — pick ONE per set.
- **Palette:** monochrome by default (single fg color), accent only if briefed.

## CANVAS & OUTPUT
| Use | Master size | Exports |
|---|---|---|
| UI icon set | 1024×1024 | 512, 256, 128, 64, 32 png+webp, transparent |
| Favicon | 512×512 | favicon.ico (16/32/48), 180 apple-touch, 512 maskable |
| iOS app icon | 1024×1024 | full Contents.json size ladder, opaque, no alpha |
| Android | 1024×1024 | adaptive fg+bg layers, 432 mipmaps |

## PROMPT TEMPLATE (per icon, fed to codex-imagegen)
> "Minimal {style} icon of {subject}, {stroke}px stroke, {radius}px rounded corners, centered on a 24px grid with even padding, single color {fg} on a solid chroma-green #00B140 background, no green in the icon itself, flat, no shadow, no gradient, no background shape, pixel-crisp edges."

Generate on the chroma plate, then key `#00B140` to alpha in post (see **CHROMA-KEY BACKGROUND** in the main SKILL.md). Never request "transparent" directly. If the icon's own color is green, set `{fg}` away from green or swap the plate to chroma-magenta `#FF00FF`.

**GOOD:** charcoal `#1A1A1A` cart glyph on a `#00B140` plate → keying leaves a clean, halo-free glyph.
**BAD:** a green recycling glyph on a `#00B140` plate → keying deletes the glyph's own green strokes, leaving a broken icon.

## CHECKS BEFORE WRITING
- [ ] Background actually transparent (chroma-green fully keyed out, not white).
- [ ] Icon color avoids the plate's green family — no interior holes after keying.
- [ ] All icons share stroke + radius + optical size.
- [ ] App icons that forbid alpha are flattened on the brand bg.
- [ ] Favicon ladder + apple-touch + maskable generated.
- [ ] Sidecar `docs/assets/<slug>.yaml` written (subject, placement/intended_use, alt_text, files).

## OUTPUT & FINISH

These specialize the main SKILL.md's OUTPUT TARGET detection and VERIFY gate for icons:

1. **Where to write.** Append the `icons/` subfolder to the framework-detected target (e.g. `public/assets/icons/`). An explicit output dir from the user always wins. Descriptors and style snapshots always go under `docs/`, regardless of framework.

2. **Post-process needs `sharp`.** Keying the chroma plate to alpha, resizing, and webp/png/ico export all need it. If it's missing, recommend `npm install sharp` and wait for the user — never ship an icon that still has its chroma-green plate.

3. **Key the plate by tolerance, then re-check.** The plate doesn't come back flat (`#00B140` arrives as a cloud of near-greens). Key by color **distance / hue band**, not exact match; suppress edge spill; then scan the opaque pixels and confirm **no plate-family color survives** and the corners read alpha 0. Residue → widen the tolerance and re-key.

4. **VERIFY before calling it done.** Confirm on the files on disk: naming `<slug>-<variant>-<WxH>.<ext>`; real pixels = the `WxH` in the name; background fully cut (no interior holes); palette in budget; every requested format/size emitted; sidecars `docs/assets/<slug>.yaml` **and** `docs/assets/styles/style-profile-<slug>.yaml` exist. Report `✓ PASS` / `✗ FAIL: <reason>` per icon and fix fails before reporting. (This is the universal gate; the icon-specific checklist above runs *on top of* it.)

> The per-asset **style snapshot** (`docs/assets/styles/style-profile-<slug>.yaml`) is the resolved style recipe that produced the icon — freeze it on write so a future variant reproduces it.
