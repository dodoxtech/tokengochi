---
type: task
status: active
priority: P0
delivery_order: 0004
estimate: 3d
created: 2026-07-11
updated: 2026-07-12
owner: AI agent
sprint: null
tags:
  - task
  - active
---

# Task: Economy engine core (token → food conversion, caps, XP, fullness)

## Context

Implements [[../../knowledge/game-economy|Game Economy]] §1–3 and §8. Pure Rust module; all constants from `economy.toml`.

## Goal

Deterministic, unit-tested economy functions: weighted conversion, daily soft/hard caps with escalation, Pantry overflow, fullness decay, mood multipliers, XP accrual.

## Scope

In scope: `economy/` module (pure functions over a ledger), SQLite ledger schema (`store/`), day-boundary handling in local time, elapsed-time reconciliation on launch (decay while app was closed).

Out of scope: evolution branching, streaks/quests (task 0009), UI.

## Acceptance Criteria

- [x] Property/unit tests cover: weighting, soft-cap escalation exactly per spec, hard cap, Pantry fill/auto-feed, decay across app-closed gaps and DST changes. See Implementation Notes for exactly which test covers which behavior.
- [x] All constants read from `economy.toml`; changing a constant requires no code change. Every function takes `&EconomyConfig` as a parameter rather than hardcoding numbers; `changing_a_constant_changes_behavior_with_no_code_change` in `conversion.rs` demonstrates it directly.
- [x] Ledger dedup: replaying the same TokenEvents is idempotent. `store::Ledger` (SQLite, `INSERT OR IGNORE` keyed by `message_id`) - see `replaying_the_same_event_is_idempotent` and `repeated_full_replay_of_a_batch_is_idempotent` in `store/ledger.rs`.

## Dependencies

- [[0003-claude-code-token-watcher|0003]]

## Risks

- Time handling (sleep, timezone, clock changes) — test explicitly.

## Implementation Notes

- **`src-tauri/src/economy/conversion.rs`**: `weighted_tokens()` (applies configured weights) and `cost_of_nth_food(n, config)` (flat at `tokens_per_food` through the soft cap, then geometric escalation by `soft_cap_escalation` per Food beyond it - continues past the hard cap too, see the Pantry note below).
- **`src-tauri/src/economy/fullness.rs`**: `mood_from_fullness()` / `mood_multiplier()`, reusing the existing `crate::pet::Mood` enum (task 0001) rather than defining a second one - this is also the first real use of that enum, so it's no longer strictly dead code (though the module-level `#[allow(dead_code)]` on `pet/mod.rs` stays, since `EvolutionStage` is still unused).
- **`src-tauri/src/economy/xp.rs`**: `xp_required_for_level()` / `level_for_xp()` implementing `XP(n) = xp_curve_base * n^xp_curve_exponent`, capped at level 50 per [[../../knowledge/game-economy|Game Economy]] §4.
- **`src-tauri/src/economy/state.rs`** (the core orchestration): `EconomyState` holds `current_day` (a plain `chrono::NaiveDate`, no time-of-day/timezone inside this module), `food_earned_today`/`banked_tokens_today` (reset each day), `pantry` (persists across days, capped at `pantry_max`), `food_inventory` (earned-but-not-yet-eaten Food - eating itself, with the pet animation, is task 0006), `fullness`, `xp`.
  - `apply_token_event()`: banks an event's weighted tokens, then repeatedly buys the next Food at `cost_of_nth_food` while affordable, routing food past the hard cap into the Pantry (up to `pantry_max`), and discarding anything left over once both are full.
  - **Design decision, not fully pinned by the source doc:** [[../../knowledge/game-economy|Game Economy]] §2 didn't specify a conversion rate for Pantry-bound overflow. I chose to continue the *same* escalating cost curve past the hard cap rather than inventing a second rate, and documented that decision directly in the doc (§2) rather than leaving it implicit in code.
  - `reconcile_elapsed_time(now_unix, today, config)`: applies fullness decay proportional to raw elapsed *unix seconds* (`now_unix - last_reconciled_unix`) - this is what makes it DST-immune: a clock change relabels wall-clock time, it doesn't change how many seconds actually passed. Then rolls day boundaries one at a time via `today - current_day` (a `NaiveDate` diff, so calendar-correct via `chrono` rather than hand-rolled), firing one Pantry auto-feed per zero-usage day passed (`food_earned_today == 0` at that day's end).
  - `eat_from_inventory()`: shared fullness/XP math (mood evaluated from fullness *before* the meal, then `+fullness_per_food` capped at 100) - used by both the (future, task 0006) `pet_ate` command and Pantry auto-feed.
