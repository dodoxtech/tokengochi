//! Auto-installs the Codex CLI `Stop`/`PermissionRequest`/`PostToolUse` hooks
//! that feed the agent-status badge, mirroring what `claude_hooks.rs` does
//! for Claude Code (task 0017, extended to Codex in task 0027) - see
//! `docs/knowledge/agent-status-notifications.md`.
//!
//! Codex CLI's hook system (added 2026) uses the same event names and the
//! same `{"hooks": {"<Event>": [{"hooks": [{"type","command"}]}]}}` shape as
//! Claude Code, so the low-level read/write/marker logic is shared via
//! `hook_settings.rs`; only the settings file location, the managed event
//! list, and the `--provider` tag passed to the notify script differ here.
//!
//! Codex hooks are configured in `~/.codex/hooks.json` (a dedicated file,
//! unlike Claude Code's single `settings.json`), so unlike `claude_hooks.rs`
//! there is no risk of clobbering unrelated top-level settings - but the
//! same defensive read-modify-write-with-backup is used regardless, since a
//! user may still hand-edit this file to add their own hooks.
//!
//! Scope is intentionally global-only (`~/.codex/hooks.json`), matching the
//! Claude Code install: "the pet should react whenever I use Codex anywhere."

use crate::hook_settings;
use crate::storage_paths;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::Manager;

/// Substring unique to the managed hook command, used to detect an existing
/// entry (whether installed by this code or copied in by hand from the docs
/// example) so install stays idempotent. Same script file as Claude Code's
/// hook - see the module doc comment.
const HOOK_MARKER: &str = "tokengochi-notify.sh";
/// Appended as an extra CLI arg, purely a tag for detecting *our* managed
/// entries specifically (as opposed to a hand-copied one without it).
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

pub fn codex_global_hooks_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("could not resolve the home directory")?;
    Ok(home.join(".codex").join("hooks.json"))
}

/// Resolves the hook script's absolute path, preferring the copy bundled as a
/// Tauri resource, same as `claude_hooks::hook_script_path()` - both
/// providers reuse the identical script file (see the module doc comment).
fn hook_script_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Ok(path) = app.path().resolve(
        "claude-hooks/tokengochi-notify.sh",
        tauri::path::BaseDirectory::Resource,
    ) {
        if path.is_file() {
            return Ok(path);
        }
    }
    manifest_hook_script_path()
}

/// Source-checkout location of the hook script, used as a dev/test fallback
/// when no bundled resource is available.
fn manifest_hook_script_path() -> Result<PathBuf, String> {
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

fn hook_command(script_path: &Path, status: &str) -> String {
    let events_path = storage_paths::watcher_data_file("agent_status_events.jsonl");
    let data_dir = events_path
        .parent()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string());
    // Invoke through `bash` rather than executing the script directly: a
    // resource copied into a packaged app bundle is not guaranteed to keep
    // its executable bit, so `'/path/script.sh' ...` could fail with EACCES.
    format!(
        "TOKENGOCHI_DATA_DIR={} bash {} {status} --provider codex {MANAGED_FLAG}",
        hook_settings::shell_quote(&data_dir),
        hook_settings::shell_quote(&script_path.to_string_lossy())
    )
}

/// The full set of `(hook event, status arg)` pairs this app manages for
/// Codex CLI. No `PermissionDenied` equivalent exists in the Codex hooks
/// reference, so `PostToolUse` alone carries `resolved` - see
/// docs/knowledge/agent-status-notifications.md.
const MANAGED_HOOKS: [(&str, &str); 3] = [
    ("Stop", "completed"),
    ("PermissionRequest", "needs_approval"),
    ("PostToolUse", "resolved"),
];

