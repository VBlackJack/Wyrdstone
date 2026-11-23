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

//! Equivalence tests: every `_with(&bundled)` variant must match its legacy wrapper exactly.

use rand::rngs::StdRng;
use rand::SeedableRng;
use wyrdstone_core::balance::{DailyTuning, ExpeditionTuning};
use wyrdstone_core::boss::BOSS_RAID_BUILDING_ID;
use wyrdstone_core::expedition::EXPEDITION_BUILDING_ID;
use wyrdstone_core::level::DAILY_TRIAL_BUILDING_ID;
use wyrdstone_core::{
    Board, BossRiposteKind, BossState, Cell, CellPos, CityState, DailyBlessing, ExpeditionBoon,
    ExpeditionRun, GameDataBundle, Language, LevelDef, LevelOutcome, LevelSession,
    MatchStepOutcome, PatronGod, RuneType, FURY_CHARGE_MAX,
};

const DAILY_DAY_COUNT: u64 = 731;
const EXPEDITION_MAX_WAVE: u32 = 8;
const EXPEDITION_SEED_COUNT: u64 = 20;
const BOON_SEED_COUNT: u64 = 32;
const FURY_SEED_COUNT: u64 = 12;
const STAR_MOVES_INITIAL: u32 = 100;
const BOSS_COMBO_LIMIT: u32 = 40;

fn data() -> GameDataBundle {
    GameDataBundle::load_bundled(Language::En).expect("bundled data must parse")
}

fn as_ron<T: serde::Serialize>(value: &T) -> String {
    ron::to_string(value).expect("value must serialize to RON")
}

fn snapshot(board: &Board) -> Vec<Cell> {
    let mut cells = Vec::new();
    for row in 0..board.rows() {
        for col in 0..board.cols() {
            cells.push(*board.get(CellPos::new(col, row)).expect("cell in bounds"));
        }
    }
    cells
}

#[test]
fn bundled_tuning_matches_defaults() {
    let d = data();
    assert_eq!(d.balance.daily, DailyTuning::default());
    assert_eq!(d.balance.expedition, ExpeditionTuning::default());
}

#[test]
fn daily_challenge_with_bundled_matches_legacy() {
    let d = data();
    for day in 0..DAILY_DAY_COUNT {
        assert_eq!(
            as_ron(&LevelDef::daily_challenge_with(day, &d.balance.daily)),
            as_ron(&LevelDef::daily_challenge(day)),
            "day {day}"
        );
    }
}

#[test]
fn daily_challenge_with_custom_tuning_changes_output() {
    let tuning = DailyTuning {
        max_moves_range: (40, 40),
        ..DailyTuning::default()
    };
    assert_eq!(LevelDef::daily_challenge_with(3, &tuning).max_moves, 40);
}

#[test]
fn expedition_waves_with_bundled_match_legacy() {
    let d = data();
    for wave in 1..=EXPEDITION_MAX_WAVE {
        for extra in [0u32, 20] {
            for seed in 0..EXPEDITION_SEED_COUNT {
                let mut run = ExpeditionRun::new();
                run.wave = wave;
                run.extra_moves = extra;
                let mut a = StdRng::seed_from_u64(seed);
                let mut b = StdRng::seed_from_u64(seed);
                assert_eq!(
                    as_ron(&run.generate_wave_level_with(&d.balance.expedition, &mut a)),
                    as_ron(&run.generate_wave_level(&mut b)),
                    "wave {wave} extra {extra} seed {seed}"
                );
            }
        }
    }
}

#[test]
fn expedition_sessions_with_bundled_match_legacy() {
    let d = data();
    for wave in [1u32, 4, 6] {
        let mut run = ExpeditionRun::new();
        run.wave = wave;
        run.starting_boosters = 2;
        run.fury_headstart = 60;
        run.score_bonus_pct = 25;
        let mut a = StdRng::seed_from_u64(9);
        let mut b = StdRng::seed_from_u64(9);
        let with = LevelSession::start_expedition_wave_with(
            &run,
            &d.balance.expedition,
            None,
            Default::default(),
            PatronGod::Thor,
            &mut a,
        );
        let legacy = LevelSession::start_expedition_wave(
            &run,
            None,
            Default::default(),
            PatronGod::Thor,
            &mut b,
        );
        assert_eq!(as_ron(&with.level), as_ron(&legacy.level));
        assert_eq!(with.fury, legacy.fury);
        assert_eq!(with.score_bonus_pct, legacy.score_bonus_pct);
        assert_eq!(with.moves_left, legacy.moves_left);
        assert_eq!(snapshot(&with.board), snapshot(&legacy.board));
    }
}

