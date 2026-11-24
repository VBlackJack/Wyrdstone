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

//! The turn step machine against the shipped behaviour: scenarios S1-S11 replayed through the new
//! API with the same pinned values, plus a differential fuzz against the legacy rule calls.

mod support;

use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};
use std::cell::RefCell;
use std::collections::HashMap;
use support::{
    apply_setup, bundled, check_scenario, fall_map_vec, legacy_play, legacy_run, level_world,
    pending_event, scenario, snap, sorted, Action, BurstEvent, ClearedEvent, EndedEvent, Event,
    FuryEvent, LokiEvent, RelicsEvent, Run, Scenario, SkipEvent, World,
};
use wyrdstone_core::{
    ActiveTool, Booster, BossRiposteKind, CellPos, CityBonuses, ExpeditionRun, FallMove,
    LevelOutcome, LevelSession, PatronGod, SwapRejected, ToolRejected, Turn, TurnStep,
    VICTORY_SKIP_MAX_DETONATIONS,
};

/// Number of fuzz seeds per level family.
const FUZZ_SEEDS: u64 = 200;
/// Maximum scripted actions per fuzz run.
const FUZZ_MAX_ACTIONS: usize = 40;
/// Salt separating the action-choice stream from the game stream.
const FUZZ_ACTION_SALT: u64 = 0x5eed_0fa0_710a;
/// Percent chance of each action kind: hint swap, then random pair, then random tool.
const FUZZ_HINT_SWAP_PCT: u32 = 50;
const FUZZ_RANDOM_PAIR_PCT: u32 = 80;
/// Percent chance to unleash a ready fury instead of the rolled action.
const FUZZ_FURY_PCT: u32 = 60;
/// Upper bound (exclusive) of celebration bursts played before a victory skip.
/// Every Nth seed opens with a Bifrost shard swapped against a bomb (the combo random play rarely
/// builds on its own).
const FUZZ_SUPER_COMBO_EVERY: u64 = 4;
const FUZZ_MAX_BURSTS: u32 = 3;
/// Expedition wave range explored by the fuzz.
const FUZZ_EXPEDITION_WAVES: u64 = 7;
/// Starting boosters granted to fuzz expedition runs.
const FUZZ_EXPEDITION_BOOSTERS: u32 = 2;

fn fall_vec(fall: &[FallMove]) -> Vec<(CellPos, i32)> {
    // Same last-write-wins map the animation layer builds from the fall moves.
    let mut map: HashMap<CellPos, i32> = HashMap::new();
    for fm in fall {
        map.insert(CellPos::new(fm.col, fm.to_row), fm.from_row);
    }
    fall_map_vec(&map)
}

/// Rule paths only observable from engine steps (the shared event type does not carry them).
#[derive(Default, Debug, Clone, Copy)]
struct StepCounters {
    super_combos: u32,
    fog_cleared: u32,
    riposte_chain: u32,
    riposte_freeze: u32,
    riposte_crates: u32,
    riposte_curse: u32,
}

thread_local! {
    static STEP_COUNTERS: RefCell<StepCounters> = RefCell::new(StepCounters::default());
}

