//! Tails `~/.codex/sessions/**/*.jsonl` token_count events.
//!
//! Verified against local Codex Desktop/CLI logs on 2026-07-12 (usage) and
//! 2026-07-22 (model/dedup): usage arrives as `type:"event_msg"` with
//! `payload.type:"token_count"` and `payload.info.last_token_usage`. We read
//! only usage counters, timestamps, and rollout/session ids; message content
//! is ignored.
//!
//! ## Model resolution (2026-07-22)
//! `token_count` records carry **no** `model` field (0/122 lines in a sampled
//! rollout), and `session_meta` only has `model_provider`, not the model id.
//! The real model lives in `turn_context` records as `payload.model` (e.g.
//! `"gpt-5.4"`), which precede the `token_count` events for that turn. So we
//! track the most-recently-seen `turn_context` model per file and stamp it
//! onto following `token_count` events; the hardcoded `"gpt-5"` is only used
//! when no `turn_context` model has been seen yet.
//!
//! ## Dedup (2026-07-22)
//! `token_count` payloads carry no `id`/`session_id` (0/122 sampled), so
//! [`process_lines`] always falls back to the synthesized `"<file>:<offset>"`
//! key — dedup-by-message-id effectively never runs for Codex. This is safe:
//! each byte offset is unique within a file and offsets never rewind (the
//! persisted per-file offset only advances), and no duplicate emission of the
//! same turn's `last_token_usage` on two lines was observed in the sample.

use super::{TokenEvent, TokenProvider};
use crate::storage_paths;
use crate::watcher::claude_code::{split_complete_lines, WatcherState};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

pub struct CodexCliProvider {
    root: PathBuf,
    state_path: PathBuf,
}

impl CodexCliProvider {
    pub fn new() -> Self {
        let root = resolve_codex_root(
            std::env::var_os("CODEX_HOME").map(PathBuf::from),
            dirs::home_dir(),
        );
        let state_path = storage_paths::watcher_data_file("codex_cli_watcher_state.json");
        Self::with_paths(root, state_path)
    }

    pub fn with_paths(root: PathBuf, state_path: PathBuf) -> Self {
        Self { root, state_path }
    }
}

/// Resolves the Codex sessions directory, honoring the `CODEX_HOME` override
/// (the same env var the Codex CLI itself reads) and falling back to
/// `~/.codex`. Kept pure (takes the env value + home dir as arguments) so it
/// can be unit-tested without mutating process-global environment state.
pub(crate) fn resolve_codex_root(codex_home: Option<PathBuf>, home: Option<PathBuf>) -> PathBuf {
    codex_home
        .or_else(|| home.map(|h| h.join(".codex")))
        .map(|base| base.join("sessions"))
        .unwrap_or_else(|| PathBuf::from(".codex/sessions"))
}

impl Default for CodexCliProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenProvider for CodexCliProvider {
    fn name(&self) -> &'static str {
        "codex_cli"
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
                format!("codex sessions directory not found: {}", root.display()),
            ));
        }
        std::thread::Builder::new()
            .name("codex-cli-watcher".into())
            .spawn(move || run_poll_loop(root, state_path, tx))
            .map_err(std::io::Error::other)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ParsedCodexUsage {
    pub message_id: Option<String>,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RawLine {
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    payload: Option<RawPayload>,
}

#[derive(Debug, Deserialize)]
struct RawPayload {
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    info: Option<RawInfo>,
}

#[derive(Debug, Deserialize)]
struct RawInfo {
    #[serde(default)]
    last_token_usage: Option<RawTokenUsage>,
}

#[derive(Debug, Default, Deserialize)]
struct RawTokenUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    reasoning_output_tokens: u64,
}

/// One meaningful record from a Codex rollout line: either a `turn_context`
/// carrying the model in effect, or a `token_count` usage record. Everything
/// else (message content, tool calls, `session_meta`, ...) parses to `None`.
pub(crate) enum CodexRecord {
    /// `turn_context` with `payload.model` - the model for the turns that
    /// follow it in the same file.
    Model(String),
    /// `event_msg` with `payload.type == "token_count"`.
    Usage(ParsedCodexUsage),
}