#[test]
fn boon_rolls_with_bundled_match_legacy() {
    let d = data();
    for seed in 0..BOON_SEED_COUNT {
        let mut a_run = ExpeditionRun::new();
        let mut b_run = ExpeditionRun::new();
        let mut a = StdRng::seed_from_u64(seed);
        let mut b = StdRng::seed_from_u64(seed);
        a_run.roll_boon_choices_with(&d.balance.expedition, &mut a);
        b_run.roll_boon_choices(&mut b);
        assert_eq!(a_run.pending_boons, b_run.pending_boons, "seed {seed}");
    }
    assert_eq!(d.balance.expedition.boons, ExpeditionBoon::ALL.to_vec());
}

#[test]
fn boon_effects_saturate_instead_of_overflowing() {
    let mut run = ExpeditionRun::new();
    let mut city = CityState::new(&data().balance);
    for _ in 0..3 {
        run.apply_boon(ExpeditionBoon::FuryHeadstart(50), &mut city);
    }
    assert_eq!(run.fury_headstart, FURY_CHARGE_MAX);
    run.apply_boon(ExpeditionBoon::ExtraMoves(u32::MAX), &mut city);
    run.apply_boon(ExpeditionBoon::ExtraMoves(1), &mut city);
    assert_eq!(run.extra_moves, u32::MAX);
}

#[test]
fn stars_with_bundled_match_legacy() {
    let d = data();
    let level = d.level_by_id("forge_1").expect("level exists").clone();
    let mut rng = StdRng::seed_from_u64(1);
    let mut session = LevelSession::start(level, &mut rng);
    session.outcome = LevelOutcome::Victory;
    for initial in [0u32, 10, STAR_MOVES_INITIAL] {
        for moves_left in 0..=STAR_MOVES_INITIAL {
            session.initial_moves = initial;
            session.moves_left = moves_left;
            assert_eq!(
                session.earned_stars_with(&d.balance.session),
                session.earned_stars(),
                "{moves_left}/{initial}"
            );
        }
    }
    session.outcome = LevelOutcome::Defeat;
    assert_eq!(session.earned_stars_with(&d.balance.session), 0);
}

#[test]
fn blessings_with_bundled_match_legacy() {
    let d = data();
    let level = d.level_by_id("forge_1").expect("level exists").clone();
    for blessing in DailyBlessing::ALL {
        let mut rng = StdRng::seed_from_u64(2);
        let mut a = LevelSession::start(level.clone(), &mut rng);
        let mut b = a.clone();
        a.apply_daily_blessing_with(blessing, &d.balance.session);
        b.apply_daily_blessing(blessing);
        assert_eq!(a.fury_charge_bonus_pct, b.fury_charge_bonus_pct);
        assert_eq!(a.resource_bonus_pct, b.resource_bonus_pct);
        assert_eq!(a.boss_damage_bonus_pct, b.boss_damage_bonus_pct);
        assert_eq!(a.initial_moves, b.initial_moves);
        assert_eq!(a.moves_left, b.moves_left);
        assert_eq!(a.daily_blessing, b.daily_blessing);
    }
}

fn sorted_cells(cells: &[CellPos]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = cells.iter().map(|p| (p.row, p.col)).collect();
    out.sort_unstable();
    out
}

#[test]
fn patron_fury_with_bundled_matches_legacy() {
    let d = data();
    let level = d.level_by_id("forge_2").expect("level exists").clone();
    for patron in PatronGod::ALL {
        for seed in 0..FURY_SEED_COUNT {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut a = LevelSession::start_full(
                level.clone(),
                None,
                Default::default(),
                patron,
                None,
                &mut rng,
            );
            a.fury = FURY_CHARGE_MAX;
            let mut b = a.clone();
            let mut ra = StdRng::seed_from_u64(seed + 100);
            let mut rb = StdRng::seed_from_u64(seed + 100);
            let (ca, sa) = a.trigger_patron_fury_detailed_with(&d.balance.session, &mut ra);
            let (cb, sb) = b.trigger_patron_fury_detailed(&mut rb);
            assert_eq!(
                ca.as_ref().map(|c| sorted_cells(&c.cells)),
                cb.as_ref().map(|c| sorted_cells(&c.cells)),
                "{patron:?} seed {seed}"
            );
            assert_eq!(sa, sb, "{patron:?} seed {seed}");
            assert_eq!(a.fury_resource_bonus_pct, b.fury_resource_bonus_pct);
            assert_eq!(a.moves_left, b.moves_left);
            assert_eq!(snapshot(&a.board), snapshot(&b.board));
        }
    }
}