- **`src-tauri/src/store/ledger.rs`** (new): SQLite-backed (`rusqlite`, `bundled` feature so no system libsqlite3 dependency), one `token_events` table, `INSERT OR IGNORE` keyed by `message_id` for idempotent replay - a second, persistent dedup layer beneath task 0003's in-memory `WatcherState`, and the actual source of truth an `EconomyState` rebuild would replay from later. Token counts are cast `u64 -> i64` for storage (SQLite has no unsigned type; real token counts are nowhere near `i64::MAX`).
- New dependencies: `chrono` (day-boundary date math only - no `chrono::Local::now()` call exists yet inside the pure economy module itself; that integration point is follow-up work, see below), `rusqlite` with the `bundled` feature.
- **Not done in this task (explicitly out of scope or follow-up):** nothing here is wired into `lib.rs`/`run()` - there's no Tauri command consuming `EconomyState`/`Ledger` yet, no persistence of `EconomyState` itself (only the token ledger persists so far), no actual `chrono::Local::now()` call feeding real wall-clock time into `reconcile_elapsed_time`. That integration (plus the `pet_ate` command and food-drop event) is task 0006+ territory. `economy::` keeps its `#[allow(dead_code)]` for the same reason as `watcher::` and `store::`.

## Verification Plan

- [x] `cargo test` economy suite; simulated 30-day usage script; record results below.

## Verification Results

**Round 1 (sandbox, no Rust toolchain):**

- Hand-traced every test against the implementation logic line-by-line, and used Python (available in this sandbox) to independently simulate the cap-escalation loop before writing the hard-cap/Pantry test, rather than trusting hand arithmetic - the geometric escalation compounds fast enough that my first guess at a "huge event" size (5,000,000 weighted tokens) turned out to be nowhere near enough to fill the Pantry (reaching the hard cap alone costs ~3.6M, and the 21st food alone costs ~1.7M more); the fixed test uses 30,000,000 tokens.
- Caught and fixed a second bug of my own this way too: the first draft of `zero_usage_day_triggers_pantry_auto_feed` advanced only 1 day, which would have checked whether *day 1* (the heavy-usage day itself) counted as zero-usage - it doesn't. Fixed by advancing 2 days so the check lands on the actual zero-usage day.
- `Cargo.toml` parses as valid TOML; brace/paren balance checked across all new files.

**Round 2 (user ran `cargo test` for real):** 31 passed, 1 failed - `hard_cap_stops_daily_food_and_overflow_goes_to_pantry_then_is_wasted`, `assertion failed: outcome.tokens_wasted > 0.0`.

This was a **real bug in `apply_token_event`**, not a bad test, and my own Python simulation had actually papered over it: the loop checked "is the next food affordable" *before* checking "is there anywhere for it to go" (hard cap + Pantry both full). Once both are full, the escalating cost of a hypothetical next food is so large that `banked_tokens_today` is almost always *less* than it - so the loop exits via the affordability check and never reaches the "discard, nothing left to spend on" branch, silently leaving leftover tokens sitting in `banked_tokens_today` forever (well, until the next day-boundary reset) instead of being reported as wasted. Re-ran the Python simulation with the checks reordered (destination-availability first, affordability second) and confirmed it now produces `food_earned=20, food_to_pantry=5, wasted=3,586,366.58` - matching the test - before touching the Rust code, then applied the same reordering to `state.rs` (see the comment left in `apply_token_event`). Traced through every other passing test by hand to confirm none of them exercise the "both full" branch differently under the new ordering (only the hard-cap/Pantry test does) - i.e. this fix shouldn't flip any of the 31 previously-passing tests.

**Still needed:** please re-run `cargo test --manifest-path src-tauri/Cargo.toml` to confirm the fix actually gets all 32 tests green (I can reason about the logic but can't execute it here). The "simulated 30-day usage script" from the original Verification Plan was still not built as a separate script - the unit tests cover the individual mechanics but not an end-to-end multi-day scenario; marking the Plan checkbox based on the unit-test suite existing and (pending re-run) passing, not on that script existing.
- **Not verified - the "simulated 30-day usage script" in the Verification Plan was not built.** The unit tests cover the individual mechanics (weighting, cap escalation, Pantry, decay, DST-immunity, ledger idempotency) but not an end-to-end multi-day simulation script. Flagging as a real gap rather than quietly skipping it - happy to build one if useful, but wanted to be upfront that "unit tests only" is narrower than the plan called for.

**Conclusion:** code + tests are complete for the in-scope mechanics; staying in `docs/tasks/active/` until `cargo test` has actually run once.