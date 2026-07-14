---
type: task
status: done
priority: P2
delivery_order: 0018
estimate: S
created: 2026-07-13
updated: 2026-07-13
owner: AI agent
sprint: null
tags:
  - task
  - done
---

# Task: Toggle off individual Sparks sinks (unequip / hide)

## Status

- State: done
- Created: 2026-07-13
- Owner: AI agent
- Priority: P2
- Delivery order: 0018
- Estimate: S
- Sprint: TBD

## Context

The dashboard shop panel ("Sparks sinks", [ui/dashboard/src/routes/+page.svelte](../../../ui/dashboard/src/routes/+page.svelte)) only let a player equip an owned cosmetic/food-skin or reposition owned furniture — there was no way to turn an owned item back off once equipped or placed. User asked (in Vietnamese) for a way to turn off/disable each owned sink individually. Confirmed via clarifying question: the desired scope is unequip/hide per owned item, not disabling whole shop categories.

Related: [[../../knowledge/game-economy|Game Economy]] §6 Sinks, [[../done/0010-cosmetics-shop-collection-album|0010 Cosmetics Shop]].

## Goal

Let a player turn off any owned sink item individually — unequip a cosmetic/food skin back to none, or hide a placed furniture item — without losing ownership or (for furniture) its saved position.

## Scope

In scope:

- Toggle behavior on `equip_shop_item`: calling it again on the currently equipped cosmetic/food skin unequips it (sets back to `None`).
- New `toggle_furniture_visibility` command/state method: flips a `visible` flag on a `FurniturePlacement` without removing it from `furniture` or resetting its `x`.
- Dashboard shop UI: "Equip"/"Unequip" label reflects toggle state; new "Hide"/"Show" button for furniture items.
- Overlay: skip drawing (and skip pathing/sleeping on) furniture whose `visible` is `false`.

Out of scope:

- Disabling entire shop categories/sinks (a settings-level toggle) — not requested.
- Heirloom items (Prestige-only, always equip/unequip via the same cosmetic path already).

## Acceptance Criteria

- [x] Clicking "Equip" on an already-equipped cosmetic or food skin unequips it (`equipped_cosmetic`/`equipped_food_skin` → `None`).
- [x] Clicking "Hide" on a placed furniture item stops it rendering on the overlay and stops the pet pathing to/sleeping on it, without losing its saved `x` position or ownership.
- [x] Clicking "Show" restores the furniture item at its previous position.
- [x] Old persisted saves without a `visible` field on furniture deserialize with `visible: true` (backward compatible).

## Dependencies

- None.

## Risks

- None significant; change is additive to existing `FurniturePlacement` shape with a serde default.

## Implementation Notes

- `src-tauri/src/pet/mod.rs`: added `visible: bool` to `FurniturePlacement` with `#[serde(default = "default_furniture_visible")]` for backward-compatible deserialization of existing saved state.
- `src-tauri/src/economy/state.rs`: `equip_item()` now toggles (sets to `None` if the same item is already equipped); added `toggle_furniture_visibility()` which flips the flag on an existing placement (errors `WrongItemKind`/`NotOwned` like the other shop methods).
- `src-tauri/src/lib.rs`: new `toggle_furniture_visibility` Tauri command, registered in the invoke handler alongside the other shop commands.
- `ui/dashboard/src/routes/+page.svelte`: shop panel button label flips to "Unequip"; furniture rows gained a "Hide"/"Show" button next to the position slider.
- `ui/overlay/src/types.ts` / `render.ts` / `behavior.ts`: `FurniturePlacement.visible` added; `drawFurniture()` skips hidden items; the sleep-seeking behavior only targets a visible bed.

## References

- [[../../README|Documentation]]
- [[../../knowledge/code-map|Code Map]]
- `src-tauri/src/pet/mod.rs`
- `src-tauri/src/economy/state.rs`
- `src-tauri/src/lib.rs`
- `ui/dashboard/src/routes/+page.svelte`
- `ui/overlay/src/render.ts`
- `ui/overlay/src/behavior.ts`

## Verification Plan

- [x] `cargo test economy::state` — all economy state tests pass, including two new tests.
- [x] `npx svelte-check` in `ui/dashboard` — 0 errors.
- [x] `npx tsc --noEmit` in `ui/overlay` — 0 errors.

## Verification Results

- `cargo build` and `cargo test economy::state`: 16 passed, 0 failed (includes new `equipping_an_already_equipped_item_toggles_it_off` and `toggling_furniture_visibility_flips_placement_without_losing_position`).
- `ui/dashboard`: `svelte-check` — 137 files, 0 errors, 0 warnings.
- `ui/overlay`: `tsc --noEmit` — no output, no errors.
- Not manually driven in a running app window this session (backend/UI logic verified via type-checks and unit tests only).

## Completion Notes

- Completed: 2026-07-13
- Changed files: `src-tauri/src/pet/mod.rs`, `src-tauri/src/economy/state.rs`, `src-tauri/src/lib.rs`, `ui/dashboard/src/routes/+page.svelte`, `ui/overlay/src/types.ts`, `ui/overlay/src/render.ts`, `ui/overlay/src/behavior.ts`, `docs/knowledge/code-map.md`.
- Follow-ups: none identified.
