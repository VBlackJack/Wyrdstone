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

//! Level definitions, objective tracking, timed mode, frost spread, amulet modifiers, and Match-3 session state.

use crate::balance::BalanceConfig;
use crate::board::{ActiveTool, Board, CellPos, MatchStepOutcome, PreparedClear, RuneType};
use crate::boss::{BossDef, BossState};
use crate::city::{AmuletEffect, CityBonuses, ResourceBundle};
use crate::expedition::ExpeditionRun;
use crate::patron::{
    PatronGod, FURY_CHARGE_MAX, FURY_PER_COMBO_STEP, FURY_PER_OBSTACLE, FURY_PER_TILE,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

const DAILY_SEED_SALT: u64 = 0x5759_5244_5354_4F4E;

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
    #[serde(default)]
    pub frost_spread: bool,
    #[serde(default)]
    pub fog_cells: Vec<(usize, usize)>,
    pub blocked_cells: Vec<(usize, usize)>,
    pub base_reward: ResourceBundle,
}

impl LevelDef {
    /// Deterministically generates today's procedural Daily Trial board from the UTC day index.
    pub fn daily_challenge(day_index: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(day_index ^ DAILY_SEED_SALT);
        let archetype = rng.gen_range(0..4usize);
        let target_rune = RuneType::ALL[rng.gen_range(0..RuneType::ALL.len())];

        let blocked_cells = match rng.gen_range(0..3usize) {
            0 => vec![(0, 0), (7, 0), (0, 7), (7, 7)],
            1 => vec![(0, 3), (0, 4), (7, 3), (7, 4)],
            _ => vec![(3, 0), (4, 0), (3, 7), (4, 7)],
        };

        let mut ice_cells = Vec::new();
        let mut chained_cells = Vec::new();
        let mut crate_cells = Vec::new();
        let mut relic_cells = Vec::new();
        let mut portals = Vec::new();
        let mut time_limit_seconds = None;
        let mut frost_spread = false;

        let objectives = match archetype {
            0 => {
                // Glacial fortress: clear ice + collect rune
                let ring_rows = [2usize, 5usize];
                for &r in &ring_rows {
                    for c in 2..=5 {
                        let layers = if c == 2 || c == 5 { 2 } else { 1 };
                        ice_cells.push((c, r, layers));
                    }
                }
                for r in 3..=4 {
                    ice_cells.push((2, r, 1));
                    ice_cells.push((5, r, 1));
                }
                chained_cells.push((3, 3));
                chained_cells.push((4, 4));
                crate_cells.push((1, 3, 1));
                crate_cells.push((6, 4, 1));
                frost_spread = rng.gen_bool(0.5);
                vec![
                    Objective::ClearIce {
                        target: ice_cells.len() as u32,
                    },
                    Objective::CollectRune {
                        rune: target_rune,
                        target: rng.gen_range(20..=26),
                    },
                ]
            }
            1 => {
                // Relic excavation: drop 2-3 relics through crates & ice
                let relic_count = rng.gen_range(2..=3usize);
                let cols = if relic_count == 2 {
                    vec![2, 5]
                } else {
                    vec![1, 4, 6]
                };
                for c in cols {
                    relic_cells.push((c, 0));
                }
                for c in 2..=5 {
                    ice_cells.push((c, 4, if c == 2 || c == 5 { 2 } else { 1 }));
                    ice_cells.push((c, 6, 1));
                }
                crate_cells.push((3, 5, 2));
                crate_cells.push((4, 5, 2));
                chained_cells.push((2, 2));
                chained_cells.push((5, 2));
                vec![
                    Objective::DropRelics {
                        target: relic_count as u32,
                    },
                    Objective::ClearIce { target: 8 },
                ]
            }
            2 => {
                // Timed Skald's Blitz: score + rune collection
                time_limit_seconds = Some(rng.gen_range(95..=115));
                for &(c, r) in &[
                    (1, 2),
                    (2, 2),
                    (5, 2),
                    (6, 2),
                    (1, 5),
                    (2, 5),
                    (5, 5),
                    (6, 5),
                ] {
                    ice_cells.push((c, r, if c == 2 || c == 5 { 2 } else { 1 }));
                }
                chained_cells.extend([(3, 3), (4, 3), (3, 4), (4, 4)]);
                vec![
                    Objective::ReachScore {
                        target: rng.gen_range(4800..=5600),
                    },
                    Objective::CollectRune {
                        rune: target_rune,
                        target: rng.gen_range(22..=26),
                    },
                ]
            }
            _ => {
                // Bifrost Portal Trials: diagonal teleporters + score & ice
                portals.push(((1, 1), (6, 6)));
                portals.push(((6, 1), (1, 6)));
                for c in 2..=5 {
                    ice_cells.push((c, 2, 1));
                    ice_cells.push((c, 5, 2));
                }
                crate_cells.push((3, 3, 1));
                crate_cells.push((4, 4, 1));
                chained_cells.push((3, 4));
                chained_cells.push((4, 3));
                vec![
                    Objective::ClearIce { target: 8 },
                    Objective::ReachScore { target: 5000 },
                ]
            }
        };

        Self {
            id: format!("daily_{day_index}"),
            building_id: "daily_trial".to_string(),
            cols: 8,
            rows: 8,
            max_moves: rng.gen_range(24..=28),
            time_limit_seconds,
            rune_types_count: 6,
            objectives,
            ice_cells,
            chained_cells,
            crate_cells,
            relic_cells,
            portals,
            frost_spread,
            fog_cells: if archetype == 3 {
                vec![(2, 3), (3, 3), (4, 3), (5, 3), (2, 4), (5, 4)]
            } else {
                vec![]
            },
            blocked_cells,
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
    pub initial_moves: u32,
    pub moves_left: u32,
    pub time_left_secs: Option<f32>,
    pub score: u32,
    pub score_bonus_pct: u32,
    pub resource_bonus_pct: u32,
    pub objectives: Vec<ObjectiveProgress>,
    pub outcome: LevelOutcome,
    pub last_combo: u32,
    pub last_shuffled: bool,
    pub turn_ice_broken: u32,
    pub last_frost_spread: Option<CellPos>,
    pub patron: PatronGod,
    pub fury: u32,
    pub fury_max: u32,
    pub fury_charge_bonus_pct: u32,
    pub fury_resource_bonus_pct: u32,
    pub boss: Option<BossState>,
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
        Self::start_full(level, amulet, city_bonuses, PatronGod::default(), None, rng)
    }

    /// Starts a Mythological Boss Raid encounter.
    pub fn start_boss<R: Rng>(
        boss: BossDef,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        rng: &mut R,
    ) -> Self {
        let level = boss.to_level_def();
        Self::start_full(
            level,
            amulet,
            city_bonuses,
            patron,
            Some(BossState::new(boss)),
            rng,
        )
    }

    /// Starts an Endless Drakkar Expedition wave applying accumulated roguelite boons.
    pub fn start_expedition_wave<R: Rng>(
        run: &ExpeditionRun,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        rng: &mut R,
    ) -> Self {
        let level = run.generate_wave_level(rng);
        let mut session = Self::start_full(level, amulet, city_bonuses, patron, None, rng);
        session.score_bonus_pct += run.score_bonus_pct;
        session.fury = (session.fury + run.fury_headstart).min(FURY_CHARGE_MAX);
        for _ in 0..run.starting_boosters {
            session.board.spawn_random_starting_booster(rng);
        }
        session
    }

    /// Full session constructor supporting Patron God, optional Boss Raid, amulet, and city bonuses.
    pub fn start_full<R: Rng>(
        level: LevelDef,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        boss: Option<BossState>,
        rng: &mut R,
    ) -> Self {
        let mut board = Board::new_for_level(&level, rng);
        if let Some(AmuletEffect::StartingBoosters(count)) = amulet {
            for _ in 0..count {
                board.spawn_random_starting_booster(rng);
            }
        }

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
        let amulet_time_pct = match amulet {
            Some(AmuletEffect::ExtraTimePct(p)) => p,
            _ => 0,
        };
        let time_left_secs = level
            .time_limit_seconds
            .map(|s| (s as f32) * (1.0 + amulet_time_pct as f32 / 100.0));
        let total_moves = level.max_moves + amulet_moves + city_bonuses.extra_moves;

        Self {
            initial_moves: total_moves,
            moves_left: total_moves,
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
            turn_ice_broken: 0,
            last_frost_spread: None,
            patron,
            fury: city_bonuses.starting_fury.min(FURY_CHARGE_MAX),
            fury_max: FURY_CHARGE_MAX,
            fury_charge_bonus_pct: city_bonuses.fury_charge_bonus_pct,
            fury_resource_bonus_pct: 0,
            boss,
        }
    }

    /// Returns true when the Divine Fury gauge is fully charged and ready to unleash.
    pub fn can_trigger_fury(&self) -> bool {
        self.outcome == LevelOutcome::InProgress && self.fury >= self.fury_max
    }

    /// Unleashes the active Patron God's Divine Fury power (free, consumes no move).
    pub fn trigger_patron_fury<R: Rng>(&mut self, rng: &mut R) -> Option<PreparedClear> {
        if !self.can_trigger_fury() {
            return None;
        }
        self.fury = 0;
        match self.patron {
            PatronGod::Freya => {
                self.fury_resource_bonus_pct += 25;
            }
            PatronGod::Odin => {
                self.moves_left += 2;
            }
            PatronGod::Thor | PatronGod::Loki => {}
        }
        self.board.apply_patron_fury(self.patron, rng)
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

    /// Attempts a player swap, updating moves, score, objectives, frost spread, and victory/defeat state.
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
            self.apply_step_progress(&step);
            self.apply_frost_spread_if_needed(rng);
            let riposte = self.tick_boss_riposte_if_needed(rng);
            if !riposte.is_empty() && !self.board.has_valid_moves() {
                self.board.shuffle_free_runes(rng);
                self.last_shuffled = true;
            }
            self.finalize_turn(balance);
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
            self.apply_step_progress(&step);
            self.finalize_turn(balance);
        }
        step
    }

