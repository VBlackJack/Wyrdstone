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

//! Mythological Boss Raid definitions, HP tracking, weakness multipliers, and counter-attacks.

use crate::balance::{add_pct_u32, BossTuning};
use crate::board::{Board, CellPos, MatchStepOutcome, RuneType};
use crate::city::{ResourceBundle, ToolInventory};
use crate::level::{LevelDef, Objective};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Building identifier carried by Mythological Boss Raid levels.
pub const BOSS_RAID_BUILDING_ID: &str = "boss_raid";
/// Number of cells along each side of a boss raid board.
const BOSS_BOARD_SIZE: usize = 8;
/// Number of rune types in play during a boss raid.
const BOSS_RUNE_TYPES: usize = 6;
/// Percent scale used by the phase 2 hit point threshold.
const PERCENT_SCALE: u64 = 100;

/// Counter-attack action executed by a boss every N turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BossRiposteKind {
    ChainRunes(usize),
    FreezeCells(usize),
    DropCrates(usize),
    CurseRunes(usize),
}

/// Static Mythological Boss definition loaded from `assets/data/bosses.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossDef {
    pub id: String,
    pub name_key: String,
    pub title_key: String,
    pub desc_key: String,
    pub max_hp: u32,
    pub max_moves: u32,
    pub weakness: RuneType,
    pub riposte_interval: u32,
    pub riposte: BossRiposteKind,
    #[serde(default)]
    pub phase2_weakness: Option<RuneType>,
    #[serde(default)]
    pub phase2_riposte: Option<BossRiposteKind>,
    pub required_glory: u32,
    pub reward_resources: ResourceBundle,
    pub reward_tools: ToolInventory,
    pub glory_reward: u32,
}

impl BossDef {
    /// Builds a playable 8x8 boss raid board tailored to this mythological foe.
    pub fn to_level_def(&self) -> LevelDef {
        let (ice_cells, chained_cells, crate_cells, fog_cells, blocked_cells) = match self.riposte {
            BossRiposteKind::ChainRunes(_) => (
                vec![(2, 2, 1), (5, 2, 1), (2, 5, 1), (5, 5, 1)],
                vec![(3, 3), (4, 3), (3, 4), (4, 4)],
                vec![],
                vec![],
                vec![(0, 0), (7, 0), (0, 7), (7, 7)],
            ),
            BossRiposteKind::FreezeCells(_) => (
                vec![
                    (2, 2, 2),
                    (3, 2, 1),
                    (4, 2, 1),
                    (5, 2, 2),
                    (2, 5, 2),
                    (3, 5, 1),
                    (4, 5, 1),
                    (5, 5, 2),
                ],
                vec![(3, 4), (4, 3)],
                vec![],
                vec![],
                vec![(0, 3), (0, 4), (7, 3), (7, 4)],
            ),
            BossRiposteKind::DropCrates(_) => (
                vec![(1, 2, 1), (6, 2, 1), (1, 5, 1), (6, 5, 1)],
                vec![(2, 3), (5, 4)],
                vec![(3, 3, 2), (4, 4, 2)],
                vec![],
                vec![(0, 0), (7, 0), (0, 7), (7, 7)],
            ),
            BossRiposteKind::CurseRunes(_) => (
                vec![(2, 2, 1), (5, 2, 1), (2, 5, 1), (5, 5, 1)],
                vec![(3, 4), (4, 3)],
                vec![],
                vec![(1, 3), (2, 3), (5, 3), (6, 3), (2, 4), (5, 4)],
                vec![(0, 0), (7, 0), (0, 7), (7, 7)],
            ),
        };

        LevelDef {
            id: self.id.clone(),
            building_id: BOSS_RAID_BUILDING_ID.to_string(),
            cols: BOSS_BOARD_SIZE,
            rows: BOSS_BOARD_SIZE,
            max_moves: self.max_moves,
            time_limit_seconds: None,
            rune_types_count: BOSS_RUNE_TYPES,
            objectives: vec![Objective::ReachScore {
                target: self.max_hp,
            }],
            ice_cells,
            chained_cells,
            crate_cells,
            relic_cells: vec![],
            portals: vec![],
            frost_spread: false,
            fog_cells,
            cursed_cells: vec![],
            blocked_cells,
            base_reward: self.reward_resources,
        }
    }
}

