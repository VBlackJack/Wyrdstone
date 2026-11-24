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

//! Externalized balance and timing configuration loaded from RON.

use crate::achievements::{
    DEFAULT_CLAN_RANK_HIGH_KING_GLORY, DEFAULT_CLAN_RANK_HUSCARL_GLORY,
    DEFAULT_CLAN_RANK_JARL_GLORY,
};
use crate::board::ActiveTool;
use crate::city::ResourceBundle;
use crate::expedition::ExpeditionBoon;
use serde::{Deserialize, Serialize};

/// Default consecutive-win threshold that grants starting boosters.
const DEFAULT_WIN_STREAK_BOOSTER_THRESHOLD: u32 = 2;
/// Default number of starting boosters granted once the win streak threshold is reached.
const DEFAULT_WIN_STREAK_STARTING_BOOSTERS: u32 = 1;
/// Default Clan Rank spoils bonus (percent) per rank.
const DEFAULT_CLAN_RANK_SCOUT_RESOURCE_BONUS_PCT: u32 = 0;
const DEFAULT_CLAN_RANK_HUSCARL_RESOURCE_BONUS_PCT: u32 = 5;
const DEFAULT_CLAN_RANK_JARL_RESOURCE_BONUS_PCT: u32 = 10;
const DEFAULT_CLAN_RANK_HIGH_KING_RESOURCE_BONUS_PCT: u32 = 15;
/// Default Clan Rank starting Divine Fury per rank.
const DEFAULT_CLAN_RANK_SCOUT_STARTING_FURY: u32 = 0;
const DEFAULT_CLAN_RANK_HUSCARL_STARTING_FURY: u32 = 10;
const DEFAULT_CLAN_RANK_JARL_STARTING_FURY: u32 = 20;
const DEFAULT_CLAN_RANK_HIGH_KING_STARTING_FURY: u32 = 30;
/// Default Daily Trial streak reward bonus (percent) per consecutive day beyond the first.
pub(crate) const DEFAULT_DAILY_STREAK_BONUS_PCT_PER_DAY: u32 = 10;
/// Default Glory granted per expedition wave reached.
const DEFAULT_EXPEDITION_GLORY_PER_WAVE: u32 = 5;

// Session tuning defaults.
const DEFAULT_BLESSING_ODIN_FURY_PCT: u32 = 20;
const DEFAULT_BLESSING_NJORD_RESOURCE_PCT: u32 = 15;
const DEFAULT_BLESSING_FENRIR_BOSS_DAMAGE_PCT: u32 = 25;
const DEFAULT_BLESSING_IDUNN_EXTRA_MOVES: u32 = 1;
const DEFAULT_FREYA_FURY_RESOURCE_PCT: u32 = 25;
const DEFAULT_ODIN_FURY_EXTRA_MOVES: u32 = 2;
const DEFAULT_THREE_STAR_MOVES_RATIO: f32 = 0.30;
const DEFAULT_TWO_STAR_MOVES_RATIO: f32 = 0.12;
const DEFAULT_THOR_FURY_STRIKE_CELLS: usize = 4;
const DEFAULT_LOKI_FURY_BOOSTERS: u32 = 2;

// Daily Trial tuning defaults.
const DEFAULT_DAILY_GLACIAL_COLLECT_MIN: u32 = 20;
const DEFAULT_DAILY_GLACIAL_COLLECT_MAX: u32 = 26;
const DEFAULT_DAILY_GLACIAL_FROST_CHANCE: f64 = 0.5;
const DEFAULT_DAILY_RELIC_COUNT_MIN: usize = 2;
const DEFAULT_DAILY_RELIC_COUNT_MAX: usize = 3;
const DEFAULT_DAILY_BLITZ_TIME_MIN: u32 = 95;
const DEFAULT_DAILY_BLITZ_TIME_MAX: u32 = 115;
const DEFAULT_DAILY_BLITZ_SCORE_MIN: u32 = 4800;
const DEFAULT_DAILY_BLITZ_SCORE_MAX: u32 = 5600;
const DEFAULT_DAILY_BLITZ_COLLECT_MIN: u32 = 22;
const DEFAULT_DAILY_BLITZ_COLLECT_MAX: u32 = 26;
const DEFAULT_DAILY_PORTAL_SCORE_TARGET: u32 = 5000;
const DEFAULT_DAILY_MAX_MOVES_MIN: u32 = 24;
const DEFAULT_DAILY_MAX_MOVES_MAX: u32 = 28;
const DEFAULT_DAILY_REWARD_WOOD: u32 = 140;
const DEFAULT_DAILY_REWARD_STONE: u32 = 140;
const DEFAULT_DAILY_REWARD_GOLD: u32 = 180;
const DEFAULT_DAILY_REWARD_MEAD: u32 = 40;

