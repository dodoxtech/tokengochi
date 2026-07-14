// Canvas drawing: pet sprite + effects, food, furniture, hover tooltip. See
// main.ts for the module overview.

import { ctx } from "./dom";
import { effectsAtlas, frameForTag, hatchlingAtlas, MODE_ANIMATION_TAG, notesAtlas, resolveGagBody } from "./atlas";
import type { SpriteAtlas } from "./atlas";
import { FOOD_BOUNCE_MS } from "./constants";
import { PET_SIZE, agentStatusBadge, clamp, foods, furnitureX, groundY, hover, pet, state } from "./state";
let animMode: string | null = null;
let animStartedAt = 0;

const ITEM_SPRITE_PATHS: Record<string, string> = {
  "hat-leaf": "./sprites/items/hat-leaf-sprite-32x32.png",
  "hat-mushroom": "./sprites/items/hat-mushroom-sprite-32x32.png",
  "food-default": "./sprites/items/food-default-sprite-32x32.png",
  "food-sushi": "./sprites/items/food-sushi-sprite-32x32.png",
  "food-banh-mi": "./sprites/items/food-banh-mi-sprite-32x32.png",
  "furniture-bed": "./sprites/items/furniture-bed-sprite-80x40.png",
  "furniture-plant": "./sprites/items/furniture-plant-sprite-80x40.png",
  "prop-drink-bottle": "./sprites/items/prop-drink-bottle-sprite-16x16.png",
};

const itemSprites = Object.fromEntries(
  Object.entries(ITEM_SPRITE_PATHS).map(([id, src]) => {
    const image = new Image();
    image.src = src;
    return [id, image];
  }),
) as Record<string, HTMLImageElement>;

function drawItemSprite(id: string, x: number, y: number, w: number, h: number): boolean {
  const image = itemSprites[id];
  if (!image?.complete || image.naturalWidth === 0) {
    return false;
  }
  ctx.drawImage(image, x, y, w, h);
  return true;
}

function drawPet(now: number): void {
  // Sitting on a ledge (climbPhase "sit") reads as idle - only the
  // approach/ascend/jump legs of "climb" are an actual walk/travel pose.
  const isSitting = pet.mode === "climb" && pet.climbPhase === "sit";

  const bob =
    pet.mode === "sleep"
      ? 2
      : (pet.mode === "seek" || pet.mode === "climb") && !isSitting
        ? Math.round(Math.sin(now / 95) * 3)
        : Math.round(Math.sin(now / 420) * 2);
  const happyHop = pet.mode === "happy" ? Math.abs(Math.sin(now / 115)) * 12 : 0;
  const landingSquash = now - pet.landedAt < 220 ? 1 - (now - pet.landedAt) / 220 : 0;

  const x = Math.round(pet.x);
  let y = Math.round(pet.y + bob - happyHop);
  let spin = 0;

  if (pet.mode === "tumble") {
    y = Math.round(pet.y);
    spin = (now / 90) % (Math.PI * 2);
  }

  // Keyed by climbPhase too so the animation clock resets when "climb"
  // switches between its walk-like legs and the idle-looking sit pose.
  const animKey = pet.mode === "climb" ? `climb:${pet.climbPhase}` : pet.mode;
  if (animKey !== animMode) {
    animMode = animKey;
    animStartedAt = now;
  }
  // `gag` picks its body atlas/tag per `pet.gagVariant` (task 0016's
  // supplemental hatchling-gag-expressions atlas for the authored variants,
  // the base atlas's `idle` tag for the still-procedural ones) rather than
  // through the generic mode->tag table other modes use.
  const bodySource =
    pet.mode === "gag" && !isSitting
      ? resolveGagBody(pet.gagVariant)
      : { atlas: hatchlingAtlas, tag: isSitting ? MODE_ANIMATION_TAG.idle : MODE_ANIMATION_TAG[pet.mode] };
  const bodyAtlas = bodySource.atlas;
  const frame = frameForTag(bodyAtlas, bodySource.tag, now - animStartedAt);

  ctx.save();
  ctx.translate(x + PET_SIZE / 2, y + PET_SIZE / 2);
  if (spin !== 0) {
    ctx.rotate(spin);
  }
  // Sprite art is authored left-facing by default, so scale(1,1) draws it facing left;
  // pet.facing is +1 for rightward movement, so invert it to mirror correctly.
  ctx.scale(-pet.facing, 1);
  if (landingSquash > 0) {
    // Brief squash-on-landing: wider and flatter for one short beat.
    ctx.scale(1 + landingSquash * 0.22, 1 - landingSquash * 0.3);
  }

  if (frame) {
    ctx.drawImage(
      bodyAtlas!.image,
      frame.x,
      frame.y,
      frame.w,
      frame.h,
      -PET_SIZE / 2,
      -PET_SIZE / 2,
      PET_SIZE,
      PET_SIZE,
    );
  }

  // No dizzy pose exists in the atlas - the spinning-eyes overlay is the
  // only way to convey it, so it's drawn on top of the (react-tagged) body
  // frame, inside the same facing/rotate transform.
  if (pet.mode === "dizzy" && now < pet.overrideUntil) {
    drawSpiralEyes(now);
  }

  drawCosmetic();

  ctx.restore();

  drawOverlayEffects(now, x, y);
}

