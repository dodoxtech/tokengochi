---
type: task
status: active
priority: P2
delivery_order: 0027
estimate: M
created: 2026-07-23
updated: 2026-07-23
owner: AI agent
sprint: null
tags:
  - task
  - active
  - ai-context
---

# Task: Codex CLI agent-status hooks (parity with Claude Code)

## Status

- State: active
- Created: 2026-07-23
- Owner: AI agent
- Priority: P2
- Delivery order: 0027
- Estimate: M
- Sprint: TBD

## Context

The pet reacts to Claude Code turn/approval state via hooks the app auto-installs
into `~/.claude/settings.json` (task 0017, code in
[`src-tauri/src/claude_hooks.rs`](../../../src-tauri/src/claude_hooks.rs), dashboard
"Install hook" / "Remove hook" buttons). As of 2026, Codex CLI ships a hooks
system that is almost 1:1 with Claude Code, including identically-named events, so
the same "which agent state is active" logic maps directly onto Codex.

Verified against Codex docs:

- Supported events include `SessionStart`, `PreToolUse`, `PermissionRequest`,
  `PostToolUse`, `UserPromptSubmit`, `Stop` (turn complete), and more.
- `PermissionRequest` fires *"when Codex is about to ask for approval, such as a
  shell escalation or managed-network approval"* — the direct equivalent of the
  Claude `PermissionRequest` hook that drives our `needs_approval` badge.
- Hooks live in `~/.codex/hooks.json` **or** inline `[hooks]` tables in
  `~/.codex/config.toml` (also repo-local `.codex/`).
