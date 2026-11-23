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

//! Endless Drakkar Expedition: roguelite Match-3 waves with escalating challenges and boons.

use crate::balance::ExpeditionTuning;
use crate::board::RuneType;
use crate::city::{CityState, ResourceBundle};
use crate::level::{LevelDef, Objective};
use crate::patron::FURY_CHARGE_MAX;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Building identifier carried by Endless Drakkar Expedition levels.
pub const EXPEDITION_BUILDING_ID: &str = "expedition";
/// Number of cells along each side of an expedition board.
const EXPEDITION_BOARD_SIZE: usize = 8;
/// Rows of the central ice field filled in reading order.
const EXPEDITION_ICE_ROWS: std::ops::RangeInclusive<usize> = 2..=5;
/// Columns of the central ice field filled in reading order.
const EXPEDITION_ICE_COLS: std::ops::RangeInclusive<usize> = 2..=5;

/// Roguelite blessing offered between Expedition waves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpeditionBoon {
    ExtraMoves(u32),
    ScoreBoostPct(u32),
    GrantMjolnir(u32),
    StartingBooster,
    FuryHeadstart(u32),
}

impl ExpeditionBoon {
    pub const ALL: [ExpeditionBoon; 5] = [
        ExpeditionBoon::ExtraMoves(3),
        ExpeditionBoon::ScoreBoostPct(25),
        ExpeditionBoon::GrantMjolnir(2),
        ExpeditionBoon::StartingBooster,
        ExpeditionBoon::FuryHeadstart(50),
    ];

    pub const fn name_key(self) -> &'static str {
        match self {
            ExpeditionBoon::ExtraMoves(_) => "expedition.boon.moves.name",
            ExpeditionBoon::ScoreBoostPct(_) => "expedition.boon.score.name",
            ExpeditionBoon::GrantMjolnir(_) => "expedition.boon.mjolnir.name",
            ExpeditionBoon::StartingBooster => "expedition.boon.booster.name",
            ExpeditionBoon::FuryHeadstart(_) => "expedition.boon.fury.name",
        }
    }

    pub const fn desc_key(self) -> &'static str {
        match self {
            ExpeditionBoon::ExtraMoves(_) => "expedition.boon.moves.desc",
            ExpeditionBoon::ScoreBoostPct(_) => "expedition.boon.score.desc",
            ExpeditionBoon::GrantMjolnir(_) => "expedition.boon.mjolnir.desc",
            ExpeditionBoon::StartingBooster => "expedition.boon.booster.desc",
            ExpeditionBoon::FuryHeadstart(_) => "expedition.boon.fury.desc",
        }
    }
}

/// Active Endless Drakkar Expedition run state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpeditionRun {
    pub wave: u32,
    pub extra_moves: u32,
    pub score_bonus_pct: u32,
    pub starting_boosters: u32,
    pub fury_headstart: u32,
    pub pending_boons: Vec<ExpeditionBoon>,
}

impl Default for ExpeditionRun {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpeditionRun {
    pub fn new() -> Self {
        Self {
            wave: 1,
            extra_moves: 0,
            score_bonus_pct: 0,
            starting_boosters: 0,
            fury_headstart: 0,
            pending_boons: Vec::new(),
        }
    }

    /// Generates an escalating procedural board for the current expedition wave.
    pub fn generate_wave_level<R: Rng>(&self, rng: &mut R) -> LevelDef {
        self.generate_wave_level_with(&ExpeditionTuning::default(), rng)
    }

