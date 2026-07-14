#!/usr/bin/env bash
# Appends a Tokengochi agent-status event so the desktop pet can react to
# Claude Code turn completions, approval prompts, and approval resolutions
# (task 0017, extended 2026-07-14). Installed as a Stop/PermissionRequest/
# PostToolUse/PermissionDenied hook command in Claude Code settings - see
# docs/knowledge/agent-status-notifications.md for the full setup and the
# `hooks` JSON snippet to add.
#
# Only the session id is read from the hook's stdin JSON - never the message
# content - matching the privacy rule in docs/knowledge/token-tracking.md.
set -euo pipefail

STATUS="${1:-}"
if [[ "$STATUS" != "completed" && "$STATUS" != "needs_approval" && "$STATUS" != "resolved" ]]; then
  echo "usage: tokengochi-notify.sh <completed|needs_approval|resolved>" >&2
  exit 1
fi

# Same base directory Rust's `dirs::data_dir()` resolves to on each OS -
# must stay in sync with `agent_status_events_path()` in
# src-tauri/src/watcher/agent_status.rs.
if [[ -n "${TOKENGOCHI_DATA_DIR:-}" ]]; then
  DATA_DIR="$TOKENGOCHI_DATA_DIR"
elif [[ "$(uname -s)" == "Darwin" ]]; then
  DATA_DIR="$HOME/Library/Application Support/tokengochi"
else
  DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/tokengochi"
fi

EVENTS_FILE="$DATA_DIR/agent_status_events.jsonl"
mkdir -p "$DATA_DIR"

# Claude Code hooks receive a JSON object on stdin (e.g. `{"session_id":
# "...", "hook_event_name": "Stop", ...}`). Pull only session_id; tolerate a
# missing jq or an unparseable/empty payload rather than failing the hook.
HOOK_INPUT="$(cat 2>/dev/null || true)"
if command -v jq >/dev/null 2>&1; then
  SESSION_ID="$(printf '%s' "$HOOK_INPUT" | jq -r '.session_id // "unknown"' 2>/dev/null || echo unknown)"
else
  SESSION_ID="unknown"
fi
# Defensive: strip anything that could break the hand-built JSON line below
# (session ids are normally UUID-like, but never trust external input).
SESSION_ID="$(printf '%s' "$SESSION_ID" | tr -d '"\\\n\r')"
[[ -n "$SESSION_ID" ]] || SESSION_ID="unknown"

TS="$(date +%s)"

printf '{"provider":"claude_code","session_id":"%s","status":"%s","ts":%s}\n' \
  "$SESSION_ID" "$STATUS" "$TS" >> "$EVENTS_FILE"
