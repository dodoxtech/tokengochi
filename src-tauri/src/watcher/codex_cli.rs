//! Tails `~/.codex/sessions/**/*.jsonl` token_count events.
//!
//! Verified against local Codex Desktop/CLI logs on 2026-07-12: usage arrives
//! as `type:"event_msg"` with `payload.type:"token_count"` and
//! `payload.info.last_token_usage`. We read only usage counters, timestamps,
//! and rollout/session ids; message content is ignored.

use super::{TokenEvent, TokenProvider};
use crate::storage_paths;
use crate::watcher::claude_code::{split_complete_lines, WatcherState};
use serde::Deserialize;
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
        let root = dirs::home_dir()
            .map(|home| home.join(".codex").join("sessions"))
            .unwrap_or_else(|| PathBuf::from(".codex/sessions"));
        let state_path = storage_paths::watcher_data_file("codex_cli_watcher_state.json");
        Self::with_paths(root, state_path)
    }

    pub fn with_paths(root: PathBuf, state_path: PathBuf) -> Self {
        Self { root, state_path }
    }
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
    #[allow(dead_code)]
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

pub(crate) fn parse_token_count_line(line: &str) -> Option<ParsedCodexUsage> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let raw: RawLine = serde_json::from_str(trimmed).ok()?;
    let payload = raw.payload?;
    if payload.r#type.as_deref() != Some("token_count") {
        return None;
    }
    let usage = payload.info?.last_token_usage?;
    Some(ParsedCodexUsage {
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
    })
}

fn process_lines(
    file: &Path,
    offset_before: u64,
    lines: &[String],
    state: &mut WatcherState,
) -> Vec<TokenEvent> {
    let mut events = Vec::new();
    let mut running_offset = offset_before;
    for line in lines {
        let this_offset = running_offset;
        running_offset += line.len() as u64 + 1;
        let Some(parsed) = parse_token_count_line(line) else {
            continue;
        };
        let message_id = parsed
            .message_id
            .unwrap_or_else(|| format!("{}:{}", file.display(), this_offset));
        if !state.record_if_new(&message_id) {
            continue;
        }
        events.push(TokenEvent {
            provider: "codex_cli".to_string(),
            message_id,
            model: parsed.model.unwrap_or_else(|| "gpt-5".to_string()),
            input_tokens: parsed.input_tokens,
            output_tokens: parsed.output_tokens,
            cache_read_tokens: parsed.cache_read_tokens,
            timestamp: parsed.timestamp.unwrap_or_else(now_unix_secs),
        });
    }
    events
}

fn run_poll_loop(root: PathBuf, state_path: PathBuf, tx: Sender<TokenEvent>) {
    let mut state = WatcherState::load(&state_path);
    loop {
        for file in list_jsonl_files(&root) {
            let offset = state.offset_for(&file);
            let Ok((lines, new_offset)) = read_new_lines(&file, offset) else {
                continue;
            };
            for event in process_lines(&file, offset, &lines, &mut state) {
                let _ = tx.send(event);
            }
            state.set_offset(&file, new_offset);
        }
        let _ = state.save(&state_path);
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
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
        let events = process_lines(&file, 0, &lines, &mut state);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message_id, "same");
        assert!(events[1].message_id.starts_with("/tmp/codex.jsonl:"));
    }
}
