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

//! Clan Ranks and Skalds' Feats (permanent achievements with resource & glory rewards).

use crate::city::{BuildingDef, CityState, ResourceBundle};

/// Progressive Viking Clan rank unlocked by total Clan Glory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanRank {
    Scout,
    Huscarl,
    Jarl,
    HighKing,
}

impl ClanRank {
    /// Resolves the player's Clan Rank from their current Glory score.
    pub const fn from_glory(glory: u32) -> Self {
        if glory >= 600 {
            ClanRank::HighKing
        } else if glory >= 280 {
            ClanRank::Jarl
        } else if glory >= 100 {
            ClanRank::Huscarl
        } else {
            ClanRank::Scout
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
}

/// Static definition of a permanent Skald's Feat (achievement).
#[derive(Debug, Clone, Copy)]
pub struct AchievementDef {
    pub id: &'static str,
    pub title_key: &'static str,
    pub desc_key: &'static str,
    pub reward: ResourceBundle,
    pub glory: u32,
}

/// All 12 permanent Skalds' Feats.
pub const ACHIEVEMENTS: [AchievementDef; 12] = [
    AchievementDef {
        id: "ach_first_win",
        title_key: "ach.first_win.title",
        desc_key: "ach.first_win.desc",
        reward: ResourceBundle {
            wood: 40,
            stone: 40,
            gold: 50,
            mead: 10,
        },
        glory: 15,
    },
    AchievementDef {
        id: "ach_veteran",
        title_key: "ach.veteran.title",
        desc_key: "ach.veteran.desc",
        reward: ResourceBundle {
            wood: 120,
            stone: 120,
            gold: 150,
            mead: 30,
        },
        glory: 35,
    },
    AchievementDef {
        id: "ach_combo4",
        title_key: "ach.combo4.title",
        desc_key: "ach.combo4.desc",
        reward: ResourceBundle {
            wood: 60,
            stone: 60,
            gold: 90,
            mead: 20,
        },
        glory: 25,
    },
    AchievementDef {
        id: "ach_combo5",
        title_key: "ach.combo5.title",
        desc_key: "ach.combo5.desc",
        reward: ResourceBundle {
            wood: 100,
            stone: 100,
            gold: 160,
            mead: 35,
        },
        glory: 45,
    },
    AchievementDef {
        id: "ach_runes_500",
        title_key: "ach.runes_500.title",
        desc_key: "ach.runes_500.desc",
        reward: ResourceBundle {
            wood: 80,
            stone: 80,
            gold: 100,
            mead: 20,
        },
        glory: 25,
    },
    AchievementDef {
        id: "ach_runes_2000",
        title_key: "ach.runes_2000.title",
        desc_key: "ach.runes_2000.desc",
        reward: ResourceBundle {
            wood: 180,
            stone: 180,
            gold: 240,
            mead: 50,
        },
        glory: 55,
    },
    AchievementDef {
        id: "ach_first_boss",
        title_key: "ach.first_boss.title",
        desc_key: "ach.first_boss.desc",
        reward: ResourceBundle {
            wood: 140,
            stone: 140,
            gold: 200,
            mead: 45,
        },
        glory: 50,
    },
    AchievementDef {
        id: "ach_all_bosses",
        title_key: "ach.all_bosses.title",
        desc_key: "ach.all_bosses.desc",
        reward: ResourceBundle {
            wood: 300,
            stone: 300,
            gold: 450,
            mead: 100,
        },
        glory: 120,
    },
    AchievementDef {
        id: "ach_expedition_3",
        title_key: "ach.expedition_3.title",
        desc_key: "ach.expedition_3.desc",
        reward: ResourceBundle {
            wood: 100,
            stone: 100,
            gold: 140,
            mead: 30,
        },
        glory: 35,
    },
    AchievementDef {
        id: "ach_expedition_5",
        title_key: "ach.expedition_5.title",
        desc_key: "ach.expedition_5.desc",
        reward: ResourceBundle {
            wood: 200,
            stone: 200,
            gold: 280,
            mead: 60,
        },
        glory: 75,
    },
    AchievementDef {
        id: "ach_master_builder",
        title_key: "ach.master_builder.title",
        desc_key: "ach.master_builder.desc",
        reward: ResourceBundle {
            wood: 150,
            stone: 150,
            gold: 200,
            mead: 40,
        },
        glory: 50,
    },
    AchievementDef {
        id: "ach_daily_streak_3",
        title_key: "ach.daily_streak_3.title",
        desc_key: "ach.daily_streak_3.desc",
        reward: ResourceBundle {
            wood: 120,
            stone: 120,
            gold: 180,
            mead: 40,
        },
        glory: 40,
    },
];

/// Evaluates whether a Skald's Feat condition is met.
pub fn is_achievement_unlocked(
    ach_id: &str,
    city: &CityState,
    buildings: &[BuildingDef],
    total_bosses: usize,
) -> bool {
    match ach_id {
        "ach_first_win" => city.stats.total_victories >= 1,
        "ach_veteran" => city.stats.total_victories >= 10,
        "ach_combo4" => city.stats.best_combo >= 4,
        "ach_combo5" => city.stats.best_combo >= 5,
        "ach_runes_500" => city.stats.total_runes_cleared >= 500,
        "ach_runes_2000" => city.stats.total_runes_cleared >= 2000,
        "ach_first_boss" => city.stats.bosses_defeated >= 1 || !city.defeated_bosses.is_empty(),
        "ach_all_bosses" => total_bosses > 0 && city.defeated_bosses.len() >= total_bosses,
        "ach_expedition_3" => city.stats.best_expedition_wave >= 3,
        "ach_expedition_5" => city.stats.best_expedition_wave >= 5,
        "ach_master_builder" => {
            !buildings.is_empty() && buildings.iter().all(|b| city.building_tier(&b.id) >= 1)
        }
        "ach_daily_streak_3" => city.stats.daily_streak >= 3,
        _ => false,
    }
}
