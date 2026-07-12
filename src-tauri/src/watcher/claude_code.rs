//! Tails `~/.claude/projects/**/*.jsonl` for per-message `usage` numbers.
//!
//! Only numeric usage fields and message ids are read - message content
//! never leaves [`parse_usage_line`] (see `docs/architecture.md` §Important
//! Constraints). The exact JSONL schema is undocumented and unverified
//! against a live Claude Code install in this environment (flagged as an
//! open question in `docs/knowledge/token-tracking.md`); the parser is
//! written defensively against that: unknown fields are ignored, missing
//! `usage` sub-fields default to 0, and malformed lines are skipped rather
//! than causing an error.

use super::{TokenEvent, TokenProvider};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

/// Tails `~/.claude/projects/**/*.jsonl` for per-message `usage` numbers.
pub struct ClaudeCodeProvider {
    /// Root directory to watch. Defaults to `~/.claude/projects`; overridable
    /// for tests.
    root: PathBuf,
    /// Where to persist byte offsets + seen message ids across restarts.
    state_path: PathBuf,
}

impl ClaudeCodeProvider {
    /// Builds a provider rooted at the real `~/.claude/projects` directory,
    /// persisting its offset/dedup state under the OS data dir convention
    /// (`dirs::data_dir()/tokengochi/claude_code_watcher_state.json`), or
    /// next to it if that's unavailable.
    pub fn new() -> Self {
        let root = dirs::home_dir()
            .map(|home| home.join(".claude").join("projects"))
            .unwrap_or_else(|| PathBuf::from(".claude/projects"));

        let state_path = dirs::data_dir()
            .map(|dir| {
                dir.join("tokengochi")
                    .join("claude_code_watcher_state.json")
            })
            .unwrap_or_else(|| PathBuf::from("claude_code_watcher_state.json"));

        Self::with_paths(root, state_path)
    }

    /// Builds a provider with an explicit root + state path - used by tests
    /// (and available for a future "point at a different account" setting).
    pub fn with_paths(root: PathBuf, state_path: PathBuf) -> Self {
        Self { root, state_path }
    }
}

impl Default for ClaudeCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenProvider for ClaudeCodeProvider {
    fn name(&self) -> &'static str {
        "claude_code"
    }

    fn detect(&self) -> bool {
        self.root.is_dir()
    }

    fn start(&self, tx: Sender<TokenEvent>) -> std::io::Result<()> {
        let root = self.root.clone();
        let state_path = self.state_path.clone();

        if !root.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "claude code project directory not found: {}",
                    root.display()
                ),
            ));
        }

        std::thread::Builder::new()
            .name("claude-code-watcher".into())
            .spawn(move || run_watch_loop(root, state_path, tx))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(())
    }
}

/// Persisted across restarts: per-file byte offsets (keyed by the file's
/// absolute path as a string) and message ids already emitted, so a restart
/// never double-counts. See acceptance criteria in
/// `docs/tasks/active/0003-claude-code-token-watcher.md`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct WatcherState {
    offsets: HashMap<String, u64>,
    seen_message_ids: HashSet<String>,
}

impl WatcherState {
    pub(crate) fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub(crate) fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(path, raw)
    }

    pub(crate) fn offset_for(&self, file: &Path) -> u64 {
        self.offsets.get(&file_key(file)).copied().unwrap_or(0)
    }

    pub(crate) fn set_offset(&mut self, file: &Path, offset: u64) {
        self.offsets.insert(file_key(file), offset);
    }

    /// Records `id` as seen; returns `true` if it was newly recorded (i.e.
    /// this event should be emitted), `false` if it was already seen (i.e.
    /// this is a duplicate and should be dropped).
    pub(crate) fn record_if_new(&mut self, id: &str) -> bool {
        self.seen_message_ids.insert(id.to_string())
    }
}

fn file_key(file: &Path) -> String {
    file.to_string_lossy().into_owned()
}

