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
use crate::boss::BossRiposteKind;
use crate::level::LevelDef;
use crate::patron::PatronGod;
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
/// Maximum attempts when regenerating free runes from the rune pool as a last resort.
const MAX_REGENERATION_ATTEMPTS: usize = 128;

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
    #[serde(default)]
    pub fog: bool,
    #[serde(default)]
    pub curse_turns: u8,
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
            fog: false,
            curse_turns: 0,
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
            fog: false,
            curse_turns: 0,
        }
    }

    /// Returns true when the content of this cell can be swapped by the player.
    pub fn can_be_swapped(&self) -> bool {
        !self.blocked
            && !self.chained
            && !self.fog
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
    pub fog_cleared: u32,
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
    pub converted_boosters: Vec<(CellPos, Booster)>,
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

        for &(col, row) in &level.fog_cells {
            let pos = CellPos::new(col, row);
            if let Some(cell) = board.get_mut(pos) {
                if !cell.blocked {
                    cell.fog = true;
                }
            }
        }

        for &(col, row, turns) in &level.cursed_cells {
            let pos = CellPos::new(col, row);
            if let Some(cell) = board.get_mut(pos) {
                if !cell.blocked
                    && !cell.fog
                    && cell.crate_layers == 0
                    && !cell.relic
                    && cell.rune.is_some()
                    && turns > 0
                {
                    cell.curse_turns = turns;
                }
            }
        }

        if !board.has_valid_moves() {
            board.ensure_playable(rng);
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
        if let Some((combo_cells, converted_boosters)) = self.check_special_swap_combo(a, b) {
            return Some(PreparedClear {
                cells: combo_cells.into_iter().collect(),
                spawned_boosters: Vec::new(),
                converted_boosters,
                is_direct_blast: true,
            });
        }
        let groups = self.find_match_groups();
        if !Self::groups_involve_swap(&groups, a, b) {
            self.swap_contents(a, b);
            return None;
        }
        Some(self.groups_to_prepared_clear(&groups, Some((a, b))))
    }

    /// Returns true when at least one match group contains one of the two swapped cells.
    fn groups_involve_swap(groups: &[MatchGroup], a: CellPos, b: CellPos) -> bool {
        groups
            .iter()
            .any(|g| g.cells.contains(&a) || g.cells.contains(&b))
    }

    /// Prepares the next cascade clear if any matches exist after gravity.
    pub fn prepare_cascade_step(&self) -> Option<PreparedClear> {
        let groups = self.find_match_groups();
        if groups.is_empty() {
            return None;
        }
        Some(self.groups_to_prepared_clear(&groups, None))
    }

    /// Returns all playable cells currently holding an active charged rune (booster).
    pub fn find_remaining_boosters(&self) -> Vec<(CellPos, Booster)> {
        let mut out = Vec::new();
        for row in 0..self.rows {
            for col in 0..self.cols {
                let pos = CellPos::new(col, row);
                let cell = self.cells[self.idx(pos)];
                if !cell.blocked && cell.booster != Booster::None {
                    out.push((pos, cell.booster));
                }
            }
        }
        out
    }

    /// Prepares a direct detonation for a single charged rune (used during Victory bonus sweeps).
    pub fn prepare_booster_detonation(&self, target: CellPos) -> Option<PreparedClear> {
        if !self.in_bounds(target) {
            return None;
        }
        let cell = self.cells[self.idx(target)];
        if cell.blocked || cell.booster == Booster::None {
            return None;
        }
        Some(PreparedClear {
            cells: vec![target],
            spawned_boosters: Vec::new(),
            converted_boosters: Vec::new(),
            is_direct_blast: true,
        })
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
                converted_boosters: Vec::new(),
                is_direct_blast: true,
            }),
            ActiveTool::SurtrEmber => Some(PreparedClear {
                cells: self.area_positions(target, 1).into_iter().collect(),
                spawned_boosters: Vec::new(),
                converted_boosters: Vec::new(),
                is_direct_blast: true,
            }),
            ActiveTool::ThorBolt => Some(PreparedClear {
                cells: self.row_positions(target.row),
                spawned_boosters: Vec::new(),
                converted_boosters: Vec::new(),
                is_direct_blast: true,
            }),
            ActiveTool::BifrostPrism => {
                let cell = self.cells[self.idx(target)];
                if cell.fog {
                    return None;
                }
                let target_rune = cell.rune?;
                let mut set = HashSet::new();
                self.collect_all_of_rune(target_rune, &mut set);
                Some(PreparedClear {
                    cells: set.into_iter().collect(),
                    spawned_boosters: Vec::new(),
                    converted_boosters: Vec::new(),
                    is_direct_blast: true,
                })
            }
            ActiveTool::DrakkarCharge => {
                let mut set: HashSet<CellPos> =
                    self.row_positions(target.row).into_iter().collect();
                set.extend(self.col_positions(target.col));
                Some(PreparedClear {
                    cells: set.into_iter().collect(),
                    spawned_boosters: Vec::new(),
                    converted_boosters: Vec::new(),
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
                    .unwrap_or_else(|| {
                        group
                            .cells
                            .iter()
                            .copied()
                            .min_by_key(|p| (p.row, p.col))
                            .unwrap()
                    });
                spawned_boosters.push((spawn_pos, group.rune, booster));
            }
        }
        PreparedClear {
            cells: to_clear.into_iter().collect(),
            spawned_boosters,
            converted_boosters: Vec::new(),
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
        for &(pos, booster) in &prepared.converted_boosters {
            if let Some(cell) = self.get_mut(pos) {
                if !cell.blocked && cell.crate_layers == 0 && !cell.relic {
                    cell.booster = booster;
                }
            }
        }
        let set: HashSet<CellPos> = prepared.cells.iter().copied().collect();
        self.clear_cell_set(
            &set,
            combo_depth,
            balance,
            &mut outcome,
            prepared.is_direct_blast,
        );

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
        self.apply_column_gravity_only(&mut moves);

        // Iteratively transfer movable pieces through Bifrost portals when destination is empty.
        let portals_snapshot = self.portals.clone();
        if !portals_snapshot.is_empty() {
            for _ in 0..(self.rows * self.cols).min(32) {
                let mut moved_any = false;
                for &(src, dst) in &portals_snapshot {
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
                            self.cells[di].curse_turns = self.cells[si].curse_turns;
                            self.cells[si].curse_turns = 0;
                            moves.push(FallMove {
                                col: dst.col,
                                from_row: (dst.row as i32) - 1,
                                to_row: dst.row,
                            });
                            moved_any = true;
                        }
                    }
                }
                if !moved_any {
                    break;
                }
                self.apply_column_gravity_only(&mut moves);
            }
        }

        // Spawn fresh random runes into any remaining empty playable cells from above.
        for col in 0..self.cols {
            let mut spawn_offset = 1i32;
            let mut write_row = self.rows as i32 - 1;
            while write_row >= 0 {
                let wpos = CellPos::new(col, write_row as usize);
                let wi = self.idx(wpos);
                let wcell = self.cells[wi];
                if !wcell.blocked
                    && !wcell.chained
                    && wcell.crate_layers == 0
                    && !wcell.has_movable_piece()
                {
                    self.cells[wi].rune = Some(self.random_rune(rng));
                    self.cells[wi].booster = Booster::None;
                    self.cells[wi].relic = false;
                    self.cells[wi].curse_turns = 0;
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

    fn apply_column_gravity_only(&mut self, moves: &mut Vec<FallMove>) {
        for col in 0..self.cols {
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
                        self.cells[wi].curse_turns = self.cells[ri].curse_turns;
                        self.cells[ri].curse_turns = 0;
                        moves.push(FallMove {
                            col,
                            from_row: read_row,
                            to_row: write_row as usize,
                        });
                        break;
                    }
                    read_row -= 1;
                }
                write_row -= 1;
            }
        }
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

        if !self.has_valid_moves() && self.ensure_playable(rng) {
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
            let recovered = self.ensure_playable(rng);
            return MatchStepOutcome {
                valid: true,
                shuffled: recovered,
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

        if !self.has_valid_moves() && self.ensure_playable(rng) {
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
        let curse_a = self.cells[ia].curse_turns;
        self.cells[ia].rune = self.cells[ib].rune;
        self.cells[ia].booster = self.cells[ib].booster;
        self.cells[ia].relic = self.cells[ib].relic;
        self.cells[ia].curse_turns = self.cells[ib].curse_turns;
        self.cells[ib].rune = rune_a;
        self.cells[ib].booster = booster_a;
        self.cells[ib].relic = relic_a;
        self.cells[ib].curse_turns = curse_a;
    }

    fn check_special_swap_combo(&self, a: CellPos, b: CellPos) -> Option<SpecialSwapCombo> {
        let ca = self.cells[self.idx(a)];
        let cb = self.cells[self.idx(b)];
        let mut affected = HashSet::new();
        let mut converted = Vec::new();

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
                Some((affected, converted))
            }
            (Booster::BifrostShard, partner_booster) => {
                let target_rune = cb.rune?;
                affected.insert(a);
                affected.insert(b);
                self.collect_all_of_rune(target_rune, &mut affected);
                if matches!(
                    partner_booster,
                    Booster::LineHorizontal | Booster::LineVertical | Booster::Bomb
                ) {
                    let mut targets: Vec<CellPos> =
                        affected.iter().copied().filter(|&p| p != a).collect();
                    targets.sort_by_key(|p| (p.row, p.col));
                    for (idx, p) in targets.into_iter().enumerate() {
                        let btype = match partner_booster {
                            Booster::Bomb => Booster::Bomb,
                            _ if idx % 2 == 0 => Booster::LineHorizontal,
                            _ => Booster::LineVertical,
                        };
                        converted.push((p, btype));
                    }
                }
                Some((affected, converted))
            }
            (partner_booster, Booster::BifrostShard) => {
                let target_rune = ca.rune?;
                affected.insert(a);
                affected.insert(b);
                self.collect_all_of_rune(target_rune, &mut affected);
                if matches!(
                    partner_booster,
                    Booster::LineHorizontal | Booster::LineVertical | Booster::Bomb
                ) {
                    let mut targets: Vec<CellPos> =
                        affected.iter().copied().filter(|&p| p != b).collect();
                    targets.sort_by_key(|p| (p.row, p.col));
                    for (idx, p) in targets.into_iter().enumerate() {
                        let btype = match partner_booster {
                            Booster::Bomb => Booster::Bomb,
                            _ if idx % 2 == 0 => Booster::LineHorizontal,
                            _ => Booster::LineVertical,
                        };
                        converted.push((p, btype));
                    }
                }
                Some((affected, converted))
            }
            (Booster::Bomb, Booster::Bomb) => {
                affected.extend(self.area_positions(b, 2));
                Some((affected, converted))
            }
            (
                Booster::LineHorizontal | Booster::LineVertical,
                Booster::LineHorizontal | Booster::LineVertical,
            ) => {
                affected.extend(self.row_positions(b.row));
                affected.extend(self.col_positions(b.col));
                Some((affected, converted))
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
                Some((affected, converted))
            }
            _ => None,
        }
    }

    fn most_frequent_rune(&self) -> Option<RuneType> {
        let mut counts = [0u32; 6];
        for cell in &self.cells {
            if !cell.blocked && !cell.fog && cell.crate_layers == 0 {
                if let Some(r) = cell.rune {
                    if let Some(idx) = RuneType::ALL.iter().position(|&x| x == r) {
                        counts[idx] += 1;
                    }
                }
            }
        }
        counts
            .iter()
            .enumerate()
            .filter(|(_, &c)| c > 0)
            .max_by_key(|(_, &c)| c)
            .map(|(idx, _)| RuneType::ALL[idx])
    }

    fn collect_all_of_rune(&self, rune: RuneType, out: &mut HashSet<CellPos>) {
        for row in 0..self.rows {
            for col in 0..self.cols {
                let p = CellPos::new(col, row);
                let cell = self.cells[self.idx(p)];
                if !cell.blocked && !cell.fog && cell.rune == Some(rune) {
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
                if cell.blocked || cell.fog || cell.crate_layers > 0 {
                    col += 1;
                    continue;
                }
                let start = col;
                while col < self.cols {
                    let cp = CellPos::new(col, row);
                    let c = self.cells[self.idx(cp)];
                    if !c.blocked && !c.fog && c.crate_layers == 0 && c.rune == Some(rune) {
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
                if cell.blocked || cell.fog || cell.crate_layers > 0 {
                    row += 1;
                    continue;
                }
                let start = row;
                while row < self.rows {
                    let cp = CellPos::new(col, row);
                    let c = self.cells[self.idx(cp)];
                    if !c.blocked && !c.fog && c.crate_layers == 0 && c.rune == Some(rune) {
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
            let matching_indices: Vec<usize> = groups
                .iter()
                .enumerate()
                .filter(|(_, g)| g.rune == rune && !g.cells.is_disjoint(&run_set))
                .map(|(i, _)| i)
                .collect();
            if let Some(&first_idx) = matching_indices.first() {
                groups[first_idx].cells.extend(&run_set);
                groups[first_idx].max_v_len = groups[first_idx].max_v_len.max(run.len());
                for &extra_idx in matching_indices[1..].iter().rev() {
                    let extra = groups.remove(extra_idx);
                    groups[first_idx].cells.extend(extra.cells);
                    groups[first_idx].max_h_len = groups[first_idx].max_h_len.max(extra.max_h_len);
                    groups[first_idx].max_v_len = groups[first_idx].max_v_len.max(extra.max_v_len);
                }
            } else {
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
        let mut adjacent_fog_hit: HashSet<CellPos> = HashSet::new();
        let mult = balance.combo_multiplier(combo_depth);

        while let Some(pos) = queue.pop_front() {
            if !visited.insert(pos) || !self.in_bounds(pos) {
                continue;
            }
            let idx = self.idx(pos);
            if self.cells[idx].blocked {
                continue;
            }

            // Dispel Niflheim Fog when directly hit by a blast or booster.
            if self.cells[idx].fog {
                self.cells[idx].fog = false;
                outcome.fog_cleared += 1;
                outcome.score_gained += (balance.ice_break_bonus as f32 * mult).round() as u32;
                outcome.cleared_positions.push(pos);
                if !is_direct_blast {
                    continue;
                }
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

            // Collect orthogonally adjacent frost crates and fog tiles revealed by this cleared cell.
            for nb in self.orthogonal_neighbors(pos) {
                let nb_idx = self.idx(nb);
                if !self.cells[nb_idx].blocked {
                    if self.cells[nb_idx].crate_layers > 0 {
                        adjacent_crates_hit.insert(nb);
                    }
                    if self.cells[nb_idx].fog {
                        adjacent_fog_hit.insert(nb);
                    }
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
                        let target_rune =
                            self.cells[idx].rune.or_else(|| self.most_frequent_rune());
                        if let Some(r) = target_rune {
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
                self.cells[idx].curse_turns = 0;
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

        // Dispel Niflheim Fog on orthogonally adjacent tiles.
        for fog_pos in adjacent_fog_hit {
            let idx = self.idx(fog_pos);
            if self.cells[idx].fog {
                self.cells[idx].fog = false;
                outcome.fog_cleared += 1;
                outcome.score_gained += (balance.ice_break_bonus as f32 * mult).round() as u32;
                outcome.cleared_positions.push(fog_pos);
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
                        || Self::groups_involve_swap(&clone.find_match_groups(), a, nb);
                    clone.swap_contents(a, nb);
                    if valid {
                        return Some((a, nb));
                    }
                }
            }
        }
        None
    }

    /// Recovers a deadlocked board: shuffles the free runes first, then falls back to
    /// regenerating them from the rune pool. Returns true when the board was changed to a
    /// state with no pending match and at least one valid move. On failure the original
    /// rune layout is restored and false is returned.
    pub fn ensure_playable<R: Rng>(&mut self, rng: &mut R) -> bool {
        if self.shuffle_free_runes(rng) {
            return true;
        }
        self.regenerate_free_runes(rng)
    }

    /// Positions of plain runes that may be re-rolled (no obstacle, relic or booster).
    fn free_rune_positions(&self) -> Vec<CellPos> {
        (0..self.rows)
            .flat_map(|row| (0..self.cols).map(move |col| CellPos::new(col, row)))
            .filter(|&p| {
                let c = self.cells[self.idx(p)];
                !c.blocked
                    && !c.chained
                    && !c.fog
                    && c.curse_turns == 0
                    && c.crate_layers == 0
                    && !c.relic
                    && c.rune.is_some()
                    && c.booster == Booster::None
            })
            .collect()
    }

    /// Re-rolls every free rune from the level's rune pool using the no-initial-match
    /// spawning logic, keeping the first layout that verifies. Restores the original runes
    /// and returns false when no attempt succeeds.
    fn regenerate_free_runes<R: Rng>(&mut self, rng: &mut R) -> bool {
        let free_positions = self.free_rune_positions();
        if free_positions.is_empty() {
            return false;
        }
        let original: Vec<Option<RuneType>> = free_positions
            .iter()
            .map(|&p| self.cells[self.idx(p)].rune)
            .collect();
        for _ in 0..MAX_REGENERATION_ATTEMPTS {
            for &pos in &free_positions {
                let rune = self.pick_non_matching_rune(pos.col, pos.row, rng);
                let idx = self.idx(pos);
                self.cells[idx].rune = Some(rune);
            }
            if self.is_settled_and_playable() {
                return true;
            }
        }
        self.restore_free_runes(&free_positions, &original);
        false
    }

    /// Shuffles all unchained free runes without creating instant matches, ensuring a valid move.
    /// Returns true when the resulting board has no pending match and at least one valid move.
    /// On failure the original rune layout is restored and false is returned.
    pub fn shuffle_free_runes<R: Rng>(&mut self, rng: &mut R) -> bool {
        let free_positions = self.free_rune_positions();

        if free_positions.len() < 2 {
            return false;
        }

        let original: Vec<Option<RuneType>> = free_positions
            .iter()
            .map(|&p| self.cells[self.idx(p)].rune)
            .collect();
        let mut runes: Vec<RuneType> = original.iter().filter_map(|r| *r).collect();

        for _ in 0..MAX_SHUFFLE_ATTEMPTS {
            runes.shuffle(rng);
            for (&pos, &rune) in free_positions.iter().zip(runes.iter()) {
                let idx = self.idx(pos);
                self.cells[idx].rune = Some(rune);
            }
            if self.is_settled_and_playable() {
                return true;
            }
        }

        if self.apply_deterministic_shuffle(&free_positions) {
            return true;
        }

        self.restore_free_runes(&free_positions, &original);
        false
    }

    fn restore_free_runes(&mut self, positions: &[CellPos], runes: &[Option<RuneType>]) {
        for (&pos, &rune) in positions.iter().zip(runes.iter()) {
            let idx = self.idx(pos);
            self.cells[idx].rune = rune;
        }
    }

    /// True when no match is pending and at least one legal move exists.
    fn is_settled_and_playable(&self) -> bool {
        self.find_match_groups().is_empty() && self.has_valid_moves()
    }

    /// Deterministic fallback: paints a match-free base layout over the free cells, then plants
    /// a pair plus a mover (horizontal or vertical) and keeps the first layout that verifies.
    fn apply_deterministic_shuffle(&mut self, free_positions: &[CellPos]) -> bool {
        let free_set: HashSet<CellPos> = free_positions.iter().copied().collect();
        let pool = self.rune_pool_len.clamp(1, RuneType::ALL.len());

        // Base layout: greedy match-free fill, rotating the starting rune per cell.
        for (n, &pos) in free_positions.iter().enumerate() {
            let mut chosen = RuneType::ALL[n % pool];
            for k in 0..pool {
                let candidate = RuneType::ALL[(n + k) % pool];
                if !self.would_complete_line(pos, candidate) {
                    chosen = candidate;
                    break;
                }
            }
            let idx = self.idx(pos);
            self.cells[idx].rune = Some(chosen);
        }
        if self.is_settled_and_playable() {
            return true;
        }

        let base: Vec<Option<RuneType>> = free_positions
            .iter()
            .map(|&p| self.cells[self.idx(p)].rune)
            .collect();

        // Line directions as (dc, dr). The mover approaches the third cell from either side
        // (perpendicular) or from beyond it (in line).
        let directions: [(isize, isize); 2] = [(1, 0), (0, 1)];
        for &target_rune in &RuneType::ALL[..pool] {
            for &start in free_positions {
                for &(dc, dr) in &directions {
                    let cell_at = |step: isize, side: isize| -> Option<CellPos> {
                        let col = start.col as isize + dc * step + dr * side;
                        let row = start.row as isize + dr * step + dc * side;
                        if col < 0 || row < 0 {
                            return None;
                        }
                        let p = CellPos::new(col as usize, row as usize);
                        (self.in_bounds(p) && free_set.contains(&p)).then_some(p)
                    };
                    let (Some(p0), Some(p1), Some(_p2)) =
                        (cell_at(0, 0), cell_at(1, 0), cell_at(2, 0))
                    else {
                        continue;
                    };
                    for mover in [cell_at(2, 1), cell_at(2, -1), cell_at(3, 0)]
                        .into_iter()
                        .flatten()
                    {
                        for p in [p0, p1, mover] {
                            let idx = self.idx(p);
                            self.cells[idx].rune = Some(target_rune);
                        }
                        if self.is_settled_and_playable() {
                            return true;
                        }
                        self.restore_free_runes(free_positions, &base);
                    }
                }
            }
        }
        false
    }

    /// True when placing `rune` at `pos` would extend a run of identical runes to three
    /// with already painted neighbors to the left or above.
    fn would_complete_line(&self, pos: CellPos, rune: RuneType) -> bool {
        let same = |col: isize, row: isize| -> bool {
            if col < 0 || row < 0 {
                return false;
            }
            let p = CellPos::new(col as usize, row as usize);
            self.in_bounds(p) && {
                let c = self.cells[self.idx(p)];
                !c.blocked && c.rune == Some(rune)
            }
        };
        let (c, r) = (pos.col as isize, pos.row as isize);
        (same(c - 1, r) && same(c - 2, r)) || (same(c, r - 1) && same(c, r - 2))
    }

    /// Places a random Line or Bomb booster on an unblocked free rune cell (Odin's Eye amulet).
    pub fn spawn_random_starting_booster<R: Rng>(&mut self, rng: &mut R) -> Option<CellPos> {
        let candidates: Vec<CellPos> = self.free_rune_positions();
        let &pos = candidates.choose(rng)?;
        let booster = match rng.gen_range(0..3usize) {
            0 => Booster::LineHorizontal,
            1 => Booster::LineVertical,
            _ => Booster::Bomb,
        };
        let idx = self.idx(pos);
        self.cells[idx].booster = booster;
        Some(pos)
    }

    /// Spreads Niflheim Frost to one random non-icy playable cell orthogonally adjacent to existing ice.
    pub fn spread_frost<R: Rng>(&mut self, rng: &mut R) -> Option<CellPos> {
        let mut candidates = HashSet::new();
        for row in 0..self.rows {
            for col in 0..self.cols {
                let pos = CellPos::new(col, row);
                if self.cells[self.idx(pos)].ice_layers > 0 {
                    for nb in self.orthogonal_neighbors(pos) {
                        let c = self.cells[self.idx(nb)];
                        if !c.blocked && c.ice_layers == 0 && c.crate_layers == 0 && !c.relic {
                            candidates.insert(nb);
                        }
                    }
                }
            }
        }
        let mut list: Vec<CellPos> = candidates.into_iter().collect();
        list.sort_by_key(|p| (p.row, p.col));
        let &chosen = list.choose(rng)?;
        let idx = self.idx(chosen);
        self.cells[idx].ice_layers = 1;
        Some(chosen)
    }

    /// Returns all adjacent swappable cell pairs where both cells hold a booster (super-combo ready).
    pub fn find_booster_combo_pairs(&self) -> Vec<(CellPos, CellPos)> {
        let mut pairs = Vec::new();
        for row in 0..self.rows {
            for col in 0..self.cols {
                let a = CellPos::new(col, row);
                let ca = self.cells[self.idx(a)];
                if !ca.can_be_swapped() || ca.booster == Booster::None {
                    continue;
                }
                for nb in [
                    (col + 1 < self.cols).then(|| CellPos::new(col + 1, row)),
                    (row + 1 < self.rows).then(|| CellPos::new(col, row + 1)),
                ]
                .into_iter()
                .flatten()
                {
                    let cb = self.cells[self.idx(nb)];
                    if cb.can_be_swapped() && cb.booster != Booster::None {
                        pairs.push((a, nb));
                    }
                }
            }
        }
        pairs
    }

    /// Returns the cells affected when hovering a target cell with an armed active tool.
    pub fn tool_preview_cells(&self, tool: ActiveTool, target: CellPos) -> Vec<CellPos> {
        self.prepare_tool_step(tool, target)
            .map(|p| p.cells)
            .unwrap_or_default()
    }

    /// Returns true if any cell on the board currently has an active Cursed Rune countdown.
    pub fn has_cursed_runes(&self) -> bool {
        self.cells.iter().any(|c| c.curse_turns > 0)
    }

    /// Decrements all active Cursed Rune timers; any rune reaching 0 detonates and chains/freezes orthogonal neighbors.
    pub fn tick_cursed_runes(&mut self) -> Vec<CellPos> {
        let mut detonated = Vec::new();
        for row in 0..self.rows {
            for col in 0..self.cols {
                let pos = CellPos::new(col, row);
                let idx = self.idx(pos);
                if self.cells[idx].curse_turns > 0 {
                    self.cells[idx].curse_turns -= 1;
                    if self.cells[idx].curse_turns == 0 {
                        detonated.push(pos);
                    }
                }
            }
        }
        let mut affected = Vec::new();
        for origin in detonated {
            affected.push(origin);
            for nb in self.orthogonal_neighbors(origin) {
                let idx = self.idx(nb);
                let c = &mut self.cells[idx];
                if !c.blocked && c.crate_layers == 0 && !c.relic && c.rune.is_some() {
                    if c.ice_layers == 0 {
                        c.ice_layers = 1;
                    } else if !c.chained {
                        c.chained = true;
                    }
                    affected.push(nb);
                }
            }
        }
        affected
    }

    /// Executes a Patron God's Divine Fury power, returning `(optional_clear, spawned_booster_positions)`.
    pub fn apply_patron_fury_detailed<R: Rng>(
        &mut self,
        god: PatronGod,
        rng: &mut R,
    ) -> (Option<PreparedClear>, Vec<CellPos>) {
        match god {
            PatronGod::Thor => {
                let mut priority: Vec<CellPos> = Vec::new();
                let mut fallback: Vec<CellPos> = Vec::new();
                for row in 0..self.rows {
                    for col in 0..self.cols {
                        let p = CellPos::new(col, row);
                        let c = self.cells[self.idx(p)];
                        if c.blocked || c.relic {
                            continue;
                        }
                        if c.crate_layers > 0 || c.chained || c.ice_layers > 0 || c.curse_turns > 0
                        {
                            priority.push(p);
                        } else if c.rune.is_some() {
                            fallback.push(p);
                        }
                    }
                }
                priority.shuffle(rng);
                fallback.shuffle(rng);
                priority.extend(fallback);
                priority.truncate(4);
                if priority.is_empty() {
                    (None, Vec::new())
                } else {
                    (
                        Some(PreparedClear {
                            cells: priority,
                            spawned_boosters: Vec::new(),
                            converted_boosters: Vec::new(),
                            is_direct_blast: true,
                        }),
                        Vec::new(),
                    )
                }
            }
            PatronGod::Freya => {
                let Some(target) = self.most_frequent_rune() else {
                    return (None, Vec::new());
                };
                let mut set = HashSet::new();
                self.collect_all_of_rune(target, &mut set);
                if set.is_empty() {
                    (None, Vec::new())
                } else {
                    (
                        Some(PreparedClear {
                            cells: set.into_iter().collect(),
                            spawned_boosters: Vec::new(),
                            converted_boosters: Vec::new(),
                            is_direct_blast: true,
                        }),
                        Vec::new(),
                    )
                }
            }
            PatronGod::Loki => {
                let mut spawned = Vec::new();
                if let Some(p1) = self.spawn_random_starting_booster(rng) {
                    spawned.push(p1);
                }
                if let Some(p2) = self.spawn_random_starting_booster(rng) {
                    spawned.push(p2);
                }
                (None, spawned)
            }
            PatronGod::Odin => {
                let candidates: Vec<CellPos> = self.free_rune_positions();
                let mut spawned = Vec::new();
                if let Some(&pos) = candidates.choose(rng) {
                    let idx = self.idx(pos);
                    self.cells[idx].booster = Booster::Bomb;
                    spawned.push(pos);
                }
                (None, spawned)
            }
        }
    }

    /// Executes a boss counter-attack on random vulnerable cells and returns the affected coordinates.
    pub fn apply_boss_riposte<R: Rng>(
        &mut self,
        riposte: BossRiposteKind,
        rng: &mut R,
    ) -> Vec<CellPos> {
        let mut hit = Vec::new();
        match riposte {
            BossRiposteKind::ChainRunes(count) => {
                let mut cands: Vec<CellPos> = (0..self.rows)
                    .flat_map(|r| (0..self.cols).map(move |c| CellPos::new(c, r)))
                    .filter(|&p| {
                        let c = self.cells[self.idx(p)];
                        !c.blocked
                            && !c.chained
                            && c.crate_layers == 0
                            && !c.relic
                            && c.rune.is_some()
                    })
                    .collect();
                cands.shuffle(rng);
                for pos in cands.into_iter().take(count) {
                    let idx = self.idx(pos);
                    self.cells[idx].chained = true;
                    hit.push(pos);
                }
            }
            BossRiposteKind::FreezeCells(count) => {
                let mut cands: Vec<CellPos> = (0..self.rows)
                    .flat_map(|r| (0..self.cols).map(move |c| CellPos::new(c, r)))
                    .filter(|&p| {
                        let c = self.cells[self.idx(p)];
                        !c.blocked && c.ice_layers == 0 && c.crate_layers == 0 && !c.relic
                    })
                    .collect();
                cands.shuffle(rng);
                for pos in cands.into_iter().take(count) {
                    let idx = self.idx(pos);
                    self.cells[idx].ice_layers = 1;
                    hit.push(pos);
                }
            }
            BossRiposteKind::DropCrates(count) => {
                let mut cands: Vec<CellPos> = (0..self.rows)
                    .flat_map(|r| (0..self.cols).map(move |c| CellPos::new(c, r)))
                    .filter(|&p| {
                        let c = self.cells[self.idx(p)];
                        !c.blocked
                            && !c.chained
                            && c.crate_layers == 0
                            && !c.relic
                            && c.booster == Booster::None
                            && c.rune.is_some()
                    })
                    .collect();
                cands.shuffle(rng);
                for pos in cands.into_iter().take(count) {
                    let idx = self.idx(pos);
                    self.cells[idx].crate_layers = 1;
                    self.cells[idx].rune = None;
                    hit.push(pos);
                }
            }
            BossRiposteKind::CurseRunes(count) => {
                let mut cands: Vec<CellPos> = (0..self.rows)
                    .flat_map(|r| (0..self.cols).map(move |c| CellPos::new(c, r)))
                    .filter(|&p| {
                        let c = self.cells[self.idx(p)];
                        !c.blocked
                            && !c.fog
                            && !c.chained
                            && c.crate_layers == 0
                            && !c.relic
                            && c.curse_turns == 0
                            && c.rune.is_some()
                    })
                    .collect();
                cands.shuffle(rng);
                for pos in cands.into_iter().take(count) {
                    let idx = self.idx(pos);
                    self.cells[idx].curse_turns = 5;
                    hit.push(pos);
                }
            }
        }
        hit
    }
}

fn merge_outcomes(total: &mut MatchStepOutcome, step: &MatchStepOutcome) {
    total.valid |= step.valid;
    total.score_gained += step.score_gained;
    total.max_combo = total.max_combo.max(step.max_combo);
    total.ice_broken += step.ice_broken;
    total.chains_broken += step.chains_broken;
    total.crates_broken += step.crates_broken;
    total.fog_cleared += step.fog_cleared;
    total.relics_dropped += step.relics_dropped;
    for (&rune, &count) in &step.cleared_runes {
        *total.cleared_runes.entry(rune).or_insert(0) += count;
    }
    total.cleared_positions.extend(&step.cleared_positions);
}

type SpecialSwapCombo = (HashSet<CellPos>, Vec<(CellPos, Booster)>);

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
