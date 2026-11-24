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

//! Turn step machine: one resolution engine for swaps, targeted tools and Divine Fury clears.
//!
//! The animation layer drives a [`Turn`] one step at a time and only presents what each step
//! reports: the preview of the cells about to clear, the committed clear with its fall map,
//! the relic drops, and the end of the turn. All rule calls (commit, per-step progress, gravity,
//! relic collection, cascades, end-of-turn hooks, shuffle check, finalize) happen here, in the
//! exact order and with the exact semantics the shipped animation layer used.
//!
//! The cursor lives outside [`LevelSession`], so the session stays cheap to clone for undo.

use crate::balance::{BalanceConfig, SessionTuning};
use crate::board::{
    ActiveTool, Board, Booster, CellPos, FallMove, MatchStepOutcome, PreparedClear,
};
use crate::level::{LevelOutcome, LevelSession};
use crate::patron::PatronGod;
use rand::Rng;

/// Maximum bonus booster detonations of the synchronous victory skip.
pub const VICTORY_SKIP_MAX_DETONATIONS: usize = 16;
/// Combo depth used for every victory sweep detonation.
const BONUS_DETONATION_COMBO: u32 = 2;
/// Combo depth of the first clear step of a turn.
const FIRST_COMBO: u32 = 1;

/// What started a turn (informational: the engine resolves every origin the same way).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOrigin {
    /// A player swap of two adjacent cells.
    Swap { a: CellPos, b: CellPos },
    /// A targeted active tool.
    Tool { tool: ActiveTool, target: CellPos },
    /// A Divine Fury that clears cells.
    Fury(PatronGod),
}

/// Why a swap did not start a turn (the board is left untouched).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapRejected {
    /// The two cells are not adjacent, in bounds and both swappable.
    NotSwappable,
    /// The swap forms no match and no super combo.
    NoMatch,
    /// The level is already won or lost.
    LevelOver,
}

/// Why a tool did not start a turn (the board is left untouched).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRejected {
    /// The target is out of bounds, blocked, fogged, or the tool has no targeted clear.
    InvalidTarget,
    /// The level is already won or lost.
    LevelOver,
}

/// Cells about to clear, reported before the clear is committed.
#[derive(Debug, Clone)]
pub struct ClearPreview {
    pub prepared: PreparedClear,
    pub combo: u32,
    /// Boosters sitting on the cells about to clear, sorted by `(row, col)`.
    pub fired_boosters: Vec<(CellPos, Booster)>,
}

/// Boss damage reported by a committed clear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossHit {
    /// Damage dealt by this step (0 when only the phase change is reported).
    pub damage: u32,
    /// True when this step pushed the boss into phase 2 (the one-shot flag is consumed here).
    pub entered_phase2: bool,
    /// True when this step staggered the boss, cancelling an imminent riposte.
    pub staggered: bool,
    /// Localization key of the boss name.
    pub name_key: String,
}

/// Lifetime statistics earned by one committed clear step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearStatsDelta {
    pub runes_cleared: u64,
    pub combo: u32,
}

/// A committed clear step.
#[derive(Debug, Clone)]
pub struct ClearReport {
    pub combo: u32,
    /// Cells of the prepared clear (popup anchor).
    pub cells: Vec<CellPos>,
    pub step: MatchStepOutcome,
    /// Score before bonuses (what the shipped popups display).
    pub score_raw: u32,
    /// Score actually added to the session, after the score bonus.
    pub score_awarded: u32,
    /// Gravity and refill moves applied right after the clear.
    pub fall: Vec<FallMove>,
    pub boss: Option<BossHit>,
    pub stats: ClearStatsDelta,
    /// True when this step ignited Ragnarok Overdrive.
    pub overdrive_triggered: bool,
}

/// Relics that reached the bottom of their column after a fall.
#[derive(Debug, Clone)]
pub struct RelicReport {
    pub positions: Vec<CellPos>,
    pub fall: Vec<FallMove>,
    pub step: MatchStepOutcome,
}

/// The end of a turn: hooks, shuffle check and finalize have all run.
#[derive(Debug, Clone)]
pub struct TurnEnd {
    pub frost: Option<CellPos>,
    pub curse_hits: Vec<CellPos>,
    /// Cells hit by the boss riposte and the boss name key, when the boss struck.
    pub riposte: Option<(Vec<CellPos>, String)>,
    /// True when cursed runes remain on the board after the hooks.
    pub cursed_runes_remain: bool,
    pub shuffled: bool,
    pub outcome: LevelOutcome,
    pub remaining_boosters: usize,
}