// Expedition tuning defaults.
const DEFAULT_EXPEDITION_WAVE_ONE_RUNE_TYPES: usize = 5;
const DEFAULT_EXPEDITION_ICE_BASE: u32 = 6;
const DEFAULT_EXPEDITION_ICE_PER_WAVE: u32 = 2;
const DEFAULT_EXPEDITION_ICE_CAP: u32 = 16;
const DEFAULT_EXPEDITION_RUNE_TARGET_BASE: u32 = 14;
const DEFAULT_EXPEDITION_RUNE_TARGET_PER_WAVE: u32 = 3;
const DEFAULT_EXPEDITION_RUNE_TARGET_CAP: u32 = 36;
const DEFAULT_EXPEDITION_SCORE_BASE: u32 = 3000;
const DEFAULT_EXPEDITION_SCORE_PER_WAVE: u32 = 900;
const DEFAULT_EXPEDITION_SCORE_WAVE_INTERVAL: u32 = 3;
const DEFAULT_EXPEDITION_BASE_MOVES: u32 = 22;
const DEFAULT_EXPEDITION_MOVES_CAP: u32 = 36;
const DEFAULT_EXPEDITION_DOUBLE_ICE_FROM_WAVE: u32 = 3;
const DEFAULT_EXPEDITION_CHAINS_FROM_WAVE: u32 = 2;
const DEFAULT_EXPEDITION_EXTRA_CHAINS_FROM_WAVE: u32 = 4;
const DEFAULT_EXPEDITION_CRATES_FROM_WAVE: u32 = 3;
const DEFAULT_EXPEDITION_PORTALS_FROM_WAVE: u32 = 4;
const DEFAULT_EXPEDITION_FOG_FROM_WAVE: u32 = 4;
const DEFAULT_EXPEDITION_FROST_FROM_WAVE: u32 = 5;
const DEFAULT_EXPEDITION_CURSE_FROM_WAVE: u32 = 6;
const DEFAULT_EXPEDITION_CURSE_TURNS: u8 = 6;
const DEFAULT_EXPEDITION_REWARD_BASE: ResourceBundle = ResourceBundle {
    wood: 50,
    stone: 45,
    gold: 65,
    mead: 12,
};
const DEFAULT_EXPEDITION_REWARD_PER_WAVE: ResourceBundle = ResourceBundle {
    wood: 20,
    stone: 20,
    gold: 25,
    mead: 5,
};
const DEFAULT_EXPEDITION_BOON_CHOICES: usize = 3;

// Boss tuning defaults.
const DEFAULT_BOSS_DAMAGE_PER_TILE: u32 = 45;
const DEFAULT_BOSS_DAMAGE_PER_OBSTACLE: u32 = 85;
const DEFAULT_BOSS_WEAKNESS_MULTIPLIER: u32 = 2;
const DEFAULT_BOSS_COMBO_DAMAGE_STEP: f32 = 0.35;
const DEFAULT_BOSS_PHASE2_HP_PCT: u32 = 50;
const DEFAULT_BOSS_CURSE_TURNS: u8 = 5;

/// One day of the Fjord Login Tribute cycle: resources plus an optional Runic Tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginTributeDay {
    pub resources: ResourceBundle,
    #[serde(default)]
    pub tool: Option<ActiveTool>,
}

/// Tool granted by the Daily Trial once the streak reaches `min_streak`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreakToolReward {
    pub min_streak: u32,
    pub tool: ActiveTool,
}

