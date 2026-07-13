// Pointer/hover input: click-through hit testing, drag-to-throw gesture
// tracking, and click reactions. See main.ts for the module overview.

import { cursorPosition } from "@tauri-apps/api/window";
import { appWindow } from "./dom";
import { DRAG_PROMOTE_PX } from "./constants";
import { beginDrop, triggerClickReaction } from "./behavior";
import {
  PET_SIZE,
  clamp,
  dpr,
  hover,
  isOverPet,
  pet,
  petMaxX,
  pointerDown,
  setHover,
  setPointerDown,
  windowOffsetX,
  windowOffsetY,
} from "./state";
import type { PointerSample } from "./types";

let lastHit = false;
let dragOffsetX = 0;
let dragOffsetY = 0;
let dragHistory: PointerSample[] = [];

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
  const nextHover = pointerDown || isOverPet(clientX, clientY);
  setHover(nextHover);
  if (nextHover === lastHit) {
    return;
  }
  lastHit = nextHover;
  void appWindow.setIgnoreCursorEvents(!nextHover);
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

export function pollCursorForHover(): void {
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

export function initInput(): void {
  window.addEventListener("mousemove", (e) => {
    updateHitTest(e.clientX, e.clientY);

    if (!pointerDown) {
      return;
    }

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
  });

  window.addEventListener("mousedown", (e) => {
    if (!isOverPet(e.clientX, e.clientY)) {
      return;
    }
    setPointerDown(true);
    dragOffsetX = e.clientX;
    dragOffsetY = e.clientY;
    dragHistory = [{ x: e.clientX, y: e.clientY, t: performance.now() }];
  });

  window.addEventListener("mouseup", (e) => {
    if (!pointerDown) {
      return;
    }
    setPointerDown(false);
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
    setPointerDown(false);
    dragHistory = [];
    if (pet.mode === "dragged") {
      beginDrop(performance.now(), 0, 0);
    }
  });
}