/// One observable step of a turn.
#[derive(Debug, Clone)]
pub enum TurnStep {
    /// Cells are about to clear; call `advance` again to commit them.
    ClearPending(ClearPreview),
    /// A clear was committed and gravity applied.
    Cleared(ClearReport),
    /// Relics were collected and gravity applied again.
    RelicsDropped(RelicReport),
    /// The turn is over.
    Ended(TurnEnd),
}

/// Result of unleashing a Divine Fury.
#[derive(Debug, Clone)]
pub struct FuryOutcome {
    pub patron: PatronGod,
    /// Cells to highlight: the struck cells, or the spawned boosters for support gods.
    pub vfx_targets: Vec<CellPos>,
    /// The clearing turn, absent for gods that only place pieces (Loki, Odin).
    pub turn: Option<(Turn, TurnStep)>,
}

/// One bonus booster detonated by the victory sweep.
#[derive(Debug, Clone)]
pub struct BonusBlast {
    pub pos: CellPos,
    pub booster: Booster,
    /// Score before bonuses.
    pub score_raw: u32,
    /// Score actually added to the session, after the score bonus.
    pub score_awarded: u32,
    pub cleared: Vec<CellPos>,
    pub stats: ClearStatsDelta,
}

#[derive(Debug, Clone)]
enum Stage {
    ClearPending(PreparedClear),
    AfterFall,
    Done,
}

/// Cursor over one turn. Never dropped before it reports [`TurnStep::Ended`].
#[must_use = "a turn must be advanced until it reports TurnStep::Ended"]
#[derive(Debug, Clone)]
pub struct Turn {
    consumes_turn: bool,
    combo: u32,
    stage: Stage,
    origin: TurnOrigin,
}

fn fired_boosters(board: &Board, prepared: &PreparedClear) -> Vec<(CellPos, Booster)> {
    let mut fired = Vec::new();
    for &pos in &prepared.cells {
        if let Some(cell) = board.get(pos) {
            if cell.booster != Booster::None {
                fired.push((pos, cell.booster));
            }
        }
    }
    fired.sort_by_key(|(p, _)| (p.row, p.col));
    fired
}

fn start_turn(
    board: &Board,
    prepared: PreparedClear,
    consumes_turn: bool,
    origin: TurnOrigin,
) -> (Turn, TurnStep) {
    let preview = ClearPreview {
        fired_boosters: fired_boosters(board, &prepared),
        prepared: prepared.clone(),
        combo: FIRST_COMBO,
    };
    let turn = Turn {
        consumes_turn,
        combo: FIRST_COMBO,
        stage: Stage::ClearPending(prepared),
        origin,
    };
    (turn, TurnStep::ClearPending(preview))
}

impl LevelSession {
    /// Starts a player swap turn: swaps the cells, consumes the move and resets the per-turn
    /// trackers. On rejection the board is left untouched.
    pub fn begin_swap(&mut self, a: CellPos, b: CellPos) -> Result<(Turn, TurnStep), SwapRejected> {
        if self.outcome != LevelOutcome::InProgress {
            return Err(SwapRejected::LevelOver);
        }
        if !self.board.can_initiate_swap(a, b) {
            return Err(SwapRejected::NotSwappable);
        }
        let Some(prepared) = self.board.begin_swap_step(a, b) else {
            return Err(SwapRejected::NoMatch);
        };
        self.consume_move();
        self.last_combo = FIRST_COMBO;
        self.last_shuffled = false;
        Ok(start_turn(
            &self.board,
            prepared,
            true,
            TurnOrigin::Swap { a, b },
        ))
    }

    /// Starts a targeted tool turn. The session is untouched at start: no move is consumed and
    /// `last_combo` is not reset. The caller consumes the tool from the inventory between the
    /// preview (`Board::prepare_tool_step`) and this call.
    pub fn begin_tool(
        &mut self,
        tool: ActiveTool,
        target: CellPos,
    ) -> Result<(Turn, TurnStep), ToolRejected> {
        if self.outcome != LevelOutcome::InProgress {
            return Err(ToolRejected::LevelOver);
        }
        let Some(prepared) = self.board.prepare_tool_step(tool, target) else {
            return Err(ToolRejected::InvalidTarget);
        };
        Ok(start_turn(
            &self.board,
            prepared,
            false,
            TurnOrigin::Tool { tool, target },
        ))
    }

