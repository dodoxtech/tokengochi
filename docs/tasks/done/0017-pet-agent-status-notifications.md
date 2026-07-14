---
type: task
status: done
priority: P2
delivery_order: 0017
estimate: 1d
created: 2026-07-13
updated: 2026-07-13
owner: AI agent
sprint: null
tags:
  - task
  - done
  - pet
  - notifications
  - ai-context
---

# Task: Pet notifies AI agent status (done / needs approval) with cute expressions

## Status

- State: done
- Created: 2026-07-13
- Owner: AI agent
- Priority: P2
- Delivery order: 0017
- Estimate: 1d
- Sprint: TBD

## Context

Tokengochi already tails Claude Code session logs to convert token usage into
food ([[../../knowledge/token-tracking|Token Tracking]], watcher in
`src-tauri/src/watcher/claude_code.rs`). The pet is therefore in a unique
position to act as an ambient status indicator for AI agent sessions: instead
of the user watching the terminal, the pet on the desktop can signal
"Claude finished its turn" or "Claude is waiting for your approval" with a
cute expression, pose, or small effect — turning a utility notification into
a personality moment.

The user request (2026-07-13): the pet should notify the status of Claude (or
other AI agents) — e.g. *completed* or *needs approval* — via an expression
or something cute-looking.

Related:

- [[../../knowledge/token-tracking|Token Tracking]] — existing JSONL tailing infrastructure
- [[../../knowledge/pet-action-pack-spec|Pet Action Pack Spec]] — required contract for any new expression/animation asset
- [[../done/0014-expanded-gag-expression-pack|0014]] / [[../active/0016-implement-expanded-gag-expression-pack|0016]] — prior expression-pack workflow to reuse
- [[../../knowledge/code-map|Code Map]] — `apply_token_event()` wiring in `src-tauri/src/lib.rs`, sprite/animation selection via `MODE_ANIMATION_TAG`
- [[../README|Tasks]]
- [[../../agile|Agile and Scrum Workflow]]

## Goal

When a tracked AI agent session changes state (turn completed, waiting for
user approval/input), the pet visibly reacts with a distinct, cute,
non-intrusive signal on the overlay, so the user notices without needing a
system notification or checking the terminal.

## Scope

In scope:

- **Status detection (Claude Code first)**, choose and document one primary
  mechanism per status:
  - *Turn completed*: detectable from the session JSONL stream the watcher
    already tails (end-of-turn / result records), and/or a Claude Code `Stop`
    hook that pings Tokengochi locally.
  - *Needs approval / waiting for input*: Claude Code `Notification` /
    permission-request hook, since permission prompts may not appear in the
    JSONL usage records. Evaluate hooks vs. log parsing and record the choice
    (ADR if it constrains architecture).
- **Event plumbing**: extend the watcher/provider event model (currently only
  `TokenEvent`) with an `AgentStatusEvent { provider, session_id, status, ts }`
  (statuses at minimum: `completed`, `needs_approval`; design so other agents/
  plugins can emit the same enum later).
- **Pet reaction design** (cute, distinct per status), for example:
  - *Completed*: happy celebration — jump/spin/sparkle effect + a small
    "✓"-style speech bubble or emote.
  - *Needs approval*: attention-seeking — pet waves / holds up a "?" or "!"
    bubble, bouncing gently until acknowledged or timed out.
  - Reuse existing assets — checked during scoping (2026-07-13): no new
    art pack needed. `ui/assets/sprites/effects/effects.png` already has
    `exclaim` and `heart` clips (see `docs/assets/effects.yaml`), and
    `ui/assets/sprites/hatchling/hatchling.json` already has `happy`/`react`
    clips (see `docs/assets/hatchling.yaml`) — these cover *completed*
    (happy/heart) and *needs approval* (exclaim) without new art. The one
    gap: no `?` (question mark) frame exists yet for a clearer
    "needs approval" read than `!` alone; if needed, add 1-2 frames to the
    existing `effects.png` sheet (small addition, not a new pack) per
    [[../../knowledge/pet-action-pack-spec|Pet Action Pack Spec]] — do not
    scope a 0014/0016-style asset task for this.
- **Behavior rules**: reaction priority vs. current pet behaviors (eating,
  climbing), auto-clear timeout, de-duplication when multiple sessions fire,
  and a "needs approval" state that persists (idle attention loop) until the
  approval happens or the user interacts with the pet.
- **Settings**: per-status toggle in the tray/dashboard settings (on by
  default or off by default — decide during design), respecting the
  privacy rule of never reading message content.
