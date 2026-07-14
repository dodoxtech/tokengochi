// Aseprite sprite atlas loading (per ADR-0003, see main.ts for the overview).
// `ui/assets/sprites/hatchling` for the pet body, `.../effects` for
// hearts/exclaim/dust/zzz. Task 0016 adds two supplemental atlases -
// `hatchling-gag-expressions` (sneeze/yawn/dance/drink-break body frames) and
// `effects-notes` (the `notes` effect for `dance`) - so the new gag variants
// don't require touching the proven base sheets. The build copies all of
// them next to `overlay.js` (see `ui/overlay/package.json`) and this module
// fetches them at startup.

import type { GagVariant, PetMode } from "./types";

export interface AtlasFrame {
  x: number;
  y: number;
  w: number;
  h: number;
  duration: number;
}

interface AtlasJson {
  frames: { frame: { x: number; y: number; w: number; h: number }; duration: number }[];
  meta: { frameTags: { name: string; from: number; to: number }[] };
}

export interface SpriteAtlas {
  image: HTMLImageElement;
  frames: AtlasFrame[];
  tags: Record<string, { from: number; to: number }>;
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error(`failed to load sprite image: ${src}`));
    image.src = src;
  });
}

async function loadAtlas(baseUrl: string): Promise<SpriteAtlas> {
  const [atlasJson, image] = await Promise.all([
    fetch(`${baseUrl}.json`).then((res) => res.json() as Promise<AtlasJson>),
    loadImage(`${baseUrl}.png`),
  ]);
  const frames = atlasJson.frames.map((entry) => ({
    x: entry.frame.x,
    y: entry.frame.y,
    w: entry.frame.w,
    h: entry.frame.h,
    duration: entry.duration,
  }));
  const tags: Record<string, { from: number; to: number }> = {};
  for (const tag of atlasJson.meta.frameTags) {
    tags[tag.name] = { from: tag.from, to: tag.to };
  }
  return { image, frames, tags };
}

/** Picks the frame for `tagName` at `elapsedMs` into that tag's loop,
 * respecting each frame's own Aseprite-authored duration rather than a
 * fixed frame rate. Falls back to the atlas's first frame for an unknown
 * tag (better a wrong pose than nothing while a future tag name typos). */
export function frameForTag(atlas: SpriteAtlas | null, tagName: string, elapsedMs: number): AtlasFrame | null {
  if (!atlas) {
    return null;
  }
  const tag = atlas.tags[tagName];
  const frames = tag ? atlas.frames.slice(tag.from, tag.to + 1) : atlas.frames.slice(0, 1);
  if (frames.length === 0) {
    return null;
  }
  const total = frames.reduce((sum, f) => sum + f.duration, 0);
  let t = total > 0 ? ((elapsedMs % total) + total) % total : 0;
  for (const f of frames) {
    if (t < f.duration) {
      return f;
    }
    t -= f.duration;
  }
  return frames[frames.length - 1];
}

export let hatchlingAtlas: SpriteAtlas | null = null;
export let effectsAtlas: SpriteAtlas | null = null;
export let gagExpressionsAtlas: SpriteAtlas | null = null;
export let notesAtlas: SpriteAtlas | null = null;
void loadAtlas("./sprites/hatchling").then((atlas) => {
  hatchlingAtlas = atlas;
});
void loadAtlas("./sprites/effects").then((atlas) => {
  effectsAtlas = atlas;
});
void loadAtlas("./sprites/hatchling-gag-expressions").then((atlas) => {
  gagExpressionsAtlas = atlas;
});
void loadAtlas("./sprites/effects-notes").then((atlas) => {
  notesAtlas = atlas;
});

/** Maps the behavior state machine's modes onto the hatchling atlas's
 * animation tags. The 0012 physics/override modes (tumble, climb, dizzy,
 * sulk) have no dedicated art yet, so each reuses the closest existing pose
 * rather than blocking on a full art pass - documented per mode below.
 * `gag` is resolved separately by `resolveGagBody()` below, since which
 * atlas/tag it plays depends on `pet.gagVariant`, not just the mode; the
 * "idle" entry here only covers the (unused) generic fallback. */
export const MODE_ANIMATION_TAG: Record<PetMode, string> = {
  idle: "idle",
  seek: "walk",
  eat: "eat",
  happy: "happy",
  sleep: "sleep",
  dragged: "drag",
  tumble: "drag", // airborne/limbs-out pose is the closest match to a fall
  climb: "walk", // climbing pace is already throttled via CLIMB_SPEED
  react: "react",
  dizzy: "react", // spiral-eyes overlay (drawSpiralEyes) supplies the "dizzy" read
  sulk: "walk", // storming off, facing away - the walk cycle carries it
  petted: "happy", // paired with the heart effect below
  gag: "idle",
  landing: "idle", // brief recovery beat after a jump-down, paired with the landing squash
};

/** Task 0016: per-variant gag body source. `sneeze`/`yawn`/`dance`/
 * `drink-break` have authored frames in the supplemental
 * `hatchling-gag-expressions` atlas; `stare`/`chase-tail` have no dedicated
 * art yet (per the pet-action-pack spec) and keep reusing the base atlas's
 * `idle` pose plus a procedural/effect overlay in render.ts. */
export const GAG_BODY_SOURCE: Record<GagVariant, { atlas: "hatchling" | "gag-expressions"; tag: string }> = {
  sneeze: { atlas: "gag-expressions", tag: "sneeze" },
  yawn: { atlas: "gag-expressions", tag: "yawn" },
  dance: { atlas: "gag-expressions", tag: "dance" },
  "drink-break": { atlas: "gag-expressions", tag: "drink-break" },
  stare: { atlas: "hatchling", tag: "idle" },
  "chase-tail": { atlas: "hatchling", tag: "idle" },
};

export function resolveGagBody(variant: GagVariant): { atlas: SpriteAtlas | null; tag: string } {
  const source = GAG_BODY_SOURCE[variant];
  return { atlas: source.atlas === "gag-expressions" ? gagExpressionsAtlas : hatchlingAtlas, tag: source.tag };
}
