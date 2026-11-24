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

//! Characterization of the shipped turn resolution (clear order determinism and scenarios S1-S11).

use wyrdstone_core::{
    ActiveTool, Board, Booster, Cell, CellPos, GameDataBundle, Language, MatchStepOutcome, RuneType,
};

/// Side length of the Bifrost determinism board.
const CHAIN_BOARD_SIZE: usize = 6;
/// Number of repeated runs used to expose hash-order dependence within one process.
const CHAIN_REPEATS: usize = 50;

/// Rune census outside the blast: Ice leads once the in-blast Fire runes are gone, Fire leads before.
const OUTSIDE_RUNE_COUNTS: [(RuneType, usize); 4] = [
    (RuneType::Fire, 6),
    (RuneType::Ice, 8),
    (RuneType::Stone, 7),
    (RuneType::Gold, 6),
];

/// Builds a board where a rune-less Bifrost Shard sits inside a Surtr Ember blast next to Fire runes,
/// so the rune it targets depends on how much of the blast was already cleared when it is reached.
fn bifrost_chain_board() -> Board {
    let area_center = CellPos::new(3, 3);
    let shard = CellPos::new(2, 2);
    let fire_in_blast = [CellPos::new(3, 2), CellPos::new(4, 2), CellPos::new(2, 3)];
    let mut outside = OUTSIDE_RUNE_COUNTS
        .iter()
        .flat_map(|&(rune, count)| vec![rune; count]);
    let mut cells = Vec::new();
    for row in 0..CHAIN_BOARD_SIZE {
        for col in 0..CHAIN_BOARD_SIZE {
            let pos = CellPos::new(col, row);
            let in_blast = col.abs_diff(area_center.col) <= 1 && row.abs_diff(area_center.row) <= 1;
            let cell = if pos == shard {
                let mut c = Cell::playable(RuneType::Water);
                c.rune = None;
                c.booster = Booster::BifrostShard;
                c
            } else if fire_in_blast.contains(&pos) {
                Cell::playable(RuneType::Fire)
            } else if in_blast {
                Cell::playable(RuneType::Water)
            } else {
                Cell::playable(outside.next().expect("outside rune list covers the board"))
            };
            cells.push(cell);
        }
    }
    Board::from_cells(CHAIN_BOARD_SIZE, CHAIN_BOARD_SIZE, 6, cells)
}

fn run_bifrost_chain() -> (Board, MatchStepOutcome) {
    let data = GameDataBundle::load_bundled(Language::En).expect("bundled data");
    let mut board = bifrost_chain_board();
    let prepared = board
        .prepare_tool_step(ActiveTool::SurtrEmber, CellPos::new(3, 3))
        .expect("Surtr Ember blast is prepared");
    let step = board.commit_clear_step(&prepared, 1, &data.balance);
    (board, step)
}

#[test]
fn clear_order_is_deterministic_for_a_bifrost_chain() {
    let (first_board, first_step) = run_bifrost_chain();
    assert!(
        first_step.cleared_runes.contains_key(&RuneType::Fire),
        "the Shard must clear the Fire runes outside the blast when reached first"
    );
    for _ in 0..CHAIN_REPEATS {
        let (board, step) = run_bifrost_chain();
        assert_eq!(board, first_board, "board must not depend on hash order");
        assert_eq!(step.cleared_positions, first_step.cleared_positions);
        assert_eq!(step.cleared_runes, first_step.cleared_runes);
        assert_eq!(step.score_gained, first_step.score_gained);
    }
}