/// Converts one engine step into the observation the legacy driver produces, applying the
/// lifetime statistics the way the animation layer does on a committed clear.
fn push_step(w: &mut World, turn: &Turn, step: TurnStep, events: &mut Vec<Event>) {
    match step {
        TurnStep::ClearPending(preview) => {
            if !preview.prepared.converted_boosters.is_empty() {
                STEP_COUNTERS.with(|c| c.borrow_mut().super_combos += 1);
            }
            events.push(Event::Pending(pending_event(
                &preview.prepared,
                preview.combo,
                turn.consumes_turn(),
                preview.fired_boosters,
                snap(w),
            )));
        }
        TurnStep::Cleared(report) => {
            STEP_COUNTERS.with(|c| c.borrow_mut().fog_cleared += report.step.fog_cleared);
            w.stats.record_clear(report.stats);
            events.push(Event::Cleared(ClearedEvent {
                combo: report.combo,
                score_raw: report.score_raw,
                runes_cleared: report.stats.runes_cleared,
                cleared_positions: sorted(report.step.cleared_positions.clone()),
                fall: fall_vec(&report.fall),
                boss_damaged: report.boss.as_ref().is_some_and(|b| b.damage > 0),
                phase2_name_key: report
                    .boss
                    .as_ref()
                    .filter(|b| b.entered_phase2)
                    .map(|b| b.name_key.clone()),
                snap: snap(w),
            }));
        }
        TurnStep::RelicsDropped(report) => events.push(Event::Relics(RelicsEvent {
            positions: sorted(report.positions),
            fall: fall_vec(&report.fall),
            snap: snap(w),
        })),
        TurnStep::Ended(end) => {
            if end.riposte.is_some() {
                let kind = w.session.boss.as_ref().map(|boss| boss.active_riposte());
                STEP_COUNTERS.with(|c| {
                    let mut c = c.borrow_mut();
                    match kind {
                        Some(BossRiposteKind::ChainRunes(_)) => c.riposte_chain += 1,
                        Some(BossRiposteKind::FreezeCells(_)) => c.riposte_freeze += 1,
                        Some(BossRiposteKind::DropCrates(_)) => c.riposte_crates += 1,
                        Some(BossRiposteKind::CurseRunes(_)) => c.riposte_curse += 1,
                        None => {}
                    }
                });
            }
            let (riposte_hits, riposte_name_key) = match end.riposte {
                Some((hits, name_key)) => (hits, Some(name_key)),
                None => (Vec::new(), None),
            };
            events.push(Event::Ended(EndedEvent {
                frost: end.frost,
                curse_hits: end.curse_hits,
                riposte_hits,
                riposte_name_key,
                cursed_runes_remain: end.cursed_runes_remain,
                shuffled: end.shuffled,
                outcome: end.outcome,
                remaining_boosters: end.remaining_boosters,
                snap: snap(w),
            }));
        }
    }
}

fn engine_turn_events(w: &mut World, first: (Turn, TurnStep), events: &mut Vec<Event>) {
    let (mut turn, step) = first;
    push_step(w, &turn, step, events);
    while let Some(step) = turn.advance(&mut w.session, &w.balance, &mut w.rng) {
        push_step(w, &turn, step, events);
    }
    assert!(turn.is_finished());
}

/// Plays one scripted action through the turn engine and returns what was observed.
fn engine_play(w: &mut World, action: Action) -> Vec<Event> {
    let mut events = Vec::new();
    if apply_setup(w, action) {
        return events;
    }
    match action {
        Action::HintSwap => match w.session.board.find_hint_move() {
            Some((a, b)) => engine_swap(w, a, b, &mut events),
            None => events.push(Event::Rejected),
        },
        Action::Swap(a, b) => engine_swap(w, a, b, &mut events),
        Action::Tool(tool, cell) => match w.session.begin_tool(tool, cell) {
            Ok(first) => engine_turn_events(w, first, &mut events),
            Err(ToolRejected::InvalidTarget | ToolRejected::LevelOver) => {
                events.push(Event::Rejected);
            }
        },
        Action::Fury => {
            let tuning = w.balance.session;
            match w.session.trigger_fury(&tuning, &mut w.rng) {
                Some(fury) => {
                    events.push(Event::Fury(FuryEvent {
                        patron: fury.patron,
                        vfx_targets: sorted(fury.vfx_targets),
                        started_turn: fury.turn.is_some(),
                        snap: snap(w),
                    }));
                    if let Some(first) = fury.turn {
                        engine_turn_events(w, first, &mut events);
                    }
                }
                None => events.push(Event::Rejected),
            }
        }
        Action::Loki => {
            let step = w.session.loki_shuffle(&w.balance, &mut w.rng);
            events.push(Event::Loki(LokiEvent {
                valid: step.valid,
                shuffled: step.shuffled,
                snap: snap(w),
            }));
        }
        Action::VictoryBurst => match w
            .session
            .detonate_next_bonus_booster(&w.balance, &mut w.rng)
        {
            Some(blast) => events.push(Event::Burst(BurstEvent {
                pos: blast.pos,
                booster: blast.booster,
                gained_raw: blast.score_raw,
                cleared: sorted(blast.cleared),
                snap: snap(w),
            })),
            None => events.push(Event::NoBooster),
        },
        Action::VictorySkip => {
            let detonations = w.session.detonate_bonus_boosters(
                &w.balance,
                &mut w.rng,
                VICTORY_SKIP_MAX_DETONATIONS,
            ) as usize;
            events.push(Event::Skip(SkipEvent {
                detonations,
                remaining_boosters: w.session.board.find_remaining_boosters().len(),
                snap: snap(w),
            }));
        }
        Action::SetMovesLeft(_) | Action::FillFury | Action::SetPatron(_) => {}
    }
    events
}

