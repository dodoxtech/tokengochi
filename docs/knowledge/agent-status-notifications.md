---
type: knowledge
status: active
created: 2026-07-13
updated: 2026-07-14
tags:
  - knowledge
  - integration
  - ai-context
owner: AI agent
---

# Agent Status Notifications

How the pet reacts to Claude Code turn-completed / needs-approval events
(task [[../tasks/done/0017-pet-agent-status-notifications|0017]]). Related:
[[token-tracking|Token Tracking]], [[code-map|Code Map]].

## Why not reuse the token-usage JSONL watcher

The Claude Code token watcher (`src-tauri/src/watcher/claude_code.rs`) tails
`~/.claude/projects/**/*.jsonl` for per-message `usage` numbers, but that
schema is undocumented and doesn't reliably carry a "turn is fully done" or
"a permission prompt is blocking" signal - inferring either from log shape
would be guesswork layered on an already-unverified format (see
[[token-tracking|Token Tracking]] Open Questions). Claude Code **hooks** are
the documented, stable mechanism for exactly this: `Stop` fires when Claude
finishes responding, `PermissionRequest` fires exactly when a permission
dialog appears. This project maps `Stop` → `completed` and
`PermissionRequest` → `needs_approval`.

**Extension (2026-07-14): `resolved`.** `Stop` only fires once the *entire*
turn ends, which can be long after a single permission prompt is answered if
more tool calls follow - live testing showed the `needs_approval` badge
correctly appearing on the prompt, but then lingering well past the user
clicking "Allow" because the turn kept going. There's no dedicated "permission
granted" hook, but `PostToolUse` (fires after a tool call succeeds) and
`PermissionDenied` (fires when a tool call is denied) both mark the prompt as
resolved either way, sooner than `Stop` would. Both are mapped to a third
status, `resolved`, which - unlike `completed` - never sets a badge of its
own; it only silently clears an existing `needs_approval` badge (see the
`agent_status_changed` listener in `ui/overlay/src/main.ts`). Kept distinct
from `completed` specifically so it doesn't replay the heart celebration on
every single tool call - `PostToolUse` fires far more often than `Stop`.

**Correction (2026-07-14):** this originally mapped `needs_approval` to the
`Notification` hook instead, on the assumption that `Notification` covers
permission prompts. Live testing inside the VS Code extension falsified that:
across several real sessions with multiple tool calls requiring approval, the
local event log (`agent_status_events.jsonl`) only ever recorded `completed`
events - `Notification` never fired for a permission prompt in that
environment, which matched exactly what the user observed (no pet reaction
until clicking "Allow", i.e. only the post-approval `Stop` firing). Claude
Code's hooks reference confirms a dedicated `PermissionRequest` event ("when a
permission dialog appears"), which is the correct, precise signal - `install()`
in `src-tauri/src/claude_hooks.rs` now wires `needs_approval` to it instead,
and also strips any leftover `Notification`-based managed entry from an older
install so the two mappings don't double up (`Notification` also fires for
plain idle-waiting, which would otherwise cause spurious badges).

## Local event bridge

Hooks run as one-shot shell commands with no direct channel back into a
running Tauri app, so the bridge is a small local file instead of a socket/IPC
mechanism (simpler, and reuses the same "watch a file with `notify`" pattern
already proven by the token watcher):

1. A hook script (`resources/claude-hooks/tokengochi-notify.sh`) appends one
   JSON line per event to
   `<data_dir>/<watcher_namespace>/agent_status_events.jsonl` - the same base
   directory the token watcher uses for its own state file. The watcher
   namespace is `tokengochi` in release builds and `tokengochi-dev` in
   debug/dev builds. Each line: `{"provider":"claude_code","session_id":"...",
   "status":"completed"|"needs_approval"|"resolved","ts":<unix_seconds>}`. Only the
   session id is read from the hook's stdin JSON - never message content,
   per the privacy rule in [[token-tracking|Token Tracking]].
2. `src-tauri/src/watcher/agent_status.rs` (`start_agent_status_watcher`)
   tails that file the same way `claude_code.rs` tails usage logs: byte
   offset persisted to `agent_status_watcher_state.json` next to it, so a
   restart never replays old lines, but a pending event written while
   Tokengochi was closed is still delivered on next launch.
3. `lib.rs` (`start_agent_status_notify_watcher`) forwards each event as an
   `agent_status_changed` Tauri event to the overlay, gated by the
   `agent_status_notifications_enabled` setting - deliberately decoupled from
   `GameRuntime`/economy so a reaction can never touch fullness/XP/food state.
