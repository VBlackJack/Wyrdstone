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

//! Shared fixtures for the turn tests.
//!
//! `legacy_*` functions are a verbatim transliteration of the RULE calls (not the visuals) made by
//! the Bevy animation layer at commit c29ab4b, so the turn engine can be compared against the
//! shipped behaviour. Scenarios S1-S11 pin that behaviour with seeded random streams.

pub mod digest;
pub mod expected;
pub mod semantics;

use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::HashMap;
use std::sync::OnceLock;
use wyrdstone_core::{
    ActiveTool, Booster, Cell, CellPos, CityBonuses, ClanStats, FallMove, GameDataBundle, Language,
    LevelDef, LevelOutcome, LevelSession, MatchStepOutcome, Objective, PatronGod, PreparedClear,
    RuneType,
};
use wyrdstone_core::{BalanceConfig, Board};

/// Cap the shipped Valhalla skip path applies to bonus booster detonations (anim.rs at c29ab4b).
pub const LEGACY_SKIP_MAX_DETONATIONS: usize = 16;
/// Combo depth used by every victory-sweep detonation (anim.rs at c29ab4b).
pub const LEGACY_BONUS_DETONATION_COMBO: u32 = 2;
/// Number of scenarios S1..=S11.
pub const SCENARIO_COUNT: usize = 11;

/// Bundled game data, parsed once per process.
pub fn bundled() -> &'static GameDataBundle {
    static DATA: OnceLock<GameDataBundle> = OnceLock::new();
    DATA.get_or_init(|| GameDataBundle::load_bundled(Language::En).expect("bundled data"))
}

/// Everything the animation layer owned that the rules touch: session, lifetime stats, RNG, balance.
#[derive(Clone)]
pub struct World {
    pub session: LevelSession,
    pub stats: ClanStats,
    pub rng: StdRng,
    pub balance: BalanceConfig,
}

impl World {
    pub fn from_session(session: LevelSession, rng: StdRng) -> Self {
        Self {
            session,
            stats: ClanStats::default(),
            rng,
            balance: bundled().balance.clone(),
        }
    }
}

/// Starts a bundled level, optionally rewriting its definition first.
pub fn level_world(level_id: &str, seed: u64, tweak: impl FnOnce(&mut LevelDef)) -> World {
    let mut level = bundled().level_by_id(level_id).expect("level").clone();
    tweak(&mut level);
    let mut rng = StdRng::seed_from_u64(seed);
    let session = LevelSession::start(level, &mut rng);
    World::from_session(session, rng)
}

/// Starts a bundled boss raid with the given patron.
pub fn boss_world(boss_id: &str, patron: PatronGod, seed: u64) -> World {
    let data = bundled();
    let def = data
        .bosses
        .iter()
        .find(|b| b.id == boss_id)
        .expect("boss")
        .clone();
    let mut rng = StdRng::seed_from_u64(seed);
    let session = LevelSession::start_boss_with(
        def,
        None,
        CityBonuses::default(),
        patron,
        &data.balance,
        &mut rng,
    );
    World::from_session(session, rng)
}

/// Sorts positions by `(row, col)`.
pub fn sorted(mut v: Vec<CellPos>) -> Vec<CellPos> {
    v.sort_by_key(|p| (p.row, p.col));
    v
}

