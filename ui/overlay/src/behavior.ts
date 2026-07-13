// Behavior AI: baseline wander/eat/sleep loop (0005/0006) plus the task 0012
// physics and override modes (drag/throw, click/petting reactions, idle
// gags, window-ledge climbing). See main.ts for the module overview.

import { invoke } from "@tauri-apps/api/core";
import {
  CLICK_COMBO_COUNT,
  CLICK_COMBO_WINDOW_MS,
  CLIMB_CHANCE,
  CLIMB_SPEED,
  DROP_SPEED,
  EAT_MS,
  GAG_VARIANTS,
  GRAVITY,
  JUMP_DOWN_SPEED,
  JUMP_UP_HEIGHT,
  JUMP_UP_SPEED,
  LANDING_PAUSE_MS,
  MAX_GAG_INTERVAL_MS,
  MAX_THROW_SPEED,
  MAX_CLIMB_INTERVAL_MS,
  MIN_CLIMB_INTERVAL_MS,
  MIN_GAG_INTERVAL_MS,
  PET_BUMP_COOLDOWN_MS,
  PET_STROKE_MS,
  REACT_VARIANTS,
  TERMINAL_FALL_SPEED,
  WALK_SPEED,
} from "./constants";
import {
  PET_SIZE,
  calmMode,
  clamp,
  foods,
  furnitureX,
  hover,
  groundY,
  landingSurfaceAt,
  pet,
  petMaxX,
  pointerDown,
  segmentById,
  setState,
  state,
  surfaceY,
  windowSegments,
  ensurePendingFoodVisible,
} from "./state";
import type { GagVariant, PetStatePayload, ReactVariant } from "./types";

let eatRequestInFlight = false;
let lastReactVariant: ReactVariant | null = null;
let clickTimestamps: number[] = [];
let comboSuppressUntil = 0;
let hoverStrokeStartedAt: number | null = null;
let lastPetBumpAt = 0;
let nextGagAt = randomGagDelay(performance.now());
let nextClimbRollAt = randomClimbDelay(performance.now());

function randomGagDelay(now: number): number {
  return now + MIN_GAG_INTERVAL_MS + Math.random() * (MAX_GAG_INTERVAL_MS - MIN_GAG_INTERVAL_MS);
}

function randomClimbDelay(now: number): number {
  return now + MIN_CLIMB_INTERVAL_MS + Math.random() * (MAX_CLIMB_INTERVAL_MS - MIN_CLIMB_INTERVAL_MS);
}

// Food waiting on the ground always takes priority over any in-progress
// "go somewhere else" action (climbing to a ledge, storming off after a
// click combo) - those should abort so the pet can redirect to the food
// instead of finishing the trip first.
function hasWaitingFood(): boolean {
  return foods.some((food) => !food.eaten && food.y >= food.targetY);
}

export function updateFood(dtMs: number): void {
  for (const food of foods) {
    if (food.y < food.targetY) {
      food.y = Math.min(food.targetY, food.y + (DROP_SPEED * dtMs) / 1000);
    }
  }
}

export function beginDrop(now: number, vx: number, vy: number): void {
  pet.vx = clamp(vx, -MAX_THROW_SPEED, MAX_THROW_SPEED);
  pet.vy = clamp(vy, -MAX_THROW_SPEED, MAX_THROW_SPEED);
  pet.supportId = "";
  pet.mode = "tumble";
  void now;
}

/** Deliberate jump-down from a window ledge, used whenever the pet decides
 * on its own to come down (to eat, sleep, or just leave the ledge) - as
 * opposed to `beginDrop`'s physics tumble, which is reserved for an actual
 * throw/drag release. A short anticipation hop ("jump-up") plays before the
 * fall ("jump-fall") so it reads as a jump rather than a slow climb or a
 * straight drop. */
function beginDescend(): void {
  pet.mode = "climb";
  pet.climbPhase = "jump-up";
  pet.jumpPeakY = Math.max(0, pet.y - JUMP_UP_HEIGHT);
}

