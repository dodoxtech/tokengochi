//! Pet state machine: mood, hunger decay, evolution.
//!
//! See `docs/knowledge/game-economy.md` §3-4 and `docs/architecture.md`
//! §Data Flow. Behavior is tracked in `docs/tasks/done/0004-economy-engine-core.md`,
//! `docs/tasks/backlog/0005-sprite-renderer-behavior-ai.md`,
//! `docs/tasks/done/0009-evolution-streaks-quests.md`, and
//! `docs/tasks/backlog/0010-cosmetics-shop-collection-album.md`.

/// Evolution stages, in order. See `docs/knowledge/game-economy.md` §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EvolutionStage {
    Egg,
    Hatchling,
    Juvenile,
    Adult,
    Elder,
}

/// Cosmetic branch picked from usage-pattern stats. Branches are deterministic
/// so the same history always produces the same form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EvolutionBranch {
    Sprout,
    Nocturnal,
    Sprinter,
    Scholar,
    Chimera,
}

impl EvolutionBranch {
    pub fn as_album_key(self, stage: EvolutionStage) -> String {
        format!("{stage:?}:{self:?}").to_ascii_lowercase()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsagePatternStats {
    pub usage_days: u32,
    pub night_events: u32,
    pub session_count: u32,
    pub short_sessions: u32,
    pub long_sessions: u32,
    pub multi_provider_days: u32,
}

impl UsagePatternStats {
    pub fn record_day(&mut self, sample: UsagePatternSample) {
        self.usage_days += 1;
        self.night_events += sample.night_events;
        self.session_count += sample.session_count;
        self.short_sessions += sample.short_sessions;
        self.long_sessions += sample.long_sessions;
        if sample.provider_count > 1 {
            self.multi_provider_days += 1;
        }
    }

    pub fn selected_branch(&self) -> EvolutionBranch {
        if self.multi_provider_days >= 5 {
            return EvolutionBranch::Chimera;
        }
        if self.usage_days > 0 && self.night_events * 2 >= self.usage_days {
            return EvolutionBranch::Nocturnal;
        }
        if self.session_count > 0 && self.short_sessions * 2 >= self.session_count {
            return EvolutionBranch::Sprinter;
        }
        if self.long_sessions * 2 >= self.session_count.max(1) {
            return EvolutionBranch::Scholar;
        }
        EvolutionBranch::Sprout
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsagePatternSample {
    pub night_events: u32,
    pub session_count: u32,
    pub short_sessions: u32,
    pub long_sessions: u32,
    pub provider_count: u32,
}

impl UsagePatternSample {
    pub fn single_event(hour: u32, provider_count: u32) -> Self {
        let is_night = !(6..21).contains(&hour);
        Self {
            night_events: u32::from(is_night),
            session_count: 1,
            short_sessions: 1,
            long_sessions: 0,
            provider_count: provider_count.max(1),
        }
    }
}

/// One-shot presentation cue consumed by overlay/dashboard UI. The Rust state
/// records the fact that an evolution happened; rendering decides how to play
/// the celebration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvolutionEvent {
    pub stage: EvolutionStage,
    pub branch: EvolutionBranch,
    pub level: u32,
    pub album_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShopItemKind {
    Cosmetic,
    FoodSkin,
    Furniture,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopItem {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: ShopItemKind,
    pub price_sparks: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FurniturePlacement {
    pub item_id: String,
    pub x: f64,
    #[serde(default = "default_furniture_visible")]
    pub visible: bool,
}

fn default_furniture_visible() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumRecord {
    pub key: String,
    pub stage: EvolutionStage,
    pub branch: EvolutionBranch,
    pub reached_day: String,
    pub level: u32,
    pub xp: f64,
    pub sparks: u32,
    pub prestige_count: u32,
}

pub const SHOP_CATALOG: &[ShopItem] = &[
    ShopItem {
        id: "hat-leaf",
        label: "Leaf Cap",
        kind: ShopItemKind::Cosmetic,
        price_sparks: 5,
    },
    ShopItem {
        id: "hat-mushroom",
        label: "Mushroom Cap",
        kind: ShopItemKind::Cosmetic,
        price_sparks: 12,
    },
    ShopItem {
        id: "food-sushi",
        label: "Sushi Food",
        kind: ShopItemKind::FoodSkin,
        price_sparks: 10,
    },
    ShopItem {
        id: "food-banh-mi",
        label: "Banh Mi Food",
        kind: ShopItemKind::FoodSkin,
        price_sparks: 10,
    },
    ShopItem {
        id: "furniture-bed",
        label: "Tiny Bed",
        kind: ShopItemKind::Furniture,
        price_sparks: 15,
    },
    ShopItem {
        id: "furniture-plant",
        label: "Desk Plant",
        kind: ShopItemKind::Furniture,
        price_sparks: 15,
    },
];

pub fn shop_item(id: &str) -> Option<&'static ShopItem> {
    SHOP_CATALOG.iter().find(|item| item.id == id)
}

pub fn album_key(stage: EvolutionStage, branch: EvolutionBranch, prestige_count: u32) -> String {
    format!("{}:p{prestige_count}", branch.as_album_key(stage))
}

pub fn stage_for_level(level: u32) -> EvolutionStage {
    match level {
        0..=2 => EvolutionStage::Egg,
        3..=9 => EvolutionStage::Hatchling,
        10..=24 => EvolutionStage::Juvenile,
        25..=44 => EvolutionStage::Adult,
        _ => EvolutionStage::Elder,
    }
}

/// Discrete mood bands, driven by Fullness. See
/// `docs/knowledge/game-economy.md` §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Mood {
    Full,
    Content,
    Peckish,
    Hungry,
    /// Fullness has (nearly) bottomed out: the pet hibernates - sleeps, sad
    /// animation, gains zero XP - but never dies and never loses levels
    /// (guilt-free by design, see `docs/knowledge/game-economy.md` §3).
    Starving,
}