pub fn status(app: &tauri::AppHandle) -> Result<AgentStatusHookStatus, String> {
    let hooks_path = codex_global_hooks_path()?;
    let script_path = hook_script_path(app)?;
    let root = hook_settings::read_settings(&hooks_path)?;
    let installed = root
        .get("hooks")
        .and_then(Value::as_object)
        .map(|hooks| {
            MANAGED_HOOKS.iter().all(|(event, status_arg)| {
                let desired_command = hook_command(&script_path, status_arg);
                hooks.get(*event).is_some_and(|entry| {
                    hook_settings::has_only_desired_managed_entry(
                        entry,
                        &desired_command,
                        HOOK_MARKER,
                    )
                })
            })
        })
        .unwrap_or(false);
    Ok(AgentStatusHookStatus {
        installed,
        settings_path: hooks_path.display().to_string(),
    })
}

pub fn install(app: &tauri::AppHandle) -> Result<AgentStatusHookInstallResult, String> {
    let hooks_path = codex_global_hooks_path()?;
    let script_path = hook_script_path(app)?;
    let mut root = hook_settings::read_settings(&hooks_path)?;

    let hooks = root.entry("hooks".to_string()).or_insert_with(|| json!({}));
    if !hooks.is_object() {
        return Err(format!(
            "{}: top-level \"hooks\" is not an object",
            hooks_path.display()
        ));
    }
    let hooks_obj = hooks.as_object_mut().expect("checked above");

    let mut changed = false;

    for (event, status_arg) in MANAGED_HOOKS {
        let entry = hooks_obj
            .entry(event.to_string())
            .or_insert_with(|| json!([]));
        if !entry.is_array() {
            return Err(format!(
                "{}: hooks.{event} is not an array",
                hooks_path.display()
            ));
        }
        let desired_command = hook_command(&script_path, status_arg);
        if !hook_settings::has_only_desired_managed_entry(entry, &desired_command, HOOK_MARKER) {
            hook_settings::remove_managed_entries(entry, HOOK_MARKER);
            hook_settings::append_hook_entry(entry, desired_command)?;
            changed = true;
        }
    }

    if changed {
        hook_settings::write_settings_atomically(&hooks_path, &root)?;
    }

    Ok(AgentStatusHookInstallResult {
        changed,
        settings_path: hooks_path.display().to_string(),
    })
}

