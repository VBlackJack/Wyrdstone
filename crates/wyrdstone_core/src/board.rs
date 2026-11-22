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

//! Match-3 board simulation: runes, obstacles, relics, boosters, cascades, and all six active tools.

use crate::balance::BalanceConfig;
use crate::level::LevelDef;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Minimum run length required to form a match.
pub const MIN_MATCH_LEN: usize = 3;
/// Run length that spawns a line-clearing booster.
pub const LINE_BOOSTER_MATCH_LEN: usize = 4;
/// Run length that spawns a Bifrost Shard color-clearing booster.
pub const BIFROST_MATCH_LEN: usize = 5;
/// Maximum attempts when searching for a non-deadlocked shuffle.
const MAX_SHUFFLE_ATTEMPTS: usize = 32;

/// Coordinates on the Match-3 grid (`col` left-to-right, `row` top-to-bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellPos {
    pub col: usize,
    pub row: usize,
}

impl CellPos {
    pub const fn new(col: usize, row: usize) -> Self {
        Self { col, row }
    }

    /// Returns true when two cells share an edge orthogonally.
    pub fn is_adjacent(self, other: Self) -> bool {
        let dc = self.col.abs_diff(other.col);
        let dr = self.row.abs_diff(other.row);
        (dc == 1 && dr == 0) || (dc == 0 && dr == 1)
    }
}

/// The six Norse elemental rune types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuneType {
    Fire,
    Ice,
    Wood,
    Stone,
    Gold,
    Water,
}

impl RuneType {
    pub const ALL: [RuneType; 6] = [
        RuneType::Fire,
        RuneType::Ice,
        RuneType::Wood,
        RuneType::Stone,
        RuneType::Gold,
        RuneType::Water,
    ];

    /// Localization key for this rune's display name.
    pub const fn i18n_key(self) -> &'static str {
        match self {
            RuneType::Fire => "rune.fire",
            RuneType::Ice => "rune.ice",
            RuneType::Wood => "rune.wood",
            RuneType::Stone => "rune.stone",
            RuneType::Gold => "rune.gold",
            RuneType::Water => "rune.water",
        }
    }
}

/// Spawned special booster attached to a board cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Booster {
    #[default]
    None,
    LineHorizontal,
    LineVertical,
    Bomb,
    BifrostShard,
}

/// All six player-triggered tools from the bottom action bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActiveTool {
    /// Smashes a single tile, breaking chains/ice/crates or triggering its booster.
    Mjolnir,
    /// Blasts a 3x3 area centered on the target cell.
    SurtrEmber,
    /// Shuffles all unchained runes on the board.
    LokiShuffle,
    /// Strikes an entire horizontal row.
    ThorBolt,
    /// Clears all runes of the targeted cell's color across the board.
    BifrostPrism,
    /// Clears both the full row and full column of the target cell.
    DrakkarCharge,
}

/// Single grid cell state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub blocked: bool,
    pub rune: Option<RuneType>,
    pub booster: Booster,
    pub ice_layers: u8,
    pub chained: bool,
    pub crate_layers: u8,
    pub relic: bool,
}

impl Cell {
    pub const fn playable(rune: RuneType) -> Self {
        Self {
            blocked: false,
            rune: Some(rune),
            booster: Booster::None,
            ice_layers: 0,
            chained: false,
            crate_layers: 0,
            relic: false,
        }
    }

    pub const fn blocked_cell() -> Self {
        Self {
            blocked: true,
            rune: None,
            booster: Booster::None,
            ice_layers: 0,
            chained: false,
            crate_layers: 0,
            relic: false,
        }
    }

    /// Returns true when the content of this cell can be swapped by the player.
    pub fn can_be_swapped(&self) -> bool {
        !self.blocked
            && !self.chained
            && self.crate_layers == 0
            && (self.rune.is_some() || self.booster == Booster::BifrostShard || self.relic)
    }

    /// Returns true when the cell currently holds a movable piece (rune, booster, or relic).
    pub fn has_movable_piece(&self) -> bool {
        self.rune.is_some() || self.booster != Booster::None || self.relic
    }
}

/// Summary of everything that occurred during a player action (swap or tool).
#[derive(Debug, Clone, Default)]
pub struct MatchStepOutcome {
    pub valid: bool,
    pub score_gained: u32,
    pub max_combo: u32,
    pub ice_broken: u32,
    pub chains_broken: u32,
    pub crates_broken: u32,
    pub relics_dropped: u32,
    pub cleared_runes: HashMap<RuneType, u32>,
    pub cleared_positions: Vec<CellPos>,
    pub shuffled: bool,
}

/// Describes one tile's vertical drop during a gravity step (`from_row < 0` means spawned above).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallMove {
    pub col: usize,
    pub from_row: i32,
    pub to_row: usize,
}

