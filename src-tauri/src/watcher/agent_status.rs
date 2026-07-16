//! Tails a small local JSONL file that agent integrations (Claude Code hooks
//! today) append to, announcing turn-completion / approval-needed events so
//! the overlay pet can react with a cute status badge. See
//! `docs/knowledge/agent-status-notifications.md` for the hook setup and why
//! this uses a local file bridge rather than parsing the token-usage JSONL
//! (task 0017).
//!
//! Only `provider`, `session_id`, `status`, and `ts` are ever read or
//! forwarded here - no message content, matching the privacy rule in
//! `docs/knowledge/token-tracking.md`.

use crate::storage_paths;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Completed,
    NeedsApproval,
    /// The permission prompt was resolved (approved and the tool ran, or
    /// denied) before the whole turn finished - fired by `PostToolUse`/
    /// `PermissionDenied` hooks so the "needs approval" badge doesn't have to
    /// wait for `Stop` (which only fires once the entire turn ends, possibly
    /// much later if more tool calls follow). Deliberately distinct from
    /// `Completed`: it should silently clear a pending badge, not replay the
    /// `Completed` celebration on every single tool call.
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentStatusEvent {
    pub provider: String,
    pub session_id: String,
    pub status: AgentStatus,
    pub ts: i64,
}

/// `<data_dir>/<watcher_namespace>/agent_status_events.jsonl` - the same base
/// directory the Claude Code token watcher uses for its own state file
/// (`claude_code_watcher_state.json`). Hook scripts under
/// `resources/claude-hooks/` append one JSON line per event here.
pub fn agent_status_events_path() -> PathBuf {
    storage_paths::watcher_data_file("agent_status_events.jsonl")
}

fn state_path_for(events_path: &Path) -> PathBuf {
    events_path.with_file_name("agent_status_watcher_state.json")
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct WatcherState {
    offset: u64,
}

impl WatcherState {
    fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string(self)?)
    }
}

/// Splits `buf` into complete (`\n`-terminated) lines, returning the decoded
/// lines and the byte length consumed. Mirrors
/// `watcher::claude_code::split_complete_lines` - kept as a separate copy
/// since these are two independently-evolving watchers over unrelated file
/// formats, not one shared subsystem.
fn split_complete_lines(buf: &[u8]) -> (Vec<String>, usize) {
    let mut lines = Vec::new();
    let mut consumed = 0;
    let mut start = 0;

    for (i, &b) in buf.iter().enumerate() {
        if b == b'\n' {
            let line = String::from_utf8_lossy(&buf[start..i]).into_owned();
            lines.push(line);
            start = i + 1;
            consumed = start;
        }
    }

    (lines, consumed)
}

/// Parses one line into an [`AgentStatusEvent`]; returns `None` for
/// blank/invalid lines rather than erroring, so a partially-written or
/// hand-edited line never crashes the watcher.
fn parse_status_line(line: &str) -> Option<AgentStatusEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

fn read_new_lines(file: &Path, offset: u64) -> std::io::Result<(Vec<String>, u64)> {
    let mut f = File::open(file)?;
    let len = f.metadata()?.len();
    if len < offset {
        // File was truncated/replaced - restart from 0 rather than seeking
        // past EOF.
        return read_new_lines(file, 0);
    }

    f.seek(SeekFrom::Start(offset))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;

    let (lines, consumed) = split_complete_lines(&buf);
    Ok((lines, offset + consumed as u64))
}

fn tail(file: &Path, state: &mut WatcherState, tx: &Sender<AgentStatusEvent>) {
    let (lines, new_offset) = match read_new_lines(file, state.offset) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "agent_status watcher: failed to read {}: {e}",
                file.display()
            );
            return;
        }
    };

    if lines.is_empty() {
        return;
    }

    state.offset = new_offset;
    for line in &lines {
        if let Some(event) = parse_status_line(line) {
            // Ignore send errors: the receiver may have been dropped (app
            // shutting down), which isn't this thread's problem to solve.
            let _ = tx.send(event);
        }
    }
}

/// Begins watching `agent_status_events_path()` in the background, creating
/// the file/parent directory if missing. Returns once the watcher thread is
/// spawned; it keeps running on its own thread.
pub fn start_agent_status_watcher(tx: Sender<AgentStatusEvent>) -> std::io::Result<()> {
    let events_path = agent_status_events_path();
    if let Some(parent) = events_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !events_path.exists() {
        fs::write(&events_path, b"")?;
    }

    std::thread::Builder::new()
        .name("agent-status-watcher".into())
        .spawn(move || run_watch_loop(events_path, tx))
        .map_err(std::io::Error::other)?;

    Ok(())
}