/// Splits `buf` into complete (`\n`-terminated) lines, returning the decoded
/// lines and the byte length consumed. Any trailing partial line (the writer
/// hasn't flushed a newline yet) is left unconsumed, so the caller's next
/// pass picks it up complete instead of parsing a half-written line.
///
/// Pure and allocation-light on purpose so it's cheaply unit-testable
/// without touching the filesystem.
pub(crate) fn split_complete_lines(buf: &[u8]) -> (Vec<String>, usize) {
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

/// One parsed usage observation, before it's turned into a [`TokenEvent`]
/// (which also needs a file/offset for the dedup fallback id).
#[derive(Debug, PartialEq)]
pub(crate) struct ParsedUsage {
    pub message_id: Option<String>,
    /// Model id from the line's `message.model`, if present.
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    /// Unix seconds, if the line had a parseable RFC3339 `timestamp`.
    pub timestamp: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RawLine {
    // `type` (e.g. "user"/"assistant"/"summary") is intentionally not kept -
    // whether a line matters is decided by "does it have `message.usage`",
    // not by trusting a `type` tag that's part of an undocumented schema.
    #[serde(default)]
    message: Option<RawMessage>,
    #[serde(default)]
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<RawUsage>,
}

#[derive(Debug, Default, Deserialize)]
struct RawUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
}

/// Parses a single JSONL line into [`ParsedUsage`], or `None` if the line
/// isn't an assistant message with usage (e.g. a user/summary line) or isn't
/// valid JSON at all. Never reads or returns message content - only the
/// fields named above.
pub(crate) fn parse_usage_line(line: &str) -> Option<ParsedUsage> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let raw: RawLine = serde_json::from_str(trimmed).ok()?;

    // Tolerate missing/unexpected `type`: what actually matters is whether
    // there's a `message.usage` object at all.
    let message = raw.message?;
    let usage = message.usage?;

    // cache_creation_input_tokens is counted at input weight (see
    // docs/knowledge/token-tracking.md Open Questions), so fold it into
    // input_tokens here rather than carrying a 4th bucket through the whole
    // pipeline.
    let input_tokens = usage
        .input_tokens
        .saturating_add(usage.cache_creation_input_tokens);

    let timestamp = raw
        .timestamp
        .as_deref()
        .and_then(parse_rfc3339_to_unix_secs);

    Some(ParsedUsage {
        message_id: message.id,
        model: message.model,
        input_tokens,
        output_tokens: usage.output_tokens,
        cache_read_tokens: usage.cache_read_input_tokens,
        timestamp,
    })
}

/// Minimal RFC3339 -> unix-seconds parser, no chrono dependency. Handles the
/// common `YYYY-MM-DDTHH:MM:SS(.fraction)?Z` shape; returns `None` for
/// anything else rather than guessing (the caller falls back to "now").
fn parse_rfc3339_to_unix_secs(s: &str) -> Option<i64> {
    let s = s.strip_suffix('Z')?;
    let (date, time) = s.split_once('T')?;

    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: i64 = date_parts.next()?.parse().ok()?;
    let day: i64 = date_parts.next()?.parse().ok()?;

    // Drop sub-second precision if present.
    let time = time.split('.').next().unwrap_or(time);
    let mut time_parts = time.split(':');
    let hour: i64 = time_parts.next()?.parse().ok()?;
    let minute: i64 = time_parts.next()?.parse().ok()?;
    let second: i64 = time_parts.next()?.parse().ok()?;

    // Days-since-epoch via the standard civil_from_days algorithm (Howard
    // Hinnant's public-domain date algorithms), avoiding a chrono
    // dependency for one small conversion.
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3600 + minute * 60 + second)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Reads any newly-appended, complete lines from `file` since `offset`,
/// returning them along with the new offset to persist. Does not read past
/// the last complete line (see [`split_complete_lines`]).
fn read_new_lines(file: &Path, offset: u64) -> std::io::Result<(Vec<String>, u64)> {
    let mut f = File::open(file)?;
    let len = f.metadata()?.len();
    if len < offset {
        // File was truncated/rotated (e.g. log rotation) - restart from 0
        // rather than seeking past EOF.
        return read_new_lines(file, 0);
    }

    f.seek(SeekFrom::Start(offset))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;

    let (lines, consumed) = split_complete_lines(&buf);
    Ok((lines, offset + consumed as u64))
}