/// Passive village bonus a building can provide (amounts live in the `*_per_tier` balance fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingBonusKind {
    ExtraMaxEnergy,
    ScoreBonusPct,
    ResourceBonusPct,
    ExtraMoves,
    EnergyRegenSpeedPct,
    FuryChargeBonusPct,
}

/// Binds a building identifier to the passive bonus it grants per tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingBonusBinding {
    pub building_id: String,
    pub kind: BuildingBonusKind,
}

/// Tunables of a Match-3 session: daily blessings, patron fury powers and star thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionTuning {
    pub blessing_odin_fury_pct: u32,
    pub blessing_njord_resource_pct: u32,
    pub blessing_fenrir_boss_damage_pct: u32,
    pub blessing_idunn_extra_moves: u32,
    pub freya_fury_resource_pct: u32,
    pub odin_fury_extra_moves: u32,
    pub three_star_moves_ratio: f32,
    pub two_star_moves_ratio: f32,
    pub thor_fury_strike_cells: usize,
    pub loki_fury_boosters: u32,
}

impl Default for SessionTuning {
    fn default() -> Self {
        Self {
            blessing_odin_fury_pct: DEFAULT_BLESSING_ODIN_FURY_PCT,
            blessing_njord_resource_pct: DEFAULT_BLESSING_NJORD_RESOURCE_PCT,
            blessing_fenrir_boss_damage_pct: DEFAULT_BLESSING_FENRIR_BOSS_DAMAGE_PCT,
            blessing_idunn_extra_moves: DEFAULT_BLESSING_IDUNN_EXTRA_MOVES,
            freya_fury_resource_pct: DEFAULT_FREYA_FURY_RESOURCE_PCT,
            odin_fury_extra_moves: DEFAULT_ODIN_FURY_EXTRA_MOVES,
            three_star_moves_ratio: DEFAULT_THREE_STAR_MOVES_RATIO,
            two_star_moves_ratio: DEFAULT_TWO_STAR_MOVES_RATIO,
            thor_fury_strike_cells: DEFAULT_THOR_FURY_STRIKE_CELLS,
            loki_fury_boosters: DEFAULT_LOKI_FURY_BOOSTERS,
        }
    }
}

/// Tunables of the procedural Daily Trial generator. Ranges are inclusive `(min, max)` pairs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DailyTuning {
    pub glacial_collect_range: (u32, u32),
    pub glacial_frost_chance: f64,
    pub relic_count_range: (usize, usize),
    pub blitz_time_range: (u32, u32),
    pub blitz_score_range: (u32, u32),
    pub blitz_collect_range: (u32, u32),
    pub portal_score_target: u32,
    pub max_moves_range: (u32, u32),
    pub base_reward: ResourceBundle,
}

impl Default for DailyTuning {
    fn default() -> Self {
        Self {
            glacial_collect_range: (
                DEFAULT_DAILY_GLACIAL_COLLECT_MIN,
                DEFAULT_DAILY_GLACIAL_COLLECT_MAX,
            ),
            glacial_frost_chance: DEFAULT_DAILY_GLACIAL_FROST_CHANCE,
            relic_count_range: (DEFAULT_DAILY_RELIC_COUNT_MIN, DEFAULT_DAILY_RELIC_COUNT_MAX),
            blitz_time_range: (DEFAULT_DAILY_BLITZ_TIME_MIN, DEFAULT_DAILY_BLITZ_TIME_MAX),
            blitz_score_range: (DEFAULT_DAILY_BLITZ_SCORE_MIN, DEFAULT_DAILY_BLITZ_SCORE_MAX),
            blitz_collect_range: (
                DEFAULT_DAILY_BLITZ_COLLECT_MIN,
                DEFAULT_DAILY_BLITZ_COLLECT_MAX,
            ),
            portal_score_target: DEFAULT_DAILY_PORTAL_SCORE_TARGET,
            max_moves_range: (DEFAULT_DAILY_MAX_MOVES_MIN, DEFAULT_DAILY_MAX_MOVES_MAX),
            base_reward: ResourceBundle {
                wood: DEFAULT_DAILY_REWARD_WOOD,
                stone: DEFAULT_DAILY_REWARD_STONE,
                gold: DEFAULT_DAILY_REWARD_GOLD,
                mead: DEFAULT_DAILY_REWARD_MEAD,
            },
        }
    }
}