    /// Unleashes the Patron God's Divine Fury when the gauge is full. Clearing gods (Thor,
    /// Freya) return a turn that consumes no move; Loki and Odin place pieces and ensure the board
    /// retains a valid move, skipping end-of-turn obstacle hooks.
    pub fn trigger_fury<R: Rng>(
        &mut self,
        tuning: &SessionTuning,
        rng: &mut R,
    ) -> Option<FuryOutcome> {
        if !self.can_trigger_fury() {
            return None;
        }
        let patron = self.patron;
        let (prepared, spawned) = self.trigger_patron_fury_detailed_with(tuning, rng);
        if self.fury >= self.fury_max {
            return None;
        }
        let vfx_targets = prepared
            .as_ref()
            .map_or(spawned, |prepared| prepared.cells.clone());
        if prepared.is_none() && !self.board.has_valid_moves() && self.board.ensure_playable(rng) {
            self.last_shuffled = true;
        }
        let turn = prepared
            .map(|prepared| start_turn(&self.board, prepared, false, TurnOrigin::Fury(patron)));
        Some(FuryOutcome {
            patron,
            vfx_targets,
            turn,
        })
    }

    /// Loki's Shuffle tool: reshuffles the free runes, then evaluates victory or defeat.
    pub fn loki_shuffle<R: Rng>(
        &mut self,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> MatchStepOutcome {
        if self.outcome != LevelOutcome::InProgress {
            return MatchStepOutcome::default();
        }
        let step = self
            .board
            .use_active_tool(ActiveTool::LokiShuffle, None, balance, rng);
        if step.valid {
            self.apply_step_progress(&step);
            self.finalize_turn(balance);
        }
        step
    }

    fn detonate_bonus_booster_at<R: Rng>(
        &mut self,
        pos: CellPos,
        booster: Booster,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> Option<BonusBlast> {
        let prepared = self.board.prepare_booster_detonation(pos)?;
        let score_before = self.score;
        let step = self
            .board
            .commit_clear_step(&prepared, BONUS_DETONATION_COMBO, balance);
        let runes_cleared: u64 = step.cleared_runes.values().map(|&v| u64::from(v)).sum();
        self.apply_step_progress(&step);
        self.board.apply_gravity_and_refill_tracked(rng);
        Some(BonusBlast {
            pos,
            booster,
            score_raw: step.score_gained,
            score_awarded: self.score.saturating_sub(score_before),
            cleared: step.cleared_positions,
            stats: ClearStatsDelta {
                runes_cleared,
                combo: BONUS_DETONATION_COMBO,
            },
        })
    }

    /// Detonates the first remaining booster of the board (one victory celebration burst).
    /// Refill matches are not resolved, and lifetime statistics are returned on [`BonusBlast::stats`].
    pub fn detonate_next_bonus_booster<R: Rng>(
        &mut self,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> Option<BonusBlast> {
        let (pos, booster) = self.board.find_remaining_boosters().into_iter().next()?;
        self.detonate_bonus_booster_at(pos, booster, balance, rng)
    }

    /// Synchronously detonates up to `max` remaining boosters and returns each [`BonusBlast`]
    /// so callers can record lifetime statistics and scores accurately.
    pub fn detonate_bonus_boosters_detailed<R: Rng>(
        &mut self,
        balance: &BalanceConfig,
        rng: &mut R,
        max: usize,
    ) -> Vec<BonusBlast> {
        let mut blasts = Vec::new();
        for _ in 0..max {
            let Some((pos, booster)) = self.board.find_remaining_boosters().into_iter().next()
            else {
                break;
            };
            if let Some(blast) = self.detonate_bonus_booster_at(pos, booster, balance, rng) {
                blasts.push(blast);
            }
        }
        blasts
    }

    /// Synchronously detonates up to `max` remaining boosters (the victory skip) and returns how
    /// many were detonated. Callers pass [`VICTORY_SKIP_MAX_DETONATIONS`].
    pub fn detonate_bonus_boosters<R: Rng>(
        &mut self,
        balance: &BalanceConfig,
        rng: &mut R,
        max: usize,
    ) -> u32 {
        self.detonate_bonus_boosters_detailed(balance, rng, max)
            .len() as u32
    }

    /// Clears the boss hit marker once the portrait flash is over.
    pub fn clear_boss_hit_marker(&mut self) {
        if let Some(boss) = self.boss.as_mut() {
            boss.last_damage_dealt = 0;
        }
    }
}

impl Turn {
    /// Combo depth of the clear step currently pending or last committed.
    pub fn combo(&self) -> u32 {
        self.combo
    }

    /// True when the end of this turn runs the frost, curse and riposte hooks.
    pub fn consumes_turn(&self) -> bool {
        self.consumes_turn
    }

    /// What started this turn.
    pub fn origin(&self) -> TurnOrigin {
        self.origin
    }

    /// True once the turn has reported its end.
    pub fn is_finished(&self) -> bool {
        matches!(self.stage, Stage::Done)
    }

    /// Resolves the next step of the turn, or `None` once it has ended.
    pub fn advance<R: Rng>(
        &mut self,
        session: &mut LevelSession,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> Option<TurnStep> {
        match std::mem::replace(&mut self.stage, Stage::Done) {
            Stage::ClearPending(prepared) => {
                self.stage = Stage::AfterFall;
                Some(TurnStep::Cleared(
                    self.commit(session, balance, rng, prepared),
                ))
            }
            Stage::AfterFall => Some(self.after_fall(session, balance, rng)),
            Stage::Done => None,
        }
    }

    fn commit<R: Rng>(
        &self,
        session: &mut LevelSession,
        balance: &BalanceConfig,
        rng: &mut R,
        prepared: PreparedClear,
    ) -> ClearReport {
        let combo = self.combo;
        let score_before = session.score;
        let step = session.board.commit_clear_step(&prepared, combo, balance);
        let score_raw = step.score_gained;
        let runes_cleared: u64 = step.cleared_runes.values().map(|&v| u64::from(v)).sum();
        session.apply_step_progress(&step);
        let mut boss_hit = None;
        if let Some(boss) = session.boss.as_mut() {
            let damage = boss.last_damage_dealt;
            let entered_phase2 = boss.just_entered_phase2;
            let staggered = boss.just_staggered;
            if entered_phase2 {
                boss.just_entered_phase2 = false;
            }
            if staggered {
                boss.just_staggered = false;
            }
            if damage > 0 || entered_phase2 || staggered {
                boss_hit = Some(BossHit {
                    damage,
                    entered_phase2,
                    staggered,
                    name_key: boss.def.name_key.clone(),
                });
            }
        }
        let overdrive_triggered = std::mem::take(&mut session.just_triggered_overdrive);
        let fall = session.board.apply_gravity_and_refill_tracked(rng);
        ClearReport {
            combo,
            cells: prepared.cells,
            step,
            score_raw,
            score_awarded: session.score - score_before,
            fall,
            boss: boss_hit,
            stats: ClearStatsDelta {
                runes_cleared,
                combo,
            },
            overdrive_triggered,
        }
    }

    fn after_fall<R: Rng>(
        &mut self,
        session: &mut LevelSession,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> TurnStep {
        // Buried relics that reached the bottom are collected before any cascade. Relic collection
        // preserves `last_combo`, `last_shuffled`, and boss damage state across the step.
        let relic_step = session.board.collect_bottom_relics(balance);
        if relic_step.relics_dropped > 0 {
            let positions = relic_step.cleared_positions.clone();
            session.apply_step_progress(&relic_step);
            let fall = session.board.apply_gravity_and_refill_tracked(rng);
            self.stage = Stage::AfterFall;
            return TurnStep::RelicsDropped(RelicReport {
                positions,
                fall,
                step: relic_step,
            });
        }

        if let Some(next) = session.board.prepare_cascade_step() {
            session.last_combo = self.combo + 1;
            self.combo += 1;
            let preview = ClearPreview {
                fired_boosters: fired_boosters(&session.board, &next),
                prepared: next.clone(),
                combo: self.combo,
            };
            self.stage = Stage::ClearPending(next);
            return TurnStep::ClearPending(preview);
        }

        TurnStep::Ended(self.end_turn(session, balance, rng))
    }

    fn end_turn<R: Rng>(
        &self,
        session: &mut LevelSession,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> TurnEnd {
        let mut frost = None;
        let mut curse_hits = Vec::new();
        let mut riposte = None;
        if self.consumes_turn {
            frost = session.apply_frost_spread_if_needed(rng);
            curse_hits = session.tick_cursed_runes_if_needed();
            let hits = session.tick_boss_riposte_if_needed(rng);
            if !hits.is_empty() {
                riposte = session
                    .boss
                    .as_ref()
                    .map(|boss| (hits, boss.def.name_key.clone()));
            }
            session.tick_overdrive_if_needed();
        }
        // The shuffle check is unconditional, even for tool and fury turns.
        let mut shuffled = false;
        if !session.board.has_valid_moves() && session.board.ensure_playable(rng) {
            session.last_shuffled = true;
            shuffled = true;
        }
        session.finalize_turn(balance);
        TurnEnd {
            frost,
            curse_hits,
            riposte,
            cursed_runes_remain: session.board.has_cursed_runes(),
            shuffled,
            outcome: session.outcome,
            remaining_boosters: session.board.find_remaining_boosters().len(),
        }
    }
}