pub(crate) fn parse_codex_line(line: &str) -> Option<CodexRecord> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let raw: RawLine = serde_json::from_str(trimmed).ok()?;

    // `turn_context` is where the real model id lives (token_count records
    // carry none); grab it so following usage events can be stamped with it.
    if raw.r#type.as_deref() == Some("turn_context") {
        let model = raw.payload.and_then(|p| p.model).filter(|m| !m.is_empty())?;
        return Some(CodexRecord::Model(model));
    }

    let payload = raw.payload?;
    if payload.r#type.as_deref() != Some("token_count") {
        return None;
    }
    let usage = payload.info?.last_token_usage?;
    Some(CodexRecord::Usage(ParsedCodexUsage {
        message_id: payload.id.or(payload.session_id),
        model: payload.model,
        input_tokens: usage.input_tokens.saturating_sub(usage.cached_input_tokens),
        output_tokens: usage
            .output_tokens
            .saturating_add(usage.reasoning_output_tokens),
        cache_read_tokens: usage.cached_input_tokens,
        timestamp: raw
            .timestamp
            .as_deref()
            .and_then(parse_rfc3339_to_unix_secs),
    }))
}

/// Test/back-compat helper: the `token_count` half of [`parse_codex_line`].
#[cfg(test)]
pub(crate) fn parse_token_count_line(line: &str) -> Option<ParsedCodexUsage> {
    match parse_codex_line(line)? {
        CodexRecord::Usage(usage) => Some(usage),
        CodexRecord::Model(_) => None,
    }
}

fn process_lines(
    file: &Path,
    offset_before: u64,
    lines: &[String],
    state: &mut WatcherState,
    current_model: &mut Option<String>,
) -> Vec<TokenEvent> {
    let mut events = Vec::new();
    let mut running_offset = offset_before;
    for line in lines {
        let this_offset = running_offset;
        running_offset += line.len() as u64 + 1;
        match parse_codex_line(line) {
            Some(CodexRecord::Model(model)) => {
                *current_model = Some(model);
                continue;
            }
            Some(CodexRecord::Usage(parsed)) => {
                let message_id = parsed
                    .message_id
                    .unwrap_or_else(|| format!("{}:{}", file.display(), this_offset));
                if !state.record_if_new(&message_id) {
                    continue;
                }
                // Prefer a model on the usage line itself (never seen in real
                // data, but honored if present), then the last turn_context
                // model, then the hardcoded default.
                let model = parsed
                    .model
                    .or_else(|| current_model.clone())
                    .unwrap_or_else(|| "gpt-5".to_string());
                events.push(TokenEvent {
                    provider: "codex_cli".to_string(),
                    message_id,
                    model,
                    input_tokens: parsed.input_tokens,
                    output_tokens: parsed.output_tokens,
                    cache_read_tokens: parsed.cache_read_tokens,
                    timestamp: parsed.timestamp.unwrap_or_else(now_unix_secs),
                });
            }
            None => continue,
        }
    }
    events
}

