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

use crate::board::RuneType;
use crate::city::{CityState, ResourceBundle};
use crate::level::{LevelDef, Objective};
use rand::seq::SliceRandom;
use rand::Rng;

/// Roguelite blessing offered between Expedition waves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
        let w = self.wave;
        let target_rune = RuneType::ALL[rng.gen_range(0..RuneType::ALL.len())];
        let ice_target = (6 + w * 2).min(16);
        let rune_target = (14 + w * 3).min(36);

        let mut ice_cells = Vec::new();
        for r in 2..=5 {
            for c in 2..=5 {
                if (ice_cells.len() as u32) < ice_target {
                    let layers = if w >= 3 && (c == 2 || c == 5) { 2 } else { 1 };
                    ice_cells.push((c, r, layers));
                }
            }
        }

        let mut chained_cells = Vec::new();
        if w >= 2 {
            chained_cells.push((3, 3));
            chained_cells.push((4, 4));
        }
        if w >= 4 {
            chained_cells.push((3, 4));
            chained_cells.push((4, 3));
        }

        let mut crate_cells = Vec::new();
        if w >= 3 {
            crate_cells.push((1, 3, 1));
            crate_cells.push((6, 4, 1));
        }

        let portals = if w >= 4 && w.is_multiple_of(2) {
            vec![((1, 1), (6, 6)), ((6, 1), (1, 6))]
        } else {
            vec![]
        };

        let objectives = if w.is_multiple_of(3) {
            vec![
                Objective::ClearIce { target: ice_target },
                Objective::ReachScore {
                    target: 3000 + w * 900,
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

        LevelDef {
            id: format!("expedition_w{w}"),
            building_id: "expedition".to_string(),
            cols: 8,
            rows: 8,
            max_moves: (22 + self.extra_moves).min(36),
            time_limit_seconds: None,
            rune_types_count: if w == 1 { 5 } else { 6 },
            objectives,
            ice_cells,
            chained_cells,
            crate_cells,
            relic_cells: vec![],
            portals,
            frost_spread: w >= 5,
            fog_cells: if w >= 4 && !w.is_multiple_of(2) {
                vec![(2, 3), (3, 3), (4, 3), (5, 3), (3, 4), (4, 4)]
            } else {
                vec![]
            },
            blocked_cells: vec![(0, 0), (7, 0), (0, 7), (7, 7)],
            base_reward: ResourceBundle {
                wood: 50 + w * 20,
                stone: 45 + w * 20,
                gold: 65 + w * 25,
                mead: 12 + w * 5,
            },
        }
    }

    /// Rolls 3 distinct roguelite boons to choose from after winning a wave.
    pub fn roll_boon_choices<R: Rng>(&mut self, rng: &mut R) {
        let mut pool = ExpeditionBoon::ALL.to_vec();
        pool.shuffle(rng);
        pool.truncate(3);
        self.pending_boons = pool;
    }

    /// Applies a chosen boon and advances to the next wave.
    pub fn apply_boon(&mut self, boon: ExpeditionBoon, city: &mut CityState) {
        match boon {
            ExpeditionBoon::ExtraMoves(m) => self.extra_moves += m,
            ExpeditionBoon::ScoreBoostPct(p) => self.score_bonus_pct += p,
            ExpeditionBoon::GrantMjolnir(n) => city.tools.mjolnir += n,
            ExpeditionBoon::StartingBooster => self.starting_boosters += 1,
            ExpeditionBoon::FuryHeadstart(f) => {
                self.fury_headstart = (self.fury_headstart + f).min(100)
            }
        }
        self.pending_boons.clear();
        self.wave += 1;
    }
}
