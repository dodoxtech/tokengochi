//! OpenAI Usage API provider.
//!
//! This is opt-in: the API key is read from the OS keychain helper, never
//! SQLite or config files. Polling uses the Usage API's bucket timestamps as
//! event timestamps so delayed usage is applied to the day it occurred.

use super::{TokenEvent, TokenProvider};
use serde::Deserialize;
use std::collections::HashSet;
use std::process::Command;
use std::sync::mpsc::Sender;

const KEYCHAIN_SERVICE: &str = "tokengochi";
const KEYCHAIN_ACCOUNT: &str = "openai_usage_api_key";

pub struct OpenAiProvider {
    poll_interval_secs: u64,
}

impl OpenAiProvider {
    pub fn new() -> Self {
        Self {
            poll_interval_secs: 15 * 60,
        }
    }

    pub fn set_api_key(api_key: &str) -> std::io::Result<()> {
        Keychain::set(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, api_key)
    }

    pub fn clear_api_key() -> std::io::Result<()> {
        Keychain::delete(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
    }

    pub fn has_api_key() -> bool {
        Keychain::get(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
            .map(|key| !key.trim().is_empty())
            .unwrap_or(false)
    }
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenProvider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn detect(&self) -> bool {
        Self::has_api_key()
    }

    fn start(&self, tx: Sender<TokenEvent>) -> std::io::Result<()> {
        let api_key = Keychain::get(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)?;
        if api_key.trim().is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "OpenAI Usage API key is not configured",
            ));
        }
        let poll_interval_secs = self.poll_interval_secs;
        std::thread::Builder::new()
            .name("openai-usage-poller".into())
            .spawn(move || run_poll_loop(api_key, poll_interval_secs, tx))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(())
    }
}

fn run_poll_loop(api_key: String, poll_interval_secs: u64, tx: Sender<TokenEvent>) {
    let mut seen = HashSet::new();
    loop {
        let end = now_unix_secs().saturating_sub(300);
        let start = end.saturating_sub(24 * 3600);
        if let Ok(raw) = fetch_usage(&api_key, start, end) {
            for event in parse_usage_response(&raw) {
                if seen.insert(event.message_id.clone()) {
                    let _ = tx.send(event);
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(poll_interval_secs));
    }
}

fn fetch_usage(api_key: &str, start_time: i64, end_time: i64) -> std::io::Result<String> {
    let url = format!(
        "https://api.openai.com/v1/organization/usage/completions?start_time={start_time}&end_time={end_time}&bucket_width=1m&group_by[]=model"
    );
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "-H",
            &format!("Authorization: Bearer {api_key}"),
            &url,
        ])
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    #[serde(default)]
    data: Vec<UsageBucket>,
}

#[derive(Debug, Deserialize)]
struct UsageBucket {
    start_time: i64,
    #[serde(default)]
    results: Vec<UsageResult>,
}

#[derive(Debug, Deserialize)]
struct UsageResult {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    input_cached_tokens: u64,
}

pub(crate) fn parse_usage_response(raw: &str) -> Vec<TokenEvent> {
    let Ok(response) = serde_json::from_str::<UsageResponse>(raw) else {
        return Vec::new();
    };
    response
        .data
        .into_iter()
        .flat_map(|bucket| {
            bucket.results.into_iter().map(move |result| {
                let model = result.model.unwrap_or_default();
                let message_id = format!(
                    "openai:{}:{}:{}:{}",
                    bucket.start_time, model, result.input_tokens, result.output_tokens
                );
                TokenEvent {
                    provider: "openai".to_string(),
                    message_id,
                    model,
                    input_tokens: result
                        .input_tokens
                        .saturating_sub(result.input_cached_tokens),
                    output_tokens: result.output_tokens,
                    cache_read_tokens: result.input_cached_tokens,
                    timestamp: bucket.start_time,
                }
            })
        })
        .collect()
}

struct Keychain;

impl Keychain {
    fn get(service: &str, account: &str) -> std::io::Result<String> {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("security")
                .args(["find-generic-password", "-s", service, "-a", account, "-w"])
                .output()?;
            if output.status.success() {
                return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "keychain item not found",
            ));
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (service, account);
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "OS keychain integration is implemented for macOS in this build",
            ))
        }
    }

    fn set(service: &str, account: &str, secret: &str) -> std::io::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let status = Command::new("security")
                .args([
                    "add-generic-password",
                    "-U",
                    "-s",
                    service,
                    "-a",
                    account,
                    "-w",
                    secret,
                ])
                .status()?;
            if status.success() {
                return Ok(());
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "failed to store keychain item",
            ));
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (service, account, secret);
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "OS keychain integration is implemented for macOS in this build",
            ))
        }
    }

    fn delete(service: &str, account: &str) -> std::io::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let status = Command::new("security")
                .args(["delete-generic-password", "-s", service, "-a", account])
                .status()?;
            if status.success() {
                return Ok(());
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "keychain item not found",
            ));
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (service, account);
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "OS keychain integration is implemented for macOS in this build",
            ))
        }
    }
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
    fn parses_usage_api_buckets_into_events_on_occurrence_time() {
        let raw = r#"{"data":[{"start_time":1783890000,"end_time":1783890060,"results":[{"model":"gpt-4.1","input_tokens":1200,"output_tokens":300,"input_cached_tokens":200}]}]}"#;
        let events = parse_usage_response(raw);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].provider, "openai");
        assert_eq!(events[0].model, "gpt-4.1");
        assert_eq!(events[0].input_tokens, 1_000);
        assert_eq!(events[0].cache_read_tokens, 200);
        assert_eq!(events[0].output_tokens, 300);
        assert_eq!(events[0].timestamp, 1_783_890_000);
    }

    #[test]
    fn malformed_usage_api_response_is_empty() {
        assert!(parse_usage_response("not json").is_empty());
    }
}
