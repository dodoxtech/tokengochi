#!/usr/bin/env bash
# PostToolUse hook: nudges the agent to refresh docs/knowledge/code-map.md
# whenever it edits one of the files that map indexes (pet behavior AI,
# state machine, overlay window plumbing). See docs/knowledge/code-map.md
# "How to Keep This Doc Useful".
set -euo pipefail

input="$(cat)"
file_path="$(printf '%s' "$input" | jq -r '.tool_input.file_path // .tool_response.filePath // empty')"

if [ -z "$file_path" ]; then
  exit 0
fi

case "$file_path" in
  */ui/overlay/src/main.ts | \
  */src-tauri/src/pet/mod.rs | \
  */src-tauri/src/overlay_window.rs | \
  */src-tauri/src/window_geometry/mod.rs | \
  */src-tauri/src/lib.rs | \
  */src-tauri/src/store/game_state.rs)
    ;;
  *)
    exit 0
    ;;
esac

msg="This file is indexed in docs/knowledge/code-map.md (pet behavior/movement code map). If this edit renamed, moved, or added a function referenced there, update that doc in the same change."

jq -n --arg msg "$msg" \
  '{systemMessage: $msg, hookSpecificOutput: {hookEventName: "PostToolUse", additionalContext: $msg}}'
