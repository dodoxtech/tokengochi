// Pet overlay gameplay loop (task 0006, extended by task 0012).
//
// The Rust side owns game truth: token events, pending Food, fullness, XP.
// This overlay is presentation: it queues visible food drops, walks the pet
// over, plays a short eat beat, and then calls `pet_ate`. Task 0012 adds a
// second layer on top: physics-driven drag/throw, click/petting reactions,
// idle gags, and walking/climbing on other windows' top edges.
//
// Rendering (per ADR-0003) is Aseprite sprite sheets, not hand-drawn canvas
// primitives: `ui/assets/sprites/hatchling` for the pet body, `.../effects`
// for hearts/exclaim/dust/zzz. The build copies both atlases next to
// `overlay.js` (see `ui/overlay/package.json`) and this file fetches them at
// startup - see `loadAtlas` below.

import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, cursorPosition } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

const canvas = document.getElementById("overlay-canvas") as HTMLCanvasElement;
const canvasContext = canvas.getContext("2d");
if (!canvasContext) {
  throw new Error("2D canvas context unavailable");
}
const ctx: CanvasRenderingContext2D = canvasContext;
ctx.imageSmoothingEnabled = false;

// --- Sprite atlas loading ---------------------------------------------------

interface AtlasFrame {
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

interface SpriteAtlas {
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
function frameForTag(atlas: SpriteAtlas | null, tagName: string, elapsedMs: number): AtlasFrame | null {
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

let hatchlingAtlas: SpriteAtlas | null = null;
let effectsAtlas: SpriteAtlas | null = null;
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
const MODE_ANIMATION_TAG: Record<PetMode, string> = {
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
};

const BASE_PET_SIZE = 72;
let PET_SIZE = BASE_PET_SIZE;
const FOOD_SIZE = 18;
const HIT_PADDING = 10;
const ACTIVE_TICK_MS = 1000 / 30;
const IDLE_TICK_MS = 1000 / 2;
const WALK_SPEED = 155;
const DROP_SPEED = 520;
const EAT_MS = 950;

// Task 0012 tuning. Kept in one place so gag/reaction frequency and physics
// feel can be tuned without hunting through the state machine.
const GRAVITY = 2100; // px/s^2
const MAX_THROW_SPEED = 1500; // px/s, hard cap so a throw can't leave the screen
const TERMINAL_FALL_SPEED = 2200; // px/s
const CLIMB_SPEED = 46; // px/s, deliberately slow per the design notes
const DRAG_PROMOTE_PX = 6; // movement past this turns a click into a drag
const HOVER_POLL_MS = 80; // OS-level cursor poll cadence while click-through is active
const CLICK_COMBO_WINDOW_MS = 2000;
const CLICK_COMBO_COUNT = 3;
const PET_STROKE_MS = 1000;
const PET_BUMP_COOLDOWN_MS = 60_000; // mirrors the server-side rate limit
const REACT_VARIANTS = ["squash", "spin", "look", "exclaim"] as const;
const GAG_VARIANTS = ["sneeze", "stare", "chase-tail"] as const;
const MIN_GAG_INTERVAL_MS = 5 * 60_000;
const MAX_GAG_INTERVAL_MS = 10 * 60_000;
const MIN_CLIMB_INTERVAL_MS = 90_000;
const MAX_CLIMB_INTERVAL_MS = 240_000;
const CLIMB_CHANCE = 0.35;

type Mood = "Full" | "Content" | "Peckish" | "Hungry" | "Starving";
type BaseMode = "idle" | "seek" | "eat" | "happy" | "sleep";
type PhysicsMode = "dragged" | "tumble" | "climb";
type OverrideMode = "react" | "dizzy" | "sulk" | "petted" | "gag";
type PetMode = BaseMode | PhysicsMode | OverrideMode;
type ReactVariant = (typeof REACT_VARIANTS)[number];
type GagVariant = (typeof GAG_VARIANTS)[number];
type ClimbPhase = "approach" | "ascend" | "sit";

interface FurniturePlacement {
  itemId: string;
  x: number;
}

interface PetStatePayload {
  fullness: number;
  mood: Mood;
  xp: number;
  level: number;
  equippedCosmetic?: string | null;
  equippedFoodSkin?: string | null;
  furniture: FurniturePlacement[];
  pendingFood: number;
  foodEarnedToday: number;
  bankedTokensToday: number;
  tokensPerFood: number;
  meterProgress: number;
}

interface FoodSpawnedPayload {
  id: string;
  pendingFood: number;
}

interface OverlaySettingsPayload {
  petSize: number;
  calmMode: boolean;
}

interface WindowSegmentPayload {
  id: number;
  x0: number;
  x1: number;
  y: number;
}

/** A horizontal ledge the pet can stand on: `"floor"` is the implicit
 * screen-bottom segment; everything else comes from `window_segments_changed`
 * and is already translated into this window's local canvas coordinates. */
interface Segment {
  id: string;
  x0: number;
  x1: number;
  /** Screen y of the surface line (where the pet's feet rest), not yet
   * offset by pet height - use `surfaceY()`. */
  y: number;
}

interface Food {
  id: string;
  x: number;
  y: number;
  targetY: number;
  eaten: boolean;
}

interface PointerSample {
  x: number;
  y: number;
  t: number;
}

const pet = {
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
  sitUntil: 0,
  landedAt: 0,
  reactVariant: "squash" as ReactVariant,
  gagVariant: "sneeze" as GagVariant,
};

let state: PetStatePayload = {
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

let calmMode = false;
let windowSegments: Segment[] = [];
// Screen-space offset of this window's top-left corner, in logical/CSS
// pixels, within the OS's global desktop coordinate space. Window geometry
// from the Rust side is reported in that global space (macOS "points"),
// which lines up 1:1 with CSS pixels on the same monitor once this offset is
// removed - see `docs/tasks/active/0012-pet-playful-interactions-window-climbing.md`
// "Overlay window bounds" risk.
let windowOffsetX = 0;
let windowOffsetY = 0;

const foods: Food[] = [];
let dpr = window.devicePixelRatio || 1;
let lastTick = performance.now();
let lastHit = false;
let hover = false;
let eatRequestInFlight = false;

// Tracks when the pet last changed `mode`, so its sprite animation always
// starts from frame 0 of the new pose instead of picking up mid-loop from
// whatever `now` happens to be.
let animMode: PetMode | null = null;
let animStartedAt = 0;

// Task 0012 interaction tracking.
let pointerDown = false;
let dragOffsetX = 0;
let dragOffsetY = 0;
let dragHistory: PointerSample[] = [];
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

function resizeCanvas(): void {
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

function applyOverlaySettings(settings: OverlaySettingsPayload): void {
  PET_SIZE = Math.round(BASE_PET_SIZE * clamp(settings.petSize, 70, 160) / 100);
  calmMode = settings.calmMode;
  pet.x = clamp(pet.x, 0, petMaxX());
  if (pet.supportId === "floor") {
    pet.y = groundY();
  }
  foods.forEach((food) => {
    food.targetY = groundY() + PET_SIZE - FOOD_SIZE - 8;
  });
}

function groundY(): number {
  return Math.max(24, window.innerHeight - PET_SIZE - 18);
}

function petMaxX(): number {
  return Math.max(0, window.innerWidth - PET_SIZE);
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function currentSegments(): Segment[] {
  const floor: Segment = { id: "floor", x0: 0, x1: window.innerWidth, y: window.innerHeight - 18 };
  return [floor, ...windowSegments];
}

function segmentById(id: string): Segment | undefined {
  return currentSegments().find((segment) => segment.id === id);
}

function surfaceY(segment: Segment): number {
  return Math.max(24, segment.y - PET_SIZE);
}

/** The surface the pet should land on if falling through `x` right now:
 * the nearest segment at or below the pet's current position, floor as the
 * ultimate fallback (always present, always spans the full width). */
function landingSurfaceAt(x: number, aboveY: number): Segment {
  const candidates = currentSegments().filter(
    (segment) => x >= segment.x0 && x <= segment.x1 && surfaceY(segment) >= aboveY - 1,
  );
  candidates.sort((a, b) => surfaceY(a) - surfaceY(b));
  return candidates[0] ?? segmentById("floor")!;
}

function spawnFood(id: string): void {
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

function ensurePendingFoodVisible(): void {
  const visible = foods.filter((food) => !food.eaten).length;
  for (let i = visible; i < state.pendingFood; i += 1) {
    spawnFood(`restored-${Date.now()}-${i}`);
  }
}

function isOverPet(clientX: number, clientY: number): boolean {
  return (
    clientX >= pet.x - HIT_PADDING &&
    clientX <= pet.x + PET_SIZE + HIT_PADDING &&
    clientY >= pet.y - HIT_PADDING &&
    clientY <= pet.y + PET_SIZE + HIT_PADDING
  );
}

function updateHitTest(clientX: number, clientY: number): void {
  // While a click/drag gesture is in progress, keep capturing cursor events
  // unconditionally instead of re-deriving hover from the pet's *current*
  // position. During a drag the pet is set to follow the cursor further
  // down in this same `mousemove` handler, so a hit test run beforehand
  // sees last frame's (stale) position; on a fast drag that stale box can
  // momentarily miss the cursor, flip `hover` to false, and re-enable
  // click-through mid-gesture - which drops every subsequent mousemove/
  // mouseup for this window and leaves `pointerDown`/`pet.mode` stuck
  // forever (the pet frozen mid-drag, and the fullscreen overlay left
  // capturing every click on the desktop, including other windows).
  hover = pointerDown || isOverPet(clientX, clientY);
  if (hover === lastHit) {
    return;
  }
  lastHit = hover;
  void appWindow.setIgnoreCursorEvents(!hover);
}

// While `setIgnoreCursorEvents(true)` is active (the idle/click-through
// state), the OS routes every mouse event - including `mousemove` - straight
// to whatever window is beneath the overlay, so this window never observes
// the cursor re-entering the pet's hitbox and `updateHitTest` above can never
// fire to turn click-through back off. `cursorPosition()` queries the OS
// cursor position directly (independent of window mouse-event routing), so
// polling it here breaks that chicken-and-egg deadlock. Once hover flips on
// and click-through is disabled, normal DOM mouse events take back over.
let cursorPollInFlight = false;

function pollCursorForHover(): void {
  if (hover || cursorPollInFlight) {
    return;
  }
  cursorPollInFlight = true;
  void cursorPosition()
    .then((position) => {
      const localX = position.x / dpr - windowOffsetX;
      const localY = position.y / dpr - windowOffsetY;
      updateHitTest(localX, localY);
    })
    .finally(() => {
      cursorPollInFlight = false;
    });
}

function updateFood(dtMs: number): void {
  for (const food of foods) {
    if (food.y < food.targetY) {
      food.y = Math.min(food.targetY, food.y + (DROP_SPEED * dtMs) / 1000);
    }
  }
}

// --- Task 0012 physics/override modes -------------------------------------

function beginDrop(now: number, vx: number, vy: number): void {
  pet.vx = clamp(vx, -MAX_THROW_SPEED, MAX_THROW_SPEED);
  pet.vy = clamp(vy, -MAX_THROW_SPEED, MAX_THROW_SPEED);
  pet.supportId = "";
  pet.mode = "tumble";
  void now;
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
    pet.mode = "idle";
    pet.landedAt = now;
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
  const targetSegment = windowSegments.find((segment) => segment.id === pet.climbTargetId);

  if (pet.climbPhase === "sit") {
    const current = segmentById(pet.supportId);
    if (!current || current.id === "floor") {
      beginDrop(now, (Math.random() - 0.5) * 40, 0);
      return;
    }
    pet.y = surfaceY(current);
    const centerX = pet.x + PET_SIZE / 2;
    if (centerX < current.x0 || centerX > current.x1) {
      beginDrop(now, (Math.random() - 0.5) * 60, 0);
      return;
    }
    if (now >= pet.sitUntil) {
      beginDrop(now, (Math.random() - 0.5) * 60, 0);
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
  pet.climbPhase = "sit";
  pet.sitUntil = now + 8000 + Math.random() * 10000;
}

function maybeTriggerIdleGag(now: number): boolean {
  if (calmMode || now < nextGagAt) {
    return false;
  }
  nextGagAt = randomGagDelay(now);
  pet.gagVariant = GAG_VARIANTS[Math.floor(Math.random() * GAG_VARIANTS.length)];
  pet.mode = "gag";
  pet.overrideUntil = now + 1800;
  return true;
}

function triggerClickReaction(now: number): void {
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
  const variant = options[Math.floor(Math.random() * options.length)];
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
  pet.x = clamp(pet.x + direction * (WALK_SPEED * 0.5 * dtMs) / 1000, 0, petMaxX());
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
        state = nextState;
      })
      .catch(() => {
        // Non-fatal: the pet still gets the visual/affection response even
        // if the rate-limited fullness bump couldn't be persisted this time.
      });
  }
}

// --- Baseline behavior (0005/0006): idle, wander toward food/bed ----------

function updatePet(dtMs: number, now: number): void {
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
    updateSulk(dtMs);
    return;
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

  maybeTriggerPetting(now);

  if (pet.supportId !== "floor") {
    // Food and the bed only ever sit at floor level, so a pet perched on a
    // window ledge has to fall back down before it can walk to either one -
    // otherwise seek/eat only ever compared x and let the pet "eat" from
    // mid-air at ledge height.
    beginDrop(now, 0, 0);
    return;
  }

  const target = foods.find((food) => !food.eaten && food.y >= food.targetY);
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
        state = nextState;
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

function drawPet(now: number): void {
  const bob = pet.mode === "sleep"
    ? 2
    : pet.mode === "seek" || pet.mode === "climb"
      ? Math.round(Math.sin(now / 95) * 3)
      : Math.round(Math.sin(now / 420) * 2);
  const happyHop = pet.mode === "happy" ? Math.abs(Math.sin(now / 115)) * 12 : 0;
  const landingSquash = now - pet.landedAt < 220 ? 1 - (now - pet.landedAt) / 220 : 0;

  let x = Math.round(pet.x);
  let y = Math.round(pet.y + bob - happyHop);
  let spin = 0;

  if (pet.mode === "tumble") {
    y = Math.round(pet.y);
    spin = (now / 90) % (Math.PI * 2);
  }

  if (pet.mode !== animMode) {
    animMode = pet.mode;
    animStartedAt = now;
  }
  const tag = MODE_ANIMATION_TAG[pet.mode];
  const frame = frameForTag(hatchlingAtlas, tag, now - animStartedAt);

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
      hatchlingAtlas!.image,
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

  drawCosmetic(now);

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

/** Effects drawn outside the pet's own rotate/scale transform, from the
 * effects atlas (`ui/assets/sprites/effects`): hearts, exclaim bubble, zzz,
 * landing/gag dust. `gagVariant === "stare"` has no matching effect frame
 * and stays a minimal procedural line - the pet's own idle pose already
 * carries that beat. */
function drawOverlayEffects(now: number, x: number, y: number): void {
  const inOverride = now < pet.overrideUntil;
  const cx = x + PET_SIZE / 2;

  if (pet.mode === "react" && inOverride && pet.reactVariant === "exclaim") {
    drawEffect("exclaim", cx, y - 14, now);
  }
  if (pet.mode === "petted" && inOverride) {
    drawEffect("heart", cx + 14, y - 10, now);
  }
  if (pet.mode === "sleep") {
    drawEffect("zzz", cx + PET_SIZE * 0.32, y - PET_SIZE * 0.12, now);
  }
  if (pet.mode === "gag" && inOverride) {
    drawGagEffect(cx, y - 16, now);
  }
  if (now - pet.landedAt < 320) {
    drawEffect("dust", cx, y + PET_SIZE - 6, now);
  }
}

function drawEffect(tagName: string, cx: number, cy: number, now: number): void {
  const frame = frameForTag(effectsAtlas, tagName, now);
  if (!effectsAtlas || !frame) {
    return;
  }
  const size = 22;
  ctx.drawImage(effectsAtlas.image, frame.x, frame.y, frame.w, frame.h, cx - size / 2, cy - size / 2, size, size);
}

function drawGagEffect(cx: number, cy: number, now: number): void {
  if (pet.gagVariant === "sneeze" || pet.gagVariant === "chase-tail") {
    drawEffect("dust", cx, cy, now);
    return;
  }
  ctx.strokeStyle = "#f4f4f4";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(cx - 10, cy);
  ctx.lineTo(cx + 10, cy);
  ctx.stroke();
}

function drawCosmetic(now: number): void {
  switch (state.equippedCosmetic) {
    case "hat-leaf":
      ctx.fillStyle = "#1a1c2c";
      ctx.fillRect(-18, -28, 34, 5);
      ctx.fillStyle = "#38b764";
      ctx.fillRect(-14, -33, 26, 7);
      ctx.fillStyle = "#a7f070";
      ctx.fillRect(4, -37, 13, 5);
      break;
    case "scarf-sunset":
      ctx.fillStyle = "#b13e53";
      ctx.fillRect(-22, 8, 43, 6);
      ctx.fillStyle = "#ef7d57";
      ctx.fillRect(12, 13, 8, 15);
      break;
    case "halo-heirloom":
      ctx.strokeStyle = "#ffcd75";
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.ellipse(0, -34 + Math.sin(now / 300) * 1.5, 20, 5, 0, 0, Math.PI * 2);
      ctx.stroke();
      break;
  }
}

function drawFood(now: number): void {
  for (const food of foods) {
    if (food.eaten) {
      continue;
    }
    const pulse = Math.sin(now / 140 + food.x) * 1.5;
    const x = Math.round(food.x);
    const y = Math.round(food.y + pulse);

    drawFoodSkin(x, y);
  }
}

function drawFoodSkin(x: number, y: number): void {
  if (state.equippedFoodSkin === "food-sushi") {
    ctx.fillStyle = "#1a1c2c";
    ctx.fillRect(x + 1, y + 5, 16, 9);
    ctx.fillStyle = "#f4f4f4";
    ctx.fillRect(x + 2, y + 6, 14, 7);
    ctx.fillStyle = "#b13e53";
    ctx.fillRect(x + 7, y + 7, 5, 5);
    return;
  }
  if (state.equippedFoodSkin === "food-banh-mi") {
    ctx.fillStyle = "#1a1c2c";
    ctx.fillRect(x + 1, y + 4, 16, 11);
    ctx.fillStyle = "#ffcd75";
    ctx.fillRect(x + 2, y + 5, 14, 9);
    ctx.fillStyle = "#38b764";
    ctx.fillRect(x + 5, y + 7, 9, 2);
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

function furnitureX(item: FurniturePlacement): number {
  return clamp(item.x, 0.05, 0.95) * Math.max(1, window.innerWidth - 72);
}

function drawFurniture(): void {
  for (const item of state.furniture) {
    const x = Math.round(furnitureX(item));
    const y = groundY() + PET_SIZE - 20;
    if (item.itemId === "furniture-bed") {
      ctx.fillStyle = "#1a1c2c";
      ctx.fillRect(x, y + 15, 78, 14);
      ctx.fillStyle = "#29366f";
      ctx.fillRect(x + 5, y + 8, 68, 16);
      ctx.fillStyle = "#73eff7";
      ctx.fillRect(x + 9, y + 4, 24, 10);
    } else if (item.itemId === "furniture-plant") {
      ctx.fillStyle = "#1a1c2c";
      ctx.fillRect(x + 12, y + 14, 22, 18);
      ctx.fillStyle = "#ef7d57";
      ctx.fillRect(x + 15, y + 17, 16, 14);
      ctx.fillStyle = "#38b764";
      ctx.fillRect(x + 7, y + 4, 12, 12);
      ctx.fillStyle = "#a7f070";
      ctx.fillRect(x + 24, y, 14, 16);
    } else if (item.itemId === "furniture-perch") {
      ctx.fillStyle = "#1a1c2c";
      ctx.fillRect(x, y + 2, 70, 8);
      ctx.fillStyle = "#566c86";
      ctx.fillRect(x + 4, y + 1, 62, 5);
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

function draw(now: number): void {
  ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
  drawFurniture();
  drawFood(now);
  drawPet(now);
  drawTooltip();
}

function tick(now: number): void {
  const physicallyActive = pet.mode === "dragged" || pet.mode === "tumble" || pet.mode === "climb" ||
    pet.mode === "sulk" || now - pet.landedAt < 300;
  const active = physicallyActive || pet.mode === "seek" || pet.mode === "eat" ||
    foods.some((food) => !food.eaten);
  const tickInterval = active ? ACTIVE_TICK_MS : IDLE_TICK_MS;

  if (now - lastTick >= tickInterval) {
    const dtMs = now - lastTick;
    lastTick = now;
    updateFood(dtMs);
    updatePet(dtMs, now);
    draw(now);
  }

  requestAnimationFrame(tick);
}

window.addEventListener("resize", resizeCanvas);

window.addEventListener("mousemove", (e) => {
  updateHitTest(e.clientX, e.clientY);

  if (pointerDown) {
    const now = performance.now();
    dragHistory.push({ x: e.clientX, y: e.clientY, t: now });
    if (dragHistory.length > 8) {
      dragHistory.shift();
    }

    if (pet.mode !== "dragged") {
      const start = dragHistory[0];
      const distance = Math.hypot(e.clientX - start.x, e.clientY - start.y);
      if (distance > DRAG_PROMOTE_PX) {
        pet.mode = "dragged";
        pet.supportId = "";
        dragOffsetX = start.x - pet.x;
        dragOffsetY = start.y - pet.y;
      }
    } else {
      pet.x = clamp(e.clientX - dragOffsetX, 0, petMaxX());
      pet.y = clamp(e.clientY - dragOffsetY, -window.innerHeight, window.innerHeight - PET_SIZE);
    }
  }
});

window.addEventListener("mousedown", (e) => {
  if (!isOverPet(e.clientX, e.clientY)) {
    return;
  }
  pointerDown = true;
  dragOffsetX = e.clientX;
  dragOffsetY = e.clientY;
  dragHistory = [{ x: e.clientX, y: e.clientY, t: performance.now() }];
});

window.addEventListener("mouseup", (e) => {
  if (!pointerDown) {
    return;
  }
  pointerDown = false;
  const now = performance.now();

  if (pet.mode === "dragged") {
    const recent = dragHistory.filter((sample) => now - sample.t <= 150);
    const first = recent[0] ?? dragHistory[0];
    const last = dragHistory[dragHistory.length - 1];
    const dt = Math.max(16, last.t - first.t) / 1000;
    const vx = (last.x - first.x) / dt;
    const vy = (last.y - first.y) / dt;
    beginDrop(now, vx, vy);
    dragHistory = [];
    return;
  }

  dragHistory = [];
  if (isOverPet(e.clientX, e.clientY)) {
    triggerClickReaction(now);
  }
});

// Safety net: if the window loses focus mid-gesture (app switch, or any
// missed mouseup) `pointerDown` would otherwise stay true forever, freezing
// the pet in "dragged" mode and leaving the fullscreen overlay stuck
// capturing cursor events over the whole desktop. Release the gesture and
// let the pet fall like a normal throw instead.
window.addEventListener("blur", () => {
  if (!pointerDown) {
    return;
  }
  pointerDown = false;
  dragHistory = [];
  if (pet.mode === "dragged") {
    beginDrop(performance.now(), 0, 0);
  }
});

void listen<FoodSpawnedPayload>("food_spawned", (event) => {
  state.pendingFood = event.payload.pendingFood;
  spawnFood(event.payload.id);
});

void listen<PetStatePayload>("pet_state_changed", (event) => {
  state = event.payload;
  ensurePendingFoodVisible();
});

void listen<OverlaySettingsPayload>("overlay_settings_changed", (event) => {
  applyOverlaySettings(event.payload);
});

void listen<WindowSegmentPayload[]>("window_segments_changed", (event) => {
  windowSegments = event.payload.map((segment) => ({
    id: String(segment.id),
    x0: segment.x0 - windowOffsetX,
    x1: segment.x1 - windowOffsetX,
    y: segment.y - windowOffsetY,
  }));
});

resizeCanvas();
pet.y = groundY();
draw(performance.now());
requestAnimationFrame(tick);
void appWindow.setIgnoreCursorEvents(true);
setInterval(pollCursorForHover, HOVER_POLL_MS);

void invoke<PetStatePayload>("get_pet_state").then((initialState) => {
  state = initialState;
  ensurePendingFoodVisible();
});