function drawSpiralEyes(now: number): void {
  ctx.strokeStyle = "#1a1c2c";
  ctx.lineWidth = 1.5;
  for (const cx of [-9, 11]) {
    ctx.beginPath();
    for (let i = 0; i <= 12; i += 1) {
      const angle = i * 0.9 + now / 120;
      const radius = i * 0.3;
      const px = cx + Math.cos(angle) * radius;
      const py = -5 + Math.sin(angle) * radius;
      if (i === 0) {
        ctx.moveTo(px, py);
      } else {
        ctx.lineTo(px, py);
      }
    }
    ctx.stroke();
  }
}

/** Only these `equippedCosmetic` values are headwear (hats) sitting just
 * above the head - `scarf-sunset` shares the same cosmetic slot but sits at
 * the neck, so it must not trigger hat clearance below. */
const HEADWEAR_COSMETICS = new Set(["hat-leaf", "hat-mushroom"]);

/** Bubbles/badges that anchor above the head clip through a worn hat's brim
 * at their plain baseline offset, so they need a bit more clearance when
 * headwear is equipped - and, symmetrically, read better sitting closer to
 * the head (not floating) when the pet's bare-headed. */
const HAT_CLEARANCE_PX = 20;

function aboveHeadY(baseY: number): number {
  const wearingHat = !!state.equippedCosmetic && HEADWEAR_COSMETICS.has(state.equippedCosmetic);
  return wearingHat ? baseY : baseY + HAT_CLEARANCE_PX;
}

/** Effects drawn outside the pet's own rotate/scale transform, from the
 * effects atlas (`ui/assets/sprites/effects`): hearts, exclaim bubble, zzz,
 * landing/gag dust. `gagVariant === "stare"` has no matching effect frame
 * and stays a minimal procedural line - the pet's own idle pose already
 * carries that beat. */
function drawOverlayEffects(now: number, x: number, y: number): void {
  const inOverride = now < pet.overrideUntil;
  const cx = x + PET_SIZE / 2;

  if (pet.mode === "react" && inOverride && pet.reactVariant === "exclaim") {
    drawEffect("exclaim", cx, aboveHeadY(y - 14), now);
  }
  if (pet.mode === "petted" && inOverride) {
    drawEffect("heart", cx + 14, y - 10, now);
  }
  if (pet.mode === "sleep") {
    drawEffect("zzz", cx + PET_SIZE * 0.32, y - PET_SIZE * 0.12, now);
  }
  if (pet.mode === "gag" && inOverride) {
    drawGagEffect(cx, y, now);
  }
  if (now - pet.landedAt < 320) {
    drawEffect("dust", cx, y + PET_SIZE - 6, now);
  }

  drawAgentStatusBadge(now, cx, y);
}

/** Task 0017: agent turn-completed / needs-approval badge. Deliberately
 * independent of `pet.mode` - it never occupies the override-mode state
 * machine, so it can never block movement/eating/climbing and can't corrupt
 * behavior state the way changing `pet.mode` would. `"needs_approval"` bobs
 * gently to draw the eye without being intrusive; it persists (state.ts /
 * behavior.ts own the clear/timeout logic) rather than looping here. */
function drawAgentStatusBadge(now: number, cx: number, y: number): void {
  if (agentStatusBadge.status === "completed") {
    drawEffect("heart", cx, aboveHeadY(y - PET_SIZE * 0.55), now);
  } else if (agentStatusBadge.status === "needs_approval") {
    const bob = Math.sin(now / 260) * 3;
    drawEffect("exclaim", cx, aboveHeadY(y - PET_SIZE * 0.55) + bob, now);
  }
}

