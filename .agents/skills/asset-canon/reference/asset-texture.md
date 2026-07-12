# TEXTURE — reference for the `asset-texture` specialist

A texture is only good if it **tiles seamlessly**. Generation is half the job; the seam check is the other half.

## SYSTEM
- **Category:** organic (paper/fabric/stone) / geometric (grid/dots/waves) / noise / gradient-mesh.
- **Tonality:** keep contrast low for backgrounds so foreground text stays readable.
- **Palette:** 1–3 colors, derive from project tokens.
- **Scale:** define the tile size the texture is designed to repeat at.

## CANVAS & OUTPUT
| Use | Master | Export |
|---|---|---|
| Background tile | 1024×1024 | 512, 256 webp + png |
| Hi-dpi surface | 2048×2048 | @1x @2x webp |

## PROMPT TEMPLATE
> "Seamless tileable {category} texture, {palette} palette, low contrast, even lighting, no focal point, no visible seams when repeated, edges designed to wrap continuously, flat top-down view."

## SEAMLESS CHECK (required before write)
Tile the image 2×2 and inspect the center cross for discontinuity. If seams appear:
- offset-wrap and heal the seam, or
- regenerate with a stronger "edges wrap continuously" constraint.
Only ship after a clean 2×2 tile.

## CHECKS BEFORE WRITING
- [ ] Tiles seamlessly (clean 2×2 wrap, no visible seam).
- [ ] Low contrast / no focal point so foreground content stays readable.
- [ ] Within the 1–3 color budget; verify with `asset-qa --max-colors 3`.
- [ ] Exported at the declared tile size, no upscale.
- [ ] Sidecar `docs/assets/<slug>.yaml` written, with `tileable: true` + `tile_size` + `tonality` so it's reused without opening the image.

## OUTPUT & FINISH

Textures are **full-bleed — no chroma keying.** These specialize the main SKILL.md's OUTPUT TARGET detection and VERIFY gate for textures:

1. **Where to write.** Append the `textures/` subfolder to the framework-detected target (e.g. `public/assets/textures/`). An explicit output dir from the user always wins. Descriptors and style snapshots always go under `docs/`, regardless of framework.

2. **Post-process needs `sharp`.** Resizing and webp/png export need it. If it's missing, recommend `npm install sharp` and wait for the user before doing the resize ladder.

3. **VERIFY before calling it done.** Confirm on the files on disk: naming `<slug>-<variant>-<WxH>.<ext>`; real pixels = the `WxH` in the name; **seamless edge-wrap verified** (no visible tile seam); within the 1–3 color budget (`asset-qa --max-colors 3`); every requested size emitted; sidecars `docs/assets/<slug>.yaml` (with `tileable`/`tile_size`/`tonality`) **and** `docs/assets/styles/style-profile-<slug>.yaml` exist. Report `✓ PASS` / `✗ FAIL: <reason>` per texture and fix fails before reporting. (Universal gate; the checklist above runs *on top of* it.)

> The per-asset **style snapshot** (`docs/assets/styles/style-profile-<slug>.yaml`) is the resolved style recipe that produced the texture — freeze it on write so a future variant reproduces it.
