# STYLE-EXTRACT — reference for the `asset-style-extract` pre-step

Turn **one reference image into a reusable style contract.** The user imports an image; you *read the style out of it* — not the subject — and write a `docs/assets/styles/style-profile-<slug>.yaml` that the generation pipeline (main SKILL.md + specialists) reads on every later asset so the whole set looks like the reference.

This pre-step **does not generate any asset.** Its only output is a **style profile** (and optionally a saved copy of the reference). After it runs, generation continues through the normal pipeline, which already reads the profile (see the main SKILL.md's STYLE PROFILE + GENERATE sections).

```
INGEST  ->  ANALYZE (A: measured · V: judged)  ->  DRAFT PROFILE  ->  CONFIRM low-confidence  ->  WRITE  ->  HAND OFF
```

## 0. HARD RULES — READ FIRST

1. **Actually look at the image.** Read the file with the vision-capable model and inspect pixels with code. **Never invent a palette or a style from the filename or the subject noun** — a profile that wasn't measured against the image is worthless. If no image is reachable or it can't be opened, stop and say so.
2. **Extract STYLE, not SUBJECT.** The reference shows *a knight* — the profile must capture *how it's drawn* (pixel-art, 3-tone cel, top-left light, selout outline), **never** "a knight." Bake the *medium and treatment* into `prompt_suffix`; leave the *what* to each later brief. Overfitting the subject into the profile is the #1 failure of this pre-step.
3. **Mark confidence; don't fake certainty.** Some fields are measurable (palette, contrast, aspect); some are a judgment call (hue-shift, shading model, stylization). Record a `confidence` per group and **confirm the low-confidence ones with the user before writing** — a senior stylist says "I read this as cel-shaded, ~70% sure," not a flat assertion.
4. **One reference → one profile version.** Extracting from a new reference **bumps the `id`** (e.g. `ref-hero-v1`). Don't silently overwrite an existing `docs/assets/styles/style-profile-<slug>.yaml` from a different source — show before→after and confirm, exactly like the main SKILL.md's RESTYLE flow.
5. **Stay backward-compatible.** The profile you write must still satisfy the canonical shape in the main SKILL.md's STYLE PROFILE section: `palette` is a **flat hex list** (required), `prompt_suffix` and `id` required. All the richer analysis (`swatches`, `ramps`, `color`, `fx`, `confidence`, `source_ref`, `medium`) goes in the **optional** blocks so the validator and every existing reader still work.

## 1. INGEST — get the reference on disk

- Resolve the image the user imported (a path, a pasted/attached image, or a URL they gave). If it isn't already in the repo, **save a copy** to `docs/assets/refs/<slug>.<ext>` and record that path as `source_ref` — provenance, and the handle a future image-conditioning backend will pass straight through.
- If the user imports **several** references for one look, treat them as one style sample: extract the *common* style, and note any divergence rather than averaging into mush.

## 2. THE EXTRACTION CHECKLIST — what to read, in priority order

Sorted by how much each one drives "does it look the same." `(A)` = **measure it with code** (sharp/quantize — see ANALYZE). `(V)` = **judge it with the eye/vision model**. When a generated asset later looks "off," debug in *this* order — fixing fx before medium is chasing ornaments while the foundation is wrong.

### 1. Color — the strongest "same/not-same" signal
- **Palette with roles + weight.** Not a flat list — each color tagged `role` (bg / subject / accent / line / shadow) **and** `weight %` (area share). The generator must know which color *dominates*, not treat 12 colors as equal. *(A)*
- **Ramps per material** — shadow → midtone → highlight, not just a base color. Shared ramps are what make a set feel like one world. *(A clusters · V names the material)*
- **Hue-shift** in shadows/highlights — do shadows lean cool/complement, highlights warm, and by how much? A ramp that only changes lightness reads dull and 3D-render-y. *(V)*
- **Saturation + temperature/white-balance** — muted / pastel / vivid; warm or cool cast. *(A)*
- **Contrast & value range** — high-key / low-key; is pure `#000`/`#fff` used or tinted near-black/off-white? *(A)*
- **Color harmony** — complementary / analogous / triadic / monochrome. *(V)*
- **Background / plate** — the bg color, and whether a chroma plate must switch (green subject → magenta plate). *(A/V)*

### 2. Medium / rendering technique — "drawn with what"
- **Medium**: pixel-art / clean-vector / painterly / cel-shaded / 3d-clay-render / photographic / gradient-mesh. Read this wrong and every other field is meaningless. *(V)*
- **If pixel-art**: native pixel resolution (32/64…), PPU, hard-edge vs anti-aliased. *(A measures grid · V confirms)*
- **Surface texture**: grain / noise / visible brush stroke / canvas, or dead-flat. *(V)*

### 3. Line / outline
- **Outline present?** If so: `weight`, `color` (pure black vs tinted), style `none | uniform | tapered | selout`. *(V)*
- **Shape language**: rounded vs angular, soft vs sharp corners. *(V)*

### 4. Light & shading
- **Light direction** (top-left…) — must be one consistent direction. *(V)*
- **Shading model**: flat / two-tone-cel / soft-gradient, and **values per material** (3–4 is the sprite norm). *(V)*
- **Specular/highlight** strength, **AO** at shape junctions, **cast shadow** baked or separate. *(V)*
- **Pillow-shading present?** If the ref has it, fine; if not, add it to `negative`. *(V)*

### 5. Form, proportion & composition
- **Stylization level**: realistic / semi / chibi; **head:body ratio**, limb scale. *(V)*
- **Level of detail** — minimal flat vs rich illustration, so the generator neither adds nor strips detail. *(V)*
- **Camera/perspective**: front-flat / side / 3-4 iso / top-down. *(V)*
- **Framing**: subject placement, margins, crop, full-bleed or not. *(A aspect · V framing)*

### 6. Post / finishing FX
- Vignette, bloom/glow, chromatic aberration, film grain, gradient overlay, drop-shadow. Easy to spot as "off," easy to forget to capture. *(V)*

## 3. ANALYZE — measure what you can, judge the rest

**(A) — measured with code (needs `sharp`).** Don't eyeball what a computer reads exactly:
- **Dominant palette + weights:** quantize the image (e.g. reduce to ~8–16 colors) and read each cluster's hex and pixel share → fills `palette` + `swatches[].weight`.
- **Saturation / temperature / contrast:** mean & spread of S and L in HSL; warm/cool from average hue → `color` block.
- **Aspect & framing box:** dimensions and the subject's bounding box vs canvas → `framing`.
- **Edge density / grid:** rough pixel-art-vs-smooth read from edge histograms.

Check `sharp` is reachable first (`node -e "require('sharp')"`); if missing, recommend `npm install sharp`, and meanwhile fall back to (V) estimates **flagged low-confidence**.

**(V) — judged with the vision model.** Medium, hue-shift, shading model, outline style, stylization, light direction, post-FX. State each as a reading with a rough confidence, not a fact.

> *Optional (repo/CI):* `node "${CLAUDE_PLUGIN_ROOT:-.}/scripts/extract-palette.mjs" --in docs/assets/refs/<slug>.png --colors 12` prints the quantized palette + weights as YAML you can paste under `palette`/`swatches`. The agent can also do this inline with a short sharp helper.

## 4. DRAFT THE PROFILE — extended, backward-compatible shape

Same file the pipeline already reads (`docs/assets/styles/style-profile-<slug>.yaml`); the new blocks are **all optional** so the validator and every existing reader keep working.

```yaml
id: ref-hero-v1                      # required — bump per new reference
source_ref: docs/assets/refs/hero.png  # the analyzed image (provenance + future conditioning)
medium: pixel-art                    # pixel-art | clean-vector | painterly | cel-shaded | 3d-clay | photographic

palette:                             # required — FLAT hex list (validator-compatible)
  - "#1A1C2C"
  - "#FF5C39"
  - "#F4F4F4"

swatches:                            # optional — the same colors, with role + area weight
  - { hex: "#1A1C2C", role: bg,     weight: 0.42 }
  - { hex: "#FF5C39", role: accent, weight: 0.08 }
  - { hex: "#F4F4F4", role: subject, weight: 0.30 }

ramps:                               # optional — shadow -> midtone -> highlight per material
  skin:  ["#7A3B2E", "#C96F4A", "#F2A47E"]
  metal: ["#3A4A5A", "#7D93A6", "#C9DCE8"]

color:                               # optional — measured character of the palette
  saturation: muted                  # muted | pastel | vivid
  temperature: warm                  # warm | neutral | cool
  contrast: high                     # low | medium | high
  harmony: complementary
  hue_shift: "shadows cooler ~20°, highlights warm"

line:    { weight: "selout", color: "#2A1B12", style: selout }
shading: cel-2tone                   # flat | two-tone | cel-2tone | soft-gradient
values_per_material: 3
lighting: top-left
fx: [film-grain-light]               # optional — vignette | bloom | chromatic-aberration | grain | none

proportions: { head_body: "1:4", stylization: chibi }
camera: side

# required — the positive style anchor, synthesized from everything above.
# STYLE only — never the subject.
prompt_suffix: "pixel-art, 3-tone cel shading, top-left light with cooler shadows, selout outline, muted warm complementary palette, light film grain, crisp edges"

# anti-slop guard — built from what the reference does NOT have
negative:
  - "pillow shading"
  - "fake 3D bevel"
  - "soft anti-aliased blur"
  - "purple/blue AI glow"

confidence:                          # optional but EXPECTED from this pre-step
  palette: high
  medium: high
  lighting: medium
  hue_shift: low

seed: 73122
```

## 5. CONFIRM — surface the low-confidence reads before writing

Show the user the draft, but specifically flag anything `confidence: low/medium` and ask:
> *"Read this reference as **pixel-art, 3-tone cel, top-left light**. Less sure about: hue-shift (looks like cooler shadows?) and stylization (chibi vs semi). Lock these as drafted, or adjust? — **looks right** / **fix a few** / **show me each**."*

A senior stylist confirms judgment calls; only measured fields ship without asking.

## 6. WRITE + HAND OFF

- **Write `docs/assets/styles/style-profile-<slug>.yaml`** (the shared project profile) with the agreed shape above, then point `docs/assets/styles/active.yaml` at it (`active: style-profile-<slug>.yaml`) so the pipeline picks up the new look. If a profile already exists from a *different* source, write the new one under its own `id` and repoint `active.yaml` — show before→after rather than clobbering the old file (see the main SKILL.md's RESTYLE section).
- **Save the reference** under `docs/assets/refs/` if not already there, matching `source_ref`.
- **Sanity-check by reading back**: `id`, a hex `palette`, and `prompt_suffix` are present (the same gate the pipeline expects). *(Optional: `validate-style-profile.mjs --in docs/assets/styles/style-profile-<slug>.yaml`.)*
- **Hand off — don't generate here.** Tell the user the profile is set and what to do next:
  - new assets in this look → describe the subject; the normal pipeline applies the profile automatically;
  - bring an existing set into this look → that's the main SKILL.md's **RESTYLE** flow (profile is already updated → confirm → regenerate from descriptors).

## CHECKS
- [ ] The reference was **actually opened** (vision read + pixel measurement), not inferred from the name/subject.
- [ ] Profile captures **style, not subject** — `prompt_suffix` has no subject noun from the reference.
- [ ] `palette` is a **flat hex list** (validator-safe); roles/weights live in `swatches`.
- [ ] Measured fields (palette, contrast, saturation, aspect) came from code; judged fields carry a `confidence`.
- [ ] Low/medium-confidence fields were **confirmed with the user** before writing.
- [ ] `source_ref` points at a saved copy of the reference under `docs/assets/refs/`.
- [ ] Writing over an existing profile from a different source **bumped `id`** and showed before→after.
- [ ] Read-back passes: `id` + hex `palette` + `prompt_suffix` present; profile is reproducible.
- [ ] Handed off (generate / RESTYLE) — this pre-step itself produced **no image asset**.

> **Scope note:** today this profile is text/structured conditioning — the pipeline injects `prompt_suffix` + `Avoid: <negative>`. `source_ref` is recorded so that when an image-conditioning backend (reference images to gpt-image-1, or a local SD + LoRA) is wired in, the reference itself can be passed for a much tighter visual lock. That backend step is future work; the profile is forward-compatible with it.