/// Processes newly-read lines against `state`, returning the [`TokenEvent`]s
/// that are genuinely new (not seen before by message id) and updating
/// `state`'s seen-id set in place. `file` and `offset_before` are only used
/// to synthesize a fallback id for lines with no message id of their own.
fn process_lines(
    file: &Path,
    offset_before: u64,
    lines: &[String],
    state: &mut WatcherState,
) -> Vec<TokenEvent> {
    let mut events = Vec::new();
    let mut running_offset = offset_before;

    for line in lines {
        let line_len = line.len() as u64 + 1; // +1 for the '\n' split away
        let this_offset = running_offset;
        running_offset += line_len;

        let Some(parsed) = parse_usage_line(line) else {
            continue;
        };

        let message_id = parsed
            .message_id
            .unwrap_or_else(|| format!("{}:{}", file.display(), this_offset));

        if !state.record_if_new(&message_id) {
            continue; // already counted, e.g. re-read after a crash mid-save
        }

        events.push(TokenEvent {
            provider: "claude_code".to_string(),
            message_id,
            model: parsed.model.unwrap_or_default(),
            input_tokens: parsed.input_tokens,
            output_tokens: parsed.output_tokens,
            cache_read_tokens: parsed.cache_read_tokens,
            timestamp: parsed.timestamp.unwrap_or_else(now_unix_secs),
        });
    }

    events
}

fn now_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Recursively lists `*.jsonl` files under `root`.
fn list_jsonl_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(list_jsonl_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "jsonl") {
            out.push(path);
        }
    }
    out
}

/// Tails one file incrementally against `state`, sending any new events to
/// `tx` and persisting the updated state. Shared by the initial scan and the
/// `notify` event loop so both paths behave identically.
fn tail_file(file: &Path, state: &mut WatcherState, tx: &Sender<TokenEvent>) {
    let offset_before = state.offset_for(file);
    let (lines, new_offset) = match read_new_lines(file, offset_before) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "claude_code watcher: failed to read {}: {e}",
                file.display()
            );
            return;
        }
    };

    if lines.is_empty() {
        return;
    }

    let events = process_lines(file, offset_before, &lines, state);
    state.set_offset(file, new_offset);

    for event in events {
        // Ignore send errors: the receiver may have been dropped (app
        // shutting down), which isn't this thread's problem to solve.
        let _ = tx.send(event);
    }
}

