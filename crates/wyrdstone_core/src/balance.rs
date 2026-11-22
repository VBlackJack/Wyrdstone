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

use serde::{Deserialize, Serialize};

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
