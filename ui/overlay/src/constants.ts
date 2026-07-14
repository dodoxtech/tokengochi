// Tuning constants for the overlay. Kept in one place so gag/reaction
// frequency and physics feel can be adjusted without hunting through the
// state machine. See main.ts for the module overview.

export const BASE_PET_SIZE = 72;
export const FOOD_SIZE = 18;
export const HIT_PADDING = 10;
export const ACTIVE_TICK_MS = 1000 / 30;
export const IDLE_TICK_MS = 1000 / 2;
// Hard cap on a single tick's delta time. Without this, a render-loop stall
// (tab backgrounded, OS/IPC hiccup) hands the next tick a huge dtMs, and
// every per-tick movement formula scales with it - large enough to let the
// pet cover its entire remaining distance to a target in one frame (reads as
// a teleport instead of a walk). A few multiples of ACTIVE_TICK_MS keeps
// motion smooth under normal frame rates while bounding the worst case.
export const MAX_FRAME_DT_MS = 100;
export const WALK_SPEED = 155;
export const FOOD_GRAVITY = 1400; // px/s^2, lighter/floatier than the pet's fall
export const FOOD_TERMINAL_FALL_SPEED = 1100; // px/s
export const EAT_MS = 950;
export const FOOD_BOUNCE_MS = 260; // duration of the little landing hop
export const FOOD_BOUNCE_MIN_HEIGHT = 3; // px, lowest possible landing hop
export const FOOD_BOUNCE_MAX_HEIGHT = 8; // px, hard cap so the hop never looks like a real jump
export const FOOD_BOUNCE_MAX_DRIFT_X = 10; // px, max sideways roll/nudge on landing, either direction

// Task 0012 tuning.
export const GRAVITY = 2100; // px/s^2
export const MAX_THROW_SPEED = 1500; // px/s, hard cap so a throw can't leave the screen
export const TERMINAL_FALL_SPEED = 2200; // px/s
export const CLIMB_SPEED = 46; // px/s, deliberately slow per the design notes
export const JUMP_UP_HEIGHT = 26; // px, the little anticipation hop before a jump-down falls
export const JUMP_UP_SPEED = 560; // px/s, quick hop up
export const LANDING_PAUSE_MS = 420; // brief "getting up" beat before the pet moves off again
export const DRAG_PROMOTE_PX = 6; // movement past this turns a click into a drag
export const HOVER_POLL_MS = 80; // OS-level cursor poll cadence while click-through is active
export const CLICK_COMBO_WINDOW_MS = 2000;
export const CLICK_COMBO_COUNT = 3;
export const PET_STROKE_MS = 1000;
export const PET_BUMP_COOLDOWN_MS = 60_000; // mirrors the server-side rate limit
export const REACT_VARIANTS = ["squash", "spin", "look", "exclaim"] as const;
export const GAG_VARIANTS = ["sneeze", "stare", "chase-tail", "yawn", "dance", "drink-break"] as const;
// Task 0016: total on-screen time for each gag override, matching task 0014's
// authored frame tables (sneeze/yawn/drink-break play once; dance is a 2x
// loop of its 6-frame cycle). `stare`/`chase-tail` have no dedicated art yet
// (idle+effect fallback per the pet-action-pack spec) so they keep the
// original flat beat.
export const GAG_VARIANT_DURATION_MS: Record<(typeof GAG_VARIANTS)[number], number> = {
  sneeze: 440, // 120 + 90 + 80 + 150
  stare: 1800,
  "chase-tail": 1800,
  yawn: 800, // 150 + 250 + 200 + 200
  dance: 1320, // (110 * 6) * 2 loops
  "drink-break": 720, // 120 + 120 + 130 + 130 + 110 + 110
};
export const MIN_GAG_INTERVAL_MS = 5 * 60_000;
export const MAX_GAG_INTERVAL_MS = 10 * 60_000;
export const MIN_CLIMB_INTERVAL_MS = 90_000;
export const MAX_CLIMB_INTERVAL_MS = 240_000;
export const CLIMB_CHANCE = 0.35;

// How often (and how likely) the pet decides to walk over to a placed bed
// for a nap, and how long that nap lasts once it's there. Previously any
// bed present made the pet beeline for it and sleep indefinitely whenever
// no food was waiting, which crowded out idle gags/climbs entirely and read
// as "sleeps forever" - this makes napping one more randomized idle option
// with a bounded duration, same pattern as the climb roll.
export const MIN_SLEEP_INTERVAL_MS = 4 * 60_000;
export const MAX_SLEEP_INTERVAL_MS = 9 * 60_000;
export const SLEEP_CHANCE = 0.4;
export const MIN_SLEEP_DURATION_MS = 2 * 60_000;
export const MAX_SLEEP_DURATION_MS = 4 * 60_000;

// Task 0017 - agent status badge (turn completed / needs approval).
export const AGENT_STATUS_COMPLETED_BADGE_MS = 1800; // celebration badge visible time
export const AGENT_STATUS_NEEDS_APPROVAL_TIMEOUT_MS = 30 * 60_000; // safety-net auto-clear
