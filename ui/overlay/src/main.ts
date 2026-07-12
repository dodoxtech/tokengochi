// Pet overlay window spike (task 0002).
//
// Proves out: transparent/borderless/always-on-top window, click-through
// everywhere except over a test square, hit-testing driven by mouse move,
// window dragging via the square, and a 30fps/2fps active/idle tick to keep
// idle CPU low. Real sprites and behavior AI are out of scope here (task
// 0005); this is deliberately a bouncing square standing in for the pet.
//
// See docs/knowledge/overlay-platform-notes.md for per-OS findings and the
// Wayland fallback decision.

import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

const canvas = document.getElementById("overlay-canvas") as HTMLCanvasElement;
const ctx = canvas.getContext("2d");
if (!ctx) {
  throw new Error("2D canvas context unavailable");
}

const SQUARE_SIZE = 64;
// Extra margin so the square is easy to grab/hover near its edges.
const HIT_PADDING = 6;
// How long (ms) the square bounces before resting, and how long it rests
// before bouncing again - lets both tick rates be observed/measured.
const ACTIVE_DURATION_MS = 6000;
const IDLE_DURATION_MS = 6000;
const ACTIVE_TICK_MS = 1000 / 30; // ~30fps while moving
const IDLE_TICK_MS = 1000 / 2; // ~2fps while idle (still redraw, e.g. blink/breathe later)

type Mode = "active" | "idle";

interface SquareState {
  x: number;
  y: number;
  vx: number;
  vy: number;
}

const square: SquareState = {
  x: 100,
  y: 100,
  vx: 180, // px/sec
  vy: 140,
};

let mode: Mode = "active";
let modeChangedAt = performance.now();
let lastTick = performance.now();
let lastHit = false;
let dpr = window.devicePixelRatio || 1;

function resizeCanvas(): void {
  dpr = window.devicePixelRatio || 1;
  canvas.width = Math.round(window.innerWidth * dpr);
  canvas.height = Math.round(window.innerHeight * dpr);
  ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
  // Keep the square inside the (possibly resized) window bounds.
  square.x = Math.min(square.x, Math.max(0, window.innerWidth - SQUARE_SIZE));
  square.y = Math.min(square.y, Math.max(0, window.innerHeight - SQUARE_SIZE));
}

function stepPhysics(dtMs: number): void {
  const dt = dtMs / 1000;
  square.x += square.vx * dt;
  square.y += square.vy * dt;

  const maxX = window.innerWidth - SQUARE_SIZE;
  const maxY = window.innerHeight - SQUARE_SIZE;

  if (square.x <= 0) {
    square.x = 0;
    square.vx = Math.abs(square.vx);
  } else if (square.x >= maxX) {
    square.x = maxX;
    square.vx = -Math.abs(square.vx);
  }

  if (square.y <= 0) {
    square.y = 0;
    square.vy = Math.abs(square.vy);
  } else if (square.y >= maxY) {
    square.y = maxY;
    square.vy = -Math.abs(square.vy);
  }
}

function draw(): void {
  ctx!.clearRect(0, 0, window.innerWidth, window.innerHeight);
  ctx!.fillStyle = mode === "active" ? "#ff5f5f" : "#8a8fff";
  ctx!.fillRect(square.x, square.y, SQUARE_SIZE, SQUARE_SIZE);
  ctx!.strokeStyle = "rgba(0, 0, 0, 0.35)";
  ctx!.lineWidth = 2;
  ctx!.strokeRect(square.x, square.y, SQUARE_SIZE, SQUARE_SIZE);
}

function isOverSquare(clientX: number, clientY: number): boolean {
  return (
    clientX >= square.x - HIT_PADDING &&
    clientX <= square.x + SQUARE_SIZE + HIT_PADDING &&
    clientY >= square.y - HIT_PADDING &&
    clientY <= square.y + SQUARE_SIZE + HIT_PADDING
  );
}

// Only call across the IPC boundary when the hit state actually flips -
// this runs on every mousemove, and set_ignore_cursor_events is not free.
function updateHitTest(clientX: number, clientY: number): void {
  const hit = isOverSquare(clientX, clientY);
  if (hit === lastHit) {
    return;
  }
  lastHit = hit;
  // hit === true  -> capture mouse (over the square, draggable/clickable)
  // hit === false -> click-through (everywhere else on the desktop)
  void appWindow.setIgnoreCursorEvents(!hit);
}

function tick(now: number): void {
  const tickInterval = mode === "active" ? ACTIVE_TICK_MS : IDLE_TICK_MS;

  if (now - modeChangedAt > (mode === "active" ? ACTIVE_DURATION_MS : IDLE_DURATION_MS)) {
    mode = mode === "active" ? "idle" : "active";
    modeChangedAt = now;
  }

  if (now - lastTick >= tickInterval) {
    const dtMs = now - lastTick;
    lastTick = now;
    if (mode === "active") {
      stepPhysics(dtMs);
    }
    draw();
  }

  requestAnimationFrame(tick);
}

window.addEventListener("resize", resizeCanvas);

window.addEventListener("mousemove", (e) => {
  updateHitTest(e.clientX, e.clientY);
});

window.addEventListener("mousedown", (e) => {
  if (isOverSquare(e.clientX, e.clientY)) {
    void appWindow.startDragging();
  }
});

resizeCanvas();
draw();
requestAnimationFrame(tick);