/// Tunables of the Endless Drakkar Expedition wave generator and boon pool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExpeditionTuning {
    pub wave_one_rune_types: usize,
    pub ice_base: u32,
    pub ice_per_wave: u32,
    pub ice_cap: u32,
    pub rune_target_base: u32,
    pub rune_target_per_wave: u32,
    pub rune_target_cap: u32,
    pub score_base: u32,
    pub score_per_wave: u32,
    pub score_wave_interval: u32,
    pub base_moves: u32,
    pub moves_cap: u32,
    pub double_ice_from_wave: u32,
    pub chains_from_wave: u32,
    pub extra_chains_from_wave: u32,
    pub crates_from_wave: u32,
    pub portals_from_wave: u32,
    pub fog_from_wave: u32,
    pub frost_from_wave: u32,
    pub curse_from_wave: u32,
    pub curse_turns: u8,
    pub reward_base: ResourceBundle,
    pub reward_per_wave: ResourceBundle,
    pub boons: Vec<ExpeditionBoon>,
    pub boon_choices: usize,
}

impl Default for ExpeditionTuning {
    fn default() -> Self {
        Self {
            wave_one_rune_types: DEFAULT_EXPEDITION_WAVE_ONE_RUNE_TYPES,
            ice_base: DEFAULT_EXPEDITION_ICE_BASE,
            ice_per_wave: DEFAULT_EXPEDITION_ICE_PER_WAVE,
            ice_cap: DEFAULT_EXPEDITION_ICE_CAP,
            rune_target_base: DEFAULT_EXPEDITION_RUNE_TARGET_BASE,
            rune_target_per_wave: DEFAULT_EXPEDITION_RUNE_TARGET_PER_WAVE,
            rune_target_cap: DEFAULT_EXPEDITION_RUNE_TARGET_CAP,
            score_base: DEFAULT_EXPEDITION_SCORE_BASE,
            score_per_wave: DEFAULT_EXPEDITION_SCORE_PER_WAVE,
            score_wave_interval: DEFAULT_EXPEDITION_SCORE_WAVE_INTERVAL,
            base_moves: DEFAULT_EXPEDITION_BASE_MOVES,
            moves_cap: DEFAULT_EXPEDITION_MOVES_CAP,
            double_ice_from_wave: DEFAULT_EXPEDITION_DOUBLE_ICE_FROM_WAVE,
            chains_from_wave: DEFAULT_EXPEDITION_CHAINS_FROM_WAVE,
            extra_chains_from_wave: DEFAULT_EXPEDITION_EXTRA_CHAINS_FROM_WAVE,
            crates_from_wave: DEFAULT_EXPEDITION_CRATES_FROM_WAVE,
            portals_from_wave: DEFAULT_EXPEDITION_PORTALS_FROM_WAVE,
            fog_from_wave: DEFAULT_EXPEDITION_FOG_FROM_WAVE,
            frost_from_wave: DEFAULT_EXPEDITION_FROST_FROM_WAVE,
            curse_from_wave: DEFAULT_EXPEDITION_CURSE_FROM_WAVE,
            curse_turns: DEFAULT_EXPEDITION_CURSE_TURNS,
            reward_base: DEFAULT_EXPEDITION_REWARD_BASE,
            reward_per_wave: DEFAULT_EXPEDITION_REWARD_PER_WAVE,
            boons: ExpeditionBoon::ALL.to_vec(),
            boon_choices: DEFAULT_EXPEDITION_BOON_CHOICES,
        }
    }
}

/// Tunables of Mythological Boss Raid combat.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BossTuning {
    pub damage_per_tile: u32,
    pub damage_per_obstacle: u32,
    pub weakness_multiplier: u32,
    pub combo_damage_step: f32,
    pub phase2_hp_pct: u32,
    pub curse_turns: u8,
}