function updateTumble(dtMs: number, now: number): void {
  const dt = dtMs / 1000;
  pet.vy = Math.min(TERMINAL_FALL_SPEED, pet.vy + GRAVITY * dt);
  const nextX = pet.x + pet.vx * dt;
  if (nextX < 0 || nextX > petMaxX()) {
    pet.vx *= -0.35;
  }
  pet.x = clamp(nextX, 0, petMaxX());
  const nextY = pet.y + pet.vy * dt;
  const centerX = pet.x + PET_SIZE / 2;
  const surface = landingSurfaceAt(centerX, pet.y);
  const landingY = surfaceY(surface);
  if (pet.vy >= 0 && nextY >= landingY) {
    pet.y = landingY;
    pet.vx = 0;
    pet.vy = 0;
    pet.supportId = surface.id;
    pet.landedAt = now;
    // Same brief recovery beat as any other landing (jump-down, climbing up
    // onto a ledge) - a fall shouldn't snap straight back to normal behavior.
    pet.overrideUntil = now + LANDING_PAUSE_MS;
    if (surface.id === "floor") {
      pet.mode = "landing";
    } else {
      // Landed on a window ledge (e.g. a throw that didn't reach the floor) -
      // hand off to the same climb/sit state machine a deliberate climb
      // uses, so it gets the same "does this ledge still exist" upkeep
      // instead of sitting there unmanaged.
      pet.mode = "climb";
      pet.climbPhase = "landed";
    }
    return;
  }
  pet.y = Math.max(0, nextY);
}

function maybeStartClimb(now: number): boolean {
  if (calmMode || windowSegments.length === 0) {
    return false;
  }
  if (now < nextClimbRollAt) {
    return false;
  }
  nextClimbRollAt = randomClimbDelay(now);
  if (Math.random() >= CLIMB_CHANCE) {
    return false;
  }
  const target = windowSegments[Math.floor(Math.random() * windowSegments.length)];
  const margin = PET_SIZE / 2 + 6;
  const low = target.x0 + margin;
  const high = target.x1 - margin;
  if (high <= low) {
    return false;
  }
  pet.climbTargetId = target.id;
  pet.approachX = low + Math.random() * (high - low);
  pet.climbPhase = "approach";
  pet.mode = "climb";
  return true;
}

function updateClimb(dtMs: number, now: number): void {
  if (pet.climbPhase === "jump-up") {
    const nextY = pet.y - (JUMP_UP_SPEED * dtMs) / 1000;
    if (nextY > pet.jumpPeakY) {
      pet.y = nextY;
      return;
    }
    pet.y = pet.jumpPeakY;
    pet.climbPhase = "jump-fall";
    return;
  }

  if (pet.climbPhase === "jump-fall") {
    const target = groundY();
    const dy = target - pet.y;
    if (Math.abs(dy) > 3) {
      // The fall should read as quick and deliberate, not a slow crawl -
      // JUMP_DOWN_SPEED is intentionally much faster than CLIMB_SPEED.
      pet.y += Math.sign(dy) * Math.min(Math.abs(dy), (JUMP_DOWN_SPEED * dtMs) / 1000);
      return;
    }
    pet.y = target;
    pet.supportId = "floor";
    // A brief "getting up" pause before the pet moves off again, mirroring
    // the landing squash already keyed off `landedAt` in render.ts.
    pet.mode = "landing";
    pet.overrideUntil = now + LANDING_PAUSE_MS;
    pet.landedAt = now;
    return;
  }

  const targetSegment = windowSegments.find((segment) => segment.id === pet.climbTargetId);

  if (pet.climbPhase === "landed") {
    if (now < pet.overrideUntil) {
      return;
    }
    pet.climbPhase = "sit";
    return;
  }

  if (pet.climbPhase === "sit") {
    const current = segmentById(pet.supportId);
    if (!current || current.id === "floor") {
      beginDescend();
      return;
    }
    pet.y = surfaceY(current);
    const centerX = pet.x + PET_SIZE / 2;
    if (centerX < current.x0 || centerX > current.x1) {
      beginDescend();
      return;
    }
    // Otherwise the pet just stays put on the ledge indefinitely - it only
    // has a reason to come down once there's food waiting to be fetched.
    if (hasWaitingFood()) {
      beginDescend();
    }
    return;
  }

  if (!targetSegment) {
    // The window went away before the pet got there - just settle where it
    // is instead of walking toward nothing.
    pet.mode = "idle";
    return;
  }

  if (pet.climbPhase === "approach") {
    if (hasWaitingFood()) {
      // Still on the floor, hasn't committed to the climb yet - abandon the
      // ledge trip and let the main loop redirect to the food next tick.
      pet.mode = "idle";
      pet.climbTargetId = null;
      return;
    }
    const dx = pet.approachX - pet.x;
    pet.facing = dx >= 0 ? 1 : -1;
    if (Math.abs(dx) > 4) {
      pet.x = clamp(pet.x + Math.sign(dx) * Math.min(Math.abs(dx), (WALK_SPEED * 0.8 * dtMs) / 1000), 0, petMaxX());
      return;
    }
    pet.climbPhase = "ascend";
    return;
  }

  // ascend
  const target = surfaceY(targetSegment);
  const dy = target - pet.y;
  if (Math.abs(dy) > 3) {
    pet.y += Math.sign(dy) * Math.min(Math.abs(dy), (CLIMB_SPEED * dtMs) / 1000);
    return;
  }
  pet.y = target;
  pet.supportId = targetSegment.id;
  // Same brief recovery beat as any other landing before the pet settles in.
  pet.climbPhase = "landed";
  pet.overrideUntil = now + LANDING_PAUSE_MS;
  pet.landedAt = now;
}

