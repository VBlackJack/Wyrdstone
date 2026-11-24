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

//! Core game logic for Wyrdstone, independent of rendering and platform layers.

pub mod achievements;
pub mod balance;
pub mod board;
pub mod boss;
pub mod city;
pub mod expedition;
pub mod i18n;
pub mod level;
pub mod patron;
pub mod save;
pub mod validate;

pub use achievements::{achievement_progress, is_achievement_unlocked, AchievementDef, ClanRank};
pub use balance::{BalanceConfig, BossTuning, DailyTuning, ExpeditionTuning, SessionTuning};
pub use board::{
    ActiveTool, Board, Booster, Cell, CellPos, FallMove, MatchStepOutcome, PreparedClear, RuneType,
};
pub use boss::{BossDef, BossRiposteKind, BossState, BOSS_RAID_BUILDING_ID};
pub use city::{
    validate_city_content, AmuletDef, AmuletEffect, BuildingDef, CityBonuses, CityState, ClanStats,
    CollectionDef, QuestCondition, QuestDef, ResourceBundle, ToolInventory, ToolRecipeDef,
    HUB_BUILDING_ID,
};
pub use expedition::{ExpeditionBoon, ExpeditionRun, EXPEDITION_BUILDING_ID};
pub use i18n::{I18nCatalog, Language};
pub use level::{
    DailyBlessing, LevelDef, LevelOutcome, LevelSession, Objective, ObjectiveProgress,
    DAILY_TRIAL_BUILDING_ID,
};
pub use patron::{PatronGod, FURY_CHARGE_MAX};
pub use save::{GameDataBundle, SaveData};