fn engine_swap(w: &mut World, a: CellPos, b: CellPos, events: &mut Vec<Event>) {
    match w.session.begin_swap(a, b) {
        Ok(first) => engine_turn_events(w, first, events),
        Err(SwapRejected::NotSwappable | SwapRejected::NoMatch | SwapRejected::LevelOver) => {
            events.push(Event::Rejected);
        }
    }
}

fn engine_run(scenario: Scenario) -> Run {
    let mut world = scenario.world;
    let initial = snap(&world);
    let events = scenario
        .actions
        .iter()
        .map(|&action| engine_play(&mut world, action))
        .collect();
    Run {
        initial,
        events,
        world,
    }
}

fn replay(n: usize) {
    let run = engine_run(scenario(n));
    check_scenario(n, &run);
}

#[test]
fn s1_plain_three_match() {
    replay(1);
}

#[test]
fn s2_two_deep_cascade_scores_and_charges_fury_per_step() {
    replay(2);
}

#[test]
fn s3_relic_drop_preserves_last_combo() {
    replay(3);
}

#[test]
fn s4_boss_crosses_phase_two_mid_cascade() {
    replay(4);
}

#[test]
fn s5_frost_spreads_before_the_shuffle_check() {
    replay(5);
}

#[test]
fn s6_curses_tick_on_swaps_not_on_tools() {
    replay(6);
}

#[test]
fn s7_mjolnir_keeps_moves_and_finalizes_the_turn() {
    replay(7);
}

#[test]
fn s8_fury_clears_cascade_without_riposte_or_frost_and_support_gods_skip_finalize() {
    replay(8);
}

#[test]
fn s9_victory_sweep_single_burst_and_capped_skip() {
    replay(9);
}

#[test]
fn s10_bifrost_and_line_super_combo() {
    replay(10);
}

#[test]
fn s11_riposte_counts_down_only_on_move_consuming_turns() {
    replay(11);
}

#[test]
fn rejected_requests_leave_the_session_untouched() {
    let mut w = level_world("forge_1", 3, |_| {});
    let before = format!("{:?}", w.session);
    // Not adjacent.
    let far = (CellPos::new(0, 0), CellPos::new(5, 5));
    assert_eq!(
        w.session.begin_swap(far.0, far.1).err(),
        Some(SwapRejected::NotSwappable)
    );
    // Out of bounds tool target.
    assert_eq!(
        w.session
            .begin_tool(ActiveTool::Mjolnir, CellPos::new(99, 99))
            .err(),
        Some(ToolRejected::InvalidTarget)
    );
    // Loki has no targeted clear.
    assert_eq!(
        w.session
            .begin_tool(ActiveTool::LokiShuffle, CellPos::new(0, 0))
            .err(),
        Some(ToolRejected::InvalidTarget)
    );
    assert_eq!(format!("{:?}", w.session), before);
    w.session.outcome = LevelOutcome::Defeat;
    assert_eq!(
        w.session
            .begin_swap(CellPos::new(0, 0), CellPos::new(1, 0))
            .err(),
        Some(SwapRejected::LevelOver)
    );
    assert_eq!(
        w.session
            .begin_tool(ActiveTool::Mjolnir, CellPos::new(0, 0))
            .err(),
        Some(ToolRejected::LevelOver)
    );
}