fn fnv(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

// ---------------------------------------------------------------------------------------------
// Observations
// ---------------------------------------------------------------------------------------------

/// Boss fields observed after a step.
#[derive(Debug, Clone, PartialEq)]
pub struct BossSnap {
    pub hp: u32,
    pub turns_until_riposte: u32,
    pub last_damage_dealt: u32,
    pub phase2_triggered: bool,
    pub just_entered_phase2: bool,
    pub last_riposte_cells: Vec<CellPos>,
}

/// Compact observation of the session, lifetime stats and board after a step.
#[derive(Debug, Clone, PartialEq)]
pub struct Snap {
    pub score: u32,
    pub fury: u32,
    pub moves_left: u32,
    pub last_combo: u32,
    pub last_shuffled: bool,
    pub turn_ice_broken: u32,
    pub last_frost_spread: Option<CellPos>,
    pub objectives: Vec<u32>,
    pub boss: Option<BossSnap>,
    pub outcome: LevelOutcome,
    pub stats: ClanStats,
    pub curses: Vec<(CellPos, u8)>,
    pub board: u64,
}

pub fn snap(w: &World) -> Snap {
    let s = &w.session;
    Snap {
        score: s.score,
        fury: s.fury,
        moves_left: s.moves_left,
        last_combo: s.last_combo,
        last_shuffled: s.last_shuffled,
        turn_ice_broken: s.turn_ice_broken,
        last_frost_spread: s.last_frost_spread,
        objectives: s.objectives.iter().map(|o| o.done).collect(),
        boss: s.boss.as_ref().map(|b| BossSnap {
            hp: b.hp,
            turns_until_riposte: b.turns_until_riposte,
            last_damage_dealt: b.last_damage_dealt,
            phase2_triggered: b.phase2_triggered,
            just_entered_phase2: b.just_entered_phase2,
            last_riposte_cells: b.last_riposte_cells.clone(),
        }),
        outcome: s.outcome,
        stats: w.stats,
        curses: curse_list(&s.board),
        board: fnv(&format!("{:?}", s.board)),
    }
}

fn curse_list(board: &Board) -> Vec<(CellPos, u8)> {
    let mut out = Vec::new();
    for row in 0..board.rows() {
        for col in 0..board.cols() {
            let pos = CellPos::new(col, row);
            if let Some(cell) = board.get(pos) {
                if cell.curse_turns > 0 {
                    out.push((pos, cell.curse_turns));
                }
            }
        }
    }
    out
}

/// Fingerprint of a position/fall list (used where the list itself is long and random-driven).
pub fn fall_digest(fall: &[(CellPos, i32)]) -> u64 {
    fnv(&format!("{fall:?}"))
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingEvent {
    pub combo: u32,
    pub consumes_turn: bool,
    pub cells: Vec<CellPos>,
    pub spawned: Vec<(CellPos, RuneType, Booster)>,
    pub converted: Vec<(CellPos, Booster)>,
    pub direct_blast: bool,
    pub fired: Vec<(CellPos, Booster)>,
    pub snap: Snap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClearedEvent {
    pub combo: u32,
    pub score_raw: u32,
    pub runes_cleared: u64,
    pub cleared_positions: Vec<CellPos>,
    pub fall: Vec<(CellPos, i32)>,
    pub boss_damaged: bool,
    pub phase2_name_key: Option<String>,
    pub snap: Snap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelicsEvent {
    pub positions: Vec<CellPos>,
    pub fall: Vec<(CellPos, i32)>,
    pub snap: Snap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EndedEvent {
    pub frost: Option<CellPos>,
    pub curse_hits: Vec<CellPos>,
    pub riposte_hits: Vec<CellPos>,
    pub riposte_name_key: Option<String>,
    pub cursed_runes_remain: bool,
    pub shuffled: bool,
    pub outcome: LevelOutcome,
    pub remaining_boosters: usize,
    pub snap: Snap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuryEvent {
    pub patron: PatronGod,
    pub vfx_targets: Vec<CellPos>,
    pub started_turn: bool,
    pub snap: Snap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LokiEvent {
    pub valid: bool,
    pub shuffled: bool,
    pub snap: Snap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BurstEvent {
    pub pos: CellPos,
    pub booster: Booster,
    pub gained_raw: u32,
    pub cleared: Vec<CellPos>,
    pub snap: Snap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipEvent {
    pub detonations: usize,
    pub remaining_boosters: usize,
    pub snap: Snap,
}

/// One observable occurrence while resolving an action.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A swap, tool or fury request that the rules refused (board and session untouched).
    Rejected,
    Pending(PendingEvent),
    Cleared(ClearedEvent),
    Relics(RelicsEvent),
    Ended(EndedEvent),
    Fury(FuryEvent),
    Loki(LokiEvent),
    /// A victory burst with no booster left to detonate.
    NoBooster,
    Burst(BurstEvent),
    Skip(SkipEvent),
}

/// Scripted input for a scenario or fuzz run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Swap the pair returned by `Board::find_hint_move` at the time of the action.
    HintSwap,
    Swap(CellPos, CellPos),
    Tool(ActiveTool, CellPos),
    Fury,
    Loki,
    /// One celebration burst (single bonus booster detonation).
    VictoryBurst,
    /// Skip the celebration (capped synchronous sweep).
    VictorySkip,
    SetMovesLeft(u32),
    /// Fills the Divine Fury gauge.
    FillFury,
    SetPatron(PatronGod),
}

/// Applies a setup action (no rules involved); returns false for any other action.
pub fn apply_setup(w: &mut World, action: Action) -> bool {
    match action {
        Action::SetMovesLeft(moves) => w.session.moves_left = moves,
        Action::FillFury => w.session.fury = w.session.fury_max,
        Action::SetPatron(patron) => w.session.patron = patron,
        _ => return false,
    }
    true
}

pub fn pending_event(
    prepared: &PreparedClear,
    combo: u32,
    consumes_turn: bool,
    fired: Vec<(CellPos, Booster)>,
    snap: Snap,
) -> PendingEvent {
    let mut fired = fired;
    fired.sort_by_key(|(p, _)| (p.row, p.col));
    PendingEvent {
        combo,
        consumes_turn,
        cells: sorted(prepared.cells.clone()),
        spawned: prepared.spawned_boosters.clone(),
        converted: prepared.converted_boosters.clone(),
        direct_blast: prepared.is_direct_blast,
        fired,
        snap,
    }
}

pub fn fall_map_vec(map: &HashMap<CellPos, i32>) -> Vec<(CellPos, i32)> {
    let mut v: Vec<(CellPos, i32)> = map.iter().map(|(&p, &r)| (p, r)).collect();
    v.sort_by_key(|(p, _)| (p.row, p.col));
    v
}

// ---------------------------------------------------------------------------------------------
// Legacy transliteration (rule calls at c29ab4b)
// ---------------------------------------------------------------------------------------------

/// A clear waiting to be committed (the `Clearing` / `SuperComboCharge` phase payload).
pub struct LegacyPending {
    pub prepared: PreparedClear,
    pub combo: u32,
    pub consumes_turn: bool,
}

/// Rule results of the end of a `Clearing` phase.
pub struct LegacyClearEnd {
    pub gained_score: u32,
    pub cleared_total: u64,
    pub boss_damaged: bool,
    pub boss_phase2_name_key: Option<String>,
    pub fall_map: HashMap<CellPos, i32>,
    pub cleared_positions: Vec<CellPos>,
}

/// Rule results of the end of a turn.
pub struct LegacyEnd {
    pub frost_pos: Option<CellPos>,
    pub curse_hits: Vec<CellPos>,
    pub boss_riposte_hits: Vec<CellPos>,
    pub boss_riposte_name_key: Option<String>,
    pub cursed_runes_remain: bool,
    pub did_shuffle: bool,
    pub outcome: LevelOutcome,
    pub remaining_boosters: usize,
}

/// Rule results of the end of a `Falling` phase.
pub enum LegacyFallEnd {
    Relics {
        dropped_positions: Vec<CellPos>,
        fall_map: HashMap<CellPos, i32>,
    },
    Cascade(LegacyPending),
    Ended(LegacyEnd),
}

pub struct LegacyFury {
    pub patron: PatronGod,
    pub vfx_targets: Vec<CellPos>,
    pub pending: Option<LegacyPending>,
}

pub struct LegacyBurst {
    pub pos: CellPos,
    pub kind: Booster,
    pub gained_pts: u32,
    pub cleared_cells: Vec<CellPos>,
}

/// Transliterates anim.rs 944-1012 at c29ab4b (swap animation end, rule calls only).
///
/// The undo snapshot clone (944-948, 954-956) is a caller concern and is not part of the rules.
pub fn legacy_swap_begin(w: &mut World, a: CellPos, b: CellPos) -> Option<LegacyPending> {
    let prepared = w.session.board.begin_swap_step(a, b);
    if let Some(prep) = prepared {
        w.session.consume_move();
        w.session.last_combo = 1;
        w.session.last_shuffled = false;
        // Both branches of the original (SuperComboCharge or Clearing) carry combo 1 and
        // consumes_turn true; the 1.85x duration test only affects timing.
        Some(LegacyPending {
            prepared: prep,
            combo: 1,
            consumes_turn: true,
        })
    } else {
        None
    }
}

/// Transliterates anim.rs 1136-1157 at c29ab4b: boosters read on the first `Clearing` tick.
pub fn legacy_fired_boosters(w: &World, pending: &LegacyPending) -> Vec<(CellPos, Booster)> {
    let mut fired = Vec::new();
    for &pos in &pending.prepared.cells {
        if let Some(cell) = w.session.board.get(pos) {
            if cell.booster != Booster::None {
                fired.push((pos, cell.booster));
            }
        }
    }
    fired
}

/// Transliterates anim.rs 1199-1230 at c29ab4b (end of the `Clearing` phase).
pub fn legacy_clear_end(w: &mut World, pending: &LegacyPending) -> LegacyClearEnd {
    let balance = w.balance.clone();
    let prepared = &pending.prepared;
    let combo = pending.combo;
    let mut boss_damaged = false;
    let mut boss_phase2_name_key: Option<String> = None;
    let mut fall_map = HashMap::new();
    let session = &mut w.session;
    let step = session.board.commit_clear_step(prepared, combo, &balance);
    let gained_score = step.score_gained;
    let cleared_total: u64 = step.cleared_runes.values().map(|&v| v as u64).sum();
    session.apply_step_progress(&step);
    if let Some(boss) = session.boss.as_mut() {
        boss_damaged = boss.last_damage_dealt > 0;
        if boss.just_entered_phase2 {
            boss.just_entered_phase2 = false;
            boss_phase2_name_key = Some(boss.def.name_key.clone());
        }
    }
    let fall_moves: Vec<FallMove> = session.board.apply_gravity_and_refill_tracked(&mut w.rng);
    for fm in fall_moves {
        fall_map.insert(CellPos::new(fm.col, fm.to_row), fm.from_row);
    }
    w.stats.total_runes_cleared += cleared_total;
    w.stats.best_combo = w.stats.best_combo.max(combo);
    LegacyClearEnd {
        gained_score,
        cleared_total,
        boss_damaged,
        boss_phase2_name_key,
        fall_map,
        cleared_positions: step.cleared_positions,
    }
}

/// Transliterates anim.rs 1362-1462 at c29ab4b (end of the `Falling` phase).
pub fn legacy_fall_end(w: &mut World, combo: u32, consumes_turn: bool) -> LegacyFallEnd {
    let balance = w.balance.clone();
    let mut relic_fall_map = HashMap::new();
    let mut dropped_positions = Vec::new();
    {
        {
            let session = &mut w.session;
            let relic_step = session.board.collect_bottom_relics(&balance);
            if relic_step.relics_dropped > 0 {
                dropped_positions = relic_step.cleared_positions.clone();
                session.apply_step_progress(&relic_step);
                let fms = session.board.apply_gravity_and_refill_tracked(&mut w.rng);
                for fm in fms {
                    relic_fall_map.insert(CellPos::new(fm.col, fm.to_row), fm.from_row);
                }
            }
        }
    }
    if !dropped_positions.is_empty() {
        return LegacyFallEnd::Relics {
            dropped_positions,
            fall_map: relic_fall_map,
        };
    }

    let next_cascade = Some(&w.session).and_then(|s| s.board.prepare_cascade_step());
    if let Some(next_prep) = next_cascade {
        w.session.last_combo = combo + 1;
        return LegacyFallEnd::Cascade(LegacyPending {
            prepared: next_prep,
            combo: combo + 1,
            consumes_turn,
        });
    }

    let mut did_shuffle = false;
    let mut frost_pos = None;
    let mut curse_hits = Vec::new();
    let mut boss_riposte_hits = Vec::new();
    let mut boss_riposte_name_key: Option<String> = None;
    {
        let session = &mut w.session;
        if consumes_turn {
            frost_pos = session.apply_frost_spread_if_needed(&mut w.rng);
            curse_hits = session.tick_cursed_runes_if_needed();
            boss_riposte_hits = session.tick_boss_riposte_if_needed(&mut w.rng);
            if !boss_riposte_hits.is_empty() {
                boss_riposte_name_key = session.boss.as_ref().map(|b| b.def.name_key.clone());
            }
        }
        if !session.board.has_valid_moves() && session.board.ensure_playable(&mut w.rng) {
            session.last_shuffled = true;
            did_shuffle = true;
        }
        session.finalize_turn(&balance);
    }
    let cursed_runes_remain = w.session.board.has_cursed_runes();
    let outcome = w.session.outcome;
    let remaining_boosters = w.session.board.find_remaining_boosters().len();
    LegacyFallEnd::Ended(LegacyEnd {
        frost_pos,
        curse_hits,
        boss_riposte_hits,
        boss_riposte_name_key,
        cursed_runes_remain,
        did_shuffle,
        outcome,
        remaining_boosters,
    })
}

/// Transliterates mod.rs 1240-1253 and 1771-1779 at c29ab4b (targeted tool, rule calls only).
///
/// Tool inventory consumption is not part of the rules and is not modelled. The session is not
/// touched when the tool starts: no move is consumed and `last_combo` is not reset.
pub fn legacy_tool(w: &mut World, tool: ActiveTool, cell: CellPos) -> Option<LegacyPending> {
    let prepared = Some(&w.session).and_then(|s| s.board.prepare_tool_step(tool, cell));
    prepared.map(|prep| LegacyPending {
        prepared: prep,
        combo: 1,
        consumes_turn: false,
    })
}

/// Transliterates mod.rs 1071-1128 at c29ab4b (Divine Fury, rule calls only).
pub fn legacy_fury(w: &mut World) -> Option<LegacyFury> {
    let (can_fire, patron) = (w.session.can_trigger_fury(), w.session.patron);
    if !can_fire {
        return None;
    }
    let session_tuning = w.balance.session;
    let (prepared, spawned_positions) = w
        .session
        .trigger_patron_fury_detailed_with(&session_tuning, &mut w.rng);
    let vfx_targets: Vec<CellPos> = prepared
        .as_ref()
        .map(|p| p.cells.clone())
        .unwrap_or(spawned_positions);
    let pending = prepared.map(|prep| LegacyPending {
        prepared: prep,
        combo: 1,
        consumes_turn: false,
    });
    Some(LegacyFury {
        patron,
        vfx_targets,
        pending,
    })
}

/// Transliterates mod.rs 1042-1069 at c29ab4b (Loki tool). The original calls
/// `LevelSession::play_tool(LokiShuffle, ..)`, whose body at that commit is inlined here so this
/// oracle does not move when `play_tool` is later rewritten.
pub fn legacy_loki(w: &mut World) -> MatchStepOutcome {
    let balance = w.balance.clone();
    let session = &mut w.session;
    if session.outcome != LevelOutcome::InProgress {
        return MatchStepOutcome::default();
    }
    let step = session
        .board
        .use_active_tool(ActiveTool::LokiShuffle, None, &balance, &mut w.rng);
    if step.valid {
        session.apply_step_progress(&step);
        session.finalize_turn(&balance);
    }
    step
}

/// Transliterates anim.rs 1648-1675 at c29ab4b (one celebration burst: single detonation).
pub fn legacy_victory_burst(w: &mut World) -> Option<LegacyBurst> {
    let next_booster =
        Some(&w.session).and_then(|s| s.board.find_remaining_boosters().into_iter().next());
    let (b_pos, b_kind) = next_booster?;
    let balance = w.balance.clone();
    let mut gained_pts = 0u32;
    let mut cleared_cells = Vec::new();
    {
        {
            let session = &mut w.session;
            if let Some(prep) = session.board.prepare_booster_detonation(b_pos) {
                let step =
                    session
                        .board
                        .commit_clear_step(&prep, LEGACY_BONUS_DETONATION_COMBO, &balance);
                gained_pts = step.score_gained;
                cleared_cells = step.cleared_positions.clone();
                session.apply_step_progress(&step);
                session.board.apply_gravity_and_refill_tracked(&mut w.rng);
            }
        }
    }
    Some(LegacyBurst {
        pos: b_pos,
        kind: b_kind,
        gained_pts,
        cleared_cells,
    })
}

/// Transliterates anim.rs 1611-1632 at c29ab4b (Valhalla skip). Returns the detonation count.
pub fn legacy_skip(w: &mut World) -> usize {
    let balance = w.balance.clone();
    let mut detonations = 0usize;
    for _ in 0..LEGACY_SKIP_MAX_DETONATIONS {
        let next_pos = Some(&w.session)
            .and_then(|s| s.board.find_remaining_boosters().into_iter().next())
            .map(|(p, _)| p);
        let Some(b_pos) = next_pos else {
            break;
        };
        {
            let session = &mut w.session;
            if let Some(prep) = session.board.prepare_booster_detonation(b_pos) {
                let step =
                    session
                        .board
                        .commit_clear_step(&prep, LEGACY_BONUS_DETONATION_COMBO, &balance);
                session.apply_step_progress(&step);
                session.board.apply_gravity_and_refill_tracked(&mut w.rng);
                detonations += 1;
            }
        }
    }
    detonations
}

fn legacy_turn_events(w: &mut World, first: LegacyPending, events: &mut Vec<Event>) {
    let mut pending = first;
    loop {
        let fired = legacy_fired_boosters(w, &pending);
        events.push(Event::Pending(pending_event(
            &pending.prepared,
            pending.combo,
            pending.consumes_turn,
            fired,
            snap(w),
        )));
        let end = legacy_clear_end(w, &pending);
        events.push(Event::Cleared(ClearedEvent {
            combo: pending.combo,
            score_raw: end.gained_score,
            runes_cleared: end.cleared_total,
            cleared_positions: sorted(end.cleared_positions.clone()),
            fall: fall_map_vec(&end.fall_map),
            boss_damaged: end.boss_damaged,
            phase2_name_key: end.boss_phase2_name_key.clone(),
            snap: snap(w),
        }));
        let combo = pending.combo;
        let consumes_turn = pending.consumes_turn;
        loop {
            match legacy_fall_end(w, combo, consumes_turn) {
                LegacyFallEnd::Relics {
                    dropped_positions,
                    fall_map,
                } => {
                    events.push(Event::Relics(RelicsEvent {
                        positions: sorted(dropped_positions),
                        fall: fall_map_vec(&fall_map),
                        snap: snap(w),
                    }));
                }
                LegacyFallEnd::Cascade(next) => {
                    pending = next;
                    break;
                }
                LegacyFallEnd::Ended(end) => {
                    events.push(Event::Ended(EndedEvent {
                        frost: end.frost_pos,
                        curse_hits: end.curse_hits,
                        riposte_hits: end.boss_riposte_hits,
                        riposte_name_key: end.boss_riposte_name_key,
                        cursed_runes_remain: end.cursed_runes_remain,
                        shuffled: end.did_shuffle,
                        outcome: end.outcome,
                        remaining_boosters: end.remaining_boosters,
                        snap: snap(w),
                    }));
                    return;
                }
            }
        }
    }
}

/// Plays one scripted action through the legacy rule calls and returns what was observed.
pub fn legacy_play(w: &mut World, action: Action) -> Vec<Event> {
    let mut events = Vec::new();
    if apply_setup(w, action) {
        return events;
    }
    match action {
        Action::HintSwap => match w.session.board.find_hint_move() {
            Some((a, b)) => legacy_play_swap(w, a, b, &mut events),
            None => events.push(Event::Rejected),
        },
        Action::Swap(a, b) => legacy_play_swap(w, a, b, &mut events),
        Action::Tool(tool, cell) => match legacy_tool(w, tool, cell) {
            Some(pending) => legacy_turn_events(w, pending, &mut events),
            None => events.push(Event::Rejected),
        },
        Action::Fury => match legacy_fury(w) {
            Some(fury) => {
                events.push(Event::Fury(FuryEvent {
                    patron: fury.patron,
                    vfx_targets: sorted(fury.vfx_targets),
                    started_turn: fury.pending.is_some(),
                    snap: snap(w),
                }));
                if let Some(pending) = fury.pending {
                    legacy_turn_events(w, pending, &mut events);
                }
            }
            None => events.push(Event::Rejected),
        },
        Action::Loki => {
            let step = legacy_loki(w);
            events.push(Event::Loki(LokiEvent {
                valid: step.valid,
                shuffled: step.shuffled,
                snap: snap(w),
            }));
        }
        Action::VictoryBurst => match legacy_victory_burst(w) {
            Some(burst) => events.push(Event::Burst(BurstEvent {
                pos: burst.pos,
                booster: burst.kind,
                gained_raw: burst.gained_pts,
                cleared: sorted(burst.cleared_cells),
                snap: snap(w),
            })),
            None => events.push(Event::NoBooster),
        },
        Action::VictorySkip => {
            let detonations = legacy_skip(w);
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

fn legacy_play_swap(w: &mut World, a: CellPos, b: CellPos, events: &mut Vec<Event>) {
    match legacy_swap_begin(w, a, b) {
        Some(pending) => legacy_turn_events(w, pending, events),
        None => events.push(Event::Rejected),
    }
}

// ---------------------------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------------------------

/// Result of running a script: the events of every action and the final world.
pub struct Run {
    pub initial: Snap,
    pub events: Vec<Vec<Event>>,
    pub world: World,
}

impl Run {
    pub fn all_events(&self) -> impl Iterator<Item = &Event> {
        self.events.iter().flatten()
    }

    pub fn cleared(&self) -> Vec<&ClearedEvent> {
        self.all_events()
            .filter_map(|e| match e {
                Event::Cleared(c) => Some(c),
                _ => None,
            })
            .collect()
    }

    pub fn ended(&self) -> Vec<&EndedEvent> {
        self.all_events()
            .filter_map(|e| match e {
                Event::Ended(c) => Some(c),
                _ => None,
            })
            .collect()
    }
}

/// A scripted scenario: a seeded world and the actions to play.
pub struct Scenario {
    pub world: World,
    pub actions: Vec<Action>,
}

/// Plays every action of the scenario through the legacy rule calls.
pub fn legacy_run(scenario: Scenario) -> Run {
    let mut world = scenario.world;
    let initial = snap(&world);
    let events = scenario
        .actions
        .iter()
        .map(|&action| legacy_play(&mut world, action))
        .collect();
    Run {
        initial,
        events,
        world,
    }
}

/// Phase 2 entry point used by S4: boss hit points chosen so the second clear step crosses it.
pub const S4_BOSS_HP: u32 = 2567;
/// Window (columns, rows) the S5 board is cut down to.
pub const S5_WINDOW: (usize, usize) = (3, 4);
/// Side length and bomb spacing of the S9 board.
pub const S9_BOARD: (usize, usize) = (16, 3);

/// Builds scenario `n` (1..=11): a seeded world and its script. Seeds and parameters were found
/// by searching for the property each scenario documents (see `semantics::check`).
pub fn scenario(n: usize) -> Scenario {
    use semantics::{CHAINED_CELL, CORNER_CELL, PLAIN_CELL};
    let (world, actions) = match n {
        1 => (s1_world(1), vec![Action::HintSwap]),
        2 => (s2_world(0), vec![Action::HintSwap]),
        3 => (
            s3_world(41),
            vec![Action::Swap(CellPos::new(0, 7), CellPos::new(1, 7))],
        ),
        4 => (s4_world(4, S4_BOSS_HP), vec![Action::HintSwap]),
        5 => (
            s5_world(4, S5_WINDOW.0, S5_WINDOW.1),
            vec![Action::HintSwap],
        ),
        6 => (
            s6_world(0),
            vec![
                Action::Tool(ActiveTool::Mjolnir, PLAIN_CELL),
                Action::HintSwap,
            ],
        ),
        7 => (
            s7_world(123),
            vec![
                Action::Tool(ActiveTool::Mjolnir, CHAINED_CELL),
                Action::Loki,
                Action::SetMovesLeft(0),
                Action::Tool(ActiveTool::Mjolnir, CORNER_CELL),
            ],
        ),
        8 => (
            s8_world(0),
            vec![
                Action::FillFury,
                Action::Fury,
                Action::SetPatron(PatronGod::Freya),
                Action::FillFury,
                Action::Fury,
                Action::SetPatron(PatronGod::Loki),
                Action::FillFury,
                Action::Fury,
                Action::SetPatron(PatronGod::Odin),
                Action::FillFury,
                Action::Fury,
            ],
        ),
        9 => (
            s9_world(0, S9_BOARD.0, S9_BOARD.1),
            vec![
                Action::HintSwap,
                Action::VictoryBurst,
                Action::VictoryBurst,
                Action::VictorySkip,
            ],
        ),
        10 => (
            s10_world(0),
            vec![Action::Swap(CellPos::new(0, 0), CellPos::new(1, 0))],
        ),
        11 => (
            s11_world(0),
            vec![
                Action::HintSwap,
                Action::Tool(ActiveTool::Mjolnir, CHAINED_CELL),
                Action::FillFury,
                Action::Fury,
                Action::HintSwap,
                Action::HintSwap,
            ],
        ),
        _ => panic!("unknown scenario {n}"),
    };
    Scenario { world, actions }
}

/// Asserts the documented property, the pinned event digest and the pinned RNG position of a run.
pub fn check_scenario(n: usize, run: &Run) {
    if let Err(message) = semantics::check(n, run) {
        panic!("scenario S{n} property: {message}");
    }
    let got = digest::digest(run);
    let want = expected::lines(n);
    assert_eq!(
        got.len(),
        want.len(),
        "scenario S{n}: {} lines expected, {} observed",
        want.len(),
        got.len()
    );
    for (index, (g, e)) in got.iter().zip(want.iter()).enumerate() {
        assert_eq!(g, e, "scenario S{n} digest line {index}");
    }
}

/// S1 world: plain 3-match on the forge level.
pub fn s1_world(seed: u64) -> World {
    level_world("forge_1", seed, |_| {})
}

/// S2 world: same level, searched for a two-deep cascade.
pub fn s2_world(seed: u64) -> World {
    level_world("forge_1", seed, |_| {})
}

/// S3 world: a buried relic one row above the bottom of column 3 and a relic objective.
pub fn s3_world(seed: u64) -> World {
    level_world("forge_1", seed, |level| {
        level.objectives = vec![
            Objective::DropRelics { target: 1 },
            Objective::ClearIce { target: 12 },
        ];
        level.relic_cells = vec![(3, 6)];
    })
}

/// Fraction of boss hit points (in percent) that triggers phase 2, read from the boss tuning.
pub fn s4_world(seed: u64, hp: u32) -> World {
    let mut w = boss_world("boss_fenrir", PatronGod::Thor, seed);
    if let Some(boss) = w.session.boss.as_mut() {
        boss.hp = hp;
    }
    w
}

/// S5 world: frost level whose board is cut down to a small window so a turn can leave no move.
pub fn s5_world(seed: u64, window_cols: usize, window_rows: usize) -> World {
    let mut w = level_world("great_hall_3", seed, |level| {
        level.relic_cells.clear();
        level.crate_cells.clear();
    });
    let cols = w.session.board.cols();
    let rows = w.session.board.rows();
    for row in 0..rows {
        for col in 0..cols {
            if col >= window_cols || row >= window_rows {
                if let Some(cell) = w.session.board.get_mut(CellPos::new(col, row)) {
                    *cell = Cell::blocked_cell();
                }
            }
        }
    }
    w
}

/// S6 world: the curse level (cursed runes tick on swaps only).
pub fn s6_world(seed: u64) -> World {
    level_world("shrine_3", seed, |_| {})
}

/// S7 world: forge level (a chained cell at (3,3)).
pub fn s7_world(seed: u64) -> World {
    level_world("forge_1", seed, |_| {})
}

/// S8 world: boss raid that would riposte on the very next move-consuming turn, on a frost level.
pub fn s8_world(seed: u64) -> World {
    let mut w = boss_world("boss_fenrir", PatronGod::Thor, seed);
    w.session.level.frost_spread = true;
    if let Some(boss) = w.session.boss.as_mut() {
        boss.turns_until_riposte = 1;
    }
    w
}

/// S9 world: a large board dotted with bombs and an objective any clear completes.
pub fn s9_world(seed: u64, side: usize, spacing: usize) -> World {
    let mut w = level_world("forge_1", seed, |level| {
        level.cols = side;
        level.rows = side;
        level.objectives = vec![Objective::ReachScore { target: 1 }];
        level.ice_cells.clear();
        level.chained_cells.clear();
    });
    let mut row = 1;
    while row < side {
        let mut col = 1;
        while col < side {
            if let Some(cell) = w.session.board.get_mut(CellPos::new(col, row)) {
                cell.booster = Booster::Bomb;
            }
            col += spacing;
        }
        row += spacing;
    }
    w
}

/// S10 world: a rune-less Bifrost Shard next to a horizontal line booster.
pub fn s10_world(seed: u64) -> World {
    let mut w = level_world("forge_1", seed, |level| {
        level.ice_cells.clear();
        level.chained_cells.clear();
    });
    if let Some(cell) = w.session.board.get_mut(CellPos::new(0, 0)) {
        cell.rune = None;
        cell.booster = Booster::BifrostShard;
    }
    if let Some(cell) = w.session.board.get_mut(CellPos::new(1, 0)) {
        cell.booster = Booster::LineHorizontal;
    }
    w
}

/// S11 world: boss raid with the default riposte countdown.
pub fn s11_world(seed: u64) -> World {
    boss_world("boss_fenrir", PatronGod::Thor, seed)
}