fn run_poll_loop(root: PathBuf, state_path: PathBuf, tx: Sender<TokenEvent>) {
    let mut state = WatcherState::load(&state_path);
    // Last `turn_context` model seen per file, so usage events read in a later
    // poll pass than their turn_context still get the right model.
    let mut models: HashMap<PathBuf, Option<String>> = HashMap::new();

    // Every launch seeds offsets to the current file end, so tokens logged
    // before the app opened (first run, or while it was closed) are never
    // counted as food; only usage produced after launch is. See
    // docs/knowledge/token-tracking.md.
    for file in list_jsonl_files(&root) {
        seed_offset_to_end(&file, &mut state, &mut models);
    }
    let _ = state.save(&state_path);

    loop {
        for file in list_jsonl_files(&root) {
            let offset = state.offset_for(&file);
            // On first sight of an already-advanced file (a restart resumed
            // past the turn_context lines), recover the model by scanning the
            // bytes we've already consumed once.
            let model = models
                .entry(file.clone())
                .or_insert_with(|| scan_latest_model(&file, offset));
            let Ok((lines, new_offset)) = read_new_lines(&file, offset) else {
                continue;
            };
            for event in process_lines(&file, offset, &lines, &mut state, model) {
                let _ = tx.send(event);
            }
            state.set_offset(&file, new_offset);
        }
        let _ = state.save(&state_path);
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

/// Seeds `state`'s offset for `file` to its current end (skipping existing
/// history) and primes the model tracker from the bytes being skipped, so a
/// later usage event still gets the right model. Used only on the first-ever
/// run (see [`run_poll_loop`]). A stat failure is logged and the file left
/// unseeded (it would be read from the start on the next poll pass).
fn seed_offset_to_end(
    file: &Path,
    state: &mut WatcherState,
    models: &mut HashMap<PathBuf, Option<String>>,
) {
    let len = match fs::metadata(file) {
        Ok(meta) => meta.len(),
        Err(e) => {
            eprintln!(
                "codex_cli watcher: failed to stat {} for initial seed: {e}",
                file.display()
            );
            return;
        }
    };
    models
        .entry(file.to_path_buf())
        .or_insert_with(|| scan_latest_model(file, len));
    state.set_offset(file, len);
}

/// Scans the first `up_to` bytes of `file` for the last `turn_context` model,
/// used to prime the in-memory model after a restart consumed those lines in
/// a previous run. Returns `None` on any read error or if none is found.
fn scan_latest_model(file: &Path, up_to: u64) -> Option<String> {
    if up_to == 0 {
        return None;
    }
    let mut f = File::open(file).ok()?;
    let mut buf = vec![0u8; up_to as usize];
    f.read_exact(&mut buf).ok()?;
    let (lines, _) = split_complete_lines(&buf);
    let mut model = None;
    for line in &lines {
        if let Some(CodexRecord::Model(m)) = parse_codex_line(line) {
            model = Some(m);
        }
    }
    model
}

fn read_new_lines(file: &Path, offset: u64) -> std::io::Result<(Vec<String>, u64)> {
    let mut f = File::open(file)?;
    let len = f.metadata()?.len();
    if len < offset {
        return read_new_lines(file, 0);
    }
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let (lines, consumed) = split_complete_lines(&buf);
    Ok((lines, offset + consumed as u64))
}

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

fn parse_rfc3339_to_unix_secs(s: &str) -> Option<i64> {
    let parsed = chrono::DateTime::parse_from_rfc3339(s).ok()?;
    Some(parsed.timestamp())
}

fn now_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_codex_token_count_shape() {
        let line = r#"{"timestamp":"2026-06-17T09:13:25.120Z","type":"event_msg","payload":{"type":"token_count","id":"turn-1","model":"gpt-5","info":{"last_token_usage":{"input_tokens":30945,"cached_input_tokens":10624,"output_tokens":499,"reasoning_output_tokens":130,"total_tokens":31444}}}}"#;
        let parsed = parse_token_count_line(line).unwrap();
        assert_eq!(parsed.message_id.as_deref(), Some("turn-1"));
        assert_eq!(parsed.model.as_deref(), Some("gpt-5"));
        assert_eq!(parsed.input_tokens, 20_321);
        assert_eq!(parsed.cache_read_tokens, 10_624);
        assert_eq!(parsed.output_tokens, 629);
        assert_eq!(parsed.timestamp, Some(1_781_687_605));
    }

    #[test]
    fn drops_non_token_count_lines_and_malformed_json() {
        assert!(parse_token_count_line("{}").is_none());
        assert!(parse_token_count_line("not json").is_none());
    }

    #[test]
    fn dedups_by_payload_id_or_fallback_offset() {
        let file = PathBuf::from("/tmp/codex.jsonl");
        let mut state = WatcherState::default();
        let lines = vec![
            r#"{"timestamp":"2026-06-17T09:13:25Z","type":"event_msg","payload":{"type":"token_count","id":"same","info":{"last_token_usage":{"input_tokens":1,"output_tokens":2}}}}"#.to_string(),
            r#"{"timestamp":"2026-06-17T09:13:26Z","type":"event_msg","payload":{"type":"token_count","id":"same","info":{"last_token_usage":{"input_tokens":3,"output_tokens":4}}}}"#.to_string(),
            r#"{"timestamp":"2026-06-17T09:13:27Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":5,"output_tokens":6}}}}"#.to_string(),
        ];
        let events = process_lines(&file, 0, &lines, &mut state, &mut None);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message_id, "same");
        assert!(events[1].message_id.starts_with("/tmp/codex.jsonl:"));
    }

    #[test]
    fn stamps_turn_context_model_onto_following_usage_events() {
        let file = PathBuf::from("/tmp/codex.jsonl");
        let mut state = WatcherState::default();
        let mut model = None;
        let lines = vec![
            // A turn_context sets the model; real token_count lines carry none.
            r#"{"timestamp":"2026-06-17T09:13:24Z","type":"turn_context","payload":{"model":"gpt-5.4","cwd":"/x"}}"#.to_string(),
            r#"{"timestamp":"2026-06-17T09:13:25Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":10,"output_tokens":2}}}}"#.to_string(),
            // Model can change mid-file; later usage picks up the new one.
            r#"{"timestamp":"2026-06-17T09:13:26Z","type":"turn_context","payload":{"model":"gpt-5-codex"}}"#.to_string(),
            r#"{"timestamp":"2026-06-17T09:13:27Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":20,"output_tokens":3}}}}"#.to_string(),
        ];
        let events = process_lines(&file, 0, &lines, &mut state, &mut model);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].model, "gpt-5.4");
        assert_eq!(events[1].model, "gpt-5-codex");
        // The tracker retains the last model for the next poll pass.
        assert_eq!(model.as_deref(), Some("gpt-5-codex"));
    }

    #[test]
    fn falls_back_to_default_model_before_any_turn_context() {
        let file = PathBuf::from("/tmp/codex.jsonl");
        let mut state = WatcherState::default();
        let lines = vec![
            r#"{"timestamp":"2026-06-17T09:13:25Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":1,"output_tokens":2}}}}"#.to_string(),
        ];
        let events = process_lines(&file, 0, &lines, &mut state, &mut None);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].model, "gpt-5");
    }

    #[test]
    fn first_run_seed_skips_history_and_primes_model() {
        let dir = std::env::temp_dir().join(format!(
            "tokengochi-codex-firstrun-{}-{}",
            std::process::id(),
            now_unix_secs()
        ));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("rollout.jsonl");
        let contents = concat!(
            r#"{"timestamp":"2026-06-17T09:13:24Z","type":"turn_context","payload":{"model":"gpt-5.4"}}"#,
            "\n",
            r#"{"timestamp":"2026-06-17T09:13:25Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":10,"output_tokens":2}}}}"#,
            "\n",
        );
        fs::write(&file, contents).unwrap();

        let mut state = WatcherState::default();
        let mut models: HashMap<PathBuf, Option<String>> = HashMap::new();
        seed_offset_to_end(&file, &mut state, &mut models);

        // Offset advanced to the end -> nothing left to read as "new".
        let end = fs::metadata(&file).unwrap().len();
        assert_eq!(state.offset_for(&file), end);
        let (lines, _) = read_new_lines(&file, state.offset_for(&file)).unwrap();
        assert!(lines.is_empty(), "seeded file must have no new lines");
        // Model recovered from the skipped history, ready for later usage events.
        assert_eq!(models.get(&file), Some(&Some("gpt-5.4".to_string())));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn codex_root_honors_env_then_home_then_relative() {
        let home = PathBuf::from("/home/alice");
        assert_eq!(
            resolve_codex_root(Some(PathBuf::from("/custom/codex")), Some(home.clone())),
            PathBuf::from("/custom/codex/sessions"),
        );
        assert_eq!(
            resolve_codex_root(None, Some(home)),
            PathBuf::from("/home/alice/.codex/sessions"),
        );
        assert_eq!(
            resolve_codex_root(None, None),
            PathBuf::from(".codex/sessions"),
        );
    }
}
