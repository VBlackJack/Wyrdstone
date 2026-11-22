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

//! Level definitions, objective tracking, timed mode, amulet modifiers, and Match-3 session state.

use crate::balance::BalanceConfig;
use crate::board::{ActiveTool, Board, CellPos, MatchStepOutcome, RuneType};
use crate::city::{AmuletEffect, CityBonuses, ResourceBundle};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Win condition attached to a Match-3 level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Objective {
    ClearIce { target: u32 },
    CollectRune { rune: RuneType, target: u32 },
    ReachScore { target: u32 },
    DropRelics { target: u32 },
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
    #[serde(default)]
    pub time_limit_seconds: Option<u32>,
    pub rune_types_count: usize,
    pub objectives: Vec<Objective>,
    pub ice_cells: Vec<(usize, usize, u8)>,
    pub chained_cells: Vec<(usize, usize)>,
    #[serde(default)]
    pub crate_cells: Vec<(usize, usize, u8)>,
    #[serde(default)]
    pub relic_cells: Vec<(usize, usize)>,
    #[serde(default)]
    pub portals: Vec<((usize, usize), (usize, usize))>,
    pub blocked_cells: Vec<(usize, usize)>,
    pub base_reward: ResourceBundle,
}

impl LevelDef {
    /// Deterministically generates today's Daily Trial board from the UTC day index.
    pub fn daily_challenge(day_index: u64) -> Self {
        let variant = (day_index % 3) as usize;
        let target_rune = RuneType::ALL[(day_index as usize) % RuneType::ALL.len()];
        let (objectives, ice_cells, chained_cells, crate_cells, relic_cells) = match variant {
            0 => (
                vec![
                    Objective::ClearIce { target: 12 },
                    Objective::CollectRune {
                        rune: target_rune,
                        target: 22,
                    },
                ],
                vec![
                    (2, 2, 2), (3, 2, 1), (4, 2, 1), (5, 2, 2),
                    (2, 3, 1), (5, 3, 1), (2, 4, 1), (5, 4, 1),
                    (2, 5, 2), (3, 5, 1), (4, 5, 1), (5, 5, 2),
                ],
                vec![(3, 3), (4, 4)],
                vec![(1, 3, 1), (6, 4, 1)],
                vec![],
            ),
            1 => (
                vec![
                    Objective::DropRelics { target: 2 },
                    Objective::ClearIce { target: 8 },
                ],
                vec![
                    (2, 4, 2), (3, 4, 1), (4, 4, 1), (5, 4, 2),
                    (2, 6, 1), (3, 6, 1), (4, 6, 1), (5, 6, 1),
                ],
                vec![(2, 2), (5, 2)],
                vec![(3, 5, 2), (4, 5, 2)],
                vec![(2, 0), (5, 0)],
            ),
            _ => (
                vec![
                    Objective::ReachScore { target: 5000 },
                    Objective::CollectRune {
                        rune: target_rune,
                        target: 24,
                    },
                ],
                vec![
                    (1, 2, 1), (2, 2, 2), (5, 2, 2), (6, 2, 1),
                    (1, 5, 1), (2, 5, 2), (5, 5, 2), (6, 5, 1),
                ],
                vec![(3, 3), (4, 3), (3, 4), (4, 4)],
                vec![(0, 3, 1), (7, 4, 1)],
                vec![],
            ),
        };

        Self {
            id: format!("daily_{day_index}"),
            building_id: "daily_trial".to_string(),
            cols: 8,
            rows: 8,
            max_moves: 25,
            time_limit_seconds: None,
            rune_types_count: 6,
            objectives,
            ice_cells,
            chained_cells,
            crate_cells,
            relic_cells,
            portals: vec![],
            blocked_cells: vec![(0, 0), (7, 0), (0, 7), (7, 7)],
            base_reward: ResourceBundle {
                wood: 140,
                stone: 140,
                gold: 180,
                mead: 40,
            },
        }
    }
}

/// Terminal state of a Match-3 level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOutcome {
    InProgress,
    Victory,
    Defeat,
}

