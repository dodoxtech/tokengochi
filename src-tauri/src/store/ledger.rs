//! SQLite-backed token event ledger.
//!
//! Records [`TokenEvent`]s idempotently by `message_id`, so replaying or
//! re-emitting the same event (a watcher restart, a bug, manual debugging)
//! never double-counts. This is a second, persistent dedup layer beneath the
//! in-memory one in `watcher::claude_code::WatcherState` - defense in depth,
//! and the actual source of truth an economy-state rebuild would replay
//! from. See `docs/architecture.md` §Key Dependencies and
//! `docs/tasks/active/0004-economy-engine-core.md`.

use crate::watcher::TokenEvent;
use rusqlite::{params, Connection};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenTotals {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub total: u64,
}

pub struct Ledger {
    conn: Connection,
}

impl Ledger {
    /// Opens (creating if needed) a ledger database file at `path`, creating
    /// its parent directory and the schema if they don't exist yet.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self::from_connection(Connection::open(path)?)
    }

    /// In-memory ledger - for tests, not for production use (nothing
    /// persists across process restarts).
    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS token_events (
                message_id        TEXT PRIMARY KEY,
                provider          TEXT NOT NULL,
                model             TEXT NOT NULL DEFAULT '',
                input_tokens      INTEGER NOT NULL,
                output_tokens     INTEGER NOT NULL,
                cache_read_tokens INTEGER NOT NULL,
                timestamp         INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    /// Records `event`. Returns `Ok(true)` if it was newly inserted,
    /// `Ok(false)` if `message_id` was already present - replaying the same
    /// event is a no-op, not a double-count. Token counts are stored as
    /// `i64` (SQLite has no native unsigned integer type); real token
    /// counts are nowhere near `i64::MAX`, so this cast is lossless in
    /// practice.
    pub fn record_event(&self, event: &TokenEvent) -> rusqlite::Result<bool> {
        let rows_changed = self.conn.execute(
            "INSERT OR IGNORE INTO token_events
                (message_id, provider, model, input_tokens, output_tokens, cache_read_tokens, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.message_id,
                event.provider,
                event.model,
                event.input_tokens as i64,
                event.output_tokens as i64,
                event.cache_read_tokens as i64,
                event.timestamp,
            ],
        )?;
        Ok(rows_changed > 0)
    }

    /// Total count of distinct events recorded - mostly for tests/debugging.
    pub fn event_count(&self) -> rusqlite::Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM token_events", [], |row| row.get(0))
    }

    pub fn token_totals_between(
        &self,
        start_unix: i64,
        end_unix: i64,
    ) -> rusqlite::Result<TokenTotals> {
        self.conn.query_row(
            "SELECT
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0),
                COALESCE(SUM(cache_read_tokens), 0)
             FROM token_events
             WHERE timestamp >= ?1 AND timestamp < ?2",
            params![start_unix, end_unix],
            |row| {
                let input = row.get::<_, i64>(0)? as u64;
                let output = row.get::<_, i64>(1)? as u64;
                let cache_read = row.get::<_, i64>(2)? as u64;
                Ok(TokenTotals {
                    input,
                    output,
                    cache_read,
                    total: input + output + cache_read,
                })
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event(id: &str) -> TokenEvent {
        TokenEvent {
            provider: "claude_code".to_string(),
            message_id: id.to_string(),
            model: "claude-sonnet-5".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            cache_read_tokens: 0,
            timestamp: 1_700_000_000,
        }
    }

    fn sample_event_at(id: &str, timestamp: i64) -> TokenEvent {
        TokenEvent {
            timestamp,
            ..sample_event(id)
        }
    }

    #[test]
    fn records_a_new_event() {
        let ledger = Ledger::in_memory().unwrap();
        assert!(ledger.record_event(&sample_event("msg_1")).unwrap());
        assert_eq!(ledger.event_count().unwrap(), 1);
    }

    #[test]
    fn replaying_the_same_event_is_idempotent() {
        let ledger = Ledger::in_memory().unwrap();
        assert!(ledger.record_event(&sample_event("msg_1")).unwrap());
        // Same message_id again - e.g. a watcher restart re-sent it.
        assert!(!ledger.record_event(&sample_event("msg_1")).unwrap());
        assert_eq!(
            ledger.event_count().unwrap(),
            1,
            "replay must not double-count"
        );
    }

    #[test]
    fn distinct_events_are_both_recorded() {
        let ledger = Ledger::in_memory().unwrap();
        assert!(ledger.record_event(&sample_event("msg_1")).unwrap());
        assert!(ledger.record_event(&sample_event("msg_2")).unwrap());
        assert_eq!(ledger.event_count().unwrap(), 2);
    }

    #[test]
    fn repeated_full_replay_of_a_batch_is_idempotent() {
        // Simulates re-processing an entire session's worth of events after
        // a crash - the whole batch replayed twice should still only count
        // once each.
        let ledger = Ledger::in_memory().unwrap();
        let batch: Vec<TokenEvent> = (0..5).map(|i| sample_event(&format!("msg_{i}"))).collect();

        for event in &batch {
            ledger.record_event(event).unwrap();
        }
        for event in &batch {
            ledger.record_event(event).unwrap(); // replay
        }

        assert_eq!(ledger.event_count().unwrap(), 5);
    }

    #[test]
    fn sums_token_totals_for_time_window() {
        let ledger = Ledger::in_memory().unwrap();
        ledger.record_event(&sample_event_at("early", 9)).unwrap();
        ledger.record_event(&sample_event_at("inside", 10)).unwrap();
        ledger.record_event(&sample_event_at("late", 20)).unwrap();

        assert_eq!(
            ledger.token_totals_between(10, 20).unwrap(),
            TokenTotals {
                input: 100,
                output: 50,
                cache_read: 0,
                total: 150,
            }
        );
    }
}