#[test]
fn patron_fury_honors_custom_tuning() {
    let d = data();
    let level = d.level_by_id("forge_2").expect("level exists").clone();
    let mut tuning = d.balance.session;
    tuning.loki_fury_boosters = 3;
    tuning.thor_fury_strike_cells = 2;
    let mut rng = StdRng::seed_from_u64(1);
    let mut loki = LevelSession::start_full(
        level.clone(),
        None,
        Default::default(),
        PatronGod::Loki,
        None,
        &mut rng,
    );
    loki.fury = FURY_CHARGE_MAX;
    let (_, spawned) = loki.trigger_patron_fury_detailed_with(&tuning, &mut rng);
    assert_eq!(spawned.len(), 3);
    let mut thor = LevelSession::start_full(
        level,
        None,
        Default::default(),
        PatronGod::Thor,
        None,
        &mut rng,
    );
    thor.fury = FURY_CHARGE_MAX;
    let (clear, _) = thor.trigger_patron_fury_detailed_with(&tuning, &mut rng);
    assert_eq!(clear.expect("thor strikes").cells.len(), 2);
}

#[test]
fn boss_sessions_with_bundled_match_legacy() {
    let d = data();
    for def in &d.bosses {
        let mut ra = StdRng::seed_from_u64(4);
        let mut rb = StdRng::seed_from_u64(4);
        let with = LevelSession::start_boss_with(
            def.clone(),
            None,
            Default::default(),
            PatronGod::Odin,
            &d.balance,
            &mut ra,
        );
        let legacy = LevelSession::start_boss(
            def.clone(),
            None,
            Default::default(),
            PatronGod::Odin,
            &mut rb,
        );
        assert_eq!(as_ron(&with.level), as_ron(&legacy.level), "{}", def.id);
        assert_eq!(with.level.building_id, BOSS_RAID_BUILDING_ID);
        let (bw, bl) = (with.boss.expect("boss"), legacy.boss.expect("boss"));
        assert_eq!(bw.tuning, bl.tuning);
        assert_eq!(bw.hp, bl.hp);
    }
}

#[test]
fn boss_damage_and_phase_with_bundled_match_legacy() {
    let d = data();
    for def in &d.bosses {
        let mut a = BossState::new_with(def.clone(), &d.balance.boss);
        let mut b = BossState::new(def.clone());
        let mut combo = 1u32;
        while !b.is_defeated() && combo < BOSS_COMBO_LIMIT {
            let mut step = MatchStepOutcome {
                max_combo: combo,
                ice_broken: combo % 3,
                chains_broken: combo % 2,
                ..Default::default()
            };
            step.cleared_runes.insert(def.weakness, 3 + combo);
            step.cleared_runes.insert(RuneType::ALL[0], 2);
            assert_eq!(
                a.apply_step_damage(&step, 25),
                b.apply_step_damage(&step, 25)
            );
            assert_eq!(a.hp, b.hp);
            assert_eq!(a.is_phase2(), b.is_phase2());
            assert_eq!(a.just_entered_phase2, b.just_entered_phase2);
            combo += 1;
        }
    }
}

#[test]
fn boss_phase2_threshold_uses_percentage() {
    let d = data();
    let mut def = d.boss_by_id("boss_fenrir").expect("boss exists").clone();
    def.max_hp = 1000;
    def.phase2_weakness = Some(RuneType::ALL[1]);
    let mut tuning = d.balance.boss;
    tuning.phase2_hp_pct = 25;
    let mut boss = BossState::new_with(def, &tuning);
    boss.hp = 250;
    assert!(!boss.is_phase2());
    boss.hp = 249;
    assert!(boss.is_phase2());
}

#[test]
fn boss_riposte_with_bundled_matches_legacy() {
    let d = data();
    let level = d.level_by_id("forge_1").expect("level exists").clone();
    let mut rng = StdRng::seed_from_u64(3);
    let board = Board::new_for_level(&level, &mut rng);
    for seed in 0..FURY_SEED_COUNT {
        let mut a = board.clone();
        let mut b = board.clone();
        let mut ra = StdRng::seed_from_u64(seed);
        let mut rb = StdRng::seed_from_u64(seed);
        let hit_a = a.apply_boss_riposte_with(
            BossRiposteKind::CurseRunes(2),
            d.balance.boss.curse_turns,
            &mut ra,
        );
        let hit_b = b.apply_boss_riposte(BossRiposteKind::CurseRunes(2), &mut rb);
        assert_eq!(hit_a, hit_b);
        assert_eq!(snapshot(&a), snapshot(&b));
    }
}

#[test]
fn synthetic_building_ids_are_stable() {
    assert_eq!(DAILY_TRIAL_BUILDING_ID, "daily_trial");
    assert_eq!(BOSS_RAID_BUILDING_ID, "boss_raid");
    assert_eq!(EXPEDITION_BUILDING_ID, "expedition");
}
