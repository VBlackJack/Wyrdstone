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

//! The property each scenario documents. Seeds were searched until the property held, and the
//! property is re-asserted on every run so a changed seed or changed rules cannot silently pass.

use super::{bundled, Event, Run};
use wyrdstone_core::{CellPos, LevelOutcome};

fn fail<T>(message: &str) -> Result<T, String> {
    Err(message.to_string())
}

/// Checks the documented property of scenario `n` (1..=11).
pub fn check(n: usize, run: &Run) -> Result<(), String> {
    let cleared = run.cleared();
    let ended = run.ended();
    let forge_moves = bundled().level_by_id("forge_1").map_or(0, |l| l.max_moves);
    match n {
        1 => {
            if cleared.len() != 1 || cleared[0].combo != 1 {
                return fail("S1 needs exactly one clear step at combo 1");
            }
            if run.all_events().any(|e| matches!(e, Event::Relics(_))) {
                return fail("S1 must not drop relics");
            }
            let Some(end) = ended.first() else {
                return fail("S1 needs an end of turn");
            };
            if end.snap.moves_left != forge_moves - 1 || end.snap.last_combo != 1 {
                return fail("S1 consumes one move and ends at combo 1");
            }
        }
        2 => {
            let combos: Vec<u32> = cleared.iter().map(|c| c.combo).collect();
            if combos != [1, 2] {
                return fail("S2 needs a clear at combo 1 followed by one cascade at combo 2");
            }
            if ended.first().map(|e| e.snap.last_combo) != Some(2) {
                return fail("S2 ends with last_combo 2");
            }
        }
        3 => {
            if !run.all_events().any(|e| matches!(e, Event::Relics(_))) {
                return fail("S3 needs a relic drop");
            }
            let Some(end) = ended.last() else {
                return fail("S3 needs an end of turn");
            };
            if end.snap.objectives.first() != Some(&1) {
                return fail("S3 completes the relic objective");
            }
            if end.snap.last_combo != 0 {
                return fail("S3 ends with last_combo 0 (relic step resets it)");
            }
        }
        4 => {
            let first_p2 = cleared
                .iter()
                .position(|c| c.phase2_name_key.is_some())
                .ok_or_else(|| "S4 needs a phase 2 entry".to_string())?;
            if first_p2 == 0 || cleared[first_p2].combo < 2 {
                return fail("S4 crosses phase 2 on a cascade step, not the first");
            }
            if cleared[first_p2]
                .snap
                .boss
                .as_ref()
                .is_none_or(|b| b.just_entered_phase2 || !b.phase2_triggered)
            {
                return fail("S4 flag is consumed by the clear step");
            }
        }
        5 => {
            let Some(end) = ended.first() else {
                return fail("S5 needs an end of turn");
            };
            if end.frost.is_none() || !end.shuffled || !end.snap.last_shuffled {
                return fail("S5 needs frost spread then a shuffle in the same turn");
            }
        }
        6 => {
            if ended.len() != 2 {
                return fail("S6 needs a tool turn and a swap turn");
            }
            if run.initial.curses.is_empty()
                || ended[0].snap.curses != run.initial.curses
                || ended[1].snap.curses == ended[0].snap.curses
            {
                return fail("S6 curses must not tick on a tool and must tick on a swap");
            }
        }
        7 => {
            if ended.len() != 2 {
                return fail("S7 needs two tool turns");
            }
            if ended[0].snap.moves_left != forge_moves
                || ended[0].outcome != LevelOutcome::InProgress
            {
                return fail("S7 tool keeps the move count and the level running");
            }
            let loki_ok = run
                .all_events()
                .any(|e| matches!(e, Event::Loki(l) if l.valid && l.shuffled));
            if !loki_ok {
                return fail("S7 Loki shuffle tool is valid and shuffles");
            }
            if ended[1].outcome != LevelOutcome::Defeat || ended[1].snap.moves_left != 0 {
                return fail("S7 finalize runs after a tool (Defeat on zero moves)");
            }
        }
        8 => {
            let furies: Vec<bool> = run
                .all_events()
                .filter_map(|e| match e {
                    Event::Fury(f) => Some(f.started_turn),
                    _ => None,
                })
                .collect();
            if furies != [true, true, false, false] {
                return fail("S8 Thor and Freya clear, Loki and Odin only place pieces");
            }
            if ended.len() != 2 || cleared.len() <= 2 {
                return fail("S8 needs a cascade after a fury clear");
            }
            for end in &ended {
                let riposte_in = end.snap.boss.as_ref().map(|b| b.turns_until_riposte);
                if end.frost.is_some() || !end.riposte_hits.is_empty() || riposte_in != Some(1) {
                    return fail("S8 fury turns tick neither frost nor riposte");
                }
            }
        }
        9 => {
            let Some(end) = ended.first() else {
                return fail("S9 needs an end of turn");
            };
            if end.outcome != LevelOutcome::Victory {
                return fail("S9 first swap wins the level");
            }
            let bursts = run
                .all_events()
                .filter(|e| matches!(e, Event::Burst(_)))
                .count();
            let skip = run.all_events().find_map(|e| match e {
                Event::Skip(s) => Some(s),
                _ => None,
            });
            match skip {
                Some(s) if bursts == 2 && s.detonations == 16 && s.remaining_boosters > 0 => {}
                _ => return fail("S9 needs two single bursts then a skip capped at 16"),
            }
        }
        10 => {
            let first = run.all_events().find_map(|e| match e {
                Event::Pending(p) => Some(p),
                _ => None,
            });
            match first {
                Some(p) if !p.converted.is_empty() && p.consumes_turn && p.combo == 1 => {}
                _ => return fail("S10 needs a converting super combo that consumes the turn"),
            }
            if ended.is_empty() {
                return fail("S10 needs an end of turn");
            }
        }
        11 => {
            let countdown: Vec<u32> = ended
                .iter()
                .map(|e| e.snap.boss.as_ref().map_or(0, |b| b.turns_until_riposte))
                .collect();
            let interval = bundled()
                .bosses
                .iter()
                .find(|b| b.id == "boss_fenrir")
                .map_or(0, |b| b.riposte_interval);
            if countdown
                != [
                    interval - 1,
                    interval - 1,
                    interval - 1,
                    interval - 2,
                    interval,
                ]
                || ended.last().is_none_or(|e| e.riposte_hits.is_empty())
            {
                return fail("S11 riposte counts down on move turns only and fires at zero");
            }
        }
        _ => return fail("unknown scenario"),
    }
    Ok(())
}

/// Positions used by the scripted tool actions.
pub const CHAINED_CELL: CellPos = CellPos::new(3, 3);
/// A corner cell far from the forge obstacles.
pub const CORNER_CELL: CellPos = CellPos::new(0, 0);
/// A cell of the curse level that is neither cursed nor an obstacle.
pub const PLAIN_CELL: CellPos = CellPos::new(4, 1);
