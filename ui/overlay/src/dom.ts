// Canvas and Tauri window handles shared across the overlay modules.

import { getCurrentWindow } from "@tauri-apps/api/window";

export const appWindow = getCurrentWindow();

export const canvas = document.getElementById("overlay-canvas") as HTMLCanvasElement;

const canvasContext = canvas.getContext("2d");
if (!canvasContext) {
  throw new Error("2D canvas context unavailable");
}

export const ctx: CanvasRenderingContext2D = canvasContext;
ctx.imageSmoothingEnabled = false;