/// Removes every managed entry (identified by [`HOOK_MARKER`]) that this app
/// added, and prunes any hook-event array we leave empty so we don't litter
/// `~/.codex/hooks.json` with dead keys. Hand-added entries are left
/// untouched. Idempotent: calling it when nothing is installed reports
/// `changed: false` and writes nothing.
pub fn uninstall(_app: &tauri::AppHandle) -> Result<AgentStatusHookInstallResult, String> {
    let hooks_path = codex_global_hooks_path()?;
    let mut root = hook_settings::read_settings(&hooks_path)?;

    let mut changed = false;

    if let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) {
        for (event, _) in MANAGED_HOOKS {
            let Some(entry) = hooks.get_mut(event) else {
                continue;
            };
            if hook_settings::remove_managed_entries(entry, HOOK_MARKER) {
                changed = true;
            }
            if entry.as_array().is_some_and(|array| array.is_empty()) {
                hooks.remove(event);
            }
        }
        if hooks.is_empty() {
            root.remove("hooks");
        }
    }

    if changed {
        hook_settings::write_settings_atomically(&hooks_path, &root)?;
    }

    Ok(AgentStatusHookInstallResult {
        changed,
        settings_path: hooks_path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;
    use std::fs;

    #[test]
    fn hook_script_resolves_to_an_existing_file() {
        let path = manifest_hook_script_path().expect("hook script should resolve");
        assert!(path.is_file());
        assert!(path.to_string_lossy().contains(HOOK_MARKER));
    }

    #[test]
    fn install_is_idempotent_and_preserves_unrelated_settings() {
        let dir = std::env::temp_dir().join(format!(
            "tokengochi-codex-hooks-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let hooks_path = dir.join("hooks.json");
        fs::write(
            &hooks_path,
            r#"{"hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":"echo hi"}]}]}}"#,
        )
        .unwrap();

        let script_path = manifest_hook_script_path().unwrap();
        let mut root = hook_settings::read_settings(&hooks_path).unwrap();
        let hooks = root.entry("hooks".to_string()).or_insert_with(|| json!({}));
        let hooks_obj = hooks.as_object_mut().unwrap();
        for (event, status_arg) in MANAGED_HOOKS {
            let entry = hooks_obj
                .entry(event.to_string())
                .or_insert_with(|| json!([]));
            hook_settings::append_hook_entry(entry, hook_command(&script_path, status_arg))
                .unwrap();
        }
        hook_settings::write_settings_atomically(&hooks_path, &root).unwrap();

        let reloaded = hook_settings::read_settings(&hooks_path).unwrap();
        let hooks_obj = reloaded.get("hooks").unwrap().as_object().unwrap();
        for (event, status_arg) in MANAGED_HOOKS {
            let desired_command = hook_command(&script_path, status_arg);
            assert!(
                hook_settings::has_only_desired_managed_entry(
                    hooks_obj.get(event).unwrap(),
                    &desired_command,
                    HOOK_MARKER
                ),
                "{event} should be managed"
            );
        }
        // The pre-existing hand-added PostToolUse entry must survive
        // alongside ours.
        let post_tool_use = hooks_obj.get("PostToolUse").unwrap().as_array().unwrap();
        assert_eq!(post_tool_use.len(), 2);

        // Re-running against the now-installed file must not duplicate
        // entries beyond the hand-added one.
        let before = reloaded.get("hooks").unwrap().clone();
        let mut root2 = hook_settings::read_settings(&hooks_path).unwrap();
        let hooks2 = root2.get_mut("hooks").unwrap().as_object_mut().unwrap();
        let mut changed_again = false;
        for (event, status_arg) in MANAGED_HOOKS {
            let entry = hooks2.get_mut(event).unwrap();
            let desired_command = hook_command(&script_path, status_arg);
            if !hook_settings::has_only_desired_managed_entry(entry, &desired_command, HOOK_MARKER)
            {
                hook_settings::remove_managed_entries(entry, HOOK_MARKER);
                hook_settings::append_hook_entry(entry, desired_command).unwrap();
                changed_again = true;
            }
        }
        assert!(!changed_again);
        assert_eq!(before, *root2.get("hooks").unwrap());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn uninstall_removes_managed_entries_and_prunes_empty_keys() {
        let script_path = manifest_hook_script_path().unwrap();
        let mut root: Map<String, Value> = Map::new();
        let hooks_obj = root
            .entry("hooks".to_string())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .unwrap();
        // A hand-added Stop hook that must survive uninstall.
        let stop = hooks_obj
            .entry("Stop".to_string())
            .or_insert_with(|| json!([]));
        hook_settings::append_hook_entry(stop, "echo custom".to_string()).unwrap();
        for (event, status_arg) in MANAGED_HOOKS {
            let entry = hooks_obj
                .entry(event.to_string())
                .or_insert_with(|| json!([]));
            hook_settings::append_hook_entry(entry, hook_command(&script_path, status_arg))
                .unwrap();
        }

        for (event, _) in MANAGED_HOOKS {
            let Some(entry) = hooks_obj.get_mut(event) else {
                continue;
            };
            hook_settings::remove_managed_entries(entry, HOOK_MARKER);
            if entry.as_array().is_some_and(|array| array.is_empty()) {
                hooks_obj.remove(event);
            }
        }

        assert!(!hooks_obj.contains_key("PermissionRequest"));
        assert!(!hooks_obj.contains_key("PostToolUse"));
        let stop = hooks_obj.get("Stop").unwrap();
        assert!(hook_settings::managed_commands(stop, HOOK_MARKER).is_empty());
        assert_eq!(stop.as_array().unwrap().len(), 1);
    }

    #[test]
    fn desired_command_tags_codex_provider() {
        let script_path = manifest_hook_script_path().unwrap();
        let command = hook_command(&script_path, "needs_approval");
        assert!(command.contains("--provider codex"));
        assert!(command.contains(HOOK_MARKER));
    }
}