/// Live boss state during an active Match-3 raid encounter.
#[derive(Debug, Clone)]
pub struct BossState {
    pub def: BossDef,
    pub hp: u32,
    pub turns_until_riposte: u32,
    pub last_damage_dealt: u32,
    pub last_riposte_cells: Vec<CellPos>,
    pub phase2_triggered: bool,
    pub just_entered_phase2: bool,
    /// Combat tuning captured when the encounter started.
    pub tuning: BossTuning,
}

impl BossState {
    pub fn new(def: BossDef) -> Self {
        Self::new_with(def, &BossTuning::default())
    }

    /// Builds a boss state that keeps a copy of the given combat tuning.
    pub fn new_with(def: BossDef, tuning: &BossTuning) -> Self {
        let hp = def.max_hp;
        let turns = def.riposte_interval.max(1);
        Self {
            tuning: *tuning,
            def,
            hp,
            turns_until_riposte: turns,
            last_damage_dealt: 0,
            last_riposte_cells: Vec::new(),
            phase2_triggered: false,
            just_entered_phase2: false,
        }
    }

    pub fn is_defeated(&self) -> bool {
        self.hp == 0
    }

    /// Returns true once the boss drops below the Phase 2 HP threshold and has a configured Phase 2.
    pub fn is_phase2(&self) -> bool {
        self.hp > 0
            && u64::from(self.hp) * PERCENT_SCALE
                < u64::from(self.def.max_hp) * u64::from(self.tuning.phase2_hp_pct)
            && (self.def.phase2_weakness.is_some() || self.def.phase2_riposte.is_some())
    }

    /// Active elemental weakness (switches in Phase 2 when configured).
    pub fn active_weakness(&self) -> RuneType {
        if self.is_phase2() {
            self.def.phase2_weakness.unwrap_or(self.def.weakness)
        } else {
            self.def.weakness
        }
    }

    /// Active counter-attack action (escalates in Phase 2 when configured).
    pub fn active_riposte(&self) -> BossRiposteKind {
        if self.is_phase2() {
            self.def.phase2_riposte.unwrap_or(self.def.riposte)
        } else {
            self.def.riposte
        }
    }

    /// Applies damage from a Match-3 clear step (2x multiplier on active boss weakness rune).
    pub fn apply_step_damage(&mut self, step: &MatchStepOutcome, score_bonus_pct: u32) -> u32 {
        let weakness = self.active_weakness();
        let mut raw_dmg = 0u32;
        for (&rune, &count) in &step.cleared_runes {
            let mult = if rune == weakness {
                self.tuning.weakness_multiplier
            } else {
                1
            };
            raw_dmg = raw_dmg.saturating_add(
                count
                    .saturating_mul(self.tuning.damage_per_tile)
                    .saturating_mul(mult),
            );
        }
        let obstacles = step
            .ice_broken
            .saturating_add(step.chains_broken)
            .saturating_add(step.crates_broken);
        raw_dmg = raw_dmg.saturating_add(obstacles.saturating_mul(self.tuning.damage_per_obstacle));
        let combo_mult =
            1.0 + step.max_combo.saturating_sub(1) as f32 * self.tuning.combo_damage_step;
        let boosted = (raw_dmg as f32 * combo_mult).round() as u32;
        let total = add_pct_u32(boosted, score_bonus_pct);
        self.hp = self.hp.saturating_sub(total);
        self.last_damage_dealt = total;
        if !self.phase2_triggered && self.is_phase2() {
            self.phase2_triggered = true;
            self.just_entered_phase2 = true;
        } else {
            self.just_entered_phase2 = false;
        }
        total
    }

    /// Ticks the riposte countdown after a player move; triggers boss counter-attack at 0.
    pub fn tick_turn_and_riposte<R: Rng>(
        &mut self,
        board: &mut Board,
        rng: &mut R,
    ) -> Vec<CellPos> {
        self.last_riposte_cells.clear();
        if self.is_defeated() {
            return Vec::new();
        }
        self.turns_until_riposte = self.turns_until_riposte.saturating_sub(1);
        if self.turns_until_riposte > 0 {
            return Vec::new();
        }
        self.turns_until_riposte = self.def.riposte_interval.max(1);
        let hit =
            board.apply_boss_riposte_with(self.active_riposte(), self.tuning.curse_turns, rng);
        self.last_riposte_cells = hit.clone();
        hit
    }
}
