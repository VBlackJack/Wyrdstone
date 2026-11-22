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
    pub remaining_move_bonus: u32,
    pub max_energy: u32,
    pub energy_cost_per_level: u32,
    pub energy_regen_seconds: u64,
    pub starting_wood: u32,
    pub starting_stone: u32,
    pub starting_gold: u32,
    pub starting_mead: u32,
    pub starting_mjolnir: u32,
    pub starting_surtr_ember: u32,
    pub starting_loki_shuffle: u32,
    pub swap_anim_seconds: f32,
    pub clear_anim_seconds: f32,
    pub fall_anim_seconds: f32,
    pub swipe_threshold_px: f32,
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