/// Preview of cells about to clear in an animated step, before mutating the board.
#[derive(Debug, Clone)]
pub struct PreparedClear {
    pub cells: Vec<CellPos>,
    pub spawned_boosters: Vec<(CellPos, RuneType, Booster)>,
    pub is_direct_blast: bool,
}

/// Match-3 board state and rule resolver.
#[derive(Debug, Clone)]
pub struct Board {
    cols: usize,
    rows: usize,
    rune_pool_len: usize,
    cells: Vec<Cell>,
    portals: Vec<(CellPos, CellPos)>,
}

impl Board {
    /// Builds a fresh board from a level definition with no initial matches and at least one valid move.
    pub fn new_for_level<R: Rng>(level: &LevelDef, rng: &mut R) -> Self {
        let cols = level.cols;
        let rows = level.rows;
        let rune_pool_len = level.rune_types_count.clamp(3, RuneType::ALL.len());
        let portals: Vec<(CellPos, CellPos)> = level
            .portals
            .iter()
            .map(|&((sc, sr), (dc, dr))| (CellPos::new(sc, sr), CellPos::new(dc, dr)))
            .collect();
        let mut board = Self {
            cols,
            rows,
            rune_pool_len,
            cells: vec![Cell::blocked_cell(); cols * rows],
            portals,
        };

        let blocked_set: HashSet<(usize, usize)> = level.blocked_cells.iter().copied().collect();
        for row in 0..rows {
            for col in 0..cols {
                if blocked_set.contains(&(col, row)) {
                    board.set(CellPos::new(col, row), Cell::blocked_cell());
                } else {
                    let rune = board.pick_non_matching_rune(col, row, rng);
                    board.set(CellPos::new(col, row), Cell::playable(rune));
                }
            }
        }

        for &(col, row, layers) in &level.ice_cells {
            let pos = CellPos::new(col, row);
            if let Some(cell) = board.get_mut(pos) {
                if !cell.blocked {
                    cell.ice_layers = layers;
                }
            }
        }

        for &(col, row) in &level.chained_cells {
            let pos = CellPos::new(col, row);
            if let Some(cell) = board.get_mut(pos) {
                if !cell.blocked {
                    cell.chained = true;
                }
            }
        }

        for &(col, row, layers) in &level.crate_cells {
            let pos = CellPos::new(col, row);
            if let Some(cell) = board.get_mut(pos) {
                if !cell.blocked && layers > 0 {
                    cell.crate_layers = layers;
                    cell.rune = None;
                    cell.booster = Booster::None;
                    cell.chained = false;
                }
            }
        }

        for &(col, row) in &level.relic_cells {
            let pos = CellPos::new(col, row);
            if let Some(cell) = board.get_mut(pos) {
                if !cell.blocked && cell.crate_layers == 0 {
                    cell.relic = true;
                    cell.rune = None;
                    cell.booster = Booster::None;
                    cell.chained = false;
                }
            }
        }

        if !board.has_valid_moves() {
            board.shuffle_free_runes(rng);
        }

        board
    }

    /// Creates a deterministic custom board for unit testing.
    pub fn from_cells(cols: usize, rows: usize, rune_pool_len: usize, cells: Vec<Cell>) -> Self {
        assert_eq!(cells.len(), cols * rows);
        Self {
            cols,
            rows,
            rune_pool_len: rune_pool_len.clamp(3, RuneType::ALL.len()),
            cells,
            portals: Vec::new(),
        }
    }

