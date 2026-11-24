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

mod support;

use support::{check_scenario, legacy_run, scenario, SCENARIO_COUNT};
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

/// Replays scenario `n` through the legacy rule calls and checks property, digest and RNG position.
fn characterize(n: usize) {
    let run = legacy_run(scenario(n));
    check_scenario(n, &run);
}

#[test]
fn s1_plain_three_match() {
    characterize(1);
}

#[test]
fn s2_two_deep_cascade_scores_and_charges_fury_per_step() {
    characterize(2);
}

#[test]
fn s3_relic_drop_resets_last_combo_to_zero() {
    characterize(3);
}

#[test]
fn s4_boss_crosses_phase_two_mid_cascade() {
    characterize(4);
}

#[test]
fn s5_frost_spreads_before_the_shuffle_check() {
    characterize(5);
}

#[test]
fn s6_curses_tick_on_swaps_not_on_tools() {
    characterize(6);
}

#[test]
fn s7_mjolnir_keeps_moves_and_finalizes_the_turn() {
    characterize(7);
}

#[test]
fn s8_fury_clears_cascade_without_riposte_or_frost_and_support_gods_skip_finalize() {
    characterize(8);
}

#[test]
fn s9_victory_sweep_single_burst_and_capped_skip() {
    characterize(9);
}

#[test]
fn s10_bifrost_and_line_super_combo() {
    characterize(10);
}

#[test]
fn s11_riposte_counts_down_only_on_move_consuming_turns() {
    characterize(11);
}

#[test]
fn scenarios_replay_identically_in_one_process() {
    for n in 1..=SCENARIO_COUNT {
        let first = support::digest::digest(&legacy_run(scenario(n)));
        let second = support::digest::digest(&legacy_run(scenario(n)));
        assert_eq!(first, second, "scenario S{n} is not reproducible");
    }
}
