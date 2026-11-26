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
    GAUNTLET_STAGE_PARTIAL_GLORY, GAUNTLET_STARTING_MOVES, GAUNTLET_UNLOCK_GLORY,
};
pub use city::{
    validate_city_content, AmuletDef, AmuletEffect, BuildingDef, CityBonuses, CityState, ClanStats,
    CollectionDef, DivineSynergy, QuestCondition, QuestDef, ResourceBundle, ResourceKind,
    ToolInventory, ToolRecipeDef, WarDraught, COLLECTION_RECHARGE_REWARD_PCT, HUB_BUILDING_ID,
    MASTERY_HARVEST_PCT_AT_4_STARS, MASTERY_HARVEST_PCT_AT_5_STARS, MASTERY_SPOILS_PCT_PER_STAR,
    MAX_DAILY_STREAK_CYCLE, MAX_MASTERY_STARS,
};
pub use expedition::{
    ExpeditionBoon, ExpeditionOath, ExpeditionRelic, ExpeditionRun, EXPEDITION_BUILDING_ID,
};
pub use i18n::{I18nCatalog, Language};
pub use level::{
    BlitzMedal, DailyBlessing, LevelDef, LevelOutcome, LevelSession, Objective, ObjectiveProgress,
    ThreatRating, BLITZ_BUILDING_ID, DAILY_TRIAL_BUILDING_ID,
};
pub use patron::{PatronGod, FURY_CHARGE_MAX};
pub use retention::{
    last_stand_cost, newly_unlocked_modes, patron_devotion_upgrade_cost,
    realm_ascension_glory_reward, skald_pass_tier_reward, skald_pass_tier_reward_for_cycle,
    weekly_challenge_week_index, weekly_medal_for_score, weekly_medal_name_key,
    weekly_medal_reward, wonder_boss_damage_bonus_pct, wonder_extra_max_energy,
    wonder_glory_reward, wonder_harvest_bonus_pct, wonder_milestone_tool, wonder_spoils_bonus_pct,
    wonder_starting_fury, wonder_upgrade_cost, DailyBounty, DailyBountyBoard, DailyBountyKind,
    EndlessDecreeKind, EndlessDecreeState, OfflineVaultSummary, PlayMode, RealmTheme,
    SkaldPassState, ValhallaFlameTier, ValkyrieAidTier, WeeklyChallengeState,
    LAST_STAND_EXTRA_MOVES, LAST_STAND_EXTRA_SECS, LAST_STAND_GOLD_COST, LAST_STAND_MEAD_COST,
    LEGEND_RANK_GLORY_STEP, LEGEND_SPOILS_MAX_BONUS_PCT, LEGEND_SPOILS_PCT_PER_LEVEL,
    MAX_PATRON_DEVOTION_TIER, MAX_VALKYRIE_FAIL_STREAK, OFFLINE_VAULT_BASE_MAX_HOURS,
    OFFLINE_VAULT_CYCLE_SECS, OFFLINE_VAULT_EXTRA_HOURS_PER_HALL_TIER,
    OFFLINE_VAULT_MIN_ABSENCE_SECS, PATRON_DEVOTION_BOSS_DMG_PCT_AT_TIER5,
    PATRON_DEVOTION_FURY_PCT_PER_TIER, PATRON_DEVOTION_SCORE_PCT_PER_TIER,
    PATRON_DEVOTION_START_FURY_AT_TIER3, REALM_ASCENSION_KEEP_RESOURCE_PCT,
    REALM_BOSS_DMG_PCT_PER_ERA, REALM_BUILDING_COST_PCT_PER_ERA, REALM_ENEMY_BOSS_HP_PCT_PER_ERA,
    REALM_ENEMY_SCALING_MAX_ERA, REALM_MAX_SCALING_ERA, REALM_REWARD_PCT_PER_ERA,
    REALM_SCORE_PCT_PER_ERA, REALM_SCORE_TARGET_PCT_PER_ERA, REALM_SPOILS_PCT_PER_ERA,
    SKALD_PASS_CYCLE_BONUS_PCT, SKALD_PASS_CYCLE_MAX_BONUS_PCT, SKALD_PASS_TIERS,
    SKALD_PASS_XP_PER_TIER, SKALD_XP_BOARD_WIN, SKALD_XP_BOSS_WIN, SKALD_XP_DAILY_WIN,
    VALKYRIE_AID_TIER1_FAILS, VALKYRIE_AID_TIER1_MOVES, VALKYRIE_AID_TIER1_SECS,
    VALKYRIE_AID_TIER2_FAILS, VALKYRIE_AID_TIER2_FURY, VALKYRIE_AID_TIER2_MOVES,
    VALKYRIE_AID_TIER2_SECS, WEEKLY_CHALLENGE_BUILDING_ID, WEEKLY_MEDAL_THRESHOLDS,
    WONDER_MAX_STAGE,
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