function drawEffectFromAtlas(atlas: SpriteAtlas | null, tagName: string, cx: number, cy: number, now: number): void {
  const frame = frameForTag(atlas, tagName, now);
  if (!atlas || !frame) {
    return;
  }
  const size = 22;
  ctx.drawImage(atlas.image, frame.x, frame.y, frame.w, frame.h, cx - size / 2, cy - size / 2, size, size);
}

function drawEffect(tagName: string, cx: number, cy: number, now: number): void {
  drawEffectFromAtlas(effectsAtlas, tagName, cx, cy, now);
}

/** `cx`/`y` are the pet's on-screen center-x and top-y (pre-effect-offset),
 * same anchor the other overlay effects use. `yawn` has no dedicated effect
 * (a wide-mouth silhouette is the whole read); `stare` still has no matching
 * effect frame and keeps its minimal procedural line. */
function drawGagEffect(cx: number, y: number, now: number): void {
  if (pet.gagVariant === "sneeze" || pet.gagVariant === "chase-tail") {
    drawEffect("dust", cx, aboveHeadY(y - 16), now);
    return;
  }
  if (pet.gagVariant === "dance") {
    drawEffectFromAtlas(notesAtlas, "notes", cx, aboveHeadY(y - PET_SIZE * 0.55), now);
    return;
  }
  if (pet.gagVariant === "drink-break") {
    // Beside the mouth on whichever side the pet is facing, rather than
    // held - the Hatchling has no arms (task 0005/0014). Not "above the
    // head", so it doesn't need hat clearance.
    drawItemSprite("prop-drink-bottle", cx + pet.facing * 10 - 8, y + PET_SIZE * 0.32, 16, 16);
    return;
  }
  if (pet.gagVariant === "yawn") {
    return;
  }
  const lineY = aboveHeadY(y - 16);
  ctx.strokeStyle = "#f4f4f4";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(cx - 10, lineY);
  ctx.lineTo(cx + 10, lineY);
  ctx.stroke();
}

function drawCosmetic(): void {
  switch (state.equippedCosmetic) {
    case "hat-leaf":
      if (drawItemSprite("hat-leaf", -20, -43, 40, 40)) {
        break;
      }
      ctx.fillStyle = "#1a1c2c";
      ctx.fillRect(-18, -28, 34, 5);
      ctx.fillStyle = "#38b764";
      ctx.fillRect(-14, -33, 26, 7);
      ctx.fillStyle = "#a7f070";
      ctx.fillRect(4, -37, 13, 5);
      break;
    case "hat-mushroom":
      if (drawItemSprite("hat-mushroom", -20, -43, 40, 40)) {
        break;
      }
      ctx.fillStyle = "#1a1c2c";
      ctx.fillRect(-18, -27, 36, 6);
      ctx.fillStyle = "#b13e53";
      ctx.fillRect(-16, -38, 32, 12);
      ctx.fillStyle = "#f4f4f4";
      ctx.fillRect(-10, -36, 5, 5);
      ctx.fillRect(2, -34, 5, 5);
      ctx.fillRect(-2, -40, 4, 4);
      break;
  }
}

function drawFood(now: number): void {
  for (const food of foods) {
    if (food.eaten) {
      continue;
    }
    const pulse = Math.sin(now / 140 + food.x) * 1.5;
    const sinceLanding = now - food.landedAt;
    const bouncing = sinceLanding >= 0 && sinceLanding < FOOD_BOUNCE_MS;
    const bounceProgress = bouncing ? sinceLanding / FOOD_BOUNCE_MS : 1;
    const bounce = bouncing ? -Math.sin(bounceProgress * Math.PI) * food.bounceHeight : 0;
    // Settle the sideways roll/nudge in ease-out fashion so it lands before the hop finishes.
    const drift = food.bounceDriftX * (1 - Math.pow(1 - bounceProgress, 3));
    const x = Math.round(food.x + drift);
    const y = Math.round(food.y + pulse + bounce);

    drawFoodSkin(x, y);
  }
}