#[test]
fn a_swap_that_forms_no_match_is_reported_and_reverted() {
    let mut w = level_world("forge_1", 3, |_| {});
    let before_board = w.session.board.clone();
    let mut found = None;
    'search: for row in 0..w.session.board.rows() {
        for col in 0..w.session.board.cols() - 1 {
            let (a, b) = (CellPos::new(col, row), CellPos::new(col + 1, row));
            if w.session.board.can_initiate_swap(a, b) {
                let mut probe = w.session.board.clone();
                if probe.begin_swap_step(a, b).is_none() {
                    found = Some((a, b));
                    break 'search;
                }
            }
        }
    }
    let (a, b) = found.expect("a non-matching swap exists on a fresh board");
    assert_eq!(
        w.session.begin_swap(a, b).err(),
        Some(SwapRejected::NoMatch)
    );
    assert_eq!(w.session.board, before_board);
    assert_eq!(w.session.moves_left, w.session.initial_moves);
}

#[test]
fn a_finished_turn_reports_nothing_more() {
    let mut w = scenario(1).world;
    let (a, b) = w.session.board.find_hint_move().expect("hint");
    let (mut turn, _) = w.session.begin_swap(a, b).expect("swap");
    while turn
        .advance(&mut w.session, &w.balance, &mut w.rng)
        .is_some()
    {}
    assert!(turn.is_finished());
    assert!(turn
        .advance(&mut w.session, &w.balance, &mut w.rng)
        .is_none());
}

#[test]
fn skip_detonation_cap_is_the_shipped_sixteen() {
    assert_eq!(VICTORY_SKIP_MAX_DETONATIONS, 16);
}

// ---------------------------------------------------------------------------------------------
// Differential fuzz
// ---------------------------------------------------------------------------------------------

fn expedition_world(seed: u64) -> World {
    let data = bundled();
    let mut run = ExpeditionRun::new();
    run.wave = 1 + (seed % FUZZ_EXPEDITION_WAVES) as u32;
    run.starting_boosters = FUZZ_EXPEDITION_BOOSTERS;
    let patron = PatronGod::ALL[(seed as usize) % PatronGod::ALL.len()];
    let mut rng = StdRng::seed_from_u64(seed);
    let session = LevelSession::start_expedition_wave_with(
        &run,
        &data.balance.expedition,
        None,
        CityBonuses::default(),
        patron,
        &mut rng,
    );
    World::from_session(session, rng)
}

fn boss_fuzz_world(seed: u64) -> World {
    let data = bundled();
    let boss = &data.bosses[(seed as usize) % data.bosses.len()];
    let patron = PatronGod::ALL[((seed / 4) as usize) % PatronGod::ALL.len()];
    support::boss_world(&boss.id, patron, seed)
}

fn fuzz_worlds(seed: u64) -> Vec<(&'static str, World)> {
    vec![
        ("forge_1", level_world("forge_1", seed, |_| {})),
        (
            "frost great_hall_3",
            level_world("great_hall_3", seed, |_| {}),
        ),
        ("curse shrine_3", level_world("shrine_3", seed, |_| {})),
        ("boss", boss_fuzz_world(seed)),
        ("expedition", expedition_world(seed)),
    ]
}

fn random_pair(w: &World, rng: &mut StdRng) -> (CellPos, CellPos) {
    let cols = w.session.board.cols();
    let rows = w.session.board.rows();
    let a = CellPos::new(rng.gen_range(0..cols), rng.gen_range(0..rows));
    let b = if rng.gen_bool(0.5) {
        CellPos::new((a.col + 1) % cols, a.row)
    } else {
        CellPos::new(a.col, (a.row + 1) % rows)
    };
    (a, b)
}

const FUZZ_TOOLS: [ActiveTool; 6] = [
    ActiveTool::Mjolnir,
    ActiveTool::SurtrEmber,
    ActiveTool::LokiShuffle,
    ActiveTool::ThorBolt,
    ActiveTool::BifrostPrism,
    ActiveTool::DrakkarCharge,
];