function maybeTriggerIdleGag(now: number): boolean {
  if (calmMode || now < nextGagAt) {
    return false;
  }
  nextGagAt = randomGagDelay(now);
  pet.gagVariant = GAG_VARIANTS[Math.floor(Math.random() * GAG_VARIANTS.length)] as GagVariant;
  pet.mode = "gag";
  pet.overrideUntil = now + 1800;
  return true;
}

export function triggerClickReaction(now: number): void {
  clickTimestamps = clickTimestamps.filter((t) => now - t < CLICK_COMBO_WINDOW_MS);
  clickTimestamps.push(now);

  if (clickTimestamps.length >= CLICK_COMBO_COUNT && now >= comboSuppressUntil) {
    const escalation = Math.random() < 0.5 ? "dizzy" : "sulk";
    pet.mode = escalation;
    pet.overrideUntil = now + (escalation === "dizzy" ? 2200 : 3000);
    comboSuppressUntil = pet.overrideUntil + 1500;
    clickTimestamps = [];
    return;
  }

  if (now < comboSuppressUntil) {
    return;
  }

  const options = REACT_VARIANTS.filter((variant) => variant !== lastReactVariant);
  const variant = options[Math.floor(Math.random() * options.length)] as ReactVariant;
  lastReactVariant = variant;
  pet.reactVariant = variant;
  pet.mode = "react";
  pet.overrideUntil = now + 650;
}

function updateSulk(dtMs: number): void {
  // Storms off away from the last click, roughly toward whichever side of
  // the screen has more room.
  const direction = pet.x > petMaxX() / 2 ? -1 : 1;
  pet.facing = direction;
  pet.x = clamp(pet.x + (direction * (WALK_SPEED * 0.5 * dtMs)) / 1000, 0, petMaxX());
}

function maybeTriggerPetting(now: number): void {
  if (!hover || pointerDown) {
    hoverStrokeStartedAt = null;
    return;
  }
  if (hoverStrokeStartedAt === null) {
    hoverStrokeStartedAt = now;
    return;
  }
  if (now - hoverStrokeStartedAt < PET_STROKE_MS) {
    return;
  }
  hoverStrokeStartedAt = null;
  pet.mode = "petted";
  pet.overrideUntil = now + 1200;
  if (now - lastPetBumpAt >= PET_BUMP_COOLDOWN_MS) {
    lastPetBumpAt = now;
    void invoke<PetStatePayload>("pet_petted")
      .then((nextState) => {
        setState(nextState);
      })
      .catch(() => {
        // Non-fatal: the pet still gets the visual/affection response even
        // if the rate-limited fullness bump couldn't be persisted this time.
      });
  }
}

