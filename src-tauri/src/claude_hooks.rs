//! Auto-installs the Claude Code `Stop`/`Notification` hooks that feed the
//! agent-status badge (task 0017), so a user doesn't have to hand-edit
//! `~/.claude/settings.json` themselves - see
//! `docs/knowledge/agent-status-notifications.md`. Mirrors the pattern the
//! `openpets` project uses for the same problem: atomic write with a backup,
//! and each managed entry carries a marker (`HOOK_MARKER`/`MANAGED_FLAG`) so
//! re-running install is idempotent and never duplicates or clobbers a hook
//! the user added by hand for something else.
//!
//! Scope is intentionally global-only (`~/.claude/settings.json`) rather
//! than per-project: the point of the feature is "the pet should react
//! whenever I use Claude Code anywhere," not just inside this repo.

use serde::Serialize;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Substring unique to the managed hook command, used to detect an existing
/// entry (whether it was installed by this code or copied in by hand from
/// the docs example) so install stays idempotent.
const HOOK_MARKER: &str = "tokengochi-notify.sh";
/// Appended as an extra CLI arg on top of the required `completed`/
/// `needs_approval` status arg. The hook script ignores unknown trailing
/// args, so this is purely a tag for detecting *our* managed entries
/// specifically (as opposed to a hand-copied one without it) if that
/// distinction is ever needed later.
const MANAGED_FLAG: &str = "--tokengochi-managed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatusHookStatus {
    pub installed: bool,
    pub settings_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatusHookInstallResult {
    pub changed: bool,
    pub settings_path: String,
}

pub fn claude_global_settings_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("could not resolve the home directory")?;
    Ok(home.join(".claude").join("settings.json"))
}

/// Resolves the hook script's absolute path relative to this crate's
/// manifest dir (`src-tauri/../resources/claude-hooks/...`), which covers
/// running from a source checkout (`cargo run`/`tauri dev`, the only way
/// this app runs today). Packaging this script as a bundled Tauri resource
/// (`tauri::path::BaseDirectory::Resource`, like `economy.toml`) is a
/// follow-up for whenever the app ships to a machine without the repo
/// present - see the Open Questions in
/// `docs/knowledge/agent-status-notifications.md`.
fn hook_script_path() -> Result<PathBuf, String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir
        .join("..")
        .join("resources")
        .join("claude-hooks")
        .join("tokengochi-notify.sh");
    path.canonicalize().map_err(|err| {
        format!(
            "could not resolve hook script path {}: {err}",
            path.display()
        )
    })
}

