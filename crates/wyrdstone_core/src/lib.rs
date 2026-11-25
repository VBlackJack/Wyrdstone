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
pub mod retention;
pub mod save;
pub mod text;
pub mod trials;
pub mod turn;
pub mod validate;

pub use achievements::{achievement_progress, is_achievement_unlocked, AchievementDef, ClanRank};
pub use balance::{
    BalanceConfig, BlitzTuning, BossTuning, DailyTuning, ExpeditionTuning, SessionTuning,
};
pub use board::{
    ActiveTool, Board, Booster, BoosterFusionKind, Cell, CellPos, FallMove, MatchStepOutcome,
    PreparedClear, RuneType,
};
pub use boss::{
    BossDef, BossRiposteKind, BossState, GauntletRun, BOSS_RAID_BUILDING_ID, GAUNTLET_BUILDING_ID,
    GAUNTLET_CLEAR_GLORY, GAUNTLET_HP_SCALE_PCT, GAUNTLET_STAGE_BONUS_MOVES,
    GAUNTLET_STAGE_PARTIAL_GLORY, GAUNTLET_STARTING_MOVES,
};
pub use city::{
    validate_city_content, AmuletDef, AmuletEffect, BuildingDef, CityBonuses, CityState, ClanStats,
    CollectionDef, DivineSynergy, QuestCondition, QuestDef, ResourceBundle, ToolInventory,
    ToolRecipeDef, WarDraught, HUB_BUILDING_ID, MAX_DAILY_STREAK_CYCLE,
};
pub use expedition::{
    ExpeditionBoon, ExpeditionOath, ExpeditionRelic, ExpeditionRun, EXPEDITION_BUILDING_ID,
};
pub use i18n::{I18nCatalog, Language};
pub use level::{
    BlitzMedal, DailyBlessing, LevelDef, LevelOutcome, LevelSession, Objective, ObjectiveProgress,
    BLITZ_BUILDING_ID, DAILY_TRIAL_BUILDING_ID,
};
pub use patron::{PatronGod, FURY_CHARGE_MAX};
pub use retention::{
    last_stand_cost, skald_pass_tier_reward, DailyBounty, DailyBountyBoard, DailyBountyKind,
    OfflineVaultSummary, PlayMode, SkaldPassState, ValhallaFlameTier, LAST_STAND_EXTRA_MOVES,
    LAST_STAND_EXTRA_SECS, LAST_STAND_GOLD_COST, LAST_STAND_MEAD_COST,
    OFFLINE_VAULT_BASE_MAX_HOURS, OFFLINE_VAULT_CYCLE_SECS,
    OFFLINE_VAULT_EXTRA_HOURS_PER_HALL_TIER, OFFLINE_VAULT_MIN_ABSENCE_SECS, SKALD_PASS_TIERS,
    SKALD_PASS_XP_PER_TIER, SKALD_XP_BOARD_WIN, SKALD_XP_BOSS_WIN, SKALD_XP_DAILY_WIN,
};
pub use save::{GameDataBundle, SaveData};
pub use text::{
    format_achievement_desc, format_blessing_desc, format_boon_desc, format_boss_desc,
    format_building_bonus, format_patron_desc, format_rank_perk,
};
pub use trials::{TrialDef, TRIAL_BUILDING_ID, TRIAL_PAR_BONUS_GLORY};
pub use turn::{
    BonusBlast, BossHit, ClearPreview, ClearReport, ClearStatsDelta, FuryOutcome, RelicReport,
    SwapRejected, ToolRejected, Turn, TurnEnd, TurnOrigin, TurnStep, VICTORY_SKIP_MAX_DETONATIONS,
};
