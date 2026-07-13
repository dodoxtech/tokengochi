// Pet overlay gameplay loop (task 0006, extended by task 0012).
//
// The Rust side owns game truth: token events, pending Food, fullness, XP.
// This overlay is presentation: it queues visible food drops, walks the pet
// over, plays a short eat beat, and then calls `pet_ate`. Task 0012 adds a
// second layer on top: physics-driven drag/throw, click/petting reactions,
// idle gags, and walking/climbing on other windows' top edges.
//
// Rendering (per ADR-0003) is Aseprite sprite sheets, not hand-drawn canvas
// primitives - see `atlas.ts`.
//
// Module map:
//   dom.ts       - canvas/ctx/appWindow handles
//   types.ts     - shared domain types
//   constants.ts - tuning constants
//   atlas.ts     - sprite atlas loading + frame selection
//   state.ts     - shared mutable world state (pet, live server state, food
//                  queue, window geometry) and the geometry helpers over it
//   behavior.ts  - the pet's behavior AI: wander/eat/sleep plus the 0012
//                  physics and override modes
//   render.ts    - canvas drawing
//   input.ts     - pointer/hover handling and click-through toggling

import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { appWindow } from "./dom";
import { ACTIVE_TICK_MS, HOVER_POLL_MS, IDLE_TICK_MS, LANDING_PAUSE_MS } from "./constants";
import { updateFood, updatePet } from "./behavior";
import { draw } from "./render";
import { initInput, pollCursorForHover } from "./input";
import {
  applyOverlaySettings,
  ensurePendingFoodVisible,
  foods,
  groundY,
  pet,
  resizeCanvas,
  setState,
  setWindowSegments,
  spawnFood,
  state,
  windowOffsetX,
  windowOffsetY,
} from "./state";
import type { FoodSpawnedPayload, OverlaySettingsPayload, PetStatePayload, WindowSegmentPayload } from "./types";

let lastTick = performance.now();

function tick(now: number): void {
  const physicallyActive =
    pet.mode === "dragged" ||
    pet.mode === "tumble" ||
    pet.mode === "climb" ||
    pet.mode === "sulk" ||
    now - pet.landedAt < 300;
  const active =
    physicallyActive || pet.mode === "seek" || pet.mode === "eat" || foods.some((food) => !food.eaten);
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
initInput();

void listen<FoodSpawnedPayload>("food_spawned", (event) => {
  state.pendingFood = event.payload.pendingFood;
  spawnFood(event.payload.id);
});

void listen<PetStatePayload>("pet_state_changed", (event) => {
  setState(event.payload);
  ensurePendingFoodVisible();
});

void listen<OverlaySettingsPayload>("overlay_settings_changed", (event) => {
  applyOverlaySettings(event.payload.petSize, event.payload.calmMode);
});

void listen<WindowSegmentPayload[]>("window_segments_changed", (event) => {
  setWindowSegments(
    event.payload.map((segment) => ({
      id: String(segment.id),
      x0: segment.x0 - windowOffsetX,
      x1: segment.x1 - windowOffsetX,
      y: segment.y - windowOffsetY,
    })),
  );
});

resizeCanvas();
pet.y = groundY();
draw(performance.now());
requestAnimationFrame(tick);
void appWindow.setIgnoreCursorEvents(true);
setInterval(pollCursorForHover, HOVER_POLL_MS);

void invoke<PetStatePayload>("get_pet_state").then((initialState) => {
  setState(initialState);
  ensurePendingFoodVisible();
});