export function updatePet(dtMs: number, now: number): void {
  if (pet.mode === "dragged") {
    return;
  }
  if (pet.mode === "tumble") {
    updateTumble(dtMs, now);
    return;
  }
  if (pet.mode === "climb") {
    updateClimb(dtMs, now);
    return;
  }
  if (pet.mode === "dizzy" && now < pet.overrideUntil) {
    return;
  }
  if (pet.mode === "sulk" && now < pet.overrideUntil) {
    if (hasWaitingFood()) {
      pet.mode = "idle";
      pet.overrideUntil = 0;
    } else {
      updateSulk(dtMs);
      return;
    }
  }
  if (pet.mode === "react" && now < pet.overrideUntil) {
    return;
  }
  if (pet.mode === "petted" && now < pet.overrideUntil) {
    return;
  }
  if (pet.mode === "gag" && now < pet.overrideUntil) {
    return;
  }
  if (pet.mode === "landing" && now < pet.overrideUntil) {
    return;
  }

  maybeTriggerPetting(now);

  const target = foods.find((food) => !food.eaten && food.y >= food.targetY);

  if (pet.supportId !== "floor") {
    // Perched on a window ledge - normally the pet just stays put there
    // (the ordinary "sit" climb phase handles that). Food only ever sits at
    // floor level though, so a pet perched up high has to come back down
    // once there's actually something to fetch - otherwise seek/eat only
    // ever compared x and let the pet "eat" from mid-air at ledge height.
    // Climb down deliberately (a jump) rather than physics-dropping: a real
    // fall only happens from an actual throw/drag release.
    if (target) {
      beginDescend();
    }
    return;
  }

  if (!target) {
    if (now < pet.happyUntil) {
      pet.mode = "happy";
      return;
    }
    const bed = state.furniture.find((item) => item.itemId === "furniture-bed");
    if (bed) {
      const bedX = furnitureX(bed) + 10;
      const dx = bedX - pet.x;
      pet.facing = dx >= 0 ? 1 : -1;
      if (Math.abs(dx) > 5) {
        pet.mode = "seek";
        pet.x += Math.sign(dx) * Math.min(Math.abs(dx), (WALK_SPEED * 0.62 * dtMs) / 1000);
        return;
      }
      pet.mode = "sleep";
      return;
    }
    if (maybeTriggerIdleGag(now)) {
      return;
    }
    if (maybeStartClimb(now)) {
      return;
    }
    pet.mode = "idle";
    return;
  }

  const targetPetX = clamp(target.x - PET_SIZE * 0.42, 0, petMaxX());
  const dx = targetPetX - pet.x;
  pet.facing = dx >= 0 ? 1 : -1;

  if (Math.abs(dx) > 4) {
    pet.mode = "seek";
    pet.x += Math.sign(dx) * Math.min(Math.abs(dx), (WALK_SPEED * dtMs) / 1000);
    return;
  }

  if (pet.mode !== "eat") {
    pet.mode = "eat";
    pet.eatStartedAt = now;
    return;
  }

  if (now - pet.eatStartedAt >= EAT_MS) {
    // A frame runs every ~33 ms, while the Tauri command is asynchronous.
    // Keep this Food claimed until the command settles so a slow IPC response
    // cannot consume several queued Food items from repeated frames.
    if (eatRequestInFlight) {
      return;
    }
    eatRequestInFlight = true;
    void invoke<PetStatePayload>("pet_ate")
      .then((nextState) => {
        target.eaten = true;
        setState(nextState);
        pet.happyUntil = performance.now() + 900;
        ensurePendingFoodVisible();
      })
      .catch(() => {
        // Leave the Food visible so a transient IPC failure can be retried
        // instead of losing a pending reward from the presentation queue.
        pet.mode = "idle";
        pet.eatStartedAt = 0;
      })
      .finally(() => {
        eatRequestInFlight = false;
      });
  }
}