impl Default for BossTuning {
    fn default() -> Self {
        Self {
            damage_per_tile: DEFAULT_BOSS_DAMAGE_PER_TILE,
            damage_per_obstacle: DEFAULT_BOSS_DAMAGE_PER_OBSTACLE,
            weakness_multiplier: DEFAULT_BOSS_WEAKNESS_MULTIPLIER,
            combo_damage_step: DEFAULT_BOSS_COMBO_DAMAGE_STEP,
            phase2_hp_pct: DEFAULT_BOSS_PHASE2_HP_PCT,
            curse_turns: DEFAULT_BOSS_CURSE_TURNS,
        }
    }
}

/// Default Fjord Login Tribute table: one entry per day of the 7-day cycle.
pub(crate) fn default_login_tribute_rewards() -> Vec<LoginTributeDay> {
    let day = |wood, stone, gold, mead, tool| LoginTributeDay {
        resources: ResourceBundle {
            wood,
            stone,
            gold,
            mead,
        },
        tool,
    };
    vec![
        day(35, 20, 30, 0, None),
        day(25, 40, 35, 10, None),
        day(30, 30, 50, 10, Some(ActiveTool::Mjolnir)),
        day(60, 55, 60, 15, None),
        day(50, 50, 75, 15, Some(ActiveTool::ThorBolt)),
        day(75, 75, 95, 25, None),
        day(100, 100, 140, 35, Some(ActiveTool::BifrostPrism)),
    ]
}

/// Default building to passive bonus mapping.
fn default_building_bonuses() -> Vec<BuildingBonusBinding> {
    let bind = |id: &str, kind| BuildingBonusBinding {
        building_id: id.to_string(),
        kind,
    };
    vec![
        bind("great_hall", BuildingBonusKind::ExtraMaxEnergy),
        bind("forge", BuildingBonusKind::ScoreBonusPct),
        bind("longship_dock", BuildingBonusKind::ResourceBonusPct),
        bind("odin_shrine", BuildingBonusKind::ExtraMoves),
        bind("meadery", BuildingBonusKind::EnergyRegenSpeedPct),
        bind("watchtower", BuildingBonusKind::FuryChargeBonusPct),
    ]
}

/// Default mastery star ladder: the tool granted for the 1st, 2nd, ... mastery star of a building.
fn default_mastery_star_tools() -> Vec<ActiveTool> {
    vec![
        ActiveTool::Mjolnir,
        ActiveTool::SurtrEmber,
        ActiveTool::ThorBolt,
        ActiveTool::BifrostPrism,
        ActiveTool::DrakkarCharge,
    ]
}

/// Default Daily Trial streak tool rewards.
fn default_daily_streak_tool_rewards() -> Vec<StreakToolReward> {
    vec![
        StreakToolReward {
            min_streak: 1,
            tool: ActiveTool::BifrostPrism,
        },
        StreakToolReward {
            min_streak: 3,
            tool: ActiveTool::SurtrEmber,
        },
        StreakToolReward {
            min_streak: 7,
            tool: ActiveTool::DrakkarCharge,
        },
    ]
}

fn default_win_streak_booster_threshold() -> u32 {
    DEFAULT_WIN_STREAK_BOOSTER_THRESHOLD
}
fn default_win_streak_starting_boosters() -> u32 {
    DEFAULT_WIN_STREAK_STARTING_BOOSTERS
}
fn default_clan_rank_scout_resource_bonus_pct() -> u32 {
    DEFAULT_CLAN_RANK_SCOUT_RESOURCE_BONUS_PCT
}
fn default_clan_rank_huscarl_resource_bonus_pct() -> u32 {
    DEFAULT_CLAN_RANK_HUSCARL_RESOURCE_BONUS_PCT
}
fn default_clan_rank_jarl_resource_bonus_pct() -> u32 {
    DEFAULT_CLAN_RANK_JARL_RESOURCE_BONUS_PCT
}
fn default_clan_rank_high_king_resource_bonus_pct() -> u32 {
    DEFAULT_CLAN_RANK_HIGH_KING_RESOURCE_BONUS_PCT
}
fn default_clan_rank_scout_starting_fury() -> u32 {
    DEFAULT_CLAN_RANK_SCOUT_STARTING_FURY
}
fn default_clan_rank_huscarl_starting_fury() -> u32 {
    DEFAULT_CLAN_RANK_HUSCARL_STARTING_FURY
}
fn default_clan_rank_jarl_starting_fury() -> u32 {
    DEFAULT_CLAN_RANK_JARL_STARTING_FURY
}
fn default_clan_rank_high_king_starting_fury() -> u32 {
    DEFAULT_CLAN_RANK_HIGH_KING_STARTING_FURY
}
fn default_daily_streak_bonus_pct_per_day() -> u32 {
    DEFAULT_DAILY_STREAK_BONUS_PCT_PER_DAY
}
fn default_expedition_glory_per_wave() -> u32 {
    DEFAULT_EXPEDITION_GLORY_PER_WAVE
}

