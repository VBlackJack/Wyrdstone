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

use crate::balance::{BalanceConfig, ExpeditionTuning};
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
/// Start/end inclusive indices of the central ice field.
const EXPEDITION_ICE_MIN: usize = 2;
const EXPEDITION_ICE_MAX: usize = 5;
/// Rows of the central ice field filled in reading order.
const EXPEDITION_ICE_ROWS: std::ops::RangeInclusive<usize> =
    EXPEDITION_ICE_MIN..=EXPEDITION_ICE_MAX;
/// Columns of the central ice field filled in reading order.
const EXPEDITION_ICE_COLS: std::ops::RangeInclusive<usize> =
    EXPEDITION_ICE_MIN..=EXPEDITION_ICE_MAX;
/// Maximum number of ice cells that can be placed in an expedition wave.
pub(crate) const MAX_EXPEDITION_ICE_CELLS: u32 = ((EXPEDITION_ICE_MAX - EXPEDITION_ICE_MIN + 1)
    * (EXPEDITION_ICE_MAX - EXPEDITION_ICE_MIN + 1))
    as u32;
/// Upper bound applied when sanitizing persisted expedition score bonuses.
pub const MAX_EXPEDITION_SCORE_BONUS_PCT: u32 = 1_000;

/// Build-defining roguelite Relic collected during an Endless Drakkar Expedition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExpeditionRelic {
    ValkyrieFeathers,
    SurtrCoal,
    FrostbreakerAxe,
    MimirScroll,
    Gjallarhorn,
    BifrostCompass,
}

impl ExpeditionRelic {
    pub const ALL: [Self; 6] = [
        Self::ValkyrieFeathers,
        Self::SurtrCoal,
        Self::FrostbreakerAxe,
        Self::MimirScroll,
        Self::Gjallarhorn,
        Self::BifrostCompass,
    ];

    pub const fn name_key(self) -> &'static str {
        match self {
            Self::ValkyrieFeathers => "expedition.relic.feathers.name",
            Self::SurtrCoal => "expedition.relic.coal.name",
            Self::FrostbreakerAxe => "expedition.relic.axe.name",
            Self::MimirScroll => "expedition.relic.scroll.name",
            Self::Gjallarhorn => "expedition.relic.horn.name",
            Self::BifrostCompass => "expedition.relic.compass.name",
        }
    }

    pub const fn desc_key(self) -> &'static str {
        match self {
            Self::ValkyrieFeathers => "expedition.relic.feathers.desc",
            Self::SurtrCoal => "expedition.relic.coal.desc",
            Self::FrostbreakerAxe => "expedition.relic.axe.desc",
            Self::MimirScroll => "expedition.relic.scroll.desc",
            Self::Gjallarhorn => "expedition.relic.horn.desc",
            Self::BifrostCompass => "expedition.relic.compass.desc",
        }
    }
}

/// Optional risk/reward Skald's Oath contract taken before an Expedition wave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExpeditionOath {
    IronVow,
    Frostbound,
    BerserkerRage,
}

impl ExpeditionOath {
    pub const ALL: [Self; 3] = [Self::IronVow, Self::Frostbound, Self::BerserkerRage];

    pub const fn name_key(self) -> &'static str {
        match self {
            Self::IronVow => "expedition.oath.iron.name",
            Self::Frostbound => "expedition.oath.frost.name",
            Self::BerserkerRage => "expedition.oath.berserker.name",
        }
    }

    pub const fn desc_key(self) -> &'static str {
        match self {
            Self::IronVow => "expedition.oath.iron.desc",
            Self::Frostbound => "expedition.oath.frost.desc",
            Self::BerserkerRage => "expedition.oath.berserker.desc",
        }
    }
}

