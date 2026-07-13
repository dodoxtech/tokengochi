---
type: decision
status: accepted
created: 2026-07-13
updated: 2026-07-13
owner: AI agent
tags:
  - decision
  - game-design
  - ui
---

# Decision: Hide the Collection Album panel from the dashboard

## Status

Accepted

## Date

2026-07-13

## Context

The dashboard (`ui/dashboard/src/routes/+page.svelte`) ships an Album panel
("Collection") implemented in task
[[../tasks/done/0010-cosmetics-shop-collection-album|0010]], displaying every
evolution form the pet has reached (day, stage, branch, level, prestige
count) per [[../knowledge/game-economy|Game Economy §6]]. Each album card is
currently text-only (no per-form artwork), and there are no suitable art
assets yet to represent each evolution stage/branch visually in the album.
Shipping a text-only album reads as incomplete/unpolished, so it is being
hidden from the UI until assets exist.

## Decision

- Comment out the entire Album panel markup in
  `ui/dashboard/src/routes/+page.svelte` (lines ~355-380), including the
  **Prestige** button, since it lives in the same panel header and prestige
  is conceptually tied to the album/legacy loop.
- No backend changes: Rust album logic (`record_album_entry` in
  `src-tauri/src/economy/state.rs`, `AlbumRecord` in `src-tauri/src/pet/mod.rs`,
  SQLite persistence in `src-tauri/src/store/game_state.rs`) keeps running
  unchanged. Album records continue to be recorded on every evolution; the
  data is just not surfaced in the dashboard yet.
- CSS rules for `.album-panel` / `.album-grid` / `.album-card` are left in
  place (shared selectors with `.shop`), producing harmless "unused CSS
  selector" warnings from `svelte-check` until the panel is restored.

## Consequences

Positive:

- No more visible text-only/placeholder album UI while assets are missing.
- Zero data loss: album history keeps accumulating in SQLite, so re-enabling
  the panel later requires no backfill.

Negative or tradeoffs:

- The Prestige action is also hidden, since it shares the album panel's
  header — Elder-stage players temporarily cannot prestige from the UI.
- Dead CSS selectors remain until the panel is restored (cosmetic
  `svelte-check` warnings only, not build-breaking).

## Alternatives Considered

- **Hide only the album grid, keep the Prestige button visible** (e.g. move
  it into the Shop panel): rejected for now to keep the change minimal;
  revisit if Elder players need prestige access before album assets are
  ready.

## References

- [[../knowledge/game-economy|Game Economy Design]] §6
- [[../tasks/done/0010-cosmetics-shop-collection-album|Task 0010]] (original album/prestige implementation)
- `ui/dashboard/src/routes/+page.svelte`
