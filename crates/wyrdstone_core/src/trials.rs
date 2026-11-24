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

//! Deterministic zero-RNG tactical Match-3 puzzles (Yggdrasil Trials).

use crate::board::{Board, Booster, Cell, RuneType};
use crate::city::ResourceBundle;
use crate::level::{LevelDef, Objective};
use serde::{Deserialize, Serialize};

/// Synthetic building identifier carried by Yggdrasil Trials puzzle levels.
pub const TRIAL_BUILDING_ID: &str = "yggdrasil_trial";

/// Bonus Glory awarded the first time a Trial is completed within Par moves.
pub const TRIAL_PAR_BONUS_GLORY: u32 = 10;

/// Static definition of a deterministic Yggdrasil Trial puzzle loaded from `assets/data/trials.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialDef {
    pub id: String,
    pub name_key: String,
    pub hint_key: String,
    pub cols: usize,
    pub rows: usize,
    pub rune_pool_len: usize,
    pub max_moves: u32,
    pub par_moves: u32,
    pub seed: u64,
    pub objectives: Vec<Objective>,
    pub grid: Vec<String>,
    pub first_clear_glory: u32,
    pub first_clear_rewards: ResourceBundle,
}

impl TrialDef {
    /// Parses `self.grid` into a deterministic `Board` without randomness.
    pub fn build_board(&self) -> Result<Board, String> {
        if self.cols == 0 || self.rows == 0 {
            return Err(format!(
                "trial '{}': dimensions {}x{} must be positive",
                self.id, self.cols, self.rows
            ));
        }
        if !(3..=RuneType::ALL.len()).contains(&self.rune_pool_len) {
            return Err(format!(
                "trial '{}': rune_pool_len {} outside 3..={}",
                self.id,
                self.rune_pool_len,
                RuneType::ALL.len()
            ));
        }
        if self.grid.len() != self.rows {
            return Err(format!(
                "trial '{}': expected {} grid rows, got {}",
                self.id,
                self.rows,
                self.grid.len()
            ));
        }

        let mut cells = Vec::with_capacity(self.cols * self.rows);
        for (row_idx, row_str) in self.grid.iter().enumerate() {
            let tokens: Vec<&str> = row_str.split_whitespace().collect();
            if tokens.len() != self.cols {
                return Err(format!(
                    "trial '{}' row {row_idx}: expected {} tokens, got {}",
                    self.id,
                    self.cols,
                    tokens.len()
                ));
            }
            for token in tokens {
                cells.push(parse_cell_token(token).map_err(|err| {
                    format!("trial '{}' row {row_idx} token '{token}': {err}", self.id)
                })?);
            }
        }
        Ok(Board::from_cells(
            self.cols,
            self.rows,
            self.rune_pool_len,
            cells,
        ))
    }

    /// Converts this puzzle into a synthetic `LevelDef` for session tracking and rewards.
    pub fn to_level_def(&self) -> LevelDef {
        LevelDef {
            id: self.id.clone(),
            building_id: TRIAL_BUILDING_ID.to_string(),
            cols: self.cols,
            rows: self.rows,
            max_moves: self.max_moves,
            time_limit_seconds: None,
            rune_types_count: self.rune_pool_len,
            objectives: self.objectives.clone(),
            ice_cells: Vec::new(),
            chained_cells: Vec::new(),
            crate_cells: Vec::new(),
            relic_cells: Vec::new(),
            portals: Vec::new(),
            frost_spread: false,
            fog_cells: Vec::new(),
            cursed_cells: Vec::new(),
            blocked_cells: Vec::new(),
            base_reward: self.first_clear_rewards,
        }
    }
}

fn parse_cell_token(token: &str) -> Result<Cell, String> {
    let mut chars = token.chars();
    let Some(base) = chars.next() else {
        return Err("empty token".to_string());
    };
    let mut cell = match base {
        '#' => return Ok(Cell::blocked_cell()),
        'R' => {
            return Ok(Cell {
                blocked: false,
                rune: None,
                booster: Booster::None,
                ice_layers: 0,
                chained: false,
                crate_layers: 0,
                relic: true,
                fog: false,
                curse_turns: 0,
            })
        }
        'k' => {
            return Ok(Cell {
                blocked: false,
                rune: None,
                booster: Booster::None,
                ice_layers: 0,
                chained: false,
                crate_layers: 1,
                relic: false,
                fog: false,
                curse_turns: 0,
            })
        }
        'K' => {
            return Ok(Cell {
                blocked: false,
                rune: None,
                booster: Booster::None,
                ice_layers: 0,
                chained: false,
                crate_layers: 2,
                relic: false,
                fog: false,
                curse_turns: 0,
            })
        }
        'X' => Cell {
            blocked: false,
            rune: None,
            booster: Booster::BifrostShard,
            ice_layers: 0,
            chained: false,
            crate_layers: 0,
            relic: false,
            fog: false,
            curse_turns: 0,
        },
        'F' => Cell::playable(RuneType::Fire),
        'I' => Cell::playable(RuneType::Ice),
        'W' => Cell::playable(RuneType::Wood),
        'S' => Cell::playable(RuneType::Stone),
        'G' => Cell::playable(RuneType::Gold),
        'A' => Cell::playable(RuneType::Water),
        other => return Err(format!("unknown base symbol '{other}'")),
    };

    for mod_ch in chars {
        match mod_ch {
            'h' if base != 'X' => cell.booster = Booster::LineHorizontal,
            'v' if base != 'X' => cell.booster = Booster::LineVertical,
            'b' if base != 'X' => cell.booster = Booster::Bomb,
            '1' => cell.ice_layers = 1,
            '2' => cell.ice_layers = 2,
            'c' => cell.chained = true,
            other => return Err(format!("invalid modifier '{other}'")),
        }
    }
    Ok(cell)
}