- Older Codex versions only had a `notify` program that fires on
  `agent-turn-complete` (no approval event — see openai/codex issue #11808), so
  those builds can only power `completed`, not `needs_approval`.

Sources:

- Codex Hooks — https://learn.chatgpt.com/docs/hooks
- Advanced config (`notify`) — https://developers.openai.com/codex/config-advanced
- notify-for-approval feature request — https://github.com/openai/codex/issues/11808

Related:

- [[../done/0017-pet-agent-status-notifications|Task 0017 - Pet agent-status notifications]]
- [[../../knowledge/agent-status-notifications|Agent Status Notifications]]
- [[../../knowledge/code-map|Code Map]]
- [[../README|Tasks]]
- [[../../agile|Agile and Scrum Workflow]]

## Goal

Let Tokengochi auto-install/uninstall agent-status hooks for Codex CLI so the pet
reacts to Codex turn-complete and approval-request state the same way it already
does for Claude Code, without the user hand-editing Codex config.

## Scope

In scope:

- New backend module `src-tauri/src/codex_hooks.rs` mirroring `claude_hooks.rs`:
  `status()` / `install()` / `uninstall()`, marker-based idempotency, atomic
  write with `.bak` backup, and pruning of keys we emptied.
- Target `~/.codex/hooks.json` (JSON) so we reuse the existing atomic-JSON
  pattern instead of editing TOML.
- Tauri commands `codex_hook_status` / `install_codex_hooks` /
  `uninstall_codex_hooks`, registered in `lib.rs`.
- Dashboard UI: a "Codex Code hook" field mirroring the Claude one
  (`ui/dashboard/src/routes/+page.svelte`), with Install / Remove buttons and
  status text.
- Adapt the notify script (or add a Codex-aware branch) so it reads the correct
  session identifier from the Codex hook payload and writes to the same
  `agent_status_events.jsonl` the overlay tailer already consumes.
- Event mapping (subject to payload verification):
  - `Stop` -> `completed`
  - `PermissionRequest` -> `needs_approval`
  - `PostToolUse` -> `resolved`

Out of scope:

- Editing `~/.codex/config.toml` inline `[hooks]` tables (JSON `hooks.json` only).
- Supporting the legacy `notify`-only path on old Codex builds (note as a
  follow-up; those can only emit `completed`).
- Changes to the overlay badge rendering or `agent_status.rs` tailer — both
  providers write to the same event file, so no overlay change is expected.
- Per-project (`.codex/`) install — global-only, matching the Claude decision.

## Acceptance Criteria

- [x] Codex payload schema is verified: we know exactly which stdin/JSON field
      carries the session id and event, documented in
      `docs/knowledge/agent-status-notifications.md`. (Verified against the
      published Codex hooks reference, not a live Codex install - see the
      outstanding item below.)
- [x] `install_codex_hooks` writes idempotent, marker-tagged entries into
      `~/.codex/hooks.json` for `Stop`, `PermissionRequest`, `PostToolUse`;
      re-running reports `changed: false` and does not duplicate.
- [x] `uninstall_codex_hooks` removes only our managed entries, preserves
      hand-added Codex hooks and unrelated config, and prunes emptied keys.
- [x] Both operations back up the file to `.bak` before writing and write
      atomically (tmp + rename), matching the Claude module.
- [x] Dashboard shows Codex hook install status and Install/Remove buttons that
      round-trip through `codex_hook_status`.
- [ ] **Outstanding:** with the hook installed and Codex driven through a turn
      + an approval prompt on a real Codex CLI install, the pet shows
      `needs_approval` then clears to `completed`. Not yet verified live - no
      Codex CLI environment was available during implementation. This is why
      the task stays in `active/` rather than moving to `done/`.
- [x] `docs/knowledge/code-map.md` gains a Codex hooks row.

## Dependencies

- ~~Verify Codex hook payload format on a real Codex CLI install~~ - resolved
  via the published Codex hooks reference (`session_id` on stdin for both
  `Stop` and `PermissionRequest`, same field name Claude Code uses), so the
  script-side `jq` extraction needed no changes. Still not confirmed against
  a live Codex CLI session - see the outstanding acceptance criterion above.

## Risks

- Codex hooks are new and the payload schema may shift between Codex releases;
  guard the script against missing fields and fail soft (no badge) rather than
  erroring.
- `hooks.json` vs inline `config.toml` precedence: if a user already configured
  hooks in `config.toml`, our `hooks.json` entry must not conflict — confirm how
  Codex merges the two config layers.
- Some Codex builds predate the hooks system; detect absence gracefully and
  surface a clear "update Codex" message rather than writing a file Codex ignores.

## Implementation Notes

- Mirror the existing constants: a `HOOK_MARKER` (script basename) and a
  `--tokengochi-managed` flag for detecting our entries. Implemented exactly
  this way in `codex_hooks.rs`.
- Reuse `resources/claude-hooks/tokengochi-notify.sh`; the Codex payload uses
  the same `session_id` field as Claude, so no script parsing change was
  needed - only an added `--provider claude_code|codex` flag (default
  `claude_code` for backward compatibility with already-installed Claude
  entries) so the emitted event's `provider` field is correct per source.
- Factored the shared JSON-hook plumbing (atomic write, backup, marker scan,
  empty-key prune) out of `claude_hooks.rs` into `src-tauri/src/hook_settings.rs`,
  used by both provider modules - avoids the drift risk flagged above.
- The overlay/tailer path (`src-tauri/src/watcher/agent_status.rs` ->
  `agent_status_changed`) is provider-agnostic; both write the same JSONL -
  confirmed no changes were needed there.
- The "Pet reacts to Claude status" dashboard toggle/copy was renamed to
  "Pet reacts to agent status" since it gates both providers' events
  (`agent_status_notifications_enabled` in `lib.rs` is provider-agnostic).

## References

- [[../../README|Documentation]]
- [[../../index|Project Map]]
- `src-tauri/src/claude_hooks.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/watcher/agent_status.rs`
- `resources/claude-hooks/tokengochi-notify.sh`
- `ui/dashboard/src/routes/+page.svelte`
- `docs/knowledge/agent-status-notifications.md`

## Verification Plan

- [x] `cargo test codex_hooks` — unit tests for install idempotency, uninstall
      cleanup, and preservation of hand-added entries (mirror the Claude tests).
- [x] `cargo test` (full suite) — no regressions in `claude_hooks` after the
      `hook_settings` extraction, or elsewhere.
- [x] `cargo fmt --check` / `cargo fmt`.
- [x] `bash -n resources/claude-hooks/tokengochi-notify.sh` + manual smoke
      test of both the `--provider codex` path and the no-flag backward-compat
      path (see Verification Results).
- [x] `npm run check` (svelte-check) in `ui/dashboard/` — dashboard UI changes
      type-check cleanly.
- [ ] **Not done:** Manual - install via dashboard, inspect `~/.codex/hooks.json`,
      run a real Codex turn with an approval prompt, confirm badge transitions.
      No Codex CLI environment was available to exercise this.
- [ ] **Not done:** Manual - uninstall against a real install, confirm only our
      entries removed and `.bak` created. Covered by unit test instead
      (`uninstall_removes_managed_entries_and_prunes_empty_keys`), not by a
      live Codex CLI round-trip.

## Verification Results

- `cargo test` (from `src-tauri/`): **76 passed, 0 failed** - includes 4 new
  `codex_hooks` tests, the existing 4 `claude_hooks` tests (unchanged
  behavior after the `hook_settings` extraction), and 5 `agent_status`
  watcher tests (provider field is a plain `String`, so `"codex"` round-trips
  through `parse_status_line` without any watcher change).
- `cargo fmt --check`: clean after one auto-formatted line wrap in
  `codex_hooks.rs`.
- `bash -n resources/claude-hooks/tokengochi-notify.sh`: syntax OK.
- Manual script smoke test:
  - `bash tokengochi-notify.sh needs_approval --provider codex --tokengochi-managed` with stdin `{"session_id":"abc-123"}` → appended
    `{"provider":"codex","session_id":"abc-123","status":"needs_approval","ts":...}`.
  - `bash tokengochi-notify.sh completed --tokengochi-managed` (no `--provider`
    flag, simulating an already-installed Claude Code hook) with stdin
    `{"session_id":"xyz"}` → appended
    `{"provider":"claude_code","session_id":"xyz","status":"completed","ts":...}`,
    confirming the backward-compat default.
- `npm run check` (in `ui/dashboard/`): `0 ERRORS 0 WARNINGS` across 151 files.

## Completion Notes

Not moved to `docs/tasks/done/` yet - the live Codex CLI verification step
above is still outstanding (no Codex CLI environment available during this
implementation pass). Move to `done/` once someone with a Codex CLI + hooks
support has confirmed the badge reacts correctly end-to-end, and update this
section then.

- Changed files:
  - `src-tauri/src/hook_settings.rs` (new - shared JSON hook-settings plumbing)
  - `src-tauri/src/codex_hooks.rs` (new - Codex CLI status/install/uninstall)
  - `src-tauri/src/claude_hooks.rs` (refactored onto `hook_settings`, added
    `--provider claude_code` to its command line, added `uninstall()` in the
    same change series)
  - `src-tauri/src/lib.rs` (registered `codex_hook_status`/`install_codex_hooks`/
    `uninstall_codex_hooks`, plus the earlier `uninstall_agent_status_hooks`)
  - `resources/claude-hooks/tokengochi-notify.sh` (added `--provider` flag)
  - `ui/dashboard/src/routes/+page.svelte` (Codex CLI hook field, generic
    "Pet reacts to agent status" toggle copy)
  - `docs/knowledge/agent-status-notifications.md` (Codex CLI support section)
  - `docs/knowledge/code-map.md` (new rows for `hook_settings.rs`/`codex_hooks.rs`)
- Follow-ups:
  - Live-verify against a real Codex CLI install with hooks support (blocks
    moving this task to `done/`).
  - Consider a Codex-version check/warning for builds that predate hooks
    support (see Open Questions in the knowledge doc).
  - Windows `.ps1` hook script equivalent remains a follow-up for both
    providers (pre-existing gap, not introduced here).