fn default_great_hall_energy_per_tier() -> u32 {
    10
}
fn default_forge_score_pct_per_tier() -> u32 {
    10
}
fn default_dock_resource_pct_per_tier() -> u32 {
    15
}
fn default_shrine_moves_per_tier() -> u32 {
    1
}
fn default_meadery_regen_pct_per_tier() -> u32 {
    20
}
fn default_move_reward_bonus_pct_per_move() -> u32 {
    3
}
fn default_move_reward_bonus_cap_pct() -> u32 {
    30
}
fn default_victory_glory() -> u32 {
    10
}
fn default_quest_glory() -> u32 {
    15
}
fn default_amulet_glory() -> u32 {
    20
}
fn default_daily_glory() -> u32 {
    35
}
fn default_collection_glory() -> u32 {
    40
}
fn default_harvest_interval_seconds() -> u64 {
    120
}
fn default_watchtower_fury_pct_per_tier() -> u32 {
    15
}
fn default_clan_rank_huscarl_glory() -> u32 {
    DEFAULT_CLAN_RANK_HUSCARL_GLORY
}
fn default_clan_rank_jarl_glory() -> u32 {
    DEFAULT_CLAN_RANK_JARL_GLORY
}
fn default_clan_rank_high_king_glory() -> u32 {
    DEFAULT_CLAN_RANK_HIGH_KING_GLORY
}