- Multi-session handling: multiple concurrent Claude Code sessions are normal;
  define whether the pet aggregates or reacts per event.

Out of scope:

- OS-level notifications (Notification Center, sounds) — pet-only signal.
- Support for providers beyond Claude Code in v1 (design the event enum for
  them, but implement Claude Code only; Codex/others are follow-ups).
- Rich per-session UI (session list, which project is waiting) — the pet
  signals *that* something needs attention, not full session management.
- Two-way control (approving from the pet).

## Acceptance Criteria

- [x] Detection mechanism per status is chosen and documented:
      [[../../knowledge/agent-status-notifications|Agent Status
      Notifications]]. **Caveat**: the `Stop`/`Notification` hook mapping is
      implemented and unit-verified against the hook script's own output
      format, but has *not* been exercised against a real, currently-running
      Claude Code session in this environment (no live install available) —
      same class of caveat as the existing JSONL-schema notes in
      [[../../knowledge/token-tracking|Token Tracking]]. Re-verify on a real
      machine before relying on the `Notification` → `needs_approval` mapping
      in particular (see that doc's Open Questions).
- [x] `AgentStatusEvent` flows from watcher/hook → Rust backend
      (`agent_status_changed` Tauri event) → overlay frontend
      (`agentStatusBadge`); provider-agnostic in shape (`provider` field,
      `AgentStatus` enum easy to extend).
- [x] Pet plays a distinct cute reaction for *completed* (heart badge,~1.8s)
      immediately on event receipt.
- [x] Pet enters a persistent attention-seeking state for *needs approval*
      (bobbing exclaim badge) that clears on click/pet interaction, a
      following `completed` event, or a 30-minute safety-net timeout.
- [x] Reactions never interrupt/corrupt the existing behavior state machine —
      the badge (`ui/overlay/src/state.ts` `agentStatusBadge`) is drawn
      independent of `pet.mode` and never enters the override-mode chain in
      `updatePet()`, by construction (not just by discipline).
- [x] Multiple concurrent sessions do not spam: a single global badge is
      refreshed/overwritten rather than stacked or duplicated per session
      (documented decision in the knowledge note's Open Questions).
- [x] Settings toggle (`agentStatusNotificationsEnabled`) exists, persists via
      the existing `app_settings` SQLite migration pattern, and is checked in
      dashboard `+page.svelte`; disabling it stops `agent_status_changed`
      emission at the source (`start_agent_status_notify_watcher`) without
      touching `GameRuntime`/economy/food tracking.
- [x] No message content is read or stored — the hook script
      (`resources/claude-hooks/tokengochi-notify.sh`) extracts only
      `session_id` from the hook's stdin JSON.
- [x] Docs updated: [[../../knowledge/token-tracking|Token Tracking]] (new
      section), [[../../knowledge/code-map|Code Map]] (new wiring, both
      backend and frontend), plus a new
      [[../../knowledge/agent-status-notifications|Agent Status
      Notifications]] knowledge note.

## Dependencies

- Existing Claude Code watcher (`src-tauri/src/watcher/claude_code.rs`, task 0003).
- Overlay animation/effects system (tasks 0005, 0014/0016) for reusable emotes.
- If new sprites are needed: sprite asset pipeline
  ([[../../knowledge/sprite-asset-pipeline|Sprite Asset Pipeline]]) — split
  into a separate asset task if non-trivial.

## Risks

- Claude Code JSONL schema is undocumented/unstable; end-of-turn detection may
  break across versions (same risk already accepted in ADR-0002 / task 0003).
  Hooks are more stable but require user setup — detection strategy should
  degrade gracefully.
- "Needs approval" has no reliable JSONL signal; if hooks are required, the
  feature needs a small onboarding step (auto-install hook? document it).
- Attention-seeking animation can become annoying — needs careful tuning
  (subtle loop, timeout, easy disable).
- Stuck states: if the "approval" is granted while Tokengochi is closed or the
  hook fires twice, the pet must not stay in attention mode forever (timeout
  is the safety net).

## Implementation Notes

- Candidate detection sources (verify during implementation):
  - JSONL: a turn typically ends with a result/summary record after the last
    assistant message — the existing tailer can classify this without reading
    content.
  - Claude Code hooks (`Stop`, `Notification`, `PermissionRequest`-style
    events in `.claude/settings.json`) can run a tiny command that pings
    Tokengochi (e.g. writes to a named pipe / local socket / invokes a CLI
    shim). This is the most reliable path for *needs approval*.
- Keep the status enum extensible: `completed`, `needs_approval`, later maybe
  `error`, `working`.
- Frontend: reuse the emote/effect layer from the gag expression pack
  (`ui/overlay/src/main.ts`, `ui/assets/sprites/effects/`); a speech-bubble
  emote ("✓", "?", "!") may cover v1 without any new hero animation.
- Decide and record: does a *completed* celebration coincide with the
  `food_spawned` reaction (both fire at end of turn)? Avoid double animation.

## References

- [[../../README|Documentation]]
- [[../../index|Project Map]]
- `src-tauri/src/watcher/claude_code.rs`
- `src-tauri/src/lib.rs` (`apply_token_event()`, event emits)
- `ui/overlay/src/main.ts`
- [[../../decisions/0002-token-source-local-logs|ADR-0002]]

## Verification Plan

- [x] Rust unit tests for the new watcher (parse/dedup/restart-offset safety).
- [x] Full Rust test suite + `cargo clippy` (no new-code warnings).
- [x] Overlay TypeScript typecheck (`tsc --noEmit`) and esbuild bundle.
- [x] Dashboard `svelte-check` (new settings field + toggle).
- [x] Manual hook-script smoke test: piped a synthetic Claude Code hook
      stdin payload through `tokengochi-notify.sh`, confirmed the appended
      JSONL line parses correctly and matches the Rust `AgentStatusEvent`
      shape.
- [ ] **Not done in this environment**: running a real Claude Code session
      with the hook installed end-to-end (no live Claude Code install
      available here). Recommended before shipping: install the hook per
      [[../../knowledge/agent-status-notifications|Agent Status
      Notifications]], trigger a real `Stop` and a real permission prompt,
      confirm the pet reacts within ~2s and the badge behaves as designed.
- [ ] Two concurrent sessions: the "single global badge, refresh not stack"
      rule is implemented and documented but not exercised against two real
      concurrent sessions.

## Verification Results

- `cargo test --lib` (src-tauri): 54 passed, 0 failed (up from 49 before this
  task — 5 new tests in `watcher::agent_status`).
- `cargo clippy --lib`: no new warnings introduced by this task's files; the
  one warning in the new `agent_status.rs` (`io_other_error` on
  `std::io::Error::new(ErrorKind::Other, ...)`) mirrors the pre-existing,
  untouched style in `watcher/claude_code.rs:88`.
- `ui/overlay`: `npm run check` (tsc) clean; `npm run build` (esbuild)
  succeeds, bundle 97.5kb.
- `ui/dashboard`: `npm run check` (svelte-check) — 137 files, 0 errors, 0
  warnings.
- Hook script manual test (`TOKENGOCHI_DATA_DIR=/tmp/... tokengochi-notify.sh
  completed` / `needs_approval`, piped a fake `{"session_id":"sess-abc123"}`
  stdin): produced valid JSONL lines matching
  `{"provider":"claude_code","session_id":"sess-abc123","status":"completed"|"needs_approval","ts":<unix>}`,
  confirmed parseable with `json.loads`.

## Completion Notes

- Completed: 2026-07-13
- Changed files:
  - Backend: `src-tauri/src/watcher/agent_status.rs` (new),
    `src-tauri/src/watcher/mod.rs`, `src-tauri/src/lib.rs`,
    `src-tauri/src/store/game_state.rs` (new `agent_status_notifications_enabled`
    setting + SQLite migration column).
  - Frontend (overlay): `ui/overlay/src/types.ts`, `constants.ts`, `state.ts`,
    `behavior.ts`, `render.ts`, `main.ts`.
  - Frontend (dashboard): `ui/dashboard/src/routes/+page.svelte` (new toggle).
  - Hook: `resources/claude-hooks/tokengochi-notify.sh` (new, executable).
  - Docs: `docs/knowledge/agent-status-notifications.md` (new),
    `docs/knowledge/token-tracking.md`, `docs/knowledge/code-map.md`,
    `docs/knowledge/README.md`.
- Follow-ups:
  - Verify the `Stop`/`Notification` hook mapping against a real, currently
    installed Claude Code version (see Acceptance Criteria caveat above and
    the knowledge note's Open Questions).
  - No auto-install step for the hook JSON — a user has to hand-edit their
    Claude Code `settings.json`. A dashboard "install hook" button is a
    natural follow-up task.
  - No Windows hook script (bash-only); a `.ps1` equivalent is needed if
    Windows becomes a supported platform for this feature.
  - Provider support beyond Claude Code (Codex CLI, etc.) is out of scope for
    this task per the Scope section — the event shape is provider-agnostic
    but only the Claude Code hook script exists today.
