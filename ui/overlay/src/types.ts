// Shared domain types for the overlay. See main.ts for the module overview.

import type { REACT_VARIANTS, GAG_VARIANTS } from "./constants";

export type Mood = "Full" | "Content" | "Peckish" | "Hungry" | "Starving";
export type BaseMode = "idle" | "seek" | "eat" | "happy" | "sleep";
export type PhysicsMode = "dragged" | "tumble" | "climb";
export type OverrideMode = "react" | "dizzy" | "sulk" | "petted" | "gag" | "landing";
export type PetMode = BaseMode | PhysicsMode | OverrideMode;
export type ReactVariant = (typeof REACT_VARIANTS)[number];
export type GagVariant = (typeof GAG_VARIANTS)[number];
export type ClimbPhase = "approach" | "ascend" | "landed" | "sit" | "jump-up" | "jump-fall";

export interface FurniturePlacement {
  itemId: string;
  x: number;
  visible: boolean;
}

export interface PetStatePayload {
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

export interface FoodSpawnedPayload {
  id: string;
  pendingFood: number;
}

/** Task 0017: agent turn-completed / needs-approval events, forwarded from a
 * Claude Code hook via the Rust `agent_status_changed` event. `resolved`
 * (added 2026-07-14) fires from `PostToolUse`/`PermissionDenied` - the
 * permission prompt got resolved (approved+ran, or denied) - and only ever
 * silently clears a pending `needs_approval` badge; it never sets a badge of
 * its own. */
export type AgentStatus = "completed" | "needs_approval" | "resolved";

export interface AgentStatusPayload {
  provider: string;
  sessionId: string;
  status: AgentStatus;
  ts: number;
}

export interface OverlaySettingsPayload {
  petSize: number;
  calmMode: boolean;
}

export interface WindowSegmentPayload {
  id: number;
  x0: number;
  x1: number;
  y: number;
}

/** A horizontal ledge the pet can stand on: `"floor"` is the implicit
 * screen-bottom segment; everything else comes from `window_segments_changed`
 * and is already translated into this window's local canvas coordinates. */
export interface Segment {
  id: string;
  x0: number;
  x1: number;
  /** Screen y of the surface line (where the pet's feet rest), not yet
   * offset by pet height - use `surfaceY()`. */
  y: number;
}

export interface Food {
  id: string;
  x: number;
  y: number;
  vy: number;
  targetY: number;
  eaten: boolean;
  landedAt: number;
  /** Randomized per-landing so the hop height varies (capped) instead of always popping the same amount. */
  bounceHeight: number;
  /** Randomized per-landing sideways roll/nudge applied over the bounce, +right/-left. */
  bounceDriftX: number;
}

export interface PointerSample {
  x: number;
  y: number;
  t: number;
}