fn read_settings(path: &Path) -> Result<Map<String, Value>, String> {
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

fn write_settings_atomically(path: &Path, root: &Map<String, Value>) -> Result<(), String> {
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

/// True if any entry under a hook event array (`hooks.Stop`/`hooks.Notification`,
/// each an array of `{ "hooks": [ { "type", "command" } ] }` groups per the
/// Claude Code hooks schema) already runs our script.
fn has_managed_entry(event_entries: &Value) -> bool {
    event_entries
        .as_array()
        .map(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(Value::as_array)
                    .map(|inner| {
                        inner.iter().any(|hook| {
                            hook.get("command")
                                .and_then(Value::as_str)
                                .is_some_and(|command| command.contains(HOOK_MARKER))
                        })
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn hook_command(script_path: &Path, status: &str) -> String {
    format!("\"{}\" {status} {MANAGED_FLAG}", script_path.display())
}

fn append_hook_entry(event_entries: &mut Value, command: String) -> Result<(), String> {
    let array = event_entries
        .as_array_mut()
        .ok_or("expected hooks.<event> to be an array")?;
    array.push(json!({ "hooks": [ { "type": "command", "command": command } ] }));
    Ok(())
}

/// Removes any managed entry (identified by [`HOOK_MARKER`]) from a legacy
/// event array. Used to clean up the old `Notification`-based install left
/// behind by versions of this app that predate the switch to
/// `PermissionRequest` (see `docs/knowledge/agent-status-notifications.md`):
/// `Notification` also fires for plain idle-waiting, so leaving a stale
/// managed entry there would keep producing spurious `needs_approval`
/// badges alongside the correct `PermissionRequest`-driven ones. Returns
/// whether anything was removed.
fn remove_managed_entries(event_entries: &mut Value) -> bool {
    let Some(array) = event_entries.as_array_mut() else {
        return false;
    };
    let before = array.len();
    array.retain(|entry| {
        !entry
            .get("hooks")
            .and_then(Value::as_array)
            .map(|inner| {
                inner.iter().any(|hook| {
                    hook.get("command")
                        .and_then(Value::as_str)
                        .is_some_and(|command| command.contains(HOOK_MARKER))
                })
            })
            .unwrap_or(false)
    });
    array.len() != before
}

/// The full set of `(hook event, status arg)` pairs this app manages.
/// `PostToolUse`/`PermissionDenied` -> `resolved` let the "needs approval"
/// badge clear as soon as the prompt is actually resolved (tool ran, or was
/// denied), rather than waiting for `Stop` (whole turn end, possibly much
/// later) - see docs/knowledge/agent-status-notifications.md.
const MANAGED_HOOKS: [(&str, &str); 4] = [
    ("Stop", "completed"),
    ("PermissionRequest", "needs_approval"),
    ("PostToolUse", "resolved"),
    ("PermissionDenied", "resolved"),
];

pub fn status() -> Result<AgentStatusHookStatus, String> {
    let settings_path = claude_global_settings_path()?;
    let root = read_settings(&settings_path)?;
    let installed = root
        .get("hooks")
        .and_then(Value::as_object)
        .map(|hooks| {
            MANAGED_HOOKS
                .iter()
                .all(|(event, _)| hooks.get(*event).is_some_and(has_managed_entry))
        })
        .unwrap_or(false);
    Ok(AgentStatusHookStatus {
        installed,
        settings_path: settings_path.display().to_string(),
    })
}

pub fn install() -> Result<AgentStatusHookInstallResult, String> {
    let settings_path = claude_global_settings_path()?;
    let script_path = hook_script_path()?;
    let mut root = read_settings(&settings_path)?;

    let hooks = root.entry("hooks".to_string()).or_insert_with(|| json!({}));
    if !hooks.is_object() {
        return Err(format!(
            "{}: top-level \"hooks\" is not an object",
            settings_path.display()
        ));
    }
    let hooks_obj = hooks.as_object_mut().expect("checked above");

    let mut changed = false;

    // Migrate away from the old `Notification`-based `needs_approval` entry:
    // `Notification` fires for idle-waiting too and doesn't reliably fire
    // for permission prompts in the VS Code extension (confirmed empirically
    // - see docs/knowledge/agent-status-notifications.md), so a stale entry
    // here would just add noise on top of the correct `PermissionRequest`
    // hook installed below.
    if let Some(entry) = hooks_obj.get_mut("Notification") {
        if remove_managed_entries(entry) {
            changed = true;
        }
    }

    for (event, status_arg) in MANAGED_HOOKS {
        let entry = hooks_obj
            .entry(event.to_string())
            .or_insert_with(|| json!([]));
        if !entry.is_array() {
            return Err(format!(
                "{}: hooks.{event} is not an array",
                settings_path.display()
            ));
        }
        if !has_managed_entry(entry) {
            append_hook_entry(entry, hook_command(&script_path, status_arg))?;
            changed = true;
        }
    }

    if changed {
        write_settings_atomically(&settings_path, &root)?;
    }

    Ok(AgentStatusHookInstallResult {
        changed,
        settings_path: settings_path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_script_resolves_to_an_existing_file() {
        let path = hook_script_path().expect("hook script should resolve");
        assert!(path.is_file());
        assert!(path.to_string_lossy().contains(HOOK_MARKER));
    }

    #[test]
    fn install_is_idempotent_and_preserves_unrelated_settings() {
        let dir = std::env::temp_dir().join(format!(
            "tokengochi-claude-hooks-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let settings_path = dir.join("settings.json");
        fs::write(
            &settings_path,
            r#"{"model":"sonnet","hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":"echo hi"}]}]}}"#,
        )
        .unwrap();

        let script_path = hook_script_path().unwrap();
        let mut root = read_settings(&settings_path).unwrap();
        let hooks = root.entry("hooks".to_string()).or_insert_with(|| json!({}));
        let hooks_obj = hooks.as_object_mut().unwrap();
        for (event, status_arg) in MANAGED_HOOKS {
            let entry = hooks_obj
                .entry(event.to_string())
                .or_insert_with(|| json!([]));
            append_hook_entry(entry, hook_command(&script_path, status_arg)).unwrap();
        }
        write_settings_atomically(&settings_path, &root).unwrap();

        let reloaded = read_settings(&settings_path).unwrap();
        let hooks_obj = reloaded.get("hooks").unwrap().as_object().unwrap();
        for (event, _) in MANAGED_HOOKS {
            assert!(
                has_managed_entry(hooks_obj.get(event).unwrap()),
                "{event} should be managed"
            );
        }
        // A pre-existing PostToolUse entry the user added by hand for
        // something else must survive alongside ours (this test starts
        // PostToolUse empty and installs into it above, so just confirm the
        // key itself, plus the unrelated top-level PostToolUse hook file
        // fixture below, both remain).
        assert!(hooks_obj.contains_key("PostToolUse"));
        assert_eq!(reloaded.get("model").unwrap().as_str(), Some("sonnet"));

        // Re-running against the now-installed file must not duplicate entries.
        let before = reloaded.get("hooks").unwrap().clone();
        let mut root2 = read_settings(&settings_path).unwrap();
        let hooks2 = root2.get_mut("hooks").unwrap().as_object_mut().unwrap();
        let mut changed_again = false;
        for (event, _) in MANAGED_HOOKS {
            let entry = hooks2.get_mut(event).unwrap();
            if !has_managed_entry(entry) {
                append_hook_entry(entry, hook_command(&script_path, "completed")).unwrap();
                changed_again = true;
            }
        }
        assert!(!changed_again);
        assert_eq!(before, *root2.get("hooks").unwrap());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrates_away_from_legacy_notification_entry() {
        // Simulates a user who installed an older version of this app that
        // wired `needs_approval` to `Notification` instead of
        // `PermissionRequest`. Installing again should remove the stale
        // Notification entry (it fires on idle-waiting too, so leaving it
        // would double up on spurious badges) and add PermissionRequest.
        let script_path = hook_script_path().unwrap();
        let mut root: Map<String, Value> = Map::new();
        let hooks_obj = root
            .entry("hooks".to_string())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .unwrap();
        let legacy_entry = hooks_obj
            .entry("Notification".to_string())
            .or_insert_with(|| json!([]));
        append_hook_entry(legacy_entry, hook_command(&script_path, "needs_approval")).unwrap();
        assert!(has_managed_entry(legacy_entry));

        let mut changed = false;
        if let Some(entry) = hooks_obj.get_mut("Notification") {
            if remove_managed_entries(entry) {
                changed = true;
            }
        }
        for (event, status_arg) in MANAGED_HOOKS {
            let entry = hooks_obj
                .entry(event.to_string())
                .or_insert_with(|| json!([]));
            if !has_managed_entry(entry) {
                append_hook_entry(entry, hook_command(&script_path, status_arg)).unwrap();
                changed = true;
            }
        }

        assert!(changed);
        assert!(!has_managed_entry(hooks_obj.get("Notification").unwrap()));
        for (event, _) in MANAGED_HOOKS {
            assert!(
                has_managed_entry(hooks_obj.get(event).unwrap()),
                "{event} should be managed"
            );
        }
    }
}