/// The actual background loop: initial scan of every existing `*.jsonl`
/// file, then `notify`-driven incremental tailing as files change.
fn run_watch_loop(root: PathBuf, state_path: PathBuf, tx: Sender<TokenEvent>) {
    use notify::{Event, RecursiveMode, Watcher};

    let mut state = WatcherState::load(&state_path);

    for file in list_jsonl_files(&root) {
        tail_file(&file, &mut state, &tx);
    }
    if let Err(e) = state.save(&state_path) {
        eprintln!("claude_code watcher: failed to save state: {e}");
    }

    let (raw_tx, raw_rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
    let mut watcher = match notify::recommended_watcher(move |res| {
        let _ = raw_tx.send(res);
    }) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("claude_code watcher: failed to create fs watcher: {e}");
            return;
        }
    };

    if let Err(e) = watcher.watch(&root, RecursiveMode::Recursive) {
        eprintln!(
            "claude_code watcher: failed to watch {}: {e}",
            root.display()
        );
        return;
    }

    for res in raw_rx {
        let Ok(event) = res else { continue };
        let touched_jsonl: Vec<PathBuf> = event
            .paths
            .into_iter()
            .filter(|p| p.extension().is_some_and(|ext| ext == "jsonl"))
            .collect();

        if touched_jsonl.is_empty() {
            continue;
        }

        for file in touched_jsonl {
            if file.is_file() {
                tail_file(&file, &mut state, &tx);
            }
        }
        if let Err(e) = state.save(&state_path) {
            eprintln!("claude_code watcher: failed to save state: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_SESSION: &str = include_str!("fixtures/claude_code/valid_session.jsonl");
    const MALFORMED: &str = include_str!("fixtures/claude_code/malformed.jsonl");
    const UNKNOWN_FIELDS: &str = include_str!("fixtures/claude_code/unknown_fields.jsonl");

    #[test]
    fn parses_valid_assistant_usage_line() {
        let line = VALID_SESSION.lines().nth(1).unwrap(); // first assistant line
        let parsed = parse_usage_line(line).expect("should parse");
        assert_eq!(parsed.message_id.as_deref(), Some("msg_001"));
        assert_eq!(parsed.model.as_deref(), Some("claude-opus-4-8"));
        assert_eq!(parsed.input_tokens, 120); // 100 input + 20 cache_creation
        assert_eq!(parsed.output_tokens, 45);
        assert_eq!(parsed.cache_read_tokens, 5);
    }

    #[test]
    fn ignores_non_assistant_lines() {
        // First line in the fixture is a user message with no usage.
        let line = VALID_SESSION.lines().next().unwrap();
        assert!(parse_usage_line(line).is_none());
    }

    #[test]
    fn tolerates_malformed_and_empty_lines() {
        let mut ok_count = 0;
        for line in MALFORMED.lines() {
            // Must never panic; either parses or returns None.
            if parse_usage_line(line).is_some() {
                ok_count += 1;
            }
        }
        // Exactly one valid assistant-usage line is mixed into the fixture.
        assert_eq!(ok_count, 1);
    }

    #[test]
    fn tolerates_unknown_fields_and_missing_usage_subfields() {
        let line = UNKNOWN_FIELDS.lines().next().unwrap();
        let parsed = parse_usage_line(line).expect("should still parse");
        // cache_read_input_tokens is absent from the fixture -> defaults to 0.
        assert_eq!(parsed.cache_read_tokens, 0);
        assert_eq!(parsed.input_tokens, 50);
        // `model` is absent too -> None, which becomes "" in the TokenEvent
        // and falls back to `model_weight_default` in the economy engine.
        assert_eq!(parsed.model, None);
    }

    #[test]
    fn split_complete_lines_holds_back_partial_trailing_line() {
        let buf = b"line one\nline two\npartial-no-newline-yet";
        let (lines, consumed) = split_complete_lines(buf);
        assert_eq!(lines, vec!["line one".to_string(), "line two".to_string()]);
        assert_eq!(consumed, "line one\nline two\n".len());
    }

    #[test]
    fn split_complete_lines_empty_buffer() {
        let (lines, consumed) = split_complete_lines(b"");
        assert!(lines.is_empty());
        assert_eq!(consumed, 0);
    }

    #[test]
    fn dedup_by_message_id_across_two_process_calls() {
        let mut state = WatcherState::default();
        let file = PathBuf::from("/tmp/does-not-need-to-exist.jsonl");
        let lines = vec![VALID_SESSION.lines().nth(1).unwrap().to_string()];

        let first_pass = process_lines(&file, 0, &lines, &mut state);
        assert_eq!(first_pass.len(), 1);

        // Same line processed again (e.g. a bug re-reads it) - must be
        // dropped as a duplicate.
        let second_pass = process_lines(&file, 0, &lines, &mut state);
        assert_eq!(second_pass.len(), 0);
    }

    #[test]
    fn restart_does_not_reprocess_lines_before_the_persisted_offset() {
        let dir = std::env::temp_dir().join(format!(
            "tokengochi-watcher-test-{}-{}",
            std::process::id(),
            now_unix_secs()
        ));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("session.jsonl");
        let state_path = dir.join("state.json");

        // Write the first "session" of lines and tail it once.
        fs::write(&file, VALID_SESSION).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut state = WatcherState::load(&state_path);
        tail_file(&file, &mut state, &tx);
        state.save(&state_path).unwrap();
        let first_run_events: Vec<_> = rx.try_iter().collect();
        assert!(!first_run_events.is_empty());

        // Simulate a restart: fresh WatcherState loaded from disk, same
        // file, nothing new appended yet -> zero new events.
        let mut restarted_state = WatcherState::load(&state_path);
        let (tx2, rx2) = std::sync::mpsc::channel();
        tail_file(&file, &mut restarted_state, &tx2);
        let restart_events: Vec<_> = rx2.try_iter().collect();
        assert_eq!(
            restart_events.len(),
            0,
            "restart must not double-count already-seen lines"
        );

        // Now append a genuinely new line and confirm it - and only it -
        // gets picked up.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap();
        use std::io::Write;
        writeln!(
            f,
            r#"{{"type":"assistant","message":{{"id":"msg_new","usage":{{"input_tokens":9,"output_tokens":1,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}},"timestamp":"2026-07-12T00:00:00Z"}}"#
        )
        .unwrap();

        let (tx3, rx3) = std::sync::mpsc::channel();
        tail_file(&file, &mut restarted_state, &tx3);
        let appended_events: Vec<_> = rx3.try_iter().collect();
        assert_eq!(appended_events.len(), 1);
        assert_eq!(appended_events[0].message_id, "msg_new");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rfc3339_parses_known_timestamp() {
        // 2026-07-12T00:00:00Z - sanity check against a known epoch value.
        let secs = parse_rfc3339_to_unix_secs("2026-07-12T00:00:00Z").unwrap();
        assert_eq!(secs, 1_783_814_400);
    }
}
