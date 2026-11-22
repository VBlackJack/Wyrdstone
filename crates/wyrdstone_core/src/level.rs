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

//! Level definitions, objective tracking, and active Match-3 session state.

use crate::balance::BalanceConfig;
use crate::board::{ActiveTool, Board, CellPos, MatchStepOutcome, RuneType};
use crate::city::ResourceBundle;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Win condition attached to a Match-3 level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Objective {
    ClearIce { target: u32 },
    CollectRune { rune: RuneType, target: u32 },
    ReachScore { target: u32 },
}

/// Live progress snapshot for a single objective.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveProgress {
    pub objective: Objective,
    pub done: u32,
    pub target: u32,
}

impl ObjectiveProgress {
    pub fn is_complete(&self) -> bool {
        self.done >= self.target
    }
}

/// Static level definition loaded from `assets/data/levels.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelDef {
    pub id: String,
    pub building_id: String,
    pub cols: usize,
    pub rows: usize,
    pub max_moves: u32,
    pub rune_types_count: usize,
    pub objectives: Vec<Objective>,
    pub ice_cells: Vec<(usize, usize, u8)>,
    pub chained_cells: Vec<(usize, usize)>,
    pub blocked_cells: Vec<(usize, usize)>,
    pub base_reward: ResourceBundle,
}

/// Terminal state of a Match-3 level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOutcome {
    InProgress,
    Victory,
    Defeat,
}

/// Playable Match-3 level session wrapping a board, moves counter, score, and objectives.
#[derive(Debug, Clone)]
pub struct LevelSession {
    pub level: LevelDef,
    pub board: Board,
    pub moves_left: u32,
    pub score: u32,
    pub objectives: Vec<ObjectiveProgress>,
    pub outcome: LevelOutcome,
    pub last_combo: u32,
    pub last_shuffled: bool,
}

impl LevelSession {
    /// Starts a new playable session for the given level definition.
    pub fn start<R: Rng>(level: LevelDef, rng: &mut R) -> Self {
        let board = Board::new_for_level(&level, rng);
        let objectives = level
            .objectives
            .iter()
            .map(|obj| {
                let target = match obj {
                    Objective::ClearIce { target }
                    | Objective::CollectRune { target, .. }
                    | Objective::ReachScore { target } => *target,
                };
                ObjectiveProgress {
                    objective: obj.clone(),
                    done: 0,
                    target,
                }
            })
            .collect();

        Self {
            moves_left: level.max_moves,
            level,
            board,
            score: 0,
            objectives,
            outcome: LevelOutcome::InProgress,
            last_combo: 0,
            last_shuffled: false,
        }
    }

    /// Attempts a player swap, updating moves, score, objectives, and victory/defeat state.
    pub fn play_swap<R: Rng>(
        &mut self,
        a: CellPos,
        b: CellPos,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> MatchStepOutcome {
        if self.outcome != LevelOutcome::InProgress {
            return MatchStepOutcome::default();
        }
        let step = self.board.try_swap(a, b, balance, rng);
        if step.valid {
            self.moves_left = self.moves_left.saturating_sub(1);
            self.apply_step_outcome(&step, balance);
        }
        step
    }

    /// Applies an active tool without consuming a move.
    pub fn play_tool<R: Rng>(
        &mut self,
        tool: ActiveTool,
        target: Option<CellPos>,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> MatchStepOutcome {
        if self.outcome != LevelOutcome::InProgress {
            return MatchStepOutcome::default();
        }
        let step = self.board.use_active_tool(tool, target, balance, rng);
        if step.valid {
            self.apply_step_outcome(&step, balance);
        }
        step
    }

    /// Decrements remaining moves by one (used by animated turn execution).
    pub fn consume_move(&mut self) {
        self.moves_left = self.moves_left.saturating_sub(1);
    }

    /// Applies score and objective progress from a single animated clear step without ending the turn yet.
    pub fn apply_step_progress(&mut self, step: &MatchStepOutcome) {
        self.score += step.score_gained;
        self.last_combo = step.max_combo;
        self.last_shuffled = step.shuffled;

        for prog in &mut self.objectives {
            match &prog.objective {
                Objective::ClearIce { .. } => {
                    prog.done = (prog.done + step.ice_broken).min(prog.target);
                }
                Objective::CollectRune { rune, .. } => {
                    if let Some(&count) = step.cleared_runes.get(rune) {
                        prog.done = (prog.done + count).min(prog.target);
                    }
                }
                Objective::ReachScore { .. } => {
                    prog.done = self.score.min(prog.target);
                }
            }
        }
    }

    /// Evaluates victory or defeat after all cascades of a turn have finished.
    pub fn finalize_turn(&mut self, balance: &BalanceConfig) {
        if self.outcome != LevelOutcome::InProgress {
            return;
        }
        if self.objectives.iter().all(ObjectiveProgress::is_complete) {
            self.score += self.moves_left * balance.remaining_move_bonus;
            self.outcome = LevelOutcome::Victory;
        } else if self.moves_left == 0 {
            self.outcome = LevelOutcome::Defeat;
        }
    }

    fn apply_step_outcome(&mut self, step: &MatchStepOutcome, balance: &BalanceConfig) {
        self.apply_step_progress(step);
        self.finalize_turn(balance);
    }

    /// Computes final resource rewards when the level is won (scaled slightly by remaining moves).
    pub fn earned_rewards(&self) -> ResourceBundle {
        if self.outcome != LevelOutcome::Victory {
            return ResourceBundle::default();
        }
        let bonus_pct = (self.moves_left * 3).min(30);
        let scale = |base: u32| base + (base * bonus_pct) / 100;
        ResourceBundle {
            wood: scale(self.level.base_reward.wood),
            stone: scale(self.level.base_reward.stone),
            gold: scale(self.level.base_reward.gold),
            mead: scale(self.level.base_reward.mead),
        }
    }
}