fn run_watch_loop(events_path: PathBuf, tx: Sender<AgentStatusEvent>) {
    use notify::{Event, RecursiveMode, Watcher};

    let state_path = state_path_for(&events_path);
    let mut state = WatcherState::load(&state_path);

    tail(&events_path, &mut state, &tx);
    if let Err(e) = state.save(&state_path) {
        eprintln!("agent_status watcher: failed to save state: {e}");
    }

    let Some(watch_dir) = events_path.parent().map(Path::to_path_buf) else {
        return;
    };

    let (raw_tx, raw_rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
    let mut watcher = match notify::recommended_watcher(move |res| {
        let _ = raw_tx.send(res);
    }) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("agent_status watcher: failed to create fs watcher: {e}");
            return;
        }
    };

    if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::NonRecursive) {
        eprintln!(
            "agent_status watcher: failed to watch {}: {e}",
            watch_dir.display()
        );
        return;
    }

    for res in raw_rx {
        let Ok(event) = res else { continue };
        if !event.paths.iter().any(|p| p == &events_path) {
            continue;
        }

        tail(&events_path, &mut state, &tx);
        if let Err(e) = state.save(&state_path) {
            eprintln!("agent_status watcher: failed to save state: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_completed_event() {
        let line = r#"{"provider":"claude_code","session_id":"abc123","status":"completed","ts":1783814400}"#;
        let event = parse_status_line(line).expect("should parse");
        assert_eq!(event.provider, "claude_code");
        assert_eq!(event.session_id, "abc123");
        assert_eq!(event.status, AgentStatus::Completed);
        assert_eq!(event.ts, 1_783_814_400);
    }

    #[test]
    fn parses_needs_approval_event() {
        let line =
            r#"{"provider":"claude_code","session_id":"abc123","status":"needs_approval","ts":1}"#;
        let event = parse_status_line(line).expect("should parse");
        assert_eq!(event.status, AgentStatus::NeedsApproval);
    }

    #[test]
    fn tolerates_blank_and_malformed_lines() {
        assert!(parse_status_line("").is_none());
        assert!(parse_status_line("   ").is_none());
        assert!(parse_status_line("{not json").is_none());
        assert!(parse_status_line(r#"{"status":"unknown_status"}"#).is_none());
    }

    #[test]
    fn split_complete_lines_holds_back_partial_trailing_line() {
        let buf = b"line one\nline two\npartial-no-newline-yet";
        let (lines, consumed) = split_complete_lines(buf);
        assert_eq!(lines, vec!["line one".to_string(), "line two".to_string()]);
        assert_eq!(consumed, "line one\nline two\n".len());
    }

    #[test]
    fn restart_does_not_reprocess_lines_before_the_persisted_offset() {
        let dir = std::env::temp_dir().join(format!(
            "tokengochi-agent-status-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("agent_status_events.jsonl");
        let state_path = dir.join("agent_status_watcher_state.json");

        fs::write(
            &file,
            "{\"provider\":\"claude_code\",\"session_id\":\"s1\",\"status\":\"completed\",\"ts\":1}\n",
        )
        .unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let mut state = WatcherState::load(&state_path);
        tail(&file, &mut state, &tx);
        state.save(&state_path).unwrap();
        let first_run: Vec<_> = rx.try_iter().collect();
        assert_eq!(first_run.len(), 1);

        // Simulate a restart: fresh state loaded from disk, nothing new
        // appended yet -> zero new events.
        let mut restarted_state = WatcherState::load(&state_path);
        let (tx2, rx2) = std::sync::mpsc::channel();
        tail(&file, &mut restarted_state, &tx2);
        let restart_events: Vec<_> = rx2.try_iter().collect();
        assert_eq!(
            restart_events.len(),
            0,
            "restart must not double-count already-seen lines"
        );

        // A genuinely new line appended after restart is picked up.
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap();
        writeln!(
            f,
            "{{\"provider\":\"claude_code\",\"session_id\":\"s1\",\"status\":\"needs_approval\",\"ts\":2}}"
        )
        .unwrap();

        let (tx3, rx3) = std::sync::mpsc::channel();
        tail(&file, &mut restarted_state, &tx3);
        let appended: Vec<_> = rx3.try_iter().collect();
        assert_eq!(appended.len(), 1);
        assert_eq!(appended[0].status, AgentStatus::NeedsApproval);

        let _ = fs::remove_dir_all(&dir);
    }
}
