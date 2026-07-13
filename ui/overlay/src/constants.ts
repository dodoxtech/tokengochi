// Tuning constants for the overlay. Kept in one place so gag/reaction
// frequency and physics feel can be adjusted without hunting through the
// state machine. See main.ts for the module overview.

export const BASE_PET_SIZE = 72;
export const FOOD_SIZE = 18;
export const HIT_PADDING = 10;
export const ACTIVE_TICK_MS = 1000 / 30;
export const IDLE_TICK_MS = 1000 / 2;
export const WALK_SPEED = 155;
export const DROP_SPEED = 520;
export const EAT_MS = 950;

// Task 0012 tuning.
export const GRAVITY = 2100; // px/s^2
export const MAX_THROW_SPEED = 1500; // px/s, hard cap so a throw can't leave the screen
export const TERMINAL_FALL_SPEED = 2200; // px/s
export const CLIMB_SPEED = 46; // px/s, deliberately slow per the design notes
export const JUMP_UP_HEIGHT = 26; // px, the little anticipation hop before a jump-down falls
export const JUMP_UP_SPEED = 560; // px/s, quick hop up
export const JUMP_DOWN_SPEED = 900; // px/s, a deliberate hop-down feels quick, unlike the climb
export const LANDING_PAUSE_MS = 420; // brief "getting up" beat before the pet moves off again
export const DRAG_PROMOTE_PX = 6; // movement past this turns a click into a drag
export const HOVER_POLL_MS = 80; // OS-level cursor poll cadence while click-through is active
export const CLICK_COMBO_WINDOW_MS = 2000;
export const CLICK_COMBO_COUNT = 3;
export const PET_STROKE_MS = 1000;
export const PET_BUMP_COOLDOWN_MS = 60_000; // mirrors the server-side rate limit
export const REACT_VARIANTS = ["squash", "spin", "look", "exclaim"] as const;
export const GAG_VARIANTS = ["sneeze", "stare", "chase-tail"] as const;
export const MIN_GAG_INTERVAL_MS = 5 * 60_000;
export const MAX_GAG_INTERVAL_MS = 10 * 60_000;
export const MIN_CLIMB_INTERVAL_MS = 90_000;
export const MAX_CLIMB_INTERVAL_MS = 240_000;
export const CLIMB_CHANCE = 0.35;