    /// Decrements remaining moves by one and resets per-turn trackers.
    pub fn consume_move(&mut self) {
        self.moves_left = self.moves_left.saturating_sub(1);
        self.turn_ice_broken = 0;
        self.last_frost_spread = None;
    }

    /// Applies score and objective progress from a single animated clear step without ending the turn yet.
    pub fn apply_step_progress(&mut self, step: &MatchStepOutcome) {
        let boosted_score = step.score_gained + (step.score_gained * self.score_bonus_pct) / 100;
        self.score += boosted_score;
        self.last_combo = step.max_combo;
        self.last_shuffled = step.shuffled;
        self.turn_ice_broken += step.ice_broken;

        let tiles_cleared: u32 = step.cleared_runes.values().copied().sum();
        let obstacles_broken = step.ice_broken + step.chains_broken + step.crates_broken;
        let combo_bonus = step.max_combo.saturating_sub(1) * FURY_PER_COMBO_STEP;
        let raw_fury =
            tiles_cleared * FURY_PER_TILE + obstacles_broken * FURY_PER_OBSTACLE + combo_bonus;
        let fury_gain = raw_fury + (raw_fury * self.fury_charge_bonus_pct) / 100;
        self.fury = (self.fury + fury_gain).min(self.fury_max);

        if let Some(boss) = self.boss.as_mut() {
            boss.apply_step_damage(step, self.score_bonus_pct);
        }

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
                    if let Some(boss) = &self.boss {
                        prog.done = boss.def.max_hp.saturating_sub(boss.hp).min(prog.target);
                    } else {
                        prog.done = self.score.min(prog.target);
                    }
                }
                Objective::DropRelics { .. } => {
                    prog.done = (prog.done + step.relics_dropped).min(prog.target);
                }
            }
        }
    }

    /// Executes the boss counter-attack countdown at end of turn if a Boss Raid is active.
    pub fn tick_boss_riposte_if_needed<R: Rng>(&mut self, rng: &mut R) -> Vec<CellPos> {
        if let Some(boss) = self.boss.as_mut() {
            boss.tick_turn_and_riposte(&mut self.board, rng)
        } else {
            Vec::new()
        }
    }

    /// On frost-spread levels, freezes 1 adjacent cell if no ice was broken during this turn.
    pub fn apply_frost_spread_if_needed<R: Rng>(&mut self, rng: &mut R) -> Option<CellPos> {
        if !self.level.frost_spread
            || self.turn_ice_broken > 0
            || self.objectives.iter().all(ObjectiveProgress::is_complete)
        {
            return None;
        }
        let spread = self.board.spread_frost(rng);
        self.last_frost_spread = spread;
        spread
    }

    /// Evaluates victory or defeat after all cascades of a turn have finished.
    pub fn finalize_turn(&mut self, balance: &BalanceConfig) {
        if self.outcome != LevelOutcome::InProgress {
            return;
        }
        let won = if let Some(boss) = &self.boss {
            boss.is_defeated()
        } else {
            self.objectives.iter().all(ObjectiveProgress::is_complete)
        };
        if won {
            self.score += self.moves_left * balance.remaining_move_bonus;
            self.outcome = LevelOutcome::Victory;
        } else if self.moves_left == 0 {
            self.outcome = LevelOutcome::Defeat;
        }
    }

    /// Computes a 1..=3 star rating for a completed victory based on remaining moves efficiency.
    pub fn earned_stars(&self) -> u8 {
        if self.outcome != LevelOutcome::Victory {
            return 0;
        }
        let ratio = if self.initial_moves > 0 {
            self.moves_left as f32 / self.initial_moves as f32
        } else {
            0.0
        };
        if ratio >= 0.30 {
            3
        } else if ratio >= 0.12 {
            2
        } else {
            1
        }
    }

    /// Computes final resource rewards when the level is won (scaled by remaining moves + bonuses).
    pub fn earned_rewards(&self, balance: &BalanceConfig) -> ResourceBundle {
        if self.outcome != LevelOutcome::Victory {
            return ResourceBundle::default();
        }
        let move_bonus = (self.moves_left * balance.move_reward_bonus_pct_per_move)
            .min(balance.move_reward_bonus_cap_pct);
        let bonus_pct = move_bonus + self.resource_bonus_pct + self.fury_resource_bonus_pct;
        let scale = |base: u32| base + (base * bonus_pct) / 100;
        ResourceBundle {
            wood: scale(self.level.base_reward.wood),
            stone: scale(self.level.base_reward.stone),
            gold: scale(self.level.base_reward.gold),
            mead: scale(self.level.base_reward.mead),
        }
    }
}