fn next_action(w: &World, rng: &mut StdRng) -> Action {
    if w.session.can_trigger_fury() && rng.gen_range(0..100) < FUZZ_FURY_PCT {
        return Action::Fury;
    }
    let roll = rng.gen_range(0..100);
    if roll < FUZZ_HINT_SWAP_PCT {
        Action::HintSwap
    } else if roll < FUZZ_RANDOM_PAIR_PCT {
        let (a, b) = random_pair(w, rng);
        Action::Swap(a, b)
    } else {
        let tool = FUZZ_TOOLS[rng.gen_range(0..FUZZ_TOOLS.len())];
        if tool == ActiveTool::LokiShuffle {
            Action::Loki
        } else {
            let cell = CellPos::new(
                rng.gen_range(0..w.session.board.cols()),
                rng.gen_range(0..w.session.board.rows()),
            );
            Action::Tool(tool, cell)
        }
    }
}

fn assert_same_world(label: &str, step: usize, action: Action, legacy: &World, engine: &World) {
    let context = format!("{label} step {step} action {action:?}");
    assert_eq!(
        format!("{:?}", legacy.session),
        format!("{:?}", engine.session),
        "session state differs: {context}"
    );
    assert!(
        legacy.session.board == engine.session.board,
        "board differs: {context}"
    );
    assert_eq!(legacy.stats, engine.stats, "stats differ: {context}");
    let mut legacy_rng = legacy.rng.clone();
    let mut engine_rng = engine.rng.clone();
    assert_eq!(
        legacy_rng.next_u64(),
        engine_rng.next_u64(),
        "rng state differs: {context}"
    );
}

/// How many times the fuzz exercised each rule path (guards against a vacuous differential run).
#[derive(Default, Debug)]
struct Coverage {
    swaps: u32,
    cascades: u32,
    relic_drops: u32,
    shuffles: u32,
    frost: u32,
    curse_hits: u32,
    riposte: u32,
    phase2: u32,
    fury_clears: u32,
    fury_placements: u32,
    tools: u32,
    loki: u32,
    rejected: u32,
    victories: u32,
    bursts: u32,
    skips: u32,
}

impl Coverage {
    fn record(&mut self, events: &[Event]) {
        for event in events {
            match event {
                Event::Pending(p) if p.combo == 1 && p.consumes_turn => self.swaps += 1,
                Event::Pending(p) if p.combo == 1 => self.tools += 1,
                Event::Pending(_) => self.cascades += 1,
                Event::Relics(_) => self.relic_drops += 1,
                Event::Cleared(c) if c.phase2_name_key.is_some() => self.phase2 += 1,
                Event::Ended(e) => {
                    self.shuffles += u32::from(e.shuffled);
                    self.frost += u32::from(e.frost.is_some());
                    self.curse_hits += u32::from(!e.curse_hits.is_empty());
                    self.riposte += u32::from(!e.riposte_hits.is_empty());
                    self.victories += u32::from(e.outcome == LevelOutcome::Victory);
                }
                Event::Fury(f) if f.started_turn => self.fury_clears += 1,
                Event::Fury(_) => self.fury_placements += 1,
                Event::Loki(_) => self.loki += 1,
                Event::Rejected => self.rejected += 1,
                Event::Burst(_) => self.bursts += 1,
                Event::Skip(_) => self.skips += 1,
                Event::Cleared(_) | Event::NoBooster => {}
            }
        }
    }
}

/// Plants a shard next to a bomb in the board centre and returns the swap that fires the combo.
fn plant_super_combo(w: &mut World) -> Option<(CellPos, CellPos)> {
    let a = CellPos::new(w.session.board.cols() / 2, w.session.board.rows() / 2);
    let b = CellPos::new(a.col + 1, a.row);
    for (pos, booster) in [(a, Booster::BifrostShard), (b, Booster::Bomb)] {
        let cell = w.session.board.get_mut(pos)?;
        if cell.blocked {
            return None;
        }
        cell.booster = booster;
        cell.rune.get_or_insert(wyrdstone_core::RuneType::Fire);
    }
    Some((a, b))
}

