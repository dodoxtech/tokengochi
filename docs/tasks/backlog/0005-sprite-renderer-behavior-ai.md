---
type: task
status: backlog
priority: P1
delivery_order: 0005
estimate: 4d
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
sprint: null
tags:
  - task
  - backlog
---

# Task: Pixel sprite renderer + pet behavior AI

## Context

Rendering approach in [[../../decisions/0003-canvas-sprite-rendering|ADR-0003]]. Builds on the proven overlay from [[0002-pet-overlay-window-spike|0002]].

## Goal

A living pet: pixel-art sprite that wanders the screen bottom, idles, sleeps, reacts to click/drag, with mood-driven animation sets.

## Scope

In scope: Canvas 2D sprite-sheet renderer (Aseprite JSON format, pixelated scaling); behavior state machine (idle/wander/sleep/dragged/react) with 30fps-active/2fps-idle ticking; placeholder Hatchling sprite set (idle, walk, sleep, eat, happy); hit-testing → click-through toggling.

Out of scope: food/eating (0006), evolution forms (0009), final art.

## Acceptance Criteria

- [ ] Pet wanders believably (random walk with pauses), sleeps after inactivity, reacts to click and drag.
- [ ] Mood from `PetStateChanged` events switches animation sets.
- [ ] CPU <1% idle, <3% while walking, on all 3 OS.

## Dependencies

- [[0002-pet-overlay-window-spike|0002]]

## Risks

- Placeholder art quality; commission/produce final sprites in parallel (follow-up task).

## Verification Plan

- [ ] Manual behavior checklist per OS + CPU profiling; record results below.

## Verification Results

TBD
