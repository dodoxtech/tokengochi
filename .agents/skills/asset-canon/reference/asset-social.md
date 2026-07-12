# SOCIAL — reference for the `asset-social` specialist

Social images live at **exact platform sizes** with hard safe areas. Get the dimensions and text legibility right.

## PLATFORM DIMENSIONS (exact)
| Platform | Size | Notes |
|---|---|---|
| OG default | 1200×630 | keep key content in center 1100×540 |
| Twitter/X summary_large | 1200×628 | |
| YouTube thumb | 1280×720 | bottom-right covered by duration chip |
| LinkedIn | 1200×627 | |
| Instagram post | 1080×1080 | |

## COMPOSITION
- **One brand palette**, one accent. Match the site.
- **Safe area:** keep logo + headline inside the inner 90%.
- **Text:** prefer overlaying real text via SVG/HTML in post, not baked into the AI raster (raster text warps). If text must be in the image, keep it short and large.
- **Hierarchy:** one focal element, clear background/foreground separation.

## PROMPT TEMPLATE (background plate)
> "{style} social card background, {palette} palette, strong left-to-right or center composition with clear empty space for a headline overlay, {brand-vibe}, no text, no logos, {WxH} aspect."

## PIPELINE NOTE
1. Generate the background plate (no text).
2. Overlay headline + logo via the project's SVG/HTML template (deterministic, crisp).
3. Export PNG at exact size + a compressed webp.

## CHECKS BEFORE WRITING
- [ ] Exact platform dimensions (e.g. 1200×630 for OG); verify with `asset-qa`.
- [ ] Key content inside the safe area (inner 90%).
- [ ] Headline text overlaid via SVG/HTML, not baked/warped into the raster.
- [ ] Single brand palette, one accent — consistent with the site.
- [ ] Sidecar `docs/assets/<slug>.yaml` written, with `platform`, `safe_area`, and `text_overlay` notes so it's reused without opening the image.

## OUTPUT & FINISH

Social cards are **full-bleed — no chroma keying.** These specialize the main SKILL.md's OUTPUT TARGET detection and VERIFY gate for social cards:

1. **Where to write.** Append the `social/` subfolder to the framework-detected target (e.g. `public/assets/social/`). An explicit output dir from the user always wins. (OG images are usually referenced by absolute URL in `<meta>` tags, so a `public/`-served path is the common case.) Descriptors and style snapshots always go under `docs/`, regardless of framework.

2. **Post-process needs `sharp`.** Resizing and webp/png/jpg export need it. If it's missing, recommend `npm install sharp` and wait for the user before the export step.

3. **VERIFY before calling it done.** Confirm on the files on disk: naming `<slug>-<variant>-<WxH>.<ext>`; **exact platform dimensions** (e.g. 1200×630 for OG); key content inside the safe area (inner 90%); headline overlaid via SVG/HTML, not baked/warped into the raster; single brand palette; sidecars `docs/assets/<slug>.yaml` (with `platform`/`safe_area`/`text_overlay`) **and** `docs/assets/styles/style-profile-<slug>.yaml` exist. Report `✓ PASS` / `✗ FAIL: <reason>` per card and fix fails before reporting. (Universal gate; the checklist above runs *on top of* it.)

> The per-asset **style snapshot** (`docs/assets/styles/style-profile-<slug>.yaml`) is the resolved style recipe that produced the card — freeze it on write so a future variant reproduces it.