fn fuzz_one(label: &str, base: World, seed: u64, coverage: &mut Coverage) {
    let mut legacy = base.clone();
    let mut engine = base;
    if seed.is_multiple_of(FUZZ_SUPER_COMBO_EVERY) {
        if let Some((a, b)) = plant_super_combo(&mut legacy) {
            let _ = plant_super_combo(&mut engine);
            let action = Action::Swap(a, b);
            let legacy_events = legacy_play(&mut legacy, action);
            let engine_events = engine_play(&mut engine, action);
            assert_eq!(
                legacy_events, engine_events,
                "super combo events differ: {label} seed {seed}"
            );
            assert_same_world(label, 0, action, &legacy, &engine);
            coverage.record(&legacy_events);
        }
    }
    let mut chooser = StdRng::seed_from_u64(seed ^ FUZZ_ACTION_SALT);
    for step in 0..FUZZ_MAX_ACTIONS {
        match legacy.session.outcome {
            LevelOutcome::InProgress => {
                let action = next_action(&legacy, &mut chooser);
                let legacy_events = legacy_play(&mut legacy, action);
                let engine_events = engine_play(&mut engine, action);
                assert_eq!(
                    legacy_events, engine_events,
                    "events differ: {label} seed {seed} step {step} action {action:?}"
                );
                assert_same_world(label, step, action, &legacy, &engine);
                coverage.record(&legacy_events);
            }
            LevelOutcome::Victory => {
                for _ in 0..chooser.gen_range(0..FUZZ_MAX_BURSTS) {
                    let legacy_events = legacy_play(&mut legacy, Action::VictoryBurst);
                    let engine_events = engine_play(&mut engine, Action::VictoryBurst);
                    assert_eq!(
                        legacy_events, engine_events,
                        "burst events: {label} seed {seed}"
                    );
                    assert_same_world(label, step, Action::VictoryBurst, &legacy, &engine);
                    coverage.record(&legacy_events);
                }
                let legacy_events = legacy_play(&mut legacy, Action::VictorySkip);
                let engine_events = engine_play(&mut engine, Action::VictorySkip);
                assert_eq!(
                    legacy_events, engine_events,
                    "skip events: {label} seed {seed}"
                );
                assert_same_world(label, step, Action::VictorySkip, &legacy, &engine);
                coverage.record(&legacy_events);
                return;
            }
            LevelOutcome::Defeat => return,
        }
    }
}

#[test]
fn engine_matches_the_legacy_rule_calls_on_random_play() {
    let mut coverage = Coverage::default();
    STEP_COUNTERS.with(|c| *c.borrow_mut() = StepCounters::default());
    for seed in 0..FUZZ_SEEDS {
        for (label, world) in fuzz_worlds(seed) {
            fuzz_one(label, world, seed, &mut coverage);
        }
    }
    let steps = STEP_COUNTERS.with(|c| *c.borrow());
    println!("fuzz coverage: {coverage:?} {steps:?}");
    let c = &coverage;
    let exercised = [
        c.swaps,
        c.cascades,
        c.relic_drops,
        c.shuffles,
        c.frost,
        c.curse_hits,
        c.riposte,
        c.phase2,
        c.fury_clears,
        c.fury_placements,
        c.tools,
        c.loki,
        c.rejected,
        c.victories,
        c.bursts,
        c.skips,
        steps.super_combos,
        steps.fog_cleared,
        steps.riposte_chain,
        steps.riposte_freeze,
        steps.riposte_crates,
        steps.riposte_curse,
    ];
    assert!(
        exercised.iter().all(|&count| count > 0),
        "the fuzz must exercise every rule path: {coverage:?} {steps:?}"
    );
}

#[test]
fn legacy_oracle_replays_scenarios_like_the_engine() {
    for n in 1..=support::SCENARIO_COUNT {
        let legacy = support::digest::digest(&legacy_run(scenario(n)));
        let engine = support::digest::digest(&engine_run(scenario(n)));
        assert_eq!(legacy, engine, "scenario S{n}");
    }
}