function drawFoodSkin(x: number, y: number): void {
  if (state.equippedFoodSkin === "food-sushi") {
    if (drawItemSprite("food-sushi", x - 6, y - 7, 32, 32)) {
      return;
    }
    ctx.fillStyle = "#1a1c2c";
    ctx.fillRect(x + 1, y + 5, 16, 9);
    ctx.fillStyle = "#f4f4f4";
    ctx.fillRect(x + 2, y + 6, 14, 7);
    ctx.fillStyle = "#b13e53";
    ctx.fillRect(x + 7, y + 7, 5, 5);
    return;
  }
  if (state.equippedFoodSkin === "food-banh-mi") {
    if (drawItemSprite("food-banh-mi", x - 6, y - 7, 32, 32)) {
      return;
    }
    ctx.fillStyle = "#1a1c2c";
    ctx.fillRect(x + 1, y + 4, 16, 11);
    ctx.fillStyle = "#ffcd75";
    ctx.fillRect(x + 2, y + 5, 14, 9);
    ctx.fillStyle = "#38b764";
    ctx.fillRect(x + 5, y + 7, 9, 2);
    return;
  }
  if (drawItemSprite("food-default", x - 6, y - 7, 32, 32)) {
    return;
  }
  ctx.fillStyle = "#1a1c2c";
  ctx.fillRect(x + 3, y + 2, 12, 14);
  ctx.fillStyle = "#b13e53";
  ctx.fillRect(x + 4, y + 3, 10, 12);
  ctx.fillStyle = "#ef7d57";
  ctx.fillRect(x + 6, y + 4, 6, 10);
  ctx.fillStyle = "#a7f070";
  ctx.fillRect(x + 9, y, 5, 4);
}

function drawFurniture(): void {
  for (const item of state.furniture) {
    if (!item.visible) {
      continue;
    }
    const x = Math.round(furnitureX(item));
    const y = groundY() + PET_SIZE - 20;
    if (item.itemId === "furniture-bed") {
      if (drawItemSprite("furniture-bed", x, y - 6, 80, 40)) {
        continue;
      }
      ctx.fillStyle = "#1a1c2c";
      ctx.fillRect(x, y + 15, 78, 14);
      ctx.fillStyle = "#29366f";
      ctx.fillRect(x + 5, y + 8, 68, 16);
      ctx.fillStyle = "#73eff7";
      ctx.fillRect(x + 9, y + 4, 24, 10);
    } else if (item.itemId === "furniture-plant") {
      if (drawItemSprite("furniture-plant", x - 18, y - 7, 80, 40)) {
        continue;
      }
      ctx.fillStyle = "#1a1c2c";
      ctx.fillRect(x + 12, y + 14, 22, 18);
      ctx.fillStyle = "#ef7d57";
      ctx.fillRect(x + 15, y + 17, 16, 14);
      ctx.fillStyle = "#38b764";
      ctx.fillRect(x + 7, y + 4, 12, 12);
      ctx.fillStyle = "#a7f070";
      ctx.fillRect(x + 24, y, 14, 16);
    }
  }
}

function drawTooltip(): void {
  if (!hover) {
    return;
  }

  const lines = [
    `Fullness ${Math.round(state.fullness)}%`,
    `Mood ${state.mood}`,
    `Food ${Math.round(state.meterProgress * 100)}% (${state.pendingFood} queued)`,
  ];
  const width = 194;
  const height = 68;
  const x = clamp(pet.x + PET_SIZE / 2 - width / 2, 8, window.innerWidth - width - 8);
  const y = clamp(pet.y - height - 10, 8, window.innerHeight - height - 8);

  ctx.fillStyle = "rgba(26, 28, 44, 0.9)";
  ctx.fillRect(x, y, width, height);
  ctx.strokeStyle = "#f4f4f4";
  ctx.strokeRect(x + 0.5, y + 0.5, width - 1, height - 1);

  ctx.font = "12px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";
  ctx.fillStyle = "#f4f4f4";
  lines.forEach((line, index) => ctx.fillText(line, x + 10, y + 18 + index * 17));

  ctx.fillStyle = "#566c86";
  ctx.fillRect(x + 10, y + height - 12, width - 20, 4);
  ctx.fillStyle = "#a7f070";
  ctx.fillRect(x + 10, y + height - 12, Math.round((width - 20) * state.meterProgress), 4);
}

export function draw(now: number): void {
  ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
  drawFurniture();
  drawFood(now);
  drawPet(now);
  drawTooltip();
}
