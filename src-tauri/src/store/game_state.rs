//! SQLite persistence for the current pet/economy state.
//!
//! The token ledger is an append-only history, but the overlay loop also
//! needs a compact "what is pending right now?" snapshot so queued Food
//! survives app restarts.

use crate::economy::{DailyQuestState, EconomyState};
use crate::pet::{
    album_key, AlbumRecord, EvolutionBranch, EvolutionEvent, EvolutionStage, FurniturePlacement,
    UsagePatternStats,
};
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub onboarding_complete: bool,
    pub starter_egg: String,
    pub claude_code_enabled: bool,
    pub codex_cli_enabled: bool,
    pub openai_enabled: bool,
    pub pet_size: u32,
    pub monitor_index: u32,
    pub wayland_fallback: bool,
    pub tracking_paused: bool,
    pub calm_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            onboarding_complete: false,
            starter_egg: "sprout".to_string(),
            claude_code_enabled: true,
            codex_cli_enabled: false,
            openai_enabled: false,
            pet_size: 100,
            monitor_index: 0,
            wayland_fallback: false,
            tracking_paused: false,
            calm_mode: false,
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
                food_earned_by_day_json TEXT NOT NULL DEFAULT '',
                banked_tokens_today   REAL NOT NULL,
                banked_tokens_by_day_json TEXT NOT NULL DEFAULT '',
                pantry                INTEGER NOT NULL,
                food_inventory        INTEGER NOT NULL,
                fullness              REAL NOT NULL,
                xp                    REAL NOT NULL,
                sparks                INTEGER NOT NULL DEFAULT 0,
                streak_days           INTEGER NOT NULL DEFAULT 0,
                streak_freezes        INTEGER NOT NULL DEFAULT 0,
                last_activity_day     TEXT,
                weekly_food_earned    INTEGER NOT NULL DEFAULT 0,
                weekly_target         INTEGER NOT NULL DEFAULT 7,
                weekly_milestone_claimed INTEGER NOT NULL DEFAULT 0,
                daily_quest_json      TEXT NOT NULL DEFAULT '',
                usage_stats_json      TEXT NOT NULL DEFAULT '',
                providers_by_day_json TEXT NOT NULL DEFAULT '',
                evolution_stage       TEXT NOT NULL DEFAULT 'Egg',
                evolution_branch      TEXT NOT NULL DEFAULT 'Sprout',
                album_json            TEXT NOT NULL DEFAULT '',
                album_records_json    TEXT NOT NULL DEFAULT '',
                owned_items_json      TEXT NOT NULL DEFAULT '',
                equipped_cosmetic     TEXT,
                equipped_food_skin    TEXT,
                furniture_json        TEXT NOT NULL DEFAULT '',
                prestige_count        INTEGER NOT NULL DEFAULT 0,
                xp_bonus_multiplier   REAL NOT NULL DEFAULT 1.0,
                pending_evolution_json TEXT,
                last_reconciled_unix  INTEGER NOT NULL,
                updated_at_unix       INTEGER NOT NULL
            )",
            [],
        )?;
        migrate_economy_state_columns(&conn)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS app_settings (
                id                    INTEGER PRIMARY KEY CHECK (id = 1),
                onboarding_complete   INTEGER NOT NULL,
                starter_egg           TEXT NOT NULL,
                claude_code_enabled   INTEGER NOT NULL,
                codex_cli_enabled     INTEGER NOT NULL DEFAULT 0,
                openai_enabled        INTEGER NOT NULL DEFAULT 0,
                pet_size              INTEGER NOT NULL,
                monitor_index         INTEGER NOT NULL,
                wayland_fallback      INTEGER NOT NULL,
                tracking_paused       INTEGER NOT NULL,
                updated_at_unix       INTEGER NOT NULL
            )",
            [],
        )?;
        migrate_app_settings_columns(&conn)?;
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
                "SELECT current_day, food_earned_today, food_earned_by_day_json,
                        banked_tokens_today, banked_tokens_by_day_json, pantry,
                        food_inventory, fullness, xp, sparks, streak_days, streak_freezes,
                        last_activity_day, weekly_food_earned, weekly_target,
                        weekly_milestone_claimed, daily_quest_json, usage_stats_json,
                        providers_by_day_json, evolution_stage, evolution_branch, album_json,
                        album_records_json, owned_items_json, equipped_cosmetic,
                        equipped_food_skin, furniture_json, prestige_count,
                        xp_bonus_multiplier, pending_evolution_json, last_reconciled_unix
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

                    let last_activity_raw: Option<String> = row.get(12)?;
                    let last_activity_day = last_activity_raw
                        .as_deref()
                        .map(|raw| parse_date_column(raw, 12))
                        .transpose()?;
                    let food_earned_by_day_raw: String = row.get(2)?;
                    let banked_tokens_by_day_raw: String = row.get(4)?;
                    let daily_quest_raw: String = row.get(16)?;
                    let usage_stats_raw: String = row.get(17)?;
                    let providers_by_day_raw: String = row.get(18)?;
                    let evolution_stage_raw: String = row.get(19)?;
                    let evolution_branch_raw: String = row.get(20)?;
                    let album_raw: String = row.get(21)?;
                    let album_records_raw: String = row.get(22)?;
                    let owned_items_raw: String = row.get(23)?;
                    let furniture_raw: String = row.get(26)?;
                    let pending_evolution_raw: Option<String> = row.get(29)?;

                    Ok(EconomyState {
                        current_day,
                        food_earned_today: row.get::<_, i64>(1)? as u32,
                        food_earned_by_day: json_or_default(
                            &food_earned_by_day_raw,
                            std::collections::BTreeMap::<String, u32>::new,
                        ),
                        banked_tokens_today: row.get(3)?,
                        banked_tokens_by_day: json_or_default(
                            &banked_tokens_by_day_raw,
                            std::collections::BTreeMap::<String, f64>::new,
                        ),
                        pantry: row.get::<_, i64>(5)? as u32,
                        food_inventory: row.get::<_, i64>(6)? as u32,
                        fullness: row.get(7)?,
                        xp: row.get(8)?,
                        sparks: row.get::<_, i64>(9)? as u32,
                        streak_days: row.get::<_, i64>(10)? as u32,
                        streak_freezes: row.get::<_, i64>(11)? as u32,
                        last_activity_day,
                        weekly_food_earned: row.get::<_, i64>(13)? as u32,
                        weekly_target: row.get::<_, i64>(14)? as u32,
                        weekly_milestone_claimed: row.get::<_, i64>(15)? != 0,
                        daily_quest: json_or_default(&daily_quest_raw, || {
                            DailyQuestState::for_day(current_day)
                        }),
                        usage_stats: json_or_default(&usage_stats_raw, UsagePatternStats::default),
                        providers_by_day: json_or_default(
                            &providers_by_day_raw,
                            std::collections::BTreeMap::<String, Vec<String>>::new,
                        ),
                        evolution_stage: json_or_default_string(
                            &evolution_stage_raw,
                            EvolutionStage::Egg,
                        ),
                        evolution_branch: json_or_default_string(
                            &evolution_branch_raw,
                            EvolutionBranch::Sprout,
                        ),
                        album: json_or_default(&album_raw, || {
                            vec![album_key(EvolutionStage::Egg, EvolutionBranch::Sprout, 0)]
                        }),
                        album_records: json_or_default(&album_records_raw, || {
                            vec![AlbumRecord {
                                key: album_key(EvolutionStage::Egg, EvolutionBranch::Sprout, 0),
                                stage: EvolutionStage::Egg,
                                branch: EvolutionBranch::Sprout,
                                reached_day: current_day.to_string(),
                                level: 0,
                                xp: row.get(8).unwrap_or(0.0),
                                sparks: row.get::<_, i64>(9).unwrap_or(0) as u32,
                                prestige_count: row.get::<_, i64>(27).unwrap_or(0) as u32,
                            }]
                        }),
                        owned_items: json_or_default(&owned_items_raw, Vec::<String>::new),
                        equipped_cosmetic: row.get(24)?,
                        equipped_food_skin: row.get(25)?,
                        furniture: json_or_default(&furniture_raw, Vec::<FurniturePlacement>::new),
                        prestige_count: row.get::<_, i64>(27)? as u32,
                        xp_bonus_multiplier: row.get(28)?,
                        pending_evolution: pending_evolution_raw
                            .as_deref()
                            .and_then(|raw| serde_json::from_str::<EvolutionEvent>(raw).ok()),
                        last_reconciled_unix: row.get(30)?,
                    })
                },
            )
            .optional()
    }

    pub fn save_economy_state(&self, state: &EconomyState, now_unix: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO economy_state (
                id, current_day, food_earned_today, food_earned_by_day_json,
                banked_tokens_today, banked_tokens_by_day_json, pantry,
                food_inventory, fullness, xp, sparks, streak_days, streak_freezes,
                last_activity_day, weekly_food_earned, weekly_target,
                weekly_milestone_claimed, daily_quest_json, usage_stats_json,
                providers_by_day_json, evolution_stage, evolution_branch, album_json, album_records_json,
                owned_items_json, equipped_cosmetic, equipped_food_skin, furniture_json,
                prestige_count, xp_bonus_multiplier, pending_evolution_json,
                last_reconciled_unix, updated_at_unix
             )
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                     ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23,
                     ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32)
             ON CONFLICT(id) DO UPDATE SET
                current_day = excluded.current_day,
                food_earned_today = excluded.food_earned_today,
                food_earned_by_day_json = excluded.food_earned_by_day_json,
                banked_tokens_today = excluded.banked_tokens_today,
                banked_tokens_by_day_json = excluded.banked_tokens_by_day_json,
                pantry = excluded.pantry,
                food_inventory = excluded.food_inventory,
                fullness = excluded.fullness,
                xp = excluded.xp,
                sparks = excluded.sparks,
                streak_days = excluded.streak_days,
                streak_freezes = excluded.streak_freezes,
                last_activity_day = excluded.last_activity_day,
                weekly_food_earned = excluded.weekly_food_earned,
                weekly_target = excluded.weekly_target,
                weekly_milestone_claimed = excluded.weekly_milestone_claimed,
                daily_quest_json = excluded.daily_quest_json,
                usage_stats_json = excluded.usage_stats_json,
                providers_by_day_json = excluded.providers_by_day_json,
                evolution_stage = excluded.evolution_stage,
                evolution_branch = excluded.evolution_branch,
                album_json = excluded.album_json,
                album_records_json = excluded.album_records_json,
                owned_items_json = excluded.owned_items_json,
                equipped_cosmetic = excluded.equipped_cosmetic,
                equipped_food_skin = excluded.equipped_food_skin,
                furniture_json = excluded.furniture_json,
                prestige_count = excluded.prestige_count,
                xp_bonus_multiplier = excluded.xp_bonus_multiplier,
                pending_evolution_json = excluded.pending_evolution_json,
                last_reconciled_unix = excluded.last_reconciled_unix,
                updated_at_unix = excluded.updated_at_unix",
            params![
                state.current_day.to_string(),
                state.food_earned_today as i64,
                serde_json::to_string(&state.food_earned_by_day).unwrap_or_default(),
                state.banked_tokens_today,
                serde_json::to_string(&state.banked_tokens_by_day).unwrap_or_default(),
                state.pantry as i64,
                state.food_inventory as i64,
                state.fullness,
                state.xp,
                state.sparks as i64,
                state.streak_days as i64,
                state.streak_freezes as i64,
                state.last_activity_day.map(|day| day.to_string()),
                state.weekly_food_earned as i64,
                state.weekly_target as i64,
                bool_to_i64(state.weekly_milestone_claimed),
                serde_json::to_string(&state.daily_quest).unwrap_or_default(),
                serde_json::to_string(&state.usage_stats).unwrap_or_default(),
                serde_json::to_string(&state.providers_by_day).unwrap_or_default(),
                serde_json::to_string(&state.evolution_stage).unwrap_or_default(),
                serde_json::to_string(&state.evolution_branch).unwrap_or_default(),
                serde_json::to_string(&state.album).unwrap_or_default(),
                serde_json::to_string(&state.album_records).unwrap_or_default(),
                serde_json::to_string(&state.owned_items).unwrap_or_default(),
                state.equipped_cosmetic.clone(),
                state.equipped_food_skin.clone(),
                serde_json::to_string(&state.furniture).unwrap_or_default(),
                state.prestige_count as i64,
                state.xp_bonus_multiplier,
                state
                    .pending_evolution
                    .as_ref()
                    .and_then(|event| serde_json::to_string(event).ok()),
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
                        codex_cli_enabled, openai_enabled, pet_size, monitor_index,
                        wayland_fallback, tracking_paused, calm_mode
                 FROM app_settings
                 WHERE id = 1",
                [],
                |row| {
                    Ok(AppSettings {
                        onboarding_complete: row.get::<_, i64>(0)? != 0,
                        starter_egg: row.get(1)?,
                        claude_code_enabled: row.get::<_, i64>(2)? != 0,
                        codex_cli_enabled: row.get::<_, i64>(3)? != 0,
                        openai_enabled: row.get::<_, i64>(4)? != 0,
                        pet_size: row.get::<_, i64>(5)? as u32,
                        monitor_index: row.get::<_, i64>(6)? as u32,
                        wayland_fallback: row.get::<_, i64>(7)? != 0,
                        tracking_paused: row.get::<_, i64>(8)? != 0,
                        calm_mode: row.get::<_, i64>(9)? != 0,
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
                codex_cli_enabled, openai_enabled, pet_size, monitor_index,
                wayland_fallback, tracking_paused, calm_mode, updated_at_unix
             )
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                onboarding_complete = excluded.onboarding_complete,
                starter_egg = excluded.starter_egg,
                claude_code_enabled = excluded.claude_code_enabled,
                codex_cli_enabled = excluded.codex_cli_enabled,
                openai_enabled = excluded.openai_enabled,
                pet_size = excluded.pet_size,
                monitor_index = excluded.monitor_index,
                wayland_fallback = excluded.wayland_fallback,
                tracking_paused = excluded.tracking_paused,
                calm_mode = excluded.calm_mode,
                updated_at_unix = excluded.updated_at_unix",
            params![
                bool_to_i64(settings.onboarding_complete),
                settings.starter_egg,
                bool_to_i64(settings.claude_code_enabled),
                bool_to_i64(settings.codex_cli_enabled),
                bool_to_i64(settings.openai_enabled),
                settings.pet_size as i64,
                settings.monitor_index as i64,
                bool_to_i64(settings.wayland_fallback),
                bool_to_i64(settings.tracking_paused),
                bool_to_i64(settings.calm_mode),
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

fn parse_date_column(raw: &str, column: usize) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Text,
            Box::new(err),
        )
    })
}

