//! SQLite persistence for the current pet/economy state.
//!
//! The token ledger is an append-only history, but the overlay loop also
//! needs a compact "what is pending right now?" snapshot so queued Food
//! survives app restarts.

use crate::economy::EconomyState;
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub onboarding_complete: bool,
    pub starter_egg: String,
    pub claude_code_enabled: bool,
    pub pet_size: u32,
    pub monitor_index: u32,
    pub wayland_fallback: bool,
    pub tracking_paused: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            onboarding_complete: false,
            starter_egg: "sprout".to_string(),
            claude_code_enabled: true,
            pet_size: 100,
            monitor_index: 0,
            wayland_fallback: false,
            tracking_paused: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoodStats {
    pub today: u32,
    pub week: u32,
}

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
        conn.execute(
            "CREATE TABLE IF NOT EXISTS app_settings (
                id                    INTEGER PRIMARY KEY CHECK (id = 1),
                onboarding_complete   INTEGER NOT NULL,
                starter_egg           TEXT NOT NULL,
                claude_code_enabled   INTEGER NOT NULL,
                pet_size              INTEGER NOT NULL,
                monitor_index         INTEGER NOT NULL,
                wayland_fallback      INTEGER NOT NULL,
                tracking_paused       INTEGER NOT NULL,
                updated_at_unix       INTEGER NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS daily_food_totals (
                day         TEXT PRIMARY KEY,
                food_earned INTEGER NOT NULL DEFAULT 0
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

    pub fn load_app_settings(&self) -> rusqlite::Result<AppSettings> {
        let settings = self
            .conn
            .query_row(
                "SELECT onboarding_complete, starter_egg, claude_code_enabled,
                        pet_size, monitor_index, wayland_fallback, tracking_paused
                 FROM app_settings
                 WHERE id = 1",
                [],
                |row| {
                    Ok(AppSettings {
                        onboarding_complete: row.get::<_, i64>(0)? != 0,
                        starter_egg: row.get(1)?,
                        claude_code_enabled: row.get::<_, i64>(2)? != 0,
                        pet_size: row.get::<_, i64>(3)? as u32,
                        monitor_index: row.get::<_, i64>(4)? as u32,
                        wayland_fallback: row.get::<_, i64>(5)? != 0,
                        tracking_paused: row.get::<_, i64>(6)? != 0,
                    })
                },
            )
            .optional()?;
        Ok(settings.unwrap_or_default())
    }

    pub fn save_app_settings(&self, settings: &AppSettings, now_unix: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO app_settings (
                id, onboarding_complete, starter_egg, claude_code_enabled,
                pet_size, monitor_index, wayland_fallback, tracking_paused, updated_at_unix
             )
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                onboarding_complete = excluded.onboarding_complete,
                starter_egg = excluded.starter_egg,
                claude_code_enabled = excluded.claude_code_enabled,
                pet_size = excluded.pet_size,
                monitor_index = excluded.monitor_index,
                wayland_fallback = excluded.wayland_fallback,
                tracking_paused = excluded.tracking_paused,
                updated_at_unix = excluded.updated_at_unix",
            params![
                bool_to_i64(settings.onboarding_complete),
                settings.starter_egg,
                bool_to_i64(settings.claude_code_enabled),
                settings.pet_size as i64,
                settings.monitor_index as i64,
                bool_to_i64(settings.wayland_fallback),
                bool_to_i64(settings.tracking_paused),
                now_unix,
            ],
        )?;
        Ok(())
    }

    pub fn increment_daily_food(&self, day: NaiveDate, food_earned: u32) -> rusqlite::Result<()> {
        if food_earned == 0 {
            return Ok(());
        }
        self.conn.execute(
            "INSERT INTO daily_food_totals (day, food_earned)
             VALUES (?1, ?2)
             ON CONFLICT(day) DO UPDATE SET
                food_earned = food_earned + excluded.food_earned",
            params![day.to_string(), food_earned as i64],
        )?;
        Ok(())
    }

    pub fn food_stats_since(
        &self,
        today: NaiveDate,
        week_start: NaiveDate,
    ) -> rusqlite::Result<FoodStats> {
        let today_food = self
            .conn
            .query_row(
                "SELECT COALESCE(food_earned, 0) FROM daily_food_totals WHERE day = ?1",
                [today.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0) as u32;
        let week_food = self.conn.query_row(
            "SELECT COALESCE(SUM(food_earned), 0)
             FROM daily_food_totals
             WHERE day >= ?1 AND day <= ?2",
            params![week_start.to_string(), today.to_string()],
            |row| row.get::<_, i64>(0),
        )? as u32;

        Ok(FoodStats {
            today: today_food,
            week: week_food,
        })
    }
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
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

    #[test]
    fn pending_food_survives_reopening_the_database() {
        let path = std::env::temp_dir().join(format!(
            "tokengochi-game-state-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut state = EconomyState::new(NaiveDate::from_ymd_opt(2026, 7, 12).unwrap(), 100);
        state.food_inventory = 2;

        {
            let store = GameStateStore::open(&path).unwrap();
            store.save_economy_state(&state, 200).unwrap();
        }

        let reopened = GameStateStore::open(&path).unwrap();
        assert_eq!(reopened.load_economy_state().unwrap(), Some(state));

        drop(reopened);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn round_trips_app_settings() {
        let store = GameStateStore::in_memory().unwrap();
        let settings = AppSettings {
            onboarding_complete: true,
            starter_egg: "ember".to_string(),
            claude_code_enabled: false,
            pet_size: 125,
            monitor_index: 1,
            wayland_fallback: true,
            tracking_paused: true,
        };

        store.save_app_settings(&settings, 300).unwrap();

        assert_eq!(store.load_app_settings().unwrap(), settings);
    }

    #[test]
    fn aggregates_daily_food_totals() {
        let store = GameStateStore::in_memory().unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 7, 12).unwrap();
        let yesterday = NaiveDate::from_ymd_opt(2026, 7, 11).unwrap();

        store.increment_daily_food(today, 2).unwrap();
        store.increment_daily_food(today, 3).unwrap();
        store.increment_daily_food(yesterday, 4).unwrap();

        assert_eq!(
            store.food_stats_since(today, yesterday).unwrap(),
            FoodStats { today: 5, week: 9 }
        );
    }
}