/// Roguelite blessing offered between Expedition waves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpeditionBoon {
    ExtraMoves(u32),
    ScoreBoostPct(u32),
    GrantMjolnir(u32),
    StartingBooster,
    FuryHeadstart(u32),
    Relic(ExpeditionRelic),
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
            ExpeditionBoon::Relic(relic) => relic.name_key(),
        }
    }

    pub const fn desc_key(self) -> &'static str {
        match self {
            ExpeditionBoon::ExtraMoves(_) => "expedition.boon.moves.desc",
            ExpeditionBoon::ScoreBoostPct(_) => "expedition.boon.score.desc",
            ExpeditionBoon::GrantMjolnir(_) => "expedition.boon.mjolnir.desc",
            ExpeditionBoon::StartingBooster => "expedition.boon.booster.desc",
            ExpeditionBoon::FuryHeadstart(_) => "expedition.boon.fury.desc",
            ExpeditionBoon::Relic(relic) => relic.desc_key(),
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
    #[serde(default)]
    pub relics: Vec<ExpeditionRelic>,
    #[serde(default)]
    pub pending_oath: Option<ExpeditionOath>,
    #[serde(default)]
    pub oath_accepted: bool,
    #[serde(default)]
    pub active_oath: Option<ExpeditionOath>,
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
            relics: Vec::new(),
            pending_oath: None,
            oath_accepted: false,
            active_oath: None,
        }
    }

    /// True on every 5th Expedition wave (Mythological Boss encounter).
    pub const fn is_boss_wave(&self) -> bool {
        self.wave > 0 && self.wave.is_multiple_of(5)
    }

    /// Clamps persisted expedition run fields to legal bounds so tampered saves cannot freeze or overflow.
    pub fn sanitize(&mut self, balance: &BalanceConfig) {
        self.wave = self.wave.max(1);
        let max_board_cells = (EXPEDITION_BOARD_SIZE * EXPEDITION_BOARD_SIZE) as u32;
        self.starting_boosters = self.starting_boosters.min(max_board_cells);
        self.extra_moves = self.extra_moves.min(balance.expedition.moves_cap);
        self.score_bonus_pct = self.score_bonus_pct.min(MAX_EXPEDITION_SCORE_BONUS_PCT);
        self.fury_headstart = self.fury_headstart.min(FURY_CHARGE_MAX);
        self.pending_boons.truncate(balance.expedition.boon_choices);
        let mut seen = Vec::new();
        self.relics.retain(|&r| {
            if seen.contains(&r) {
                false
            } else {
                seen.push(r);
                true
            }
        });
    }

    /// Generates an escalating procedural board for the current expedition wave.
    pub fn generate_wave_level<R: Rng>(&self, rng: &mut R) -> LevelDef {
        self.generate_wave_level_with(&ExpeditionTuning::default(), rng)
    }

    /// Generates the current wave board driven by the given expedition tuning.
    pub fn generate_wave_level_with<R: Rng>(&self, t: &ExpeditionTuning, rng: &mut R) -> LevelDef {
        let w = self.wave.max(1);
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

        let placed_ice = ice_cells.len() as u32;
        let objectives = if w.is_multiple_of(t.score_wave_interval) {
            vec![
                Objective::ClearIce { target: placed_ice },
                Objective::ReachScore {
                    target: t
                        .score_base
                        .saturating_add(w.saturating_mul(t.score_per_wave)),
                },
            ]
        } else {
            vec![
                Objective::ClearIce { target: placed_ice },
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

    /// Generates an Expedition 2.0 wave with rotating board shapes, Relics, and Skald's Oaths.
    pub fn generate_wave_level_v2<R: Rng>(&self, t: &ExpeditionTuning, rng: &mut R) -> LevelDef {
        let mut lvl = self.generate_wave_level_with(t, rng);
        let w = self.wave.max(1);
        let mut max_moves = t.base_moves.saturating_add(self.extra_moves);
        if self.relics.contains(&ExpeditionRelic::BifrostCompass) {
            max_moves = max_moves.saturating_add(2);
        }
        if self.active_oath == Some(ExpeditionOath::IronVow) {
            max_moves = max_moves.saturating_sub(3).max(12);
        }
        lvl.max_moves = max_moves.min(t.moves_cap);
        lvl.blocked_cells = match (w - 1) % 6 {
            0 => vec![(0, 0), (7, 0), (0, 7), (7, 7)],
            1 => vec![(0, 3), (0, 4), (7, 3), (7, 4)],
            2 => vec![(3, 0), (4, 0), (3, 7), (4, 7)],
            3 => vec![
                (0, 0),
                (1, 0),
                (0, 1),
                (7, 0),
                (6, 0),
                (7, 1),
                (0, 7),
                (7, 7),
            ],
            4 => vec![
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (7, 2),
                (7, 3),
                (7, 4),
                (7, 5),
            ],
            _ => vec![(0, 1), (0, 6), (7, 1), (7, 6)],
        };
        if self.active_oath == Some(ExpeditionOath::Frostbound) {
            lvl.frost_spread = true;
        }
        if self.active_oath == Some(ExpeditionOath::BerserkerRage) {
            for cpos in [(2, 5, t.curse_turns), (5, 2, t.curse_turns)] {
                if !lvl
                    .cursed_cells
                    .iter()
                    .any(|&(c, r, _)| c == cpos.0 && r == cpos.1)
                {
                    lvl.cursed_cells.push(cpos);
                }
            }
        }
        lvl
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

    /// Rolls Expedition 2.0 choices: offers an unowned Relic slot and rolls an optional Skald's Oath.
    pub fn roll_boon_choices_v2<R: Rng>(&mut self, tuning: &ExpeditionTuning, rng: &mut R) {
        self.roll_boon_choices_with(tuning, rng);
        let unowned: Vec<ExpeditionRelic> = ExpeditionRelic::ALL
            .iter()
            .copied()
            .filter(|r| !self.relics.contains(r))
            .collect();
        if !unowned.is_empty() && !self.pending_boons.is_empty() {
            let pick = unowned[rng.gen_range(0..unowned.len())];
            self.pending_boons[0] = ExpeditionBoon::Relic(pick);
        }
        self.oath_accepted = false;
        self.pending_oath = if self.wave >= 2 {
            Some(ExpeditionOath::ALL[rng.gen_range(0..ExpeditionOath::ALL.len())])
        } else {
            None
        };
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
            ExpeditionBoon::Relic(relic) => {
                if !self.relics.contains(&relic) {
                    self.relics.push(relic);
                }
            }
        }
        self.active_oath = if self.oath_accepted {
            self.pending_oath.take()
        } else {
            self.pending_oath = None;
            None
        };
        self.oath_accepted = false;
        self.pending_boons.clear();
        self.wave = self.wave.saturating_add(1);
    }
}