fn json_or_default<T, F>(raw: &str, fallback: F) -> T
where
    T: serde::de::DeserializeOwned,
    F: FnOnce() -> T,
{
    if raw.is_empty() {
        return fallback();
    }
    serde_json::from_str(raw).unwrap_or_else(|_| fallback())
}

fn json_or_default_string<T>(raw: &str, fallback: T) -> T
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(raw).unwrap_or(fallback)
}

fn migrate_economy_state_columns(conn: &Connection) -> rusqlite::Result<()> {
    let columns = [
        ("sparks", "INTEGER NOT NULL DEFAULT 0"),
        ("food_earned_by_day_json", "TEXT NOT NULL DEFAULT ''"),
        ("banked_tokens_by_day_json", "TEXT NOT NULL DEFAULT ''"),
        ("streak_days", "INTEGER NOT NULL DEFAULT 0"),
        ("streak_freezes", "INTEGER NOT NULL DEFAULT 0"),
        ("last_activity_day", "TEXT"),
        ("weekly_food_earned", "INTEGER NOT NULL DEFAULT 0"),
        ("weekly_target", "INTEGER NOT NULL DEFAULT 7"),
        ("weekly_milestone_claimed", "INTEGER NOT NULL DEFAULT 0"),
        ("daily_quest_json", "TEXT NOT NULL DEFAULT ''"),
        ("usage_stats_json", "TEXT NOT NULL DEFAULT ''"),
        ("providers_by_day_json", "TEXT NOT NULL DEFAULT ''"),
        ("evolution_stage", "TEXT NOT NULL DEFAULT 'Egg'"),
        ("evolution_branch", "TEXT NOT NULL DEFAULT 'Sprout'"),
        ("album_json", "TEXT NOT NULL DEFAULT ''"),
        ("album_records_json", "TEXT NOT NULL DEFAULT ''"),
        ("owned_items_json", "TEXT NOT NULL DEFAULT ''"),
        ("equipped_cosmetic", "TEXT"),
        ("equipped_food_skin", "TEXT"),
        ("furniture_json", "TEXT NOT NULL DEFAULT ''"),
        ("prestige_count", "INTEGER NOT NULL DEFAULT 0"),
        ("xp_bonus_multiplier", "REAL NOT NULL DEFAULT 1.0"),
        ("pending_evolution_json", "TEXT"),
    ];

    for (name, definition) in columns {
        if !table_has_column(conn, "economy_state", name)? {
            conn.execute(
                &format!("ALTER TABLE economy_state ADD COLUMN {name} {definition}"),
                [],
            )?;
        }
    }

    Ok(())
}

fn migrate_app_settings_columns(conn: &Connection) -> rusqlite::Result<()> {
    let columns = [
        ("codex_cli_enabled", "INTEGER NOT NULL DEFAULT 0"),
        ("openai_enabled", "INTEGER NOT NULL DEFAULT 0"),
        ("calm_mode", "INTEGER NOT NULL DEFAULT 0"),
    ];

    for (name, definition) in columns {
        if !table_has_column(conn, "app_settings", name)? {
            conn.execute(
                &format!("ALTER TABLE app_settings ADD COLUMN {name} {definition}"),
                [],
            )?;
        }
    }

    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
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
            codex_cli_enabled: true,
            openai_enabled: true,
            pet_size: 125,
            monitor_index: 1,
            wayland_fallback: true,
            tracking_paused: true,
            calm_mode: true,
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
