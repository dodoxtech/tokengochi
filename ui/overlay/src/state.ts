// Shared mutable world state (pet, live pet-state snapshot, food queue,
// window geometry) plus the geometry helpers that read it. See main.ts for
// the module overview.

import { appWindow, canvas, ctx } from "./dom";
import { BASE_PET_SIZE, FOOD_SIZE, HIT_PADDING } from "./constants";
import type { ClimbPhase, Food, FurniturePlacement, PetMode, PetStatePayload, ReactVariant, GagVariant, Segment } from "./types";

export let PET_SIZE = BASE_PET_SIZE;

export const pet = {
  x: 110,
  y: 120,
  vx: 0,
  vy: 0,
  facing: 1,
  mode: "idle" as PetMode,
  overrideUntil: 0,
  eatStartedAt: 0,
  happyUntil: 0,
  supportId: "floor",
  climbTargetId: null as string | null,
  climbPhase: "approach" as ClimbPhase,
  approachX: 0,
  jumpPeakY: 0,
  landedAt: 0,
  reactVariant: "squash" as ReactVariant,
  gagVariant: "sneeze" as GagVariant,
};

export let state: PetStatePayload = {
  fullness: 100,
  mood: "Full",
  xp: 0,
  level: 0,
  furniture: [],
  pendingFood: 0,
  foodEarnedToday: 0,
  bankedTokensToday: 0,
  tokensPerFood: 20_000,
  meterProgress: 0,
};

export function setState(next: PetStatePayload): void {
  state = next;
}

export let calmMode = false;

export let windowSegments: Segment[] = [];
export function setWindowSegments(next: Segment[]): void {
  windowSegments = next;
}

// Screen-space offset of this window's top-left corner, in logical/CSS
// pixels, within the OS's global desktop coordinate space. Window geometry
// from the Rust side is reported in that global space (macOS "points"),
// which lines up 1:1 with CSS pixels on the same monitor once this offset is
// removed - see `docs/tasks/active/0012-pet-playful-interactions-window-climbing.md`
// "Overlay window bounds" risk.
export let windowOffsetX = 0;
export let windowOffsetY = 0;

export const foods: Food[] = [];
export let dpr = window.devicePixelRatio || 1;

// Whether the cursor is currently within the pet's hitbox (or a gesture is
// in progress) - drives click-through toggling and the petting/tooltip UI.
export let hover = false;
export function setHover(next: boolean): void {
  hover = next;
}

export let pointerDown = false;
export function setPointerDown(next: boolean): void {
  pointerDown = next;
}

export function resizeCanvas(): void {
  dpr = window.devicePixelRatio || 1;
  canvas.width = Math.round(window.innerWidth * dpr);
  canvas.height = Math.round(window.innerHeight * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  pet.x = clamp(pet.x, 0, petMaxX());
  if (pet.supportId === "floor") {
    pet.y = groundY();
  }
  refreshWindowOffset();
}

function refreshWindowOffset(): void {
  void appWindow.outerPosition().then((position) => {
    const factor = window.devicePixelRatio || 1;
    windowOffsetX = position.x / factor;
    windowOffsetY = position.y / factor;
  });
}

export function applyOverlaySettings(petSize: number, nextCalmMode: boolean): void {
  PET_SIZE = Math.round((BASE_PET_SIZE * clamp(petSize, 70, 160)) / 100);
  calmMode = nextCalmMode;
  pet.x = clamp(pet.x, 0, petMaxX());
  if (pet.supportId === "floor") {
    pet.y = groundY();
  }
  foods.forEach((food) => {
    food.targetY = groundY() + PET_SIZE - FOOD_SIZE - 8;
  });
}

export function groundY(): number {
  return Math.max(24, window.innerHeight - PET_SIZE - 18);
}

export function petMaxX(): number {
  return Math.max(0, window.innerWidth - PET_SIZE);
}

export function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export function currentSegments(): Segment[] {
  const floor: Segment = { id: "floor", x0: 0, x1: window.innerWidth, y: window.innerHeight - 18 };
  return [floor, ...windowSegments];
}

export function segmentById(id: string): Segment | undefined {
  return currentSegments().find((segment) => segment.id === id);
}

export function surfaceY(segment: Segment): number {
  return Math.max(24, segment.y - PET_SIZE);
}

/** The surface the pet should land on if falling through `x` right now:
 * the nearest segment strictly below the pet's current position, floor as
 * the ultimate fallback (always present, always spans the full width).
 *
 * Uses a strict `>` (not `>=`) so the ledge the pet is falling *from* is
 * never re-selected as the landing target: on the very first tumble frame
 * `pet.y` still equals that ledge's surfaceY, and an inclusive comparison
 * would immediately re-land the pet on the same ledge, flipping it back to
 * "idle" and re-triggering the drop next frame - an infinite tumble/idle
 * loop that never reaches the floor. */
export function landingSurfaceAt(x: number, aboveY: number): Segment {
  const candidates = currentSegments().filter(
    (segment) => x >= segment.x0 && x <= segment.x1 && surfaceY(segment) > aboveY + 0.5,
  );
  candidates.sort((a, b) => surfaceY(a) - surfaceY(b));
  return candidates[0] ?? segmentById("floor")!;
}

export function spawnFood(id: string): void {
  const targetY = groundY() + PET_SIZE - FOOD_SIZE - 8;
  const minX = 32;
  const maxX = Math.max(minX, window.innerWidth - FOOD_SIZE - 32);
  foods.push({
    id,
    x: clamp(minX + Math.random() * (maxX - minX), minX, maxX),
    y: -FOOD_SIZE,
    targetY,
    eaten: false,
  });
}

export function ensurePendingFoodVisible(): void {
  const visible = foods.filter((food) => !food.eaten).length;
  for (let i = visible; i < state.pendingFood; i += 1) {
    spawnFood(`restored-${Date.now()}-${i}`);
  }
}

export function isOverPet(clientX: number, clientY: number): boolean {
  return (
    clientX >= pet.x - HIT_PADDING &&
    clientX <= pet.x + PET_SIZE + HIT_PADDING &&
    clientY >= pet.y - HIT_PADDING &&
    clientY <= pet.y + PET_SIZE + HIT_PADDING
  );
}

export function furnitureX(item: FurniturePlacement): number {
  return clamp(item.x, 0.05, 0.95) * Math.max(1, window.innerWidth - 72);
}