/// Tunable gameplay and animation parameters loaded from `assets/data/balance.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceConfig {
    pub default_board_cols: usize,
    pub default_board_rows: usize,
    pub points_per_tile: u32,
    pub combo_multiplier_step: f32,
    pub booster_activation_bonus: u32,
    pub ice_break_bonus: u32,
    pub chain_break_bonus: u32,
    pub crate_break_bonus: u32,
    pub relic_drop_bonus: u32,
    pub remaining_move_bonus: u32,
    pub max_energy: u32,
    pub energy_cost_per_level: u32,
    pub energy_regen_seconds: u64,
    pub mead_refill_cost: u32,
    pub mead_refill_energy: u32,
    pub starting_wood: u32,
    pub starting_stone: u32,
    pub starting_gold: u32,
    pub starting_mead: u32,
    pub starting_mjolnir: u32,
    pub starting_surtr_ember: u32,
    pub starting_loki_shuffle: u32,
    pub starting_thor_bolt: u32,
    pub starting_bifrost_prism: u32,
    pub starting_drakkar_charge: u32,
    pub swap_anim_seconds: f32,
    pub clear_anim_seconds: f32,
    pub fall_anim_seconds: f32,
    pub swipe_threshold_px: f32,
    #[serde(default = "default_great_hall_energy_per_tier")]
    pub great_hall_energy_per_tier: u32,
    #[serde(default = "default_forge_score_pct_per_tier")]
    pub forge_score_pct_per_tier: u32,
    #[serde(default = "default_dock_resource_pct_per_tier")]
    pub dock_resource_pct_per_tier: u32,
    #[serde(default = "default_shrine_moves_per_tier")]
    pub shrine_moves_per_tier: u32,
    #[serde(default = "default_meadery_regen_pct_per_tier")]
    pub meadery_regen_pct_per_tier: u32,
    #[serde(default = "default_move_reward_bonus_pct_per_move")]
    pub move_reward_bonus_pct_per_move: u32,
    #[serde(default = "default_move_reward_bonus_cap_pct")]
    pub move_reward_bonus_cap_pct: u32,
    #[serde(default = "default_victory_glory")]
    pub victory_glory: u32,
    #[serde(default = "default_quest_glory")]
    pub quest_glory: u32,
    #[serde(default = "default_amulet_glory")]
    pub amulet_glory: u32,
    #[serde(default = "default_daily_glory")]
    pub daily_glory: u32,
    #[serde(default = "default_collection_glory")]
    pub collection_glory: u32,
    #[serde(default = "default_harvest_interval_seconds")]
    pub harvest_interval_seconds: u64,
    #[serde(default = "default_watchtower_fury_pct_per_tier")]
    pub watchtower_fury_pct_per_tier: u32,
    #[serde(default = "default_clan_rank_huscarl_glory")]
    pub clan_rank_huscarl_glory: u32,
    #[serde(default = "default_clan_rank_jarl_glory")]
    pub clan_rank_jarl_glory: u32,
    #[serde(default = "default_clan_rank_high_king_glory")]
    pub clan_rank_high_king_glory: u32,
    #[serde(default = "default_login_tribute_rewards")]
    pub login_tribute_rewards: Vec<LoginTributeDay>,
    #[serde(default = "default_building_bonuses")]
    pub building_bonuses: Vec<BuildingBonusBinding>,
    #[serde(default = "default_win_streak_booster_threshold")]
    pub win_streak_booster_threshold: u32,
    #[serde(default = "default_win_streak_starting_boosters")]
    pub win_streak_starting_boosters: u32,
    #[serde(default = "default_clan_rank_scout_resource_bonus_pct")]
    pub clan_rank_scout_resource_bonus_pct: u32,
    #[serde(default = "default_clan_rank_huscarl_resource_bonus_pct")]
    pub clan_rank_huscarl_resource_bonus_pct: u32,
    #[serde(default = "default_clan_rank_jarl_resource_bonus_pct")]
    pub clan_rank_jarl_resource_bonus_pct: u32,
    #[serde(default = "default_clan_rank_high_king_resource_bonus_pct")]
    pub clan_rank_high_king_resource_bonus_pct: u32,
    #[serde(default = "default_clan_rank_scout_starting_fury")]
    pub clan_rank_scout_starting_fury: u32,
    #[serde(default = "default_clan_rank_huscarl_starting_fury")]
    pub clan_rank_huscarl_starting_fury: u32,
    #[serde(default = "default_clan_rank_jarl_starting_fury")]
    pub clan_rank_jarl_starting_fury: u32,
    #[serde(default = "default_clan_rank_high_king_starting_fury")]
    pub clan_rank_high_king_starting_fury: u32,
    #[serde(default = "default_mastery_star_tools")]
    pub mastery_star_tools: Vec<ActiveTool>,
    #[serde(default = "default_daily_streak_bonus_pct_per_day")]
    pub daily_streak_bonus_pct_per_day: u32,
    #[serde(default = "default_daily_streak_tool_rewards")]
    pub daily_streak_tool_rewards: Vec<StreakToolReward>,
    #[serde(default = "default_expedition_glory_per_wave")]
    pub expedition_glory_per_wave: u32,
    #[serde(default)]
    pub session: SessionTuning,
    #[serde(default)]
    pub daily: DailyTuning,
    #[serde(default)]
    pub expedition: ExpeditionTuning,
    #[serde(default)]
    pub boss: BossTuning,
}

impl BalanceConfig {
    /// Parses balance configuration from a RON document string.
    pub fn from_ron_str(raw: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(raw)
    }

    /// Computes combo multiplier for a 1-based cascade depth.
    pub fn combo_multiplier(&self, combo_depth: u32) -> f32 {
        let extra = combo_depth.saturating_sub(1) as f32;
        1.0 + extra * self.combo_multiplier_step
    }
}

/// Computes `base + (base * pct) / 100` using widened integer math and saturates on `u32::MAX`.
pub(crate) fn add_pct_u32(base: u32, pct: u32) -> u32 {
    let total = u128::from(base) + (u128::from(base) * u128::from(pct)) / 100;
    u32::try_from(total).unwrap_or(u32::MAX)
}