    /// Returns all Bifrost portal pairs `(exit_cell, entry_cell)` on this board.
    pub fn portals(&self) -> &[(CellPos, CellPos)] {
        &self.portals
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    fn idx(&self, pos: CellPos) -> usize {
        pos.row * self.cols + pos.col
    }

    pub fn in_bounds(&self, pos: CellPos) -> bool {
        pos.col < self.cols && pos.row < self.rows
    }

    pub fn get(&self, pos: CellPos) -> Option<&Cell> {
        if self.in_bounds(pos) {
            Some(&self.cells[self.idx(pos)])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, pos: CellPos) -> Option<&mut Cell> {
        if self.in_bounds(pos) {
            let i = self.idx(pos);
            Some(&mut self.cells[i])
        } else {
            None
        }
    }

    fn set(&mut self, pos: CellPos, cell: Cell) {
        let i = self.idx(pos);
        self.cells[i] = cell;
    }

    fn random_rune<R: Rng>(&self, rng: &mut R) -> RuneType {
        RuneType::ALL[rng.gen_range(0..self.rune_pool_len)]
    }

    fn pick_non_matching_rune<R: Rng>(&self, col: usize, row: usize, rng: &mut R) -> RuneType {
        let mut candidates: Vec<RuneType> = RuneType::ALL[..self.rune_pool_len].to_vec();
        candidates.shuffle(rng);
        for rune in candidates {
            let horiz_match = col >= 2
                && self.get(CellPos::new(col - 1, row)).and_then(|c| c.rune) == Some(rune)
                && self.get(CellPos::new(col - 2, row)).and_then(|c| c.rune) == Some(rune);
            let vert_match = row >= 2
                && self.get(CellPos::new(col, row - 1)).and_then(|c| c.rune) == Some(rune)
                && self.get(CellPos::new(col, row - 2)).and_then(|c| c.rune) == Some(rune);
            if !horiz_match && !vert_match {
                return rune;
            }
        }
        self.random_rune(rng)
    }

    /// Returns true if two cells are orthogonally adjacent and both swappable.
    pub fn can_initiate_swap(&self, a: CellPos, b: CellPos) -> bool {
        if !a.is_adjacent(b) || !self.in_bounds(a) || !self.in_bounds(b) {
            return false;
        }
        self.cells[self.idx(a)].can_be_swapped() && self.cells[self.idx(b)].can_be_swapped()
    }

    /// Performs the raw swap and returns a `PreparedClear` if valid; reverts and returns `None` if invalid.
    pub fn begin_swap_step(&mut self, a: CellPos, b: CellPos) -> Option<PreparedClear> {
        if !self.can_initiate_swap(a, b) {
            return None;
        }
        self.swap_contents(a, b);
        if let Some(combo_cells) = self.check_special_swap_combo(a, b) {
            return Some(PreparedClear {
                cells: combo_cells.into_iter().collect(),
                spawned_boosters: Vec::new(),
                is_direct_blast: true,
            });
        }
        let groups = self.find_match_groups();
        if groups.is_empty() {
            self.swap_contents(a, b);
            return None;
        }
        Some(self.groups_to_prepared_clear(&groups, Some((a, b))))
    }

    /// Prepares the next cascade clear if any matches exist after gravity.
    pub fn prepare_cascade_step(&self) -> Option<PreparedClear> {
        let groups = self.find_match_groups();
        if groups.is_empty() {
            return None;
        }
        Some(self.groups_to_prepared_clear(&groups, None))
    }

    /// Prepares a direct-blast clear for targeted active tools.
    pub fn prepare_tool_step(&self, tool: ActiveTool, target: CellPos) -> Option<PreparedClear> {
        if !self.in_bounds(target) || self.cells[self.idx(target)].blocked {
            return None;
        }
        match tool {
            ActiveTool::Mjolnir => Some(PreparedClear {
                cells: vec![target],
                spawned_boosters: Vec::new(),
                is_direct_blast: true,
            }),
            ActiveTool::SurtrEmber => Some(PreparedClear {
                cells: self.area_positions(target, 1).into_iter().collect(),
                spawned_boosters: Vec::new(),
                is_direct_blast: true,
            }),
            ActiveTool::ThorBolt => Some(PreparedClear {
                cells: self.row_positions(target.row),
                spawned_boosters: Vec::new(),
                is_direct_blast: true,
            }),
            ActiveTool::BifrostPrism => {
                let target_rune = self.cells[self.idx(target)].rune?;
                let mut set = HashSet::new();
                self.collect_all_of_rune(target_rune, &mut set);
                Some(PreparedClear {
                    cells: set.into_iter().collect(),
                    spawned_boosters: Vec::new(),
                    is_direct_blast: true,
                })
            }
            ActiveTool::DrakkarCharge => {
                let mut set: HashSet<CellPos> = self.row_positions(target.row).into_iter().collect();
                set.extend(self.col_positions(target.col));
                Some(PreparedClear {
                    cells: set.into_iter().collect(),
                    spawned_boosters: Vec::new(),
                    is_direct_blast: true,
                })
            }
            ActiveTool::LokiShuffle => None,
        }
    }

    fn groups_to_prepared_clear(
        &self,
        groups: &[MatchGroup],
        swap_pair: Option<(CellPos, CellPos)>,
    ) -> PreparedClear {
        let mut to_clear: HashSet<CellPos> = HashSet::new();
        let mut spawned_boosters: Vec<(CellPos, RuneType, Booster)> = Vec::new();
        for group in groups {
            to_clear.extend(&group.cells);
            let booster = group.spawned_booster();
            if booster != Booster::None {
                let spawn_pos = swap_pair
                    .and_then(|(a, b)| {
                        if group.cells.contains(&b) {
                            Some(b)
                        } else if group.cells.contains(&a) {
                            Some(a)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| *group.cells.iter().next().unwrap());
                spawned_boosters.push((spawn_pos, group.rune, booster));
            }
        }
        PreparedClear {
            cells: to_clear.into_iter().collect(),
            spawned_boosters,
            is_direct_blast: false,
        }
    }

    /// Commits a previously prepared clear onto the board and returns the resulting step outcome.
    pub fn commit_clear_step(
        &mut self,
        prepared: &PreparedClear,
        combo_depth: u32,
        balance: &BalanceConfig,
    ) -> MatchStepOutcome {
        let mut outcome = MatchStepOutcome {
            valid: true,
            max_combo: combo_depth,
            ..Default::default()
        };
        let set: HashSet<CellPos> = prepared.cells.iter().copied().collect();
        self.clear_cell_set(&set, combo_depth, balance, &mut outcome, prepared.is_direct_blast);

        for &(pos, rune, booster) in &prepared.spawned_boosters {
            if let Some(cell) = self.get_mut(pos) {
                if !cell.blocked && cell.crate_layers == 0 && !cell.relic {
                    cell.booster = booster;
                    cell.rune = if booster == Booster::BifrostShard {
                        None
                    } else {
                        Some(rune)
                    };
                }
            }
        }
        outcome
    }

    /// Collects any Buried Relics resting on the lowest playable cell of their column.
    pub fn collect_bottom_relics(&mut self, balance: &BalanceConfig) -> MatchStepOutcome {
        let mut outcome = MatchStepOutcome::default();
        for col in 0..self.cols {
            // Find the lowest non-blocked cell in this column.
            let bottom_row = (0..self.rows)
                .rev()
                .find(|&r| !self.cells[self.idx(CellPos::new(col, r))].blocked);
            if let Some(row) = bottom_row {
                let pos = CellPos::new(col, row);
                let idx = self.idx(pos);
                if self.cells[idx].relic {
                    self.cells[idx].relic = false;
                    outcome.valid = true;
                    outcome.relics_dropped += 1;
                    outcome.score_gained += balance.relic_drop_bonus;
                    outcome.cleared_positions.push(pos);
                }
            }
        }
        outcome
    }

    /// Applies gravity, Bifrost portal transfers, and refills empty cells, returning fall trajectories.
    pub fn apply_gravity_and_refill_tracked<R: Rng>(&mut self, rng: &mut R) -> Vec<FallMove> {
        let mut moves = Vec::new();
        // Transfer movable pieces through Bifrost portals when destination cell is empty.
        let portals_snapshot = self.portals.clone();
        for (src, dst) in portals_snapshot {
            if self.in_bounds(src) && self.in_bounds(dst) {
                let si = self.idx(src);
                let di = self.idx(dst);
                let src_c = self.cells[si];
                let dst_c = self.cells[di];
                if !src_c.blocked
                    && !src_c.chained
                    && src_c.crate_layers == 0
                    && src_c.has_movable_piece()
                    && !dst_c.blocked
                    && !dst_c.chained
                    && dst_c.crate_layers == 0
                    && !dst_c.has_movable_piece()
                {
                    self.cells[di].rune = self.cells[si].rune.take();
                    self.cells[di].booster = self.cells[si].booster;
                    self.cells[si].booster = Booster::None;
                    self.cells[di].relic = self.cells[si].relic;
                    self.cells[si].relic = false;
                    moves.push(FallMove {
                        col: dst.col,
                        from_row: (dst.row as i32) - 1,
                        to_row: dst.row,
                    });
                }
            }
        }
        for col in 0..self.cols {
            let mut spawn_offset = 1i32;
            let mut write_row = self.rows as i32 - 1;
            while write_row >= 0 {
                let wpos = CellPos::new(col, write_row as usize);
                let wcell = self.cells[self.idx(wpos)];
                if wcell.blocked
                    || wcell.chained
                    || wcell.crate_layers > 0
                    || wcell.has_movable_piece()
                {
                    write_row -= 1;
                    continue;
                }

                let mut read_row = write_row - 1;
                let mut found = false;
                while read_row >= 0 {
                    let rpos = CellPos::new(col, read_row as usize);
                    let rcell = self.cells[self.idx(rpos)];
                    if rcell.blocked || rcell.chained || rcell.crate_layers > 0 {
                        read_row -= 1;
                        continue;
                    }
                    if rcell.has_movable_piece() {
                        let wi = self.idx(wpos);
                        let ri = self.idx(rpos);
                        self.cells[wi].rune = self.cells[ri].rune.take();
                        self.cells[wi].booster = self.cells[ri].booster;
                        self.cells[ri].booster = Booster::None;
                        self.cells[wi].relic = self.cells[ri].relic;
                        self.cells[ri].relic = false;
                        moves.push(FallMove {
                            col,
                            from_row: read_row,
                            to_row: write_row as usize,
                        });
                        found = true;
                        break;
                    }
                    read_row -= 1;
                }

                if !found {
                    let wi = self.idx(wpos);
                    self.cells[wi].rune = Some(self.random_rune(rng));
                    self.cells[wi].booster = Booster::None;
                    self.cells[wi].relic = false;
                    moves.push(FallMove {
                        col,
                        from_row: -spawn_offset,
                        to_row: write_row as usize,
                    });
                    spawn_offset += 1;
                }
                write_row -= 1;
            }
        }
        moves
    }

    /// Attempts to swap two adjacent cells and resolve all resulting matches, relics, and cascades.
    pub fn try_swap<R: Rng>(
        &mut self,
        a: CellPos,
        b: CellPos,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> MatchStepOutcome {
        let Some(first_clear) = self.begin_swap_step(a, b) else {
            return MatchStepOutcome::default();
        };
        let mut total = self.commit_clear_step(&first_clear, 1, balance);
        self.apply_gravity_and_refill_tracked(rng);
        self.collect_relics_loop(balance, rng, &mut total);

        let mut depth = 2;
        while let Some(next_clear) = self.prepare_cascade_step() {
            let step = self.commit_clear_step(&next_clear, depth, balance);
            merge_outcomes(&mut total, &step);
            self.apply_gravity_and_refill_tracked(rng);
            self.collect_relics_loop(balance, rng, &mut total);
            depth += 1;
        }

        if !self.has_valid_moves() {
            self.shuffle_free_runes(rng);
            total.shuffled = true;
        }
        total
    }

    /// Executes an active player tool from the action bar (instant resolution for tests/Loki).
    pub fn use_active_tool<R: Rng>(
        &mut self,
        tool: ActiveTool,
        target: Option<CellPos>,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> MatchStepOutcome {
        if tool == ActiveTool::LokiShuffle {
            self.shuffle_free_runes(rng);
            return MatchStepOutcome {
                valid: true,
                shuffled: true,
                ..Default::default()
            };
        }
        let Some(pos) = target else {
            return MatchStepOutcome::default();
        };
        let Some(prepared) = self.prepare_tool_step(tool, pos) else {
            return MatchStepOutcome::default();
        };
        let mut total = self.commit_clear_step(&prepared, 1, balance);
        self.apply_gravity_and_refill_tracked(rng);
        self.collect_relics_loop(balance, rng, &mut total);

        let mut depth = 2;
        while let Some(next_clear) = self.prepare_cascade_step() {
            let step = self.commit_clear_step(&next_clear, depth, balance);
            merge_outcomes(&mut total, &step);
            self.apply_gravity_and_refill_tracked(rng);
            self.collect_relics_loop(balance, rng, &mut total);
            depth += 1;
        }

        if !self.has_valid_moves() {
            self.shuffle_free_runes(rng);
            total.shuffled = true;
        }
        total
    }

    fn collect_relics_loop<R: Rng>(
        &mut self,
        balance: &BalanceConfig,
        rng: &mut R,
        total: &mut MatchStepOutcome,
    ) {
        loop {
            let dropped = self.collect_bottom_relics(balance);
            if dropped.relics_dropped == 0 {
                break;
            }
            merge_outcomes(total, &dropped);
            self.apply_gravity_and_refill_tracked(rng);
        }
    }

    fn swap_contents(&mut self, a: CellPos, b: CellPos) {
        let ia = self.idx(a);
        let ib = self.idx(b);
        let rune_a = self.cells[ia].rune;
        let booster_a = self.cells[ia].booster;
        let relic_a = self.cells[ia].relic;
        self.cells[ia].rune = self.cells[ib].rune;
        self.cells[ia].booster = self.cells[ib].booster;
        self.cells[ia].relic = self.cells[ib].relic;
        self.cells[ib].rune = rune_a;
        self.cells[ib].booster = booster_a;
        self.cells[ib].relic = relic_a;
    }

    fn check_special_swap_combo(&self, a: CellPos, b: CellPos) -> Option<HashSet<CellPos>> {
        let ca = self.cells[self.idx(a)];
        let cb = self.cells[self.idx(b)];
        let mut affected = HashSet::new();

        match (ca.booster, cb.booster) {
            (Booster::BifrostShard, Booster::BifrostShard) => {
                for row in 0..self.rows {
                    for col in 0..self.cols {
                        let p = CellPos::new(col, row);
                        if !self.cells[self.idx(p)].blocked {
                            affected.insert(p);
                        }
                    }
                }
                Some(affected)
            }
            (Booster::BifrostShard, _) => {
                affected.insert(a);
                affected.insert(b);
                if let Some(target_rune) = cb.rune {
                    self.collect_all_of_rune(target_rune, &mut affected);
                }
                Some(affected)
            }
            (_, Booster::BifrostShard) => {
                affected.insert(a);
                affected.insert(b);
                if let Some(target_rune) = ca.rune {
                    self.collect_all_of_rune(target_rune, &mut affected);
                }
                Some(affected)
            }
            (Booster::Bomb, Booster::Bomb) => {
                affected.extend(self.area_positions(b, 2));
                Some(affected)
            }
            (
                Booster::LineHorizontal | Booster::LineVertical,
                Booster::LineHorizontal | Booster::LineVertical,
            ) => {
                affected.extend(self.row_positions(b.row));
                affected.extend(self.col_positions(b.col));
                Some(affected)
            }
            (Booster::Bomb, Booster::LineHorizontal | Booster::LineVertical)
            | (Booster::LineHorizontal | Booster::LineVertical, Booster::Bomb) => {
                for dr in -1i32..=1 {
                    let r = b.row as i32 + dr;
                    if r >= 0 && (r as usize) < self.rows {
                        affected.extend(self.row_positions(r as usize));
                    }
                }
                for dc in -1i32..=1 {
                    let c = b.col as i32 + dc;
                    if c >= 0 && (c as usize) < self.cols {
                        affected.extend(self.col_positions(c as usize));
                    }
                }
                Some(affected)
            }
            _ => None,
        }
    }

    fn collect_all_of_rune(&self, rune: RuneType, out: &mut HashSet<CellPos>) {
        for row in 0..self.rows {
            for col in 0..self.cols {
                let p = CellPos::new(col, row);
                let cell = self.cells[self.idx(p)];
                if !cell.blocked && cell.rune == Some(rune) {
                    out.insert(p);
                }
            }
        }
    }

    fn row_positions(&self, row: usize) -> Vec<CellPos> {
        (0..self.cols)
            .map(|col| CellPos::new(col, row))
            .filter(|&p| !self.cells[self.idx(p)].blocked)
            .collect()
    }

    fn col_positions(&self, col: usize) -> Vec<CellPos> {
        (0..self.rows)
            .map(|row| CellPos::new(col, row))
            .filter(|&p| !self.cells[self.idx(p)].blocked)
            .collect()
    }

    fn area_positions(&self, center: CellPos, radius: i32) -> HashSet<CellPos> {
        let mut set = HashSet::new();
        for dr in -radius..=radius {
            for dc in -radius..=radius {
                let nc = center.col as i32 + dc;
                let nr = center.row as i32 + dr;
                if nc >= 0 && nr >= 0 && (nc as usize) < self.cols && (nr as usize) < self.rows {
                    let p = CellPos::new(nc as usize, nr as usize);
                    if !self.cells[self.idx(p)].blocked {
                        set.insert(p);
                    }
                }
            }
        }
        set
    }

    fn orthogonal_neighbors(&self, pos: CellPos) -> Vec<CellPos> {
        let mut nbs = Vec::with_capacity(4);
        if pos.col > 0 {
            nbs.push(CellPos::new(pos.col - 1, pos.row));
        }
        if pos.col + 1 < self.cols {
            nbs.push(CellPos::new(pos.col + 1, pos.row));
        }
        if pos.row > 0 {
            nbs.push(CellPos::new(pos.col, pos.row - 1));
        }
        if pos.row + 1 < self.rows {
            nbs.push(CellPos::new(pos.col, pos.row + 1));
        }
        nbs
    }

    fn find_match_groups(&self) -> Vec<MatchGroup> {
        let mut h_runs: Vec<Vec<CellPos>> = Vec::new();
        let mut v_runs: Vec<Vec<CellPos>> = Vec::new();

        for row in 0..self.rows {
            let mut col = 0;
            while col < self.cols {
                let p = CellPos::new(col, row);
                let cell = self.cells[self.idx(p)];
                let Some(rune) = cell.rune else {
                    col += 1;
                    continue;
                };
                if cell.blocked || cell.crate_layers > 0 {
                    col += 1;
                    continue;
                }
                let start = col;
                while col < self.cols {
                    let cp = CellPos::new(col, row);
                    let c = self.cells[self.idx(cp)];
                    if !c.blocked && c.crate_layers == 0 && c.rune == Some(rune) {
                        col += 1;
                    } else {
                        break;
                    }
                }
                if col - start >= MIN_MATCH_LEN {
                    h_runs.push((start..col).map(|c| CellPos::new(c, row)).collect());
                }
            }
        }

        for col in 0..self.cols {
            let mut row = 0;
            while row < self.rows {
                let p = CellPos::new(col, row);
                let cell = self.cells[self.idx(p)];
                let Some(rune) = cell.rune else {
                    row += 1;
                    continue;
                };
                if cell.blocked || cell.crate_layers > 0 {
                    row += 1;
                    continue;
                }
                let start = row;
                while row < self.rows {
                    let cp = CellPos::new(col, row);
                    let c = self.cells[self.idx(cp)];
                    if !c.blocked && c.crate_layers == 0 && c.rune == Some(rune) {
                        row += 1;
                    } else {
                        break;
                    }
                }
                if row - start >= MIN_MATCH_LEN {
                    v_runs.push((start..row).map(|r| CellPos::new(col, r)).collect());
                }
            }
        }

        let mut groups: Vec<MatchGroup> = Vec::new();
        for run in h_runs {
            let rune = self.cells[self.idx(run[0])].rune.unwrap();
            let set: HashSet<CellPos> = run.iter().copied().collect();
            groups.push(MatchGroup {
                rune,
                cells: set,
                max_h_len: run.len(),
                max_v_len: 1,
            });
        }
        for run in v_runs {
            let rune = self.cells[self.idx(run[0])].rune.unwrap();
            let run_set: HashSet<CellPos> = run.iter().copied().collect();
            let mut merged_into: Option<usize> = None;
            for (i, g) in groups.iter_mut().enumerate() {
                if g.rune == rune && !g.cells.is_disjoint(&run_set) {
                    g.cells.extend(&run_set);
                    g.max_v_len = g.max_v_len.max(run.len());
                    merged_into = Some(i);
                    break;
                }
            }
            if merged_into.is_none() {
                groups.push(MatchGroup {
                    rune,
                    cells: run_set,
                    max_h_len: 1,
                    max_v_len: run.len(),
                });
            }
        }

        groups
    }

    fn clear_cell_set(
        &mut self,
        initial: &HashSet<CellPos>,
        combo_depth: u32,
        balance: &BalanceConfig,
        outcome: &mut MatchStepOutcome,
        is_direct_blast: bool,
    ) {
        let mut queue: VecDeque<CellPos> = initial.iter().copied().collect();
        let mut visited: HashSet<CellPos> = HashSet::new();
        let mut adjacent_crates_hit: HashSet<CellPos> = HashSet::new();
        let mult = balance.combo_multiplier(combo_depth);

        while let Some(pos) = queue.pop_front() {
            if !visited.insert(pos) || !self.in_bounds(pos) {
                continue;
            }
            let idx = self.idx(pos);
            if self.cells[idx].blocked {
                continue;
            }

            // Frost crates take 1 hit from direct blasts or boosters.
            if self.cells[idx].crate_layers > 0 {
                self.cells[idx].crate_layers -= 1;
                if self.cells[idx].crate_layers == 0 {
                    outcome.crates_broken += 1;
                }
                outcome.score_gained += (balance.crate_break_bonus as f32 * mult).round() as u32;
                outcome.cleared_positions.push(pos);
                continue;
            }

            // Buried Relics are indestructible by matches or blasts.
            if self.cells[idx].relic {
                continue;
            }

            // Collect orthogonally adjacent frost crates damaged by this cleared cell.
            for nb in self.orthogonal_neighbors(pos) {
                let nb_idx = self.idx(nb);
                if !self.cells[nb_idx].blocked && self.cells[nb_idx].crate_layers > 0 {
                    adjacent_crates_hit.insert(nb);
                }
            }

            // Trigger any existing booster on this cell.
            let booster = self.cells[idx].booster;
            if booster != Booster::None {
                self.cells[idx].booster = Booster::None;
                outcome.score_gained +=
                    (balance.booster_activation_bonus as f32 * mult).round() as u32;
                match booster {
                    Booster::LineHorizontal => {
                        for p in self.row_positions(pos.row) {
                            queue.push_back(p);
                        }
                    }
                    Booster::LineVertical => {
                        for p in self.col_positions(pos.col) {
                            queue.push_back(p);
                        }
                    }
                    Booster::Bomb => {
                        for p in self.area_positions(pos, 1) {
                            queue.push_back(p);
                        }
                    }
                    Booster::BifrostShard => {
                        if let Some(r) = self.cells[idx].rune {
                            let mut same = HashSet::new();
                            self.collect_all_of_rune(r, &mut same);
                            for p in same {
                                queue.push_back(p);
                            }
                        }
                    }
                    Booster::None => {}
                }
            }

            if self.cells[idx].chained {
                self.cells[idx].chained = false;
                outcome.chains_broken += 1;
                outcome.score_gained += (balance.chain_break_bonus as f32 * mult).round() as u32;
                outcome.cleared_positions.push(pos);
                if !is_direct_blast {
                    continue;
                }
            }

            if self.cells[idx].ice_layers > 0 {
                self.cells[idx].ice_layers -= 1;
                if self.cells[idx].ice_layers == 0 {
                    outcome.ice_broken += 1;
                }
                outcome.score_gained += (balance.ice_break_bonus as f32 * mult).round() as u32;
            }

            if let Some(rune) = self.cells[idx].rune.take() {
                *outcome.cleared_runes.entry(rune).or_insert(0) += 1;
                outcome.score_gained += (balance.points_per_tile as f32 * mult).round() as u32;
                outcome.cleared_positions.push(pos);
            }
        }

        // Apply 1 damage to each orthogonally adjacent frost crate not already directly hit.
        for crate_pos in adjacent_crates_hit {
            if visited.contains(&crate_pos) {
                continue;
            }
            let idx = self.idx(crate_pos);
            if self.cells[idx].crate_layers > 0 {
                self.cells[idx].crate_layers -= 1;
                if self.cells[idx].crate_layers == 0 {
                    outcome.crates_broken += 1;
                }
                outcome.score_gained += (balance.crate_break_bonus as f32 * mult).round() as u32;
                outcome.cleared_positions.push(crate_pos);
            }
        }
    }

    /// Checks whether at least one legal move exists on the current board.
    pub fn has_valid_moves(&self) -> bool {
        self.find_hint_move().is_some()
    }

    /// Returns the first valid adjacent swap `(a, b)` for auto-hinting, if one exists.
    pub fn find_hint_move(&self) -> Option<(CellPos, CellPos)> {
        let mut clone = self.clone();
        for row in 0..self.rows {
            for col in 0..self.cols {
                let a = CellPos::new(col, row);
                if !clone.cells[clone.idx(a)].can_be_swapped() {
                    continue;
                }
                let neighbors = [
                    (col + 1 < self.cols).then(|| CellPos::new(col + 1, row)),
                    (row + 1 < self.rows).then(|| CellPos::new(col, row + 1)),
                ];
                for nb in neighbors.into_iter().flatten() {
                    if !clone.cells[clone.idx(nb)].can_be_swapped() {
                        continue;
                    }
                    clone.swap_contents(a, nb);
                    let valid = clone.check_special_swap_combo(a, nb).is_some()
                        || !clone.find_match_groups().is_empty();
                    clone.swap_contents(a, nb);
                    if valid {
                        return Some((a, nb));
                    }
                }
            }
        }
        None
    }

    /// Shuffles all unchained free runes without creating instant matches, ensuring a valid move.
    pub fn shuffle_free_runes<R: Rng>(&mut self, rng: &mut R) {
        let free_positions: Vec<CellPos> = (0..self.rows)
            .flat_map(|row| (0..self.cols).map(move |col| CellPos::new(col, row)))
            .filter(|&p| {
                let c = self.cells[self.idx(p)];
                !c.blocked
                    && !c.chained
                    && c.crate_layers == 0
                    && !c.relic
                    && c.rune.is_some()
                    && c.booster == Booster::None
            })
            .collect();

        if free_positions.len() < 2 {
            return;
        }

        let mut runes: Vec<RuneType> = free_positions
            .iter()
            .filter_map(|&p| self.cells[self.idx(p)].rune)
            .collect();

        for _ in 0..MAX_SHUFFLE_ATTEMPTS {
            runes.shuffle(rng);
            for (&pos, &rune) in free_positions.iter().zip(runes.iter()) {
                let idx = self.idx(pos);
                self.cells[idx].rune = Some(rune);
            }
            if self.find_match_groups().is_empty() && self.has_valid_moves() {
                return;
            }
        }
    }
}

fn merge_outcomes(total: &mut MatchStepOutcome, step: &MatchStepOutcome) {
    total.valid |= step.valid;
    total.score_gained += step.score_gained;
    total.max_combo = total.max_combo.max(step.max_combo);
    total.ice_broken += step.ice_broken;
    total.chains_broken += step.chains_broken;
    total.crates_broken += step.crates_broken;
    total.relics_dropped += step.relics_dropped;
    for (&rune, &count) in &step.cleared_runes {
        *total.cleared_runes.entry(rune).or_insert(0) += count;
    }
    total.cleared_positions.extend(&step.cleared_positions);
}

struct MatchGroup {
    rune: RuneType,
    cells: HashSet<CellPos>,
    max_h_len: usize,
    max_v_len: usize,
}

impl MatchGroup {
    fn spawned_booster(&self) -> Booster {
        if self.max_h_len >= BIFROST_MATCH_LEN || self.max_v_len >= BIFROST_MATCH_LEN {
            Booster::BifrostShard
        } else if self.max_h_len >= MIN_MATCH_LEN && self.max_v_len >= MIN_MATCH_LEN {
            Booster::Bomb
        } else if self.max_h_len == LINE_BOOSTER_MATCH_LEN {
            Booster::LineVertical
        } else if self.max_v_len == LINE_BOOSTER_MATCH_LEN {
            Booster::LineHorizontal
        } else {
            Booster::None
        }
    }
}
