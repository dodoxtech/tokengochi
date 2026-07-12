// Pet overlay gameplay loop (task 0006).
//
// The Rust side owns game truth: token events, pending Food, fullness, XP.
// This overlay is presentation: it queues visible food drops, walks the pet
// over, plays a short eat beat, and then calls `pet_ate`.

import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

const canvas = document.getElementById("overlay-canvas") as HTMLCanvasElement;
const canvasContext = canvas.getContext("2d");
if (!canvasContext) {
  throw new Error("2D canvas context unavailable");
}
const ctx: CanvasRenderingContext2D = canvasContext;

const PET_SIZE = 72;
const FOOD_SIZE = 18;
const HIT_PADDING = 10;
const ACTIVE_TICK_MS = 1000 / 30;
const IDLE_TICK_MS = 1000 / 2;
const WALK_SPEED = 155;
const DROP_SPEED = 520;
const EAT_MS = 950;

type Mood = "Full" | "Content" | "Peckish" | "Hungry" | "Starving";
type PetMode = "idle" | "seek" | "eat" | "happy";

interface PetStatePayload {
  fullness: number;
  mood: Mood;
  xp: number;
  level: number;
  pendingFood: number;
  pantry: number;
  foodEarnedToday: number;
  bankedTokensToday: number;
  tokensPerFood: number;
  meterProgress: number;
}

interface FoodSpawnedPayload {
  id: string;
  pendingFood: number;
}

interface Food {
  id: string;
  x: number;
  y: number;
  targetY: number;
  eaten: boolean;
}

const pet = {
  x: 110,
  y: 120,
  facing: 1,
  mode: "idle" as PetMode,
  eatStartedAt: 0,
  happyUntil: 0,
};

let state: PetStatePayload = {
  fullness: 100,
  mood: "Full",
  xp: 0,
  level: 0,
  pendingFood: 0,
  pantry: 0,
  foodEarnedToday: 0,
  bankedTokensToday: 0,
  tokensPerFood: 20_000,
  meterProgress: 0,
};

const foods: Food[] = [];
let dpr = window.devicePixelRatio || 1;
let lastTick = performance.now();
let lastHit = false;
let hover = false;
let eatRequestInFlight = false;

function resizeCanvas(): void {
  dpr = window.devicePixelRatio || 1;
  canvas.width = Math.round(window.innerWidth * dpr);
  canvas.height = Math.round(window.innerHeight * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  pet.x = clamp(pet.x, 0, petMaxX());
  pet.y = groundY();
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

function spawnFood(id: string): void {
  const targetY = groundY() + PET_SIZE - FOOD_SIZE - 8;
  const minX = 32;
  const maxX = Math.max(minX, window.innerWidth - FOOD_SIZE - 32);
  const existingOffset = foods.length * 28;
  foods.push({
    id,
    x: clamp(minX + ((performance.now() / 7 + existingOffset) % (maxX - minX + 1)), minX, maxX),
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
  hover = isOverPet(clientX, clientY);
  if (hover === lastHit) {
    return;
  }
  lastHit = hover;
  void appWindow.setIgnoreCursorEvents(!hover);
}

function updateFood(dtMs: number): void {
  for (const food of foods) {
    if (food.y < food.targetY) {
      food.y = Math.min(food.targetY, food.y + (DROP_SPEED * dtMs) / 1000);
    }
  }
}

function updatePet(dtMs: number, now: number): void {
  const target = foods.find((food) => !food.eaten && food.y >= food.targetY);
  if (!target) {
    pet.mode = now < pet.happyUntil ? "happy" : "idle";
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
  const bob = pet.mode === "seek" ? Math.round(Math.sin(now / 95) * 3) : Math.round(Math.sin(now / 420) * 2);
  const squash = pet.mode === "eat" ? Math.sin((now - pet.eatStartedAt) / 80) * 3 : 0;
  const happyHop = pet.mode === "happy" ? Math.abs(Math.sin(now / 115)) * 12 : 0;
  const x = Math.round(pet.x);
  const y = Math.round(pet.y + bob - happyHop);

  ctx.save();
  ctx.translate(x + PET_SIZE / 2, y + PET_SIZE / 2);
  ctx.scale(pet.facing, 1);

  ctx.fillStyle = "rgba(26, 28, 44, 0.2)";
  ctx.beginPath();
  ctx.ellipse(0, PET_SIZE / 2 - 5, 24, 7, 0, 0, Math.PI * 2);
  ctx.fill();

  ctx.fillStyle = "#1a1c2c";
  ctx.beginPath();
  ctx.ellipse(0, 2, 30 + squash, 27 - squash, 0, 0, Math.PI * 2);
  ctx.fill();

  ctx.fillStyle = moodColor(state.mood);
  ctx.beginPath();
  ctx.ellipse(0, 0, 26 + squash, 24 - squash, 0, 0, Math.PI * 2);
  ctx.fill();

  ctx.fillStyle = "#f4f4f4";
  ctx.beginPath();
  ctx.ellipse(4, 10, 13, 9, 0, 0, Math.PI * 2);
  ctx.fill();

  ctx.fillStyle = "#1a1c2c";
  if (state.mood === "Starving") {
    ctx.fillRect(-13, -7, 7, 2);
    ctx.fillRect(8, -7, 7, 2);
  } else {
    ctx.fillRect(-12, -8, 5, 5);
    ctx.fillRect(9, -8, 5, 5);
  }

  ctx.fillStyle = "#ef7d57";
  ctx.fillRect(-8, 0, 6, 3);
  ctx.fillRect(14, 0, 6, 3);

  const mouthOpen = pet.mode === "eat" ? 5 + Math.abs(Math.sin((now - pet.eatStartedAt) / 90)) * 7 : 3;
  ctx.fillStyle = "#1a1c2c";
  ctx.fillRect(-21, -1, Math.round(mouthOpen), 4);

  ctx.fillStyle = "#38b764";
  ctx.fillRect(-3, -31, 6, 9);
  ctx.fillStyle = "#a7f070";
  ctx.fillRect(0, -34, 12, 5);

  ctx.fillStyle = "#ef7d57";
  ctx.fillRect(-14, 24, 13, 5);
  ctx.fillRect(8, 24, 13, 5);

  ctx.restore();
}

function moodColor(mood: Mood): string {
  switch (mood) {
    case "Full":
      return "#ffcd75";
    case "Content":
      return "#a7f070";
    case "Peckish":
      return "#73eff7";
    case "Hungry":
      return "#ef7d57";
    case "Starving":
      return "#94b0c2";
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

    ctx.fillStyle = "#1a1c2c";
    ctx.fillRect(x + 3, y + 2, 12, 14);
    ctx.fillStyle = "#b13e53";
    ctx.fillRect(x + 4, y + 3, 10, 12);
    ctx.fillStyle = "#ef7d57";
    ctx.fillRect(x + 6, y + 4, 6, 10);
    ctx.fillStyle = "#a7f070";
    ctx.fillRect(x + 9, y, 5, 4);
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
  drawFood(now);
  drawPet(now);
  drawTooltip();
}

function tick(now: number): void {
  const active = pet.mode === "seek" || pet.mode === "eat" || foods.some((food) => !food.eaten);
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
});

window.addEventListener("mousedown", (e) => {
  if (isOverPet(e.clientX, e.clientY)) {
    void appWindow.startDragging();
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

resizeCanvas();
pet.y = groundY();
draw(performance.now());
requestAnimationFrame(tick);
void appWindow.setIgnoreCursorEvents(true);

void invoke<PetStatePayload>("get_pet_state").then((initialState) => {
  state = initialState;
  ensurePendingFoodVisible();
});
