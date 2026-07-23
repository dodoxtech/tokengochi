//! Shared JSON hook-settings plumbing used by both `claude_hooks.rs` (Claude
//! Code, `~/.claude/settings.json`) and `codex_hooks.rs` (Codex CLI,
//! `~/.codex/hooks.json`) - task 0027. Both tools use the identical
//! `{"hooks": {"<Event>": [{"hooks": [{"type":"command","command":"..."}]}]}}`
//! shape, so the atomic read/write and marker-based managed-entry detection
//! lives here once instead of drifting between two near-identical copies.
//! Provider-specific bits (which events to manage, the hook command line,
//! the settings file path) stay in each provider's own module.

use serde_json::{json, Map, Value};
use std::fs;
use std::path::Path;

/// Reads a JSON object from `path`, treating a missing or empty file as an
/// empty object rather than an error (first run before anything is installed).
pub fn read_settings(path: &Path) -> Result<Map<String, Value>, String> {
    if !path.exists() {
        return Ok(Map::new());
    }
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("could not read {}: {err}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str(&raw)
        .map_err(|err| format!("{} is not valid JSON: {err}", path.display()))?
    {
        Value::Object(map) => Ok(map),
        _ => Err(format!(
            "{} does not contain a JSON object at the top level",
            path.display()
        )),
    }
}

/// Writes `root` to `path`, backing up any existing file to `<path>.bak`
/// first and writing via a temp-file + rename so a crash mid-write never
/// leaves a truncated/corrupt settings file.
pub fn write_settings_atomically(path: &Path, root: &Map<String, Value>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("could not create {}: {err}", parent.display()))?;
    }
    if path.exists() {
        let backup_path = format!("{}.bak", path.display());
        fs::copy(path, &backup_path).map_err(|err| {
            format!(
                "could not back up {} to {backup_path}: {err}",
                path.display()
            )
        })?;
    }
    let pretty = serde_json::to_string_pretty(&Value::Object(root.clone()))
        .map_err(|err| format!("could not serialize {}: {err}", path.display()))?;
    let tmp_path = format!("{}.tmp", path.display());
    fs::write(&tmp_path, format!("{pretty}\n"))
        .map_err(|err| format!("could not write {tmp_path}: {err}"))?;
    fs::rename(&tmp_path, path)
        .map_err(|err| format!("could not finalize {}: {err}", path.display()))?;
    Ok(())
}

/// True if any entry under a hook event array (e.g. `hooks.Stop`, an array of
/// `{ "hooks": [ { "type", "command" } ] }` groups per the Claude Code /
/// Codex CLI hooks schema) already runs a command containing `marker`.
pub fn managed_commands<'a>(event_entries: &'a Value, marker: &str) -> Vec<&'a str> {
    event_entries
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .flat_map(|entry| {
                    entry
                        .get("hooks")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                })
                .filter_map(|hook| hook.get("command").and_then(Value::as_str))
                .filter(|command| command.contains(marker))
                .collect()
        })
        .unwrap_or_default()
}

pub fn has_only_desired_managed_entry(
    event_entries: &Value,
    desired_command: &str,
    marker: &str,
) -> bool {
    let commands = managed_commands(event_entries, marker);
    commands.len() == 1 && commands[0] == desired_command
}

fn hook_group_has_managed_entry(entry: &Value, marker: &str) -> bool {
    entry
        .get("hooks")
        .and_then(Value::as_array)
        .map(|inner| {
            inner.iter().any(|hook| {
                hook.get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|command| command.contains(marker))
            })
        })
        .unwrap_or(false)
}

pub fn shell_quote(raw: &str) -> String {
    format!("'{}'", raw.replace('\'', "'\\''"))
}

pub fn append_hook_entry(event_entries: &mut Value, command: String) -> Result<(), String> {
    let array = event_entries
        .as_array_mut()
        .ok_or("expected hooks.<event> to be an array")?;
    array.push(json!({ "hooks": [ { "type": "command", "command": command } ] }));
    Ok(())
}

/// Removes any managed entry (identified by `marker`) from an event array.
/// Returns whether anything was removed.
pub fn remove_managed_entries(event_entries: &mut Value, marker: &str) -> bool {
    let Some(array) = event_entries.as_array_mut() else {
        return false;
    };
    let before = array.len();
    array.retain(|entry| !hook_group_has_managed_entry(entry, marker));
    array.len() != before
}
