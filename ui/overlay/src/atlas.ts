// Aseprite sprite atlas loading (per ADR-0003, see main.ts for the overview).
// `ui/assets/sprites/hatchling` for the pet body, `.../effects` for
// hearts/exclaim/dust/zzz. The build copies both atlases next to
// `overlay.js` (see `ui/overlay/package.json`) and this module fetches them
// at startup.

import type { PetMode } from "./types";

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
void loadAtlas("./sprites/hatchling").then((atlas) => {
  hatchlingAtlas = atlas;
});
void loadAtlas("./sprites/effects").then((atlas) => {
  effectsAtlas = atlas;
});

/** Maps the behavior state machine's modes onto the hatchling atlas's
 * animation tags. The 0012 physics/override modes (tumble, climb, dizzy,
 * sulk, gag) have no dedicated art yet, so each reuses the closest existing
 * pose rather than blocking on a full art pass - documented per mode below. */
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
  gag: "idle", // paired with a gag-specific effect (sneeze/stare/chase-tail)
  landing: "idle", // brief recovery beat after a jump-down, paired with the landing squash
};