/// Playable Match-3 level session wrapping a board, moves/timer, score, amulet buffs, and objectives.
#[derive(Debug, Clone)]
pub struct LevelSession {
    pub level: LevelDef,
    pub board: Board,
    pub moves_left: u32,
    pub time_left_secs: Option<f32>,
    pub score: u32,
    pub score_bonus_pct: u32,
    pub resource_bonus_pct: u32,
    pub objectives: Vec<ObjectiveProgress>,
    pub outcome: LevelOutcome,
    pub last_combo: u32,
    pub last_shuffled: bool,
}

impl LevelSession {
    /// Starts a new playable session without an amulet.
    pub fn start<R: Rng>(level: LevelDef, rng: &mut R) -> Self {
        Self::start_with_amulet(level, None, rng)
    }

    /// Starts a new playable session applying an optional equipped Nordic Amulet.
    pub fn start_with_amulet<R: Rng>(
        level: LevelDef,
        amulet: Option<AmuletEffect>,
        rng: &mut R,
    ) -> Self {
        Self::start_with_bonuses(level, amulet, CityBonuses::default(), rng)
    }

    /// Starts a new playable session combining equipped Amulet and rebuilt village bonuses.
    pub fn start_with_bonuses<R: Rng>(
        level: LevelDef,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        rng: &mut R,
    ) -> Self {
        let board = Board::new_for_level(&level, rng);
        let objectives = level
            .objectives
            .iter()
            .map(|obj| {
                let target = match obj {
                    Objective::ClearIce { target }
                    | Objective::CollectRune { target, .. }
                    | Objective::ReachScore { target }
                    | Objective::DropRelics { target } => *target,
                };
                ObjectiveProgress {
                    objective: obj.clone(),
                    done: 0,
                    target,
                }
            })
            .collect();

        let amulet_moves = match amulet {
            Some(AmuletEffect::ExtraMoves(m)) => m,
            _ => 0,
        };
        let amulet_score = match amulet {
            Some(AmuletEffect::ScoreBonusPct(p)) => p,
            _ => 0,
        };
        let amulet_resources = match amulet {
            Some(AmuletEffect::ResourceBonusPct(p)) => p,
            _ => 0,
        };
        let time_left_secs = level.time_limit_seconds.map(|s| s as f32);

        Self {
            moves_left: level.max_moves + amulet_moves + city_bonuses.extra_moves,
            time_left_secs,
            level,
            board,
            score: 0,
            score_bonus_pct: amulet_score + city_bonuses.score_bonus_pct,
            resource_bonus_pct: amulet_resources + city_bonuses.resource_bonus_pct,
            objectives,
            outcome: LevelOutcome::InProgress,
            last_combo: 0,
            last_shuffled: false,
        }
    }

    /// Advances the optional countdown timer and triggers Defeat when time expires.
    pub fn tick_timer(&mut self, dt_secs: f32) -> bool {
        if self.outcome != LevelOutcome::InProgress {
            return false;
        }
        if let Some(ref mut remaining) = self.time_left_secs {
            let prev_whole = remaining.ceil() as u32;
            *remaining = (*remaining - dt_secs).max(0.0);
            let new_whole = remaining.ceil() as u32;
            if *remaining <= 0.0 {
                self.outcome = LevelOutcome::Defeat;
                return true;
            }
            return new_whole != prev_whole;
        }
        false
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
            self.consume_move();
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
        let boosted_score =
            step.score_gained + (step.score_gained * self.score_bonus_pct) / 100;
        self.score += boosted_score;
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
                Objective::DropRelics { .. } => {
                    prog.done = (prog.done + step.relics_dropped).min(prog.target);
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

    /// Computes final resource rewards when the level is won (scaled by remaining moves + Freya amulet).
    pub fn earned_rewards(&self) -> ResourceBundle {
        if self.outcome != LevelOutcome::Victory {
            return ResourceBundle::default();
        }
        let bonus_pct = (self.moves_left * 3).min(30) + self.resource_bonus_pct;
        let scale = |base: u32| base + (base * bonus_pct) / 100;
        ResourceBundle {
            wood: scale(self.level.base_reward.wood),
            stone: scale(self.level.base_reward.stone),
            gold: scale(self.level.base_reward.gold),
            mead: scale(self.level.base_reward.mead),
        }
    }
}