    /// Generates the current wave board driven by the given expedition tuning.
    pub fn generate_wave_level_with<R: Rng>(&self, t: &ExpeditionTuning, rng: &mut R) -> LevelDef {
        let w = self.wave;
        let rune_types_count = if w == 1 {
            t.wave_one_rune_types.min(RuneType::ALL.len())
        } else {
            RuneType::ALL.len()
        };
        let target_rune = RuneType::ALL[rng.gen_range(0..rune_types_count)];
        let ice_target = t
            .ice_base
            .saturating_add(w.saturating_mul(t.ice_per_wave))
            .min(t.ice_cap);
        let rune_target = t
            .rune_target_base
            .saturating_add(w.saturating_mul(t.rune_target_per_wave))
            .min(t.rune_target_cap);

        let mut ice_cells = Vec::new();
        for r in EXPEDITION_ICE_ROWS {
            for c in EXPEDITION_ICE_COLS {
                if (ice_cells.len() as u32) < ice_target {
                    let layers = if w >= t.double_ice_from_wave && (c == 2 || c == 5) {
                        2
                    } else {
                        1
                    };
                    ice_cells.push((c, r, layers));
                }
            }
        }

        let mut chained_cells = Vec::new();
        if w >= t.chains_from_wave {
            chained_cells.push((3, 3));
            chained_cells.push((4, 4));
        }
        if w >= t.extra_chains_from_wave {
            chained_cells.push((3, 4));
            chained_cells.push((4, 3));
        }

        let mut crate_cells = Vec::new();
        if w >= t.crates_from_wave {
            crate_cells.push((1, 3, 1));
            crate_cells.push((6, 4, 1));
        }

        let portals = if w >= t.portals_from_wave && w.is_multiple_of(2) {
            vec![((1, 1), (6, 6)), ((6, 1), (1, 6))]
        } else {
            vec![]
        };

        let objectives = if w.is_multiple_of(t.score_wave_interval) {
            vec![
                Objective::ClearIce { target: ice_target },
                Objective::ReachScore {
                    target: t
                        .score_base
                        .saturating_add(w.saturating_mul(t.score_per_wave)),
                },
            ]
        } else {
            vec![
                Objective::ClearIce { target: ice_target },
                Objective::CollectRune {
                    rune: target_rune,
                    target: rune_target,
                },
            ]
        };

        let wave_reward =
            |base: u32, per_wave: u32| base.saturating_add(w.saturating_mul(per_wave));

        LevelDef {
            id: format!("expedition_w{w}"),
            building_id: EXPEDITION_BUILDING_ID.to_string(),
            cols: EXPEDITION_BOARD_SIZE,
            rows: EXPEDITION_BOARD_SIZE,
            max_moves: t
                .base_moves
                .saturating_add(self.extra_moves)
                .min(t.moves_cap),
            time_limit_seconds: None,
            rune_types_count,
            objectives,
            ice_cells,
            chained_cells,
            crate_cells,
            relic_cells: vec![],
            portals,
            frost_spread: w >= t.frost_from_wave,
            fog_cells: if w >= t.fog_from_wave && !w.is_multiple_of(2) {
                vec![(2, 3), (3, 3), (4, 3), (5, 3), (3, 4), (4, 4)]
            } else {
                vec![]
            },
            cursed_cells: if w >= t.curse_from_wave {
                vec![(2, 2, t.curse_turns), (5, 5, t.curse_turns)]
            } else {
                vec![]
            },
            blocked_cells: vec![(0, 0), (7, 0), (0, 7), (7, 7)],
            base_reward: ResourceBundle {
                wood: wave_reward(t.reward_base.wood, t.reward_per_wave.wood),
                stone: wave_reward(t.reward_base.stone, t.reward_per_wave.stone),
                gold: wave_reward(t.reward_base.gold, t.reward_per_wave.gold),
                mead: wave_reward(t.reward_base.mead, t.reward_per_wave.mead),
            },
        }
    }

    /// Rolls 3 distinct roguelite boons to choose from after winning a wave.
    pub fn roll_boon_choices<R: Rng>(&mut self, rng: &mut R) {
        self.roll_boon_choices_with(&ExpeditionTuning::default(), rng);
    }

    /// Rolls the configured number of distinct boons from the configured boon pool.
    pub fn roll_boon_choices_with<R: Rng>(&mut self, tuning: &ExpeditionTuning, rng: &mut R) {
        let mut pool = tuning.boons.clone();
        pool.shuffle(rng);
        pool.truncate(tuning.boon_choices);
        self.pending_boons = pool;
    }

    /// Applies a chosen boon and advances to the next wave.
    pub fn apply_boon(&mut self, boon: ExpeditionBoon, city: &mut CityState) {
        match boon {
            ExpeditionBoon::ExtraMoves(m) => {
                self.extra_moves = self.extra_moves.saturating_add(m);
            }
            ExpeditionBoon::ScoreBoostPct(p) => {
                self.score_bonus_pct = self.score_bonus_pct.saturating_add(p);
            }
            ExpeditionBoon::GrantMjolnir(n) => {
                city.tools.mjolnir = city.tools.mjolnir.saturating_add(n);
            }
            ExpeditionBoon::StartingBooster => {
                self.starting_boosters = self.starting_boosters.saturating_add(1);
            }
            ExpeditionBoon::FuryHeadstart(f) => {
                self.fury_headstart = self.fury_headstart.saturating_add(f).min(FURY_CHARGE_MAX);
            }
        }
        self.pending_boons.clear();
        self.wave += 1;
    }
}