4. The overlay (`ui/overlay/src/state.ts` → `agentStatusBadge`) draws the
   reaction as a badge layered on top of the pet, **independent of
   `pet.mode`** - it never enters the override-mode state machine that drives
   movement/eating/climbing, so it can't corrupt or block those behaviors.
   `completed` shows a brief heart; `needs_approval` shows a gently bobbing
   exclaim bubble that persists until the user clicks/pets the pet, a
   `completed` or `resolved` event arrives, or a 30-minute safety-net timeout
   elapses (`AGENT_STATUS_NEEDS_APPROVAL_TIMEOUT_MS` in `constants.ts`).
   `resolved` only ever clears an existing `needs_approval` badge - it never
   sets one, so it's a no-op when nothing is pending.

## Installing the hook

**Auto-install (recommended):** the Dashboard's Settings section has an
"Install hook" button (visible whenever the hook isn't detected yet) that
writes the `Stop`/`PermissionRequest`/`PostToolUse`/`PermissionDenied`
entries into the user's **global** `~/.claude/settings.json` for you -
`src-tauri/src/claude_hooks.rs` (`MANAGED_HOOKS`, `install()`/`status()`,
exposed as the `install_agent_status_hooks`/`agent_status_hook_status` Tauri
commands). Global-only by design: the point is "the pet reacts whenever I use
Claude Code anywhere," not just in one project.

It follows the same shape the `openpets` project uses for the identical
problem (see its `packages/claude/src/hook-settings.ts`): read-modify-write
with a `.bak` backup made before every write, a temp-file + rename for the
actual write, and each managed hook command tagged with a `tokengochi-notify.sh`
substring + `--tokengochi-managed` marker so re-running install is idempotent
(detects an already-installed entry, whether installed by this button or
hand-copied from the JSON snippet below) and never duplicates or clobbers
other hooks (e.g. this repo's own `PostToolUse` hook) already in the file.
`serde_json`'s `preserve_order` feature is enabled specifically so this
read-modify-write doesn't alphabetize-reorder the rest of the user's
hand-maintained `settings.json`.

**Manual (per Claude Code project or globally):** the user (or, for a
project other than this one, an equivalent one-off edit) can instead hand-add
this to their Claude Code `settings.json` (project-level
`.claude/settings.json` or global `~/.claude/settings.json`, depending on
whether they want it for one project or every session):

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/absolute/path/to/tokengochi/resources/claude-hooks/tokengochi-notify.sh completed"
          }
        ]
      }
    ],
    "PermissionRequest": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/absolute/path/to/tokengochi/resources/claude-hooks/tokengochi-notify.sh needs_approval"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/absolute/path/to/tokengochi/resources/claude-hooks/tokengochi-notify.sh resolved"
          }
        ]
      }
    ],
    "PermissionDenied": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/absolute/path/to/tokengochi/resources/claude-hooks/tokengochi-notify.sh resolved"
          }
        ]
      }
    ]
  }
}
```

The script auto-detects the production OS data directory (macOS
`~/Library/Application Support/tokengochi`, Linux `$XDG_DATA_HOME/tokengochi`
or `~/.local/share/tokengochi`) unless `TOKENGOCHI_DATA_DIR` is set. The app's
hook installer sets `TOKENGOCHI_DATA_DIR` explicitly so debug/dev builds write
to `tokengochi-dev` while release builds write to `tokengochi`; keep this in
sync with `storage_paths::watcher_data_file()` if either side changes.

## Open Questions / Follow-ups

- Windows has no hook script yet (bash-only); a `.ps1` equivalent is a
  follow-up if Windows becomes a supported platform for this feature.
- `claude_hooks::hook_script_path()` resolves the script relative to
  `CARGO_MANIFEST_DIR` (i.e. assumes the repo checkout is present on disk).
  That's fine for how this app runs today (`cargo run`/`tauri dev` from
  source), but will break for a packaged build shipped to a machine without
  the repo - bundling the script as a Tauri resource (`BaseDirectory::Resource`,
  same pattern as `economy.toml`) is a follow-up for whenever this ships.
- Auto-install only targets the global `~/.claude/settings.json`; there's no
  per-project install path from the dashboard (the manual JSON snippet above
  still covers that case if someone wants it).
- `PermissionRequest` → `needs_approval` has only been verified in the VS Code
  extension so far; the terminal CLI path is assumed to fire it the same way
  per the hooks reference but hasn't been separately confirmed live.
- Multi-session behavior: the current design does not distinguish which
  session an event came from beyond carrying `session_id` through the event -
  the badge is a single global "something needs attention" signal, not
  per-session. Concurrent sessions overwrite/refresh the same badge rather
  than stacking.
