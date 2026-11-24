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

//! One readable line per observed event; pinned expectations are lists of these lines.

use super::{fall_digest, Event, Run, Snap};
use wyrdstone_core::CellPos;

fn cells_text(cells: &[CellPos]) -> String {
    cells
        .iter()
        .map(|p| format!("{},{}", p.col, p.row))
        .collect::<Vec<_>>()
        .join(" ")
}

fn snap_text(s: &Snap) -> String {
    let boss = match &s.boss {
        Some(b) => format!(
            "boss(hp={} riposte_in={} last_dmg={} p2={} p2_now={} riposte_cells=[{}])",
            b.hp,
            b.turns_until_riposte,
            b.last_damage_dealt,
            b.phase2_triggered,
            b.just_entered_phase2,
            cells_text(&b.last_riposte_cells)
        ),
        None => "boss(none)".to_string(),
    };
    let curses = s
        .curses
        .iter()
        .map(|(p, t)| format!("{},{}:{}", p.col, p.row, t))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "score={} fury={} moves={} last_combo={} shuffled={} ice={} frost={:?} obj={:?} {} out={:?} stats=({},{}) curses=[{}] board={:016x}",
        s.score,
        s.fury,
        s.moves_left,
        s.last_combo,
        s.last_shuffled,
        s.turn_ice_broken,
        s.last_frost_spread.map(|p| (p.col, p.row)),
        s.objectives,
        boss,
        s.outcome,
        s.stats.total_runes_cleared,
        s.stats.best_combo,
        curses,
        s.board
    )
}

/// Describes one event on a single line.
pub fn describe(event: &Event) -> String {
    match event {
        Event::Rejected => "rejected".to_string(),
        Event::NoBooster => "no-booster".to_string(),
        Event::Pending(p) => format!(
            "pending combo={} consumes_turn={} direct={} cells=[{}] spawned={:?} converted={:?} fired={:?} | {}",
            p.combo,
            p.consumes_turn,
            p.direct_blast,
            cells_text(&p.cells),
            p.spawned,
            p.converted,
            p.fired,
            snap_text(&p.snap)
        ),
        Event::Cleared(c) => format!(
            "cleared combo={} raw={} runes={} cleared=[{}] fall={}:{:016x} boss_hit={} phase2={:?} | {}",
            c.combo,
            c.score_raw,
            c.runes_cleared,
            cells_text(&c.cleared_positions),
            c.fall.len(),
            fall_digest(&c.fall),
            c.boss_damaged,
            c.phase2_name_key,
            snap_text(&c.snap)
        ),
        Event::Relics(r) => format!(
            "relics positions=[{}] fall={}:{:016x} | {}",
            cells_text(&r.positions),
            r.fall.len(),
            fall_digest(&r.fall),
            snap_text(&r.snap)
        ),
        Event::Ended(e) => format!(
            "ended frost={:?} curse_hits=[{}] riposte=[{}]{:?} cursed_remain={} shuffled={} outcome={:?} boosters_left={} | {}",
            e.frost.map(|p| (p.col, p.row)),
            cells_text(&e.curse_hits),
            cells_text(&e.riposte_hits),
            e.riposte_name_key,
            e.cursed_runes_remain,
            e.shuffled,
            e.outcome,
            e.remaining_boosters,
            snap_text(&e.snap)
        ),
        Event::Fury(f) => format!(
            "fury patron={:?} started_turn={} targets=[{}] | {}",
            f.patron,
            f.started_turn,
            cells_text(&f.vfx_targets),
            snap_text(&f.snap)
        ),
        Event::Loki(l) => format!(
            "loki valid={} shuffled={} | {}",
            l.valid,
            l.shuffled,
            snap_text(&l.snap)
        ),
        Event::Burst(b) => format!(
            "burst at={},{} booster={:?} raw={} cleared=[{}] | {}",
            b.pos.col,
            b.pos.row,
            b.booster,
            b.gained_raw,
            cells_text(&b.cleared),
            snap_text(&b.snap)
        ),
        Event::Skip(s) => format!(
            "skip detonations={} boosters_left={} | {}",
            s.detonations,
            s.remaining_boosters,
            snap_text(&s.snap)
        ),
    }
}

/// Digest of a whole run: every event line plus the next value of the RNG stream.
pub fn digest(run: &Run) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, events) in run.events.iter().enumerate() {
        for event in events {
            lines.push(format!("a{index}: {}", describe(event)));
        }
    }
    let mut rng = run.world.rng.clone();
    lines.push(format!("rng_next={}", rand::RngCore::next_u64(&mut rng)));
    lines
}
