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
import {
  ACTIVE_TICK_MS,
  AGENT_STATUS_COMPLETED_BADGE_MS,
  AGENT_STATUS_NEEDS_APPROVAL_TIMEOUT_MS,
  HOVER_POLL_MS,
  IDLE_TICK_MS,
  LANDING_PAUSE_MS,
  MAX_FRAME_DT_MS,
} from "./constants";
import { updateAgentStatusBadge, updateFood, updatePet } from "./behavior";
import { draw } from "./render";
import { initInput, pollCursorForHover } from "./input";
import {
  agentStatusBadge,
  applyOverlaySettings,
  clearAgentStatusBadge,
  ensurePendingFoodVisible,
  foods,
  groundY,
  pet,
  pruneEatenFood,
  resizeCanvas,
  setAgentStatusBadge,
  setState,
  setWindowSegments,
  spawnFood,
  state,
  windowOffsetX,
  windowOffsetY,
} from "./state";
import type {
  AgentStatusPayload,
  FoodSpawnedPayload,
  OverlaySettingsPayload,
  PetStatePayload,
  WindowSegmentPayload,
} from "./types";

let lastTick = performance.now();

function tick(now: number): void {
  const physicallyActive =
    pet.mode === "dragged" ||
    pet.mode === "tumble" ||
    pet.mode === "climb" ||
    pet.mode === "sulk" ||
    now - pet.landedAt < LANDING_PAUSE_MS;
  const active =
    physicallyActive ||
    pet.mode === "seek" ||
    pet.mode === "eat" ||
    foods.some((food) => !food.eaten) ||
    agentStatusBadge.status === "needs_approval"; // keep the attention bob smooth
  const tickInterval = active ? ACTIVE_TICK_MS : IDLE_TICK_MS;

  if (now - lastTick >= tickInterval) {
    // Cap the frame delta: a stall in the render loop (tab backgrounded,
    // OS/IPC hiccup right around a drag-release) would otherwise hand a
    // single tick a huge `dtMs`, and every per-tick movement formula here
    // (walk-speed steps, tumble physics) scales with it - a large enough
    // dtMs lets the pet cover its *entire* remaining distance to a target
    // (e.g. the bed) in one frame, reading as an instant teleport right on
    // landing instead of a walk, even though the landing-pause gate itself
    // (wall-clock `now < overrideUntil`) was technically still honored.
    const dtMs = Math.min(now - lastTick, MAX_FRAME_DT_MS);
    lastTick = now;
    updateFood(dtMs, now);
    updatePet(dtMs, now);
    updateAgentStatusBadge(now);
    pruneEatenFood();
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

// Task 0017 (extended 2026-07-14): a "completed" event always wins
// immediately (short celebration, also clears any stale "needs_approval" -
// the turn presumably resolved it). A "needs_approval" event doesn't clobber
// an existing badge of the same status; it just refreshes the safety-net
// timeout. A "resolved" event (PostToolUse/PermissionDenied - the prompt got
// approved+ran, or denied) silently clears a pending "needs_approval" badge
// without waiting for the whole turn to end via "completed"; it never sets a
// badge of its own, so it can't interrupt an in-progress "completed"
// celebration or fire spuriously when nothing was pending.
void listen<AgentStatusPayload>("agent_status_changed", (event) => {
  const now = performance.now();
  if (event.payload.status === "completed") {
    setAgentStatusBadge("completed", now + AGENT_STATUS_COMPLETED_BADGE_MS);
  } else if (event.payload.status === "needs_approval") {
    setAgentStatusBadge("needs_approval", now + AGENT_STATUS_NEEDS_APPROVAL_TIMEOUT_MS);
  } else if (agentStatusBadge.status === "needs_approval") {
    clearAgentStatusBadge();
  }
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
