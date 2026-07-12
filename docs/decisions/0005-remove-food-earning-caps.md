---
type: decision
status: accepted
created: 2026-07-12
updated: 2026-07-12
owner: AI agent
tags:
  - decision
  - game-design
  - economy
---

# Decision: Remove daily soft/hard cap and Pantry overflow from Food earning

## Status

Accepted

## Date

2026-07-12

## Context

[[../knowledge/game-economy|Game Economy Design]] §2/§7 originally specified a
deliberate anti-farming mechanic: the first 10 Food/day at full rate, then
1.5x geometric cost escalation per Food, a hard cap of 20 Food/day, and a
5-Food Pantry to absorb overflow (auto-fed on zero-usage days). Once both the
hard cap and the Pantry were full, further tokens that day were discarded
entirely ("token-burning is strictly irrational").

In real usage (task 0006 live verification and everyday dev sessions in this
repo), a single active coding day routinely blows past 20 Food well before
the day is over - Claude Code sessions generate tens of thousands of token
events. Hitting the cap early in the day meant the pet stopped visibly
reacting to further usage, which read as "broken" rather than "capped" (see
user report: real Claude Code usage continued, no new Food appeared). The
project owner decided the pacing goal isn't worth the confusion for this
project: unlimited Food, proportional to tokens spent, no ceiling.

## Decision

- Token → Food conversion is now a flat rate: every `tokens_per_food`
  weighted tokens earns exactly one Food, with no daily soft cap, no cost
  escalation, and no hard cap. Leftover tokens below one Food's cost still
  carry over as banked progress.
- The Pantry (overflow storage + zero-usage-day auto-feed) is removed
  entirely - it existed only to catch hard-cap overflow, so removing the cap
  makes it dead weight. Its SQLite column is kept (written as `0`) so
  existing installs' `NOT NULL` schema doesn't need a destructive migration.
- Food now spawns at a random reachable ground position in the overlay
  (`ui/overlay/src/main.ts`), rather than a deterministic offset.

## Consequences

Positive:

- The pet now visibly reacts to every unit of real usage, with no silent
  "your tokens did nothing today" cliff - directly fixes the reported
  confusion.
- Simpler engine: `apply_token_event_on_day` is a single division instead of
  an escalating-cost loop with two destinations and a waste branch.

Negative or tradeoffs:

- Removes the only documented anti-"burn tokens to feed the pet" mechanic.
  §7 of the game-economy doc no longer holds; there is currently no
  mechanism discouraging inflating usage purely for in-game reward, beyond
  whatever real cost/friction using Claude Code itself carries.
- The Pantry's zero-usage-day auto-feed also disappears, so a pet with no
  activity for several days no longer gets a small automatic feeding - it
  simply decays per the existing fullness mechanic ([[../knowledge/game-economy|§3]]).
- Very heavy single-day usage can now push a pet through many levels/XP in
  one sitting, which softens the "months-long stickiness" pacing goal in the
  original design doc.

## Alternatives Considered

- **Raise the caps instead of removing them** (e.g. 200 Food/day, larger
  Pantry): keeps an anti-abuse ceiling but still hits a wall eventually;
  rejected because the owner explicitly wants no ceiling at all.
- **Keep the Pantry as a separate "away from keyboard" feeding mechanic,
  decoupled from cap overflow:** would need its own stocking rule (e.g. bank
  a fraction of every Food earned); out of scope for this request, left as a
  future idea if "no activity for N days" retention pressure is wanted
  later.

## References

- [[../knowledge/game-economy|Game Economy Design]] §2, §7, §8
- [[../tasks/done/0004-economy-engine-core|Task 0004]] (original cap/Pantry implementation)
- [[../tasks/done/0006-food-drop-eating-loop|Task 0006]] (live verification that first hit the cap)
- `src-tauri/src/economy/state.rs`, `src-tauri/src/economy/conversion.rs`, `src-tauri/economy.toml`
