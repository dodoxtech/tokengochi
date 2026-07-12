---
type: knowledge
status: active
created: 2026-07-11
updated: 2026-07-12
tags:
  - knowledge
  - game-design
  - economy
  - ai-context
owner: AI agent
---

# Game Economy Design

Design goals: reward *consistency* over *volume*, keep the pet emotionally sticky for months, and never incentivize burning tokens for the game's sake. Related: [[../product|Product Context]], [[token-tracking|Token Tracking]].

## 1. Currencies and Resources

| Resource | Earned by | Spent on | Notes |
|---|---|---|---|
| **Bits** (raw fuel) | Real tokens consumed (converted) | Auto-converts to Food | Never stored long; buffer only |
| **Food** | Bits crossing a threshold | Pet eats → Fullness + XP | The visible on-screen item |
| **XP** | Eating, streaks, milestones | Levels → evolution stages | Primary progression |
| **Sparks** (premium-feel soft currency) | Streaks, quests, evolutions | Cosmetics, food skins, furniture | Deliberately scarce; the long-term sink |
| **Fullness** (0–100) | Eating | Decays over time | Drives mood, not punishment-heavy |

## 2. Token → Food Conversion

- Base rate: **1 Food per 20k tokens** (tunable constant `TOKENS_PER_FOOD`).
- Token weighting to reflect real effort, not raw count: output tokens ×1.0, input tokens ×0.25, cache-read tokens ×0.05. Prevents cache-heavy sessions from flooding food.
- **Per-model tier multipliers:** tokens from bigger (pricier) models are worth more Food. The whole event's weighted total is scaled by a tier multiplier looked up from `[model_weights]` in `economy.toml` — keys are case-insensitive *substrings* of the model id (`"opus"` matches `"claude-opus-4-8"`), checked in sorted key order, first match wins; unknown or missing model ids use `model_weight_default` (×1.0). Initial tiers: Opus ×2.0, Sonnet ×1.0, Haiku ×0.4.
- **Daily soft cap with diminishing returns:** first 10 Food/day at full rate, then each subsequent Food costs ×1.5 more tokens (10 → 15 → 22.5…). Hard cap 20 Food/day. This is the core anti-"burn tokens to feed pet" mechanism.
- Overflow tokens beyond the hard cap trickle into a **Pantry** (max 5 stored Food) that auto-feeds the pet on days with zero usage — smooths weekends and protects streak-adjacent mechanics.
  - **Implementation decision (task 0004, `src-tauri/src/economy/state.rs`):** this doc didn't pin an exact conversion rate for Pantry-bound overflow, so the engine continues the *same* geometric escalation curve past the hard cap for Pantry food too (food #21 costs the same as the formula predicts, etc.), rather than a flat/discounted rate. Once the Pantry is also full (5/5), any further tokens that day are discarded entirely — reinforcing "token-burning is irrational" (§7) rather than finding a third bucket to put them in.

## 3. Hunger and Mood (retention without guilt)

- **The pet needs to eat every day.** The daily need is an explicit constant: `DAILY_FOOD_NEED` Food/day (1.5) holds fullness steady; decay is *derived* from it (`DAILY_FOOD_NEED × FULLNESS_PER_FOOD` = 30 points/24h) rather than tuned as a separate number. States: Full (75+), Content (40–74), Peckish (15–39), Hungry (5–14), **Starving (<5)**.
- A hungry pet gets sad, sits by the tray, and sleeps more — **it never dies and never loses levels**. Guilt mechanics kill desktop pets; melancholy is enough signal.
- **Starving = hibernation:** after roughly 3+ days unfed, the pet curls up and sleeps deeply — sad animation, XP gain ×0 (including the meal that wakes it, since mood is evaluated before eating). Still no death, no level loss, nothing taken away; the penalty is purely "progress pauses until you come back."
- Mood multiplies XP gain: Full ×1.2, Content ×1.0, Peckish ×0.8, Hungry ×0.5, Starving ×0. Feeding a hungry pet triggers a happy burst animation (positive reinforcement on return).

## 4. Progression and Evolution

- Levels 1–50 on a gentle exponential XP curve (`XP(n) = 50 · n^1.6`). A typical daily Claude Code user reaches ~L10 in week one, ~L30 in month two.
- **Evolution stages:** Egg (day 0) → Hatchling (L3) → Juvenile (L10) → Adult (L25) → Elder (L45).
- **Branching at Juvenile and Adult**, decided by *how* the user works, not RNG:
  - Night-heavy usage → nocturnal branch
  - Many short sessions → sprinter branch
  - Long deep sessions → scholar branch
  - Multi-provider usage → chimera branch (requires the multi-LLM plugin)
- Branches are cosmetic + minor perk (e.g., scholar: +5% XP from long sessions). Collection album records every form ever reached.

## 5. Streaks, Quests, and Events

- **Streak:** any real usage day (≥1 Food earned or Pantry auto-feed) continues the streak. Rewards: Sparks at 3/7/14/30/100 days; 1 free "streak freeze" earned per 7-day streak (max 2 banked). Forgiving by design.
- **Daily quest (1/day, auto-detected, no UI burden):** e.g., "earn 3 Food", "feed the pet before noon". Reward: 1–2 Sparks.
- **Weekly milestone:** cumulative weekly Food target scaled to the user's trailing 4-week average (personalized, so heavy users aren't bored and light users aren't excluded).
- **Seasonal events (v2):** limited-time food skins and one event evolution form per season.

## 6. Sinks (where Sparks go)

- Cosmetics: hats, scarves, palettes (5–30 Sparks).
- Food skins: sushi set, bánh mì set, bento set (10 Sparks).
- Desk furniture: a tiny bed, plant, monitor-top perch the pet actually uses (15–40 Sparks).
- **Prestige (Elder only):** retire the pet to the album Hall of Fame, hatch a new egg with +10% permanent XP and an heirloom cosmetic. Resets the loop for long-term players.

## 7. Anti-Abuse and Economy Health

- Diminishing returns + hard daily cap (see §2) make token-burning strictly irrational past ~200k weighted tokens/day.
- Manual/demo mode earns Food at ×0.25 rate and cannot progress past Juvenile — keeps real usage as the true engine.
- All balance constants live in one versioned config (`economy.toml`) so tuning never requires code changes; telemetry stays local (no server), tuning is done via releases.

## 8. Balance Reference (initial constants)

```
TOKENS_PER_FOOD        = 20_000 (weighted)
WEIGHTS                = out 1.0 / in 0.25 / cache_read 0.05
MODEL_WEIGHTS          = opus ×2.0 / sonnet ×1.0 / haiku ×0.4, default ×1.0
DAILY_SOFT_CAP         = 10 food, then ×1.5 escalation
DAILY_HARD_CAP         = 20 food
PANTRY_MAX             = 5 food
FULLNESS_PER_FOOD      = +20
DAILY_FOOD_NEED        = 1.5 food/day (decay derived: 1.5 × 20 = 30 / 24h)
XP_PER_FOOD            = 10 × mood multiplier (Starving = ×0)
XP_CURVE               = 50 · n^1.6
```

## Open Questions

- Should cache-read weighting be 0 instead of 0.05?
- Per-project pets vs. one global pet — affects whether Food is pooled.
- Sparks pricing needs a playtest pass once cosmetics count is known.
