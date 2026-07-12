//! SQLite persistence for the current pet/economy state.
//!
//! The token ledger is an append-only history, but the overlay loop also
//! needs a compact "what is pending right now?" snapshot so queued Food
//! survives app restarts.

use crate::economy::EconomyState;
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct GameStateStore {
    conn: Connection,
}

impl GameStateStore {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS economy_state (
                id                    INTEGER PRIMARY KEY CHECK (id = 1),
                current_day           TEXT NOT NULL,
                food_earned_today     INTEGER NOT NULL,
                banked_tokens_today   REAL NOT NULL,
                pantry                INTEGER NOT NULL,
                food_inventory        INTEGER NOT NULL,
                fullness              REAL NOT NULL,
                xp                    REAL NOT NULL,
                last_reconciled_unix  INTEGER NOT NULL,
                updated_at_unix       INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub fn load_economy_state(&self) -> rusqlite::Result<Option<EconomyState>> {
        self.conn
            .query_row(
                "SELECT current_day, food_earned_today, banked_tokens_today, pantry,
                        food_inventory, fullness, xp, last_reconciled_unix
                 FROM economy_state
                 WHERE id = 1",
                [],
                |row| {
                    let current_day_raw: String = row.get(0)?;
                    let current_day = NaiveDate::parse_from_str(&current_day_raw, "%Y-%m-%d")
                        .map_err(|err| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(err),
                            )
                        })?;

                    Ok(EconomyState {
                        current_day,
                        food_earned_today: row.get::<_, i64>(1)? as u32,
                        banked_tokens_today: row.get(2)?,
                        pantry: row.get::<_, i64>(3)? as u32,
                        food_inventory: row.get::<_, i64>(4)? as u32,
                        fullness: row.get(5)?,
                        xp: row.get(6)?,
                        last_reconciled_unix: row.get(7)?,
                    })
                },
            )
            .optional()
    }

    pub fn save_economy_state(&self, state: &EconomyState, now_unix: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO economy_state (
                id, current_day, food_earned_today, banked_tokens_today, pantry,
                food_inventory, fullness, xp, last_reconciled_unix, updated_at_unix
             )
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                current_day = excluded.current_day,
                food_earned_today = excluded.food_earned_today,
                banked_tokens_today = excluded.banked_tokens_today,
                pantry = excluded.pantry,
                food_inventory = excluded.food_inventory,
                fullness = excluded.fullness,
                xp = excluded.xp,
                last_reconciled_unix = excluded.last_reconciled_unix,
                updated_at_unix = excluded.updated_at_unix",
            params![
                state.current_day.to_string(),
                state.food_earned_today as i64,
                state.banked_tokens_today,
                state.pantry as i64,
                state.food_inventory as i64,
                state.fullness,
                state.xp,
                state.last_reconciled_unix,
                now_unix,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_pending_food_inventory() {
        let store = GameStateStore::in_memory().unwrap();
        let mut state = EconomyState::new(NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(), 100);
        state.food_inventory = 3;
        state.banked_tokens_today = 1234.5;
        state.fullness = 42.0;
        state.xp = 99.0;

        store.save_economy_state(&state, 200).unwrap();

        assert_eq!(store.load_economy_state().unwrap(), Some(state));
    }
}
