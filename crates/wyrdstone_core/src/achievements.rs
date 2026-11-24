/*
 * Copyright 2026 Julien Bombled
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Clan Ranks and Skalds' Feats (permanent achievements loaded from RON with live progress).

use crate::balance::BalanceConfig;
use crate::city::{BuildingDef, CityState, ResourceBundle};
use serde::{Deserialize, Serialize};

/// Default Glory required to reach the Huscarl Clan Rank.
pub const DEFAULT_CLAN_RANK_HUSCARL_GLORY: u32 = 100;
/// Default Glory required to reach the Jarl Clan Rank.
pub const DEFAULT_CLAN_RANK_JARL_GLORY: u32 = 280;
/// Default Glory required to reach the High King Clan Rank.
pub const DEFAULT_CLAN_RANK_HIGH_KING_GLORY: u32 = 600;

/// Progressive Viking Clan rank unlocked by total Clan Glory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanRank {
    Scout,
    Huscarl,
    Jarl,
    HighKing,
}

impl ClanRank {
    /// Resolves the player's Clan Rank using default Glory thresholds.
    pub const fn from_glory(glory: u32) -> Self {
        if glory >= DEFAULT_CLAN_RANK_HIGH_KING_GLORY {
            ClanRank::HighKing
        } else if glory >= DEFAULT_CLAN_RANK_JARL_GLORY {
            ClanRank::Jarl
        } else if glory >= DEFAULT_CLAN_RANK_HUSCARL_GLORY {
            ClanRank::Huscarl
        } else {
            ClanRank::Scout
        }
    }

    /// Resolves the player's Clan Rank using externalized thresholds from `BalanceConfig`.
    pub const fn from_glory_with_balance(glory: u32, balance: &BalanceConfig) -> Self {
        if glory >= balance.clan_rank_high_king_glory {
            ClanRank::HighKing
        } else if glory >= balance.clan_rank_jarl_glory {
            ClanRank::Jarl
        } else if glory >= balance.clan_rank_huscarl_glory {
            ClanRank::Huscarl
        } else {
            ClanRank::Scout
        }
    }

    /// Passive spoils bonus percentage granted by this Clan Rank.
    pub const fn resource_bonus_pct(self) -> u32 {
        match self {
            ClanRank::Scout => 0,
            ClanRank::Huscarl => 5,
            ClanRank::Jarl => 10,
            ClanRank::HighKing => 15,
        }
    }

    /// Passive spoils bonus percentage of this Clan Rank using externalized balance values.
    pub const fn resource_bonus_pct_with(self, balance: &BalanceConfig) -> u32 {
        match self {
            ClanRank::Scout => balance.clan_rank_scout_resource_bonus_pct,
            ClanRank::Huscarl => balance.clan_rank_huscarl_resource_bonus_pct,
            ClanRank::Jarl => balance.clan_rank_jarl_resource_bonus_pct,
            ClanRank::HighKing => balance.clan_rank_high_king_resource_bonus_pct,
        }
    }

    /// Starting Divine Fury headstart of this Clan Rank using externalized balance values.
    pub const fn starting_fury_with(self, balance: &BalanceConfig) -> u32 {
        match self {
            ClanRank::Scout => balance.clan_rank_scout_starting_fury,
            ClanRank::Huscarl => balance.clan_rank_huscarl_starting_fury,
            ClanRank::Jarl => balance.clan_rank_jarl_starting_fury,
            ClanRank::HighKing => balance.clan_rank_high_king_starting_fury,
        }
    }

    /// Starting Divine Fury headstart granted at the start of every board.
    pub const fn starting_fury(self) -> u32 {
        match self {
            ClanRank::Scout => 0,
            ClanRank::Huscarl => 10,
            ClanRank::Jarl => 20,
            ClanRank::HighKing => 30,
        }
    }

    pub const fn i18n_key(self) -> &'static str {
        match self {
            ClanRank::Scout => "rank.scout",
            ClanRank::Huscarl => "rank.huscarl",
            ClanRank::Jarl => "rank.jarl",
            ClanRank::HighKing => "rank.high_king",
        }
    }

    pub const fn perk_key(self) -> &'static str {
        match self {
            ClanRank::Scout => "rank.scout.perk",
            ClanRank::Huscarl => "rank.huscarl.perk",
            ClanRank::Jarl => "rank.jarl.perk",
            ClanRank::HighKing => "rank.high_king.perk",
        }
    }
}

/// Unlock condition of a Skald's Feat, evaluated against the live city state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AchievementCondition {
    /// Lifetime victories reach the threshold.
    TotalVictories(u32),
    /// Best cascade combo reaches the threshold.
    BestCombo(u32),
    /// Lifetime cleared runes reach the threshold.
    RunesCleared(u64),
    /// Distinct or counted boss victories reach the threshold.
    BossesDefeated(u32),
    /// Every boss of the roster is defeated.
    AllBossesDefeated,
    /// Best Endless Drakkar Expedition wave reaches the threshold.
    BestExpeditionWave(u32),
    /// Every building of the village is rebuilt at least once.
    AllBuildingsBuilt,
    /// Current Daily Trial streak reaches the threshold.
    DailyStreak(u32),
}

impl AchievementCondition {
    /// Numeric threshold carried by the condition, when it has one.
    pub const fn threshold(self) -> Option<u64> {
        match self {
            AchievementCondition::TotalVictories(n)
            | AchievementCondition::BossesDefeated(n)
            | AchievementCondition::BestCombo(n)
            | AchievementCondition::BestExpeditionWave(n)
            | AchievementCondition::DailyStreak(n) => Some(n as u64),
            AchievementCondition::RunesCleared(n) => Some(n),
            AchievementCondition::AllBossesDefeated | AchievementCondition::AllBuildingsBuilt => {
                None
            }
        }
    }
}

/// Static definition of a permanent Skald's Feat loaded from `assets/data/achievements.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementDef {
    pub id: String,
    pub title_key: String,
    pub desc_key: String,
    pub reward: ResourceBundle,
    pub glory: u32,
    /// Unlock condition; a feat without one never unlocks (reported by content validation).
    #[serde(default)]
    pub condition: Option<AchievementCondition>,
}

/// Returns `(current, target)` numeric progress for a Skald's Feat so UI can display progress badges.
pub fn achievement_progress(
    def: &AchievementDef,
    city: &CityState,
    buildings: &[BuildingDef],
    total_bosses: usize,
) -> (u64, u64) {
    let Some(condition) = def.condition else {
        return (0, 1);
    };
    match condition {
        AchievementCondition::TotalVictories(n) => (
            u64::from(city.stats.total_victories).min(u64::from(n)),
            u64::from(n),
        ),
        AchievementCondition::BestCombo(n) => (
            u64::from(city.stats.best_combo).min(u64::from(n)),
            u64::from(n),
        ),
        AchievementCondition::RunesCleared(n) => (city.stats.total_runes_cleared.min(n), n),
        AchievementCondition::BossesDefeated(n) => {
            let done = u64::from(city.stats.bosses_defeated).max(city.defeated_bosses.len() as u64);
            (done.min(u64::from(n)), u64::from(n))
        }
        AchievementCondition::AllBossesDefeated => {
            let target = total_bosses as u64;
            let done = (city.defeated_bosses.len() as u64).min(target);
            (done, target)
        }
        AchievementCondition::BestExpeditionWave(n) => (
            u64::from(city.stats.best_expedition_wave).min(u64::from(n)),
            u64::from(n),
        ),
        AchievementCondition::AllBuildingsBuilt => {
            let target = buildings.len() as u64;
            let done = buildings
                .iter()
                .filter(|b| city.building_tier(&b.id) >= 1)
                .count() as u64;
            (done, target)
        }
        AchievementCondition::DailyStreak(n) => (
            u64::from(city.stats.best_daily_streak.max(city.stats.daily_streak)).min(u64::from(n)),
            u64::from(n),
        ),
    }
}

/// Evaluates whether a Skald's Feat condition is met.
pub fn is_achievement_unlocked(
    def: &AchievementDef,
    city: &CityState,
    buildings: &[BuildingDef],
    total_bosses: usize,
) -> bool {
    let (done, target) = achievement_progress(def, city, buildings, total_bosses);
    target > 0 && done >= target
}
