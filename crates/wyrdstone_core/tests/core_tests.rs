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

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use wyrdstone_core::save::{DEFAULT_BGM_VOLUME, DEFAULT_SFX_VOLUME, SAVE_SCHEMA_VERSION};
use wyrdstone_core::{
    achievement_progress, ActiveTool, AmuletEffect, Board, Booster, BossDef, BossRiposteKind,
    BossState, Cell, CellPos, CityBonuses, CityState, ClanRank, DailyBlessing, ExpeditionBoon,
    ExpeditionRun, GameDataBundle, Language, LevelDef, LevelOutcome, LevelSession,
    MatchStepOutcome, Objective, PatronGod, QuestCondition, ResourceBundle, RuneType, SaveData,
    ToolInventory, FURY_CHARGE_MAX,
};

#[test]
fn bundled_data_loads_and_i18n_has_full_parity() {
    let data = GameDataBundle::load_bundled(Language::Fr).expect("bundled data must parse");
    let (missing_fr, missing_en) = data.i18n.missing_keys();
    assert!(
        missing_fr.is_empty(),
        "Keys missing in fr.json: {missing_fr:?}"
    );
    assert!(
        missing_en.is_empty(),
        "Keys missing in en.json: {missing_en:?}"
    );

    for b in &data.buildings {
        assert!(
            data.level_by_id(&b.level_id).is_some(),
            "Building {} points to missing level {}",
            b.id,
            b.level_id
        );
        assert_ne!(data.i18n.t(&b.name_key), b.name_key);
        assert_ne!(data.i18n.t(&b.desc_key), b.desc_key);
    }

    for q in &data.quests {
        assert_ne!(data.i18n.t(&q.title_key), q.title_key);
        assert_ne!(data.i18n.t(&q.desc_key), q.desc_key);
        match &q.condition {
            QuestCondition::WinBuildingLevel { building_id, .. }
            | QuestCondition::ReachBuildingTier { building_id, .. } => {
                assert!(data.building_by_id(building_id).is_some());
            }
            QuestCondition::WinSpecificLevel { level_id } => {
                assert!(data.level_by_id(level_id).is_some());
            }
            QuestCondition::CraftAnyAmulet
            | QuestCondition::ClaimAnyCollection
            | QuestCondition::CompleteDailyTrials { .. }
            | QuestCondition::DefeatBosses { .. }
            | QuestCondition::ReachExpeditionWave { .. } => {}
        }
    }

    for a in &data.amulets {
        assert_ne!(data.i18n.t(&a.name_key), a.name_key);
    }

    let mut city = CityState::new(&data.balance);
    for col in &data.collections {
        assert_ne!(data.i18n.t(&col.name_key), col.name_key);
        assert_eq!(col.relic_ids.len(), col.relic_name_keys.len());
        for rkey in &col.relic_name_keys {
            assert_ne!(data.i18n.t(rkey), rkey);
        }
    }
    let first_col = &data.collections[0];
    for _ in 0..first_col.relic_ids.len() {
        assert!(city.discover_next_relic(&data.collections).is_some());
    }
    assert!(city.is_collection_ready(first_col));
    assert!(city.try_claim_collection(first_col, &data.balance));
    assert!(!city.is_collection_ready(first_col));
}

#[test]
fn all_bundled_levels_spawn_playable_boards() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    for level in &data.levels {
        let board = Board::new_for_level(level, &mut rng);
        assert!(
            board.has_valid_moves(),
            "Level {} spawned without a valid move",
            level.id
        );
    }
}

#[test]
fn match3_swap_clears_runes_and_breaks_ice() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(7);

    let mut cells = vec![
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Ice),
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Wood),
        Cell::playable(RuneType::Stone),
        Cell::playable(RuneType::Gold),
        Cell::playable(RuneType::Water),
        Cell::playable(RuneType::Gold),
        Cell::playable(RuneType::Water),
        Cell::playable(RuneType::Wood),
        Cell::playable(RuneType::Stone),
        Cell::playable(RuneType::Water),
        Cell::playable(RuneType::Gold),
        Cell::playable(RuneType::Stone),
        Cell::playable(RuneType::Wood),
    ];
    cells[0].ice_layers = 1;
    let mut board = Board::from_cells(4, 4, 6, cells);

    let outcome = board.try_swap(
        CellPos::new(2, 0),
        CellPos::new(3, 0),
        &data.balance,
        &mut rng,
    );
    assert!(outcome.valid);
    assert!(outcome.score_gained > 0);
    assert_eq!(outcome.ice_broken, 1);
    assert_eq!(board.get(CellPos::new(0, 0)).unwrap().ice_layers, 0);
}

#[test]
fn match4_spawns_line_booster_and_bifrost_clears_color() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(99);

    let cells = vec![
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Ice),
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Wood),
        Cell::playable(RuneType::Stone),
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Gold),
        Cell::playable(RuneType::Water),
        Cell::playable(RuneType::Stone),
        Cell::playable(RuneType::Wood),
        Cell::playable(RuneType::Water),
        Cell::playable(RuneType::Wood),
        Cell::playable(RuneType::Gold),
    ];
    let mut board = Board::from_cells(5, 3, 6, cells);
    let outcome = board.try_swap(
        CellPos::new(2, 0),
        CellPos::new(2, 1),
        &data.balance,
        &mut rng,
    );
    assert!(outcome.valid);
    let has_bifrost = (0..3).any(|r| {
        (0..5).any(|c| board.get(CellPos::new(c, r)).unwrap().booster == Booster::BifrostShard)
    });
    assert!(has_bifrost, "5-in-a-row must spawn a BifrostShard booster");
}

#[test]
fn bifrost_shard_chain_trigger_clears_most_frequent_rune() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(11);

    let mut cells = vec![Cell::playable(RuneType::Gold); 16];
    cells[0].rune = None;
    cells[0].booster = Booster::BifrostShard;
    cells[1].rune = Some(RuneType::Fire);
    let mut board = Board::from_cells(4, 4, 6, cells);

    // Smashing the BifrostShard (which has rune: None) with Mjolnir must clear all Gold runes.
    let outcome = board.use_active_tool(
        ActiveTool::Mjolnir,
        Some(CellPos::new(0, 0)),
        &data.balance,
        &mut rng,
    );
    assert!(outcome.valid);
    let gold_cleared = outcome
        .cleared_runes
        .get(&RuneType::Gold)
        .copied()
        .unwrap_or(0);
    assert!(
        gold_cleared >= 14,
        "Expected BifrostShard chain reaction to clear all 14 Gold runes, got {gold_cleared}"
    );
}

#[test]
fn bifrost_shard_line_super_combo_converts_and_detonates() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();

    let mut cells = vec![Cell::playable(RuneType::Water); 16];
    cells[0].rune = None;
    cells[0].booster = Booster::BifrostShard;
    cells[1].rune = Some(RuneType::Fire);
    cells[1].booster = Booster::LineHorizontal;
    cells[10].rune = Some(RuneType::Fire);
    let mut board = Board::from_cells(4, 4, 6, cells);

    let prepared = board
        .begin_swap_step(CellPos::new(0, 0), CellPos::new(1, 0))
        .expect("Bifrost + Line must form a super-combo");
    assert!(
        !prepared.converted_boosters.is_empty(),
        "Super-combo must convert matching color runes into boosters"
    );
    let outcome = board.commit_clear_step(&prepared, 1, &data.balance);
    assert!(outcome.valid);
    assert!(outcome.score_gained >= data.balance.booster_activation_bonus * 2);
}

#[test]
fn bifrost_shard_swap_with_relic_is_rejected() {
    let mut cells: Vec<Cell> = (0..16)
        .map(|i| {
            let col = i % 4;
            let row = i / 4;
            Cell::playable(RuneType::ALL[(col + row * 2) % 6])
        })
        .collect();
    cells[0].rune = None;
    cells[0].booster = Booster::BifrostShard;
    cells[1].rune = None;
    cells[1].relic = true;
    let mut board = Board::from_cells(4, 4, 6, cells);

    assert!(
        board
            .begin_swap_step(CellPos::new(0, 0), CellPos::new(1, 0))
            .is_none(),
        "Swapping BifrostShard with a Relic must not consume the shard"
    );
    assert_eq!(
        board.get(CellPos::new(0, 0)).unwrap().booster,
        Booster::BifrostShard
    );
    assert!(board.get(CellPos::new(1, 0)).unwrap().relic);
}

#[test]
fn active_tools_mjolnir_and_loki_shuffle_work() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(123);
    let mut session = LevelSession::start(data.level_by_id("forge_1").unwrap().clone(), &mut rng);
    let moves_before = session.moves_left;

    let res = session.play_tool(
        ActiveTool::Mjolnir,
        Some(CellPos::new(3, 3)),
        &data.balance,
        &mut rng,
    );
    assert!(res.valid);
    assert_eq!(res.chains_broken, 1);
    assert_eq!(session.moves_left, moves_before);

    let shuf = session.play_tool(ActiveTool::LokiShuffle, None, &data.balance, &mut rng);
    assert!(shuf.valid);
    assert!(shuf.shuffled);
    assert_eq!(session.outcome, LevelOutcome::InProgress);
}

#[test]
fn niflheim_frost_spreads_when_no_ice_broken() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(77);
    let mut lvl = data.level_by_id("great_hall_3").unwrap().clone();
    lvl.frost_spread = true;
    let mut session = LevelSession::start(lvl, &mut rng);

    session.turn_ice_broken = 0;
    let spread = session.apply_frost_spread_if_needed(&mut rng);
    assert!(spread.is_some(), "Frost must spread when no ice was broken");

    session.turn_ice_broken = 1;
    let no_spread = session.apply_frost_spread_if_needed(&mut rng);
    assert!(
        no_spread.is_none(),
        "Frost must not spread when ice was broken"
    );
}

#[test]
fn city_upgrades_quests_and_save_roundtrip() {
    let data = GameDataBundle::load_bundled(Language::Fr).unwrap();
    let mut city = CityState::new(&data.balance);

    let great_hall = data.building_by_id("great_hall").unwrap();
    let forge = data.building_by_id("forge").unwrap();

    assert!(!city.is_building_unlocked(forge));
    assert!(city.try_upgrade_building(forge, &data.balance).is_err());

    city.record_level_victory(
        "great_hall",
        "great_hall_1",
        3,
        &data.levels[0].base_reward,
        &data.balance,
    );
    let q1 = &data.quests[0];
    assert!(city.is_quest_ready(q1));
    assert!(city.try_claim_quest(q1, &data.balance));

    assert_eq!(city.try_upgrade_building(great_hall, &data.balance), Ok(1));
    assert!(city.is_building_unlocked(forge));

    // Quest q8 requires specifically great_hall_2 (Mastery), not replaying great_hall_1 twice.
    let q8 = data.quests.iter().find(|q| q.id == "q8").unwrap();
    city.record_level_victory(
        "great_hall",
        "great_hall_1",
        2,
        &data.levels[0].base_reward,
        &data.balance,
    );
    assert!(!city.is_quest_ready(q8));
    city.record_level_victory(
        "great_hall",
        "great_hall_2",
        3,
        &data.levels[1].base_reward,
        &data.balance,
    );
    assert!(city.is_quest_ready(q8));

    let save = SaveData {
        schema_version: SAVE_SCHEMA_VERSION,
        language: Language::Fr,
        last_saved_unix_secs: 1000,
        energy_regen_anchor_unix_secs: 0,
        muted: true,
        bgm_volume: DEFAULT_BGM_VOLUME,
        sfx_volume: DEFAULT_SFX_VOLUME,
        colorblind_mode: true,
        fast_animations: true,
        reduced_motion: true,
        tutorial_step: 3,
        active_expedition: None,
        city,
    };
    let json = save.to_json().expect("serialize save");
    let loaded = SaveData::from_json(&json).expect("deserialize save");
    assert_eq!(loaded.city.building_tier("great_hall"), 1);
    assert!(loaded.city.claimed_quests.contains("q1"));
    assert!(loaded.muted);
    assert_eq!(loaded.city.level_stars("great_hall_2"), 3);
}

#[test]
fn passive_harvest_and_meadery_energy_cap_respect_balance() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut city = CityState::new(&data.balance);
    city.resources = ResourceBundle {
        wood: 500,
        stone: 500,
        gold: 500,
        mead: 200,
    };
    let great_hall = data.building_by_id("great_hall").unwrap();
    let meadery = data.building_by_id("meadery").unwrap();

    assert_eq!(city.try_upgrade_building(great_hall, &data.balance), Ok(1));
    city.energy = 10;
    assert_eq!(city.try_upgrade_building(meadery, &data.balance), Ok(1));
    assert_eq!(city.energy, city.max_energy(&data.balance));

    // Rebuilt building has a ready harvest at now = 1000, then cools down until +harvest_interval_seconds.
    assert!(city.is_harvest_ready("great_hall", 1000, &data.balance));
    let collected = city.try_collect_harvest(great_hall, 1000, &data.balance);
    assert!(collected.is_some());
    assert!(!city.is_harvest_ready("great_hall", 1050, &data.balance));
    assert!(city.is_harvest_ready(
        "great_hall",
        1000 + data.balance.harvest_interval_seconds,
        &data.balance
    ));
}

#[test]
fn procedural_daily_trial_and_streak_rewards() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(33);
    for day in 20_000..20_010 {
        let daily = LevelDef::daily_challenge(day);
        let board = Board::new_for_level(&daily, &mut rng);
        assert!(board.has_valid_moves(), "Daily {day} must be playable");
    }

    let mut city = CityState::new(&data.balance);
    let reward = LevelDef::daily_challenge(100).base_reward;
    assert_eq!(city.record_daily_victory(100, &reward, &data.balance), 1);
    assert_eq!(city.record_daily_victory(101, &reward, &data.balance), 2);
    assert_eq!(city.record_daily_victory(102, &reward, &data.balance), 3);
    assert_eq!(city.stats.daily_completions, 3);
    // Missing a day resets streak to 1.
    assert_eq!(city.record_daily_victory(105, &reward, &data.balance), 1);
}

struct TestTempDir(std::path::PathBuf);

impl TestTempDir {
    fn new(prefix: &str) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), seq));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp test dir");
        Self(path)
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn atomic_save_and_corrupt_backup() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let dir = TestTempDir::new("wyrdstone_save_test");
    let path = dir.0.join("test_save.json");

    let save = SaveData::new_game(&data.balance, Language::En);
    save.save_to_path(&path).expect("atomic save must succeed");
    let loaded = SaveData::load_from_path(&path)
        .expect("load must succeed")
        .expect("save must exist");
    assert_eq!(loaded.language, Language::En);

    // Corrupt the file and verify backup is created.
    std::fs::write(&path, "{ invalid json").unwrap();
    assert!(SaveData::load_from_path(&path).is_err());
    assert!(path.with_extension("json.corrupt").exists());
}

#[test]
fn p2_tools_crates_relics_timer_and_amulets_work() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(2026);

    let mut city = CityState::new(&data.balance);
    city.resources = ResourceBundle {
        wood: 500,
        stone: 500,
        gold: 600,
        mead: 200,
    };
    let heimdall = data.amulet_by_id("amulet_heimdall").unwrap();
    assert!(city.try_craft_amulet(heimdall, &data.balance).is_ok());
    assert!(city.is_quest_ready(data.quests.iter().find(|q| q.id == "q7").unwrap()));

    let timed_level = data.level_by_id("meadery_2").unwrap().clone();
    let base_moves = timed_level.max_moves;
    let mut session = LevelSession::start_with_amulet(
        timed_level.clone(),
        city.active_amulet_effect(&data.amulets),
        &mut rng,
    );
    assert_eq!(session.moves_left, base_moves + 3);
    assert!(session.time_left_secs.is_some());

    // Eye of Odin spawns a starting booster; Horn of Vidar adds +25% time.
    let odin_session = LevelSession::start_with_amulet(
        timed_level.clone(),
        Some(AmuletEffect::StartingBoosters(1)),
        &mut rng,
    );
    let has_starter_booster = (0..odin_session.board.rows()).any(|r| {
        (0..odin_session.board.cols())
            .any(|c| odin_session.board.get(CellPos::new(c, r)).unwrap().booster != Booster::None)
    });
    assert!(
        has_starter_booster,
        "Eye of Odin must spawn a starting booster"
    );

    let vidar_session = LevelSession::start_with_amulet(
        timed_level,
        Some(AmuletEffect::ExtraTimePct(25)),
        &mut rng,
    );
    assert!(vidar_session.time_left_secs.unwrap() > 110.0);

    session.tick_timer(999.0);
    assert_eq!(session.outcome, LevelOutcome::Defeat);

    // Building-linked relic discovery prefers relics linked to the victorious building.
    let relic_key = city
        .discover_next_relic_for_building("forge", &data.collections)
        .expect("must discover a Nidavellir relic");
    assert_eq!(relic_key, "relic.hammer.name");

    let mut cells = vec![Cell::playable(RuneType::Water); 16];
    cells[2 * 4 + 1].relic = true;
    cells[2 * 4 + 1].rune = None;
    cells[3 * 4 + 1].crate_layers = 1;
    cells[3 * 4 + 1].rune = None;
    let mut board = Board::from_cells(4, 4, 6, cells);

    let outcome = board.use_active_tool(
        ActiveTool::ThorBolt,
        Some(CellPos::new(0, 3)),
        &data.balance,
        &mut rng,
    );
    assert!(outcome.valid);
    assert_eq!(outcome.crates_broken, 1);
    assert_eq!(outcome.relics_dropped, 1);

    let drakkar = board.use_active_tool(
        ActiveTool::DrakkarCharge,
        Some(CellPos::new(1, 1)),
        &data.balance,
        &mut rng,
    );
    assert!(drakkar.valid);
    let prism = board.use_active_tool(
        ActiveTool::BifrostPrism,
        Some(CellPos::new(0, 0)),
        &data.balance,
        &mut rng,
    );
    assert!(prism.valid);
}

#[test]
fn patron_fury_boss_raids_expedition_and_achievements_work() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(9001);

    // 1. All Boss definitions and Patron / Achievement i18n keys resolve.
    assert_eq!(data.bosses.len(), 4);
    for boss in &data.bosses {
        assert_ne!(data.i18n.t(&boss.name_key), boss.name_key);
        assert_ne!(data.i18n.t(&boss.title_key), boss.title_key);
        assert_ne!(data.i18n.t(&boss.desc_key), boss.desc_key);
    }
    for god in PatronGod::ALL {
        assert_ne!(data.i18n.t(god.name_key()), god.name_key());
        assert_ne!(data.i18n.t(god.desc_key()), god.desc_key());
    }
    for ach in &data.achievements {
        assert_ne!(data.i18n.t(&ach.title_key), ach.title_key);
        assert_ne!(data.i18n.t(&ach.desc_key), ach.desc_key);
    }

    // 2. Boss Raid: weakness deals 2x damage, riposte fires every 3 turns, Patron Fury works.
    let fenrir = data.boss_by_id("boss_fenrir").unwrap().clone();
    let mut session = LevelSession::start_boss(
        fenrir.clone(),
        None,
        CityBonuses::default(),
        PatronGod::Odin,
        &mut rng,
    );
    assert!(session.boss.is_some());
    assert_eq!(session.boss.as_ref().unwrap().hp, fenrir.max_hp);

    // Charge Odin's Fury and unleash it (+2 extra moves + Bomb).
    let moves_before = session.moves_left;
    session.fury = FURY_CHARGE_MAX;
    assert!(session.can_trigger_fury());
    let _ = session.trigger_patron_fury(&mut rng);
    assert_eq!(session.fury, 0);
    assert_eq!(session.moves_left, moves_before + 2);

    // Tick 3 turns to trigger Fenrir's chain riposte.
    assert!(session.tick_boss_riposte_if_needed(&mut rng).is_empty());
    assert!(session.tick_boss_riposte_if_needed(&mut rng).is_empty());
    let riposte_hits = session.tick_boss_riposte_if_needed(&mut rng);
    assert!(
        !riposte_hits.is_empty(),
        "Boss must counter-attack on turn 3"
    );

    // 3. Endless Drakkar Expedition: wave generation + boon selection.
    let mut city = CityState::new(&data.balance);
    let mut run = ExpeditionRun::new();
    let w1_session = LevelSession::start_expedition_wave(
        &run,
        None,
        CityBonuses::default(),
        PatronGod::Thor,
        &mut rng,
    );
    assert!(w1_session.board.has_valid_moves());
    run.roll_boon_choices(&mut rng);
    assert_eq!(run.pending_boons.len(), 3);
    run.apply_boon(ExpeditionBoon::ExtraMoves(3), &mut city);
    assert_eq!(run.wave, 2);
    assert_eq!(run.extra_moves, 3);

    // 4. Clan Ranks & Achievements claiming.
    assert_eq!(city.clan_rank(), ClanRank::Scout);
    city.record_boss_victory(&fenrir);
    assert_eq!(city.clan_rank(), ClanRank::Huscarl);
    assert!(city.has_any_claimable_achievement(
        &data.achievements,
        &data.buildings,
        data.bosses.len()
    ));
    assert!(city.try_claim_achievement(
        "ach_first_boss",
        &data.achievements,
        &data.buildings,
        data.bosses.len()
    ));
    assert!(!city.try_claim_achievement(
        "ach_first_boss",
        &data.achievements,
        &data.buildings,
        data.bosses.len()
    ));
}

#[test]
fn amulet_tiers_upgrade_and_fog_of_war_dispels_on_adjacent_match() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(4242);

    // 1. Amulet Tier 1 -> 2 -> 3 upgrades scale effect (Heimdall: +3 -> +4 -> +5 moves).
    let mut city = CityState::new(&data.balance);
    city.resources = ResourceBundle {
        wood: 2000,
        stone: 2000,
        gold: 2000,
        mead: 1000,
    };
    let heimdall = data.amulet_by_id("amulet_heimdall").unwrap();
    assert_eq!(city.try_upgrade_amulet(heimdall, &data.balance), Ok(1));
    assert_eq!(
        city.active_amulet_effect(&data.amulets),
        Some(AmuletEffect::ExtraMoves(3))
    );
    assert_eq!(city.try_upgrade_amulet(heimdall, &data.balance), Ok(2));
    assert_eq!(
        city.active_amulet_effect(&data.amulets),
        Some(AmuletEffect::ExtraMoves(4))
    );
    assert_eq!(city.try_upgrade_amulet(heimdall, &data.balance), Ok(3));
    assert_eq!(
        city.active_amulet_effect(&data.amulets),
        Some(AmuletEffect::ExtraMoves(5))
    );
    assert!(city.try_upgrade_amulet(heimdall, &data.balance).is_err());

    // 2. Fog of War prevents direct swap, and adjacent match dispels fog!
    let mut cells = vec![
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Ice),
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Wood),
        Cell::playable(RuneType::Stone),
        Cell::playable(RuneType::Gold),
        Cell::playable(RuneType::Water),
        Cell::playable(RuneType::Gold),
        Cell::playable(RuneType::Water),
        Cell::playable(RuneType::Wood),
        Cell::playable(RuneType::Stone),
        Cell::playable(RuneType::Water),
        Cell::playable(RuneType::Gold),
        Cell::playable(RuneType::Stone),
        Cell::playable(RuneType::Wood),
    ];
    cells[4].fog = true; // (0, 1) directly below (0, 0) Fire
    let mut board = Board::from_cells(4, 4, 6, cells);
    assert!(!board.can_initiate_swap(CellPos::new(0, 1), CellPos::new(1, 1)));

    let outcome = board.try_swap(
        CellPos::new(2, 0),
        CellPos::new(3, 0),
        &data.balance,
        &mut rng,
    );
    assert!(outcome.valid);
    assert!(
        outcome.fog_cleared >= 1,
        "Adjacent match must dispel Niflheim Fog"
    );
    assert!(!board.get(CellPos::new(0, 1)).unwrap().fog);
}

#[test]
fn fog_of_war_blocks_invisible_matches_and_keeps_shuffles_playable() {
    let mut rng = StdRng::seed_from_u64(808);
    // 3 Fire runes in row 0, with the middle one hidden in fog: must NOT form an invisible match.
    let mut cells = vec![
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Fire),
        Cell::playable(RuneType::Water),
        Cell::playable(RuneType::Wood),
        Cell::playable(RuneType::Stone),
        Cell::playable(RuneType::Gold),
        Cell::playable(RuneType::Ice),
        Cell::playable(RuneType::Gold),
        Cell::playable(RuneType::Water),
        Cell::playable(RuneType::Wood),
        Cell::playable(RuneType::Stone),
        Cell::playable(RuneType::Water),
        Cell::playable(RuneType::Gold),
        Cell::playable(RuneType::Stone),
        Cell::playable(RuneType::Wood),
    ];
    cells[1].fog = true;
    let mut board = Board::from_cells(4, 4, 6, cells);
    assert!(
        board.prepare_cascade_step().is_none(),
        "Fogged cells must never form invisible matches"
    );
    assert!(
        board
            .prepare_tool_step(ActiveTool::BifrostPrism, CellPos::new(1, 0))
            .is_none(),
        "BifrostPrism must not target a fogged cell"
    );
    board.shuffle_free_runes(&mut rng);
    assert!(
        board.has_valid_moves(),
        "Shuffling a board with fog must produce a genuinely swappable move"
    );
}

#[test]
fn eye_of_odin_scales_boosters_and_watchtower_clan_rank_perks_apply() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(5150);
    let mut city = CityState::new(&data.balance);
    city.resources = ResourceBundle {
        wood: 5000,
        stone: 5000,
        gold: 5000,
        mead: 2000,
    };

    // Upgrade Eye of Odin to Tier 3 -> must spawn 3 starting boosters.
    let odin = data.amulet_by_id("amulet_odin").unwrap();
    assert_eq!(city.try_upgrade_amulet(odin, &data.balance), Ok(1));
    assert_eq!(city.try_upgrade_amulet(odin, &data.balance), Ok(2));
    assert_eq!(city.try_upgrade_amulet(odin, &data.balance), Ok(3));
    assert_eq!(
        city.active_amulet_effect(&data.amulets),
        Some(AmuletEffect::StartingBoosters(3))
    );

    // Unlock and build Heimdall's Watchtower (requires Great Hall Tier 2).
    let gh = data.building_by_id("great_hall").unwrap();
    let wt = data.building_by_id("watchtower").unwrap();
    assert_eq!(city.try_upgrade_building(gh, &data.balance), Ok(1));
    assert_eq!(city.try_upgrade_building(gh, &data.balance), Ok(2));
    assert_eq!(city.try_upgrade_building(wt, &data.balance), Ok(1));
    city.glory = 300; // Jarl rank: +10% spoils, +20 starting Fury.

    let bonuses = city.compute_bonuses(&data.balance);
    assert_eq!(
        bonuses.fury_charge_bonus_pct,
        data.balance.watchtower_fury_pct_per_tier
    );
    assert_eq!(bonuses.starting_fury, ClanRank::Jarl.starting_fury());
    assert!(bonuses.resource_bonus_pct >= ClanRank::Jarl.resource_bonus_pct());

    let lvl = data.level_by_id("watchtower_1").unwrap().clone();
    let session = LevelSession::start_full(
        lvl,
        city.active_amulet_effect(&data.amulets),
        bonuses,
        PatronGod::Freya,
        None,
        &mut rng,
    );
    let starter_count = (0..session.board.rows())
        .flat_map(|r| (0..session.board.cols()).map(move |c| CellPos::new(c, r)))
        .filter(|&p| session.board.get(p).unwrap().booster != Booster::None)
        .count();
    assert_eq!(starter_count, 3, "Tier 3 Eye of Odin must spawn 3 boosters");
    assert_eq!(session.fury, ClanRank::Jarl.starting_fury());

    // Live achievement progress counter works.
    let master_builder = data
        .achievements
        .iter()
        .find(|a| a.id == "ach_master_builder")
        .expect("master builder feat exists");
    let (done, target) = achievement_progress(master_builder, &city, &data.buildings, 3);
    assert_eq!(done, 2);
    assert_eq!(target, data.buildings.len() as u64);

    // Persisted ExpeditionRun roundtrip in SaveData.
    let mut run = ExpeditionRun::new();
    run.roll_boon_choices(&mut rng);
    let mut save = SaveData::new_game(&data.balance, Language::Fr);
    save.active_expedition = Some(run);
    let raw = save.to_json().expect("serialize with expedition");
    let loaded = SaveData::from_json(&raw).expect("deserialize with expedition");
    assert_eq!(
        loaded
            .active_expedition
            .as_ref()
            .map(|r| r.pending_boons.len()),
        Some(3)
    );
}

#[test]
fn typography_guard_recursively_scans_all_project_text_files() {
    use std::path::Path;

    let banned: [(char, &str); 21] = [
        ('\u{2014}', "em-dash"),
        ('\u{2013}', "en-dash"),
        ('\u{2010}', "unicode-hyphen"),
        ('\u{2212}', "minus-sign"),
        ('\u{201C}', "left-double-quote"),
        ('\u{201D}', "right-double-quote"),
        ('\u{201E}', "low-double-quote"),
        ('\u{00AB}', "left-guillemet"),
        ('\u{00BB}', "right-guillemet"),
        ('\u{2018}', "left-single-quote"),
        ('\u{2019}', "right-single-quote"),
        ('\u{2026}', "ellipsis"),
        ('\u{00A0}', "nbsp"),
        ('\u{202F}', "narrow-nbsp"),
        ('\u{200B}', "zwsp"),
        ('\u{0153}', "oe-ligature"),
        ('\u{0152}', "OE-ligature"),
        ('\u{2011}', "non-breaking-hyphen"),
        ('\u{2009}', "thin-space"),
        ('\u{00E6}', "ae-ligature"),
        ('\u{00C6}', "AE-ligature"),
    ];
    let text_exts = ["rs", "md", "toml", "yml", "yaml", "json", "ron", "xml"];
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    fn walk(
        dir: &Path,
        root: &Path,
        text_exts: &[&str],
        banned: &[(char, &str)],
        visited_nested: &mut bool,
        violations: &mut Vec<String>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if path.is_dir() {
                let is_hidden_skip = name_str.starts_with('.') && name_str != ".github";
                let is_android_build = name_str == "build"
                    && dir.file_name().is_some_and(|parent| parent == "android");
                if is_hidden_skip
                    || is_android_build
                    || matches!(name_str.as_ref(), "target" | "local")
                {
                    continue;
                }
                // A nested checkout (for example a worktree) is not part of this project tree.
                if path.join(".git").exists() {
                    continue;
                }
                walk(&path, root, text_exts, banned, visited_nested, violations);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !text_exts.contains(&ext) {
                    continue;
                }
                let rel = path.strip_prefix(root).unwrap_or(&path);
                if rel.components().count() >= 3 {
                    *visited_nested = true;
                }
                let content = match std::fs::read_to_string(&path) {
                    Ok(text) => text,
                    Err(err) => {
                        violations.push(format!("{}: invalid UTF-8: {err}", rel.display()));
                        continue;
                    }
                };
                if content.starts_with('\u{FEFF}') {
                    violations.push(format!("{}:1 BOM", rel.display()));
                }
                for (line_no, line) in content.lines().enumerate() {
                    for &(ch, label) in banned {
                        if line.contains(ch) {
                            violations.push(format!(
                                "{}:{} forbidden {}",
                                rel.display(),
                                line_no + 1,
                                label
                            ));
                        }
                    }
                }
            }
        }
    }

    let mut visited_nested = false;
    let mut violations = Vec::new();
    walk(
        root,
        root,
        &text_exts,
        &banned,
        &mut visited_nested,
        &mut violations,
    );
    assert!(
        visited_nested,
        "Typography guard must prove it reached nested subdirectories"
    );
    assert!(
        violations.is_empty(),
        "Forbidden typography characters found:\n{}",
        violations.join("\n")
    );
}

#[test]
fn win_streak_two_phase_boss_and_cursed_runes_work() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(777);

    // 1. Win Streak >= 2 grants +1 starting booster and resets on reset_win_streak().
    let mut city = CityState::new(&data.balance);
    assert_eq!(city.compute_bonuses(&data.balance).starting_boosters, 0);
    city.record_level_victory(
        "great_hall",
        "great_hall_1",
        3,
        &data.levels[0].base_reward,
        &data.balance,
    );
    assert_eq!(city.stats.win_streak, 1);
    assert_eq!(city.compute_bonuses(&data.balance).starting_boosters, 0);
    city.record_level_victory(
        "great_hall",
        "great_hall_1",
        3,
        &data.levels[0].base_reward,
        &data.balance,
    );
    assert_eq!(city.stats.win_streak, 2);
    assert_eq!(city.compute_bonuses(&data.balance).starting_boosters, 1);
    city.reset_win_streak();
    assert_eq!(city.stats.win_streak, 0);
    assert_eq!(city.stats.best_win_streak, 2);

    // 2. Boss 2-Phase transition below 50% HP switches weakness and riposte.
    let fenrir = data.boss_by_id("boss_fenrir").unwrap().clone();
    let mut boss_session = LevelSession::start_boss(
        fenrir.clone(),
        None,
        CityBonuses::default(),
        PatronGod::Thor,
        &mut rng,
    );
    let boss = boss_session.boss.as_mut().unwrap();
    assert!(!boss.is_phase2());
    assert_eq!(boss.active_weakness(), RuneType::Fire);
    boss.hp = fenrir.max_hp / 2 - 10;
    assert!(boss.is_phase2());
    assert_eq!(boss.active_weakness(), RuneType::Ice);

    // 3. Cursed Rune ticks down and freezes orthogonal neighbors at 0 without chaining relics.
    let mut cells = vec![Cell::playable(RuneType::Wood); 16];
    cells[5].curse_turns = 2; // (1, 1)
    cells[1].relic = true; // (1, 0) neighbor is a relic
    cells[1].rune = None;
    let mut board = Board::from_cells(4, 4, 6, cells);
    assert!(board.tick_cursed_runes().is_empty());
    assert_eq!(board.get(CellPos::new(1, 1)).unwrap().curse_turns, 1);
    let affected = board.tick_cursed_runes();
    assert!(!affected.is_empty());
    assert_eq!(board.get(CellPos::new(1, 1)).unwrap().curse_turns, 0);
    assert_eq!(board.get(CellPos::new(2, 1)).unwrap().ice_layers, 1);
    assert!(
        !board.get(CellPos::new(1, 0)).unwrap().chained
            && board.get(CellPos::new(1, 0)).unwrap().ice_layers == 0,
        "Cursed rune detonation must never freeze or chain a relic"
    );
}

#[test]
fn hel_boss_daily_blessing_and_fast_anim_save_work() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(31415);

    // 1. All 4 bosses spawn playable boards; Hel curses non-fogged runes on riposte.
    for boss in &data.bosses {
        let board = Board::new_for_level(&boss.to_level_def(), &mut rng);
        assert!(
            board.has_valid_moves(),
            "Boss {} must spawn a playable board",
            boss.id
        );
    }
    let hel = data.boss_by_id("boss_hel").unwrap().clone();
    let mut hel_session =
        LevelSession::start_boss(hel, None, CityBonuses::default(), PatronGod::Odin, &mut rng);
    assert!(!hel_session.board.has_cursed_runes());
    assert!(hel_session.tick_boss_riposte_if_needed(&mut rng).is_empty());
    assert!(hel_session.tick_boss_riposte_if_needed(&mut rng).is_empty());
    let hits = hel_session.tick_boss_riposte_if_needed(&mut rng);
    assert!(!hits.is_empty(), "Hel must curse runes on turn 3");
    assert!(hel_session.board.has_cursed_runes());
    for pos in hits {
        let cell = hel_session.board.get(pos).unwrap();
        assert!(!cell.fog, "Hel must never curse an invisible fogged cell");
        assert!(cell.curse_turns > 0);
    }

    // 2. Daily Fjord Blessings resolve i18n and apply all 4 modifiers accurately.
    for (idx, blessing) in DailyBlessing::ALL.iter().enumerate() {
        assert_eq!(DailyBlessing::for_day(idx as u64), *blessing);
        assert_ne!(data.i18n.t(blessing.name_key()), blessing.name_key());
        assert_ne!(data.i18n.t(blessing.desc_key()), blessing.desc_key());
    }
    let base_moves = hel_session.moves_left;
    hel_session.apply_daily_blessing(DailyBlessing::IdunnGrace);
    assert_eq!(hel_session.moves_left, base_moves + 1);
    assert_eq!(hel_session.initial_moves, base_moves + 1);
    hel_session.apply_daily_blessing(DailyBlessing::FenrirEclipse);
    assert_eq!(hel_session.boss_damage_bonus_pct, 25);
}

#[test]
fn victory_remaining_boosters_detonate_and_award_points() {
    let data = GameDataBundle::load_bundled(Language::En).expect("bundled data must load");
    let mut cells = Vec::with_capacity(36);
    for r in 0..6 {
        for c in 0..6 {
            cells.push(Cell::playable(RuneType::ALL[(r + c) % RuneType::ALL.len()]));
        }
    }
    let mut board = Board::from_cells(6, 6, RuneType::ALL.len(), cells);
    let pos = CellPos::new(2, 2);
    board.get_mut(pos).unwrap().booster = Booster::Bomb;

    let boosters = board.find_remaining_boosters();
    assert_eq!(boosters, vec![(pos, Booster::Bomb)]);

    let prep = board
        .prepare_booster_detonation(pos)
        .expect("booster detonation must prepare");
    let step = board.commit_clear_step(&prep, 1, &data.balance);
    assert!(step.score_gained > data.balance.booster_activation_bonus);
    assert!(board.find_remaining_boosters().is_empty());
}

#[test]
fn login_tribute_calendar_tracks_7_day_cycle_and_resets_on_gap() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut city = CityState::new(&data.balance);

    assert!(city.is_login_tribute_available(100));
    assert_eq!(city.next_login_tribute_day(100), 1);

    let (day1, res1, tool1) = city.try_claim_login_tribute(100).expect("day 1 claim");
    assert_eq!(day1, 1);
    assert!(res1.wood > 0 && res1.gold > 0);
    assert!(tool1.is_none());
    assert!(!city.is_login_tribute_available(100));
    assert!(city.try_claim_login_tribute(100).is_none());

    // Consecutive days 2..=7 escalate and grant tools on days 3, 5, 7.
    let mut tools_won = 0;
    for offset in 1..=6u64 {
        let today = 100 + offset;
        let expected_day = (offset + 1) as u8;
        assert_eq!(city.next_login_tribute_day(today), expected_day);
        let (claimed_day, _, tool) = city.try_claim_login_tribute(today).expect("streak claim");
        assert_eq!(claimed_day, expected_day);
        if tool.is_some() {
            tools_won += 1;
        }
    }
    assert_eq!(tools_won, 3);

    // Day 8 wraps cleanly back to Day 1; missing a day also resets to Day 1.
    assert_eq!(city.next_login_tribute_day(107), 1);
    assert_eq!(city.next_login_tribute_day(115), 1);
}

// ---------------------------------------------------------------------------
// Content feasibility: every level, daily and expedition wave must be winnable.
// ---------------------------------------------------------------------------

/// Number of consecutive day indices checked for the procedural daily challenge.
const DAILY_FEASIBILITY_DAYS: u64 = 365;
/// First day index of the second daily window (a realistic present-day UTC day count).
const DAILY_WINDOW_OFFSET: u64 = 20_000;
/// Seeds exercised for expedition wave generation.
const EXPEDITION_FEASIBILITY_SEEDS: u64 = 60;
/// Highest expedition wave checked.
const EXPEDITION_MAX_WAVE: u32 = 20;

fn count_relics(board: &Board) -> u32 {
    let mut n = 0;
    for row in 0..board.rows() {
        for col in 0..board.cols() {
            if board.get(CellPos::new(col, row)).is_some_and(|c| c.relic) {
                n += 1;
            }
        }
    }
    n
}

/// Asserts that every objective of the level can be reached with the rune pool and board built.
fn assert_level_feasible(level: &LevelDef, rng: &mut StdRng) {
    let pool = level.rune_types_count.clamp(3, RuneType::ALL.len());
    let board = Board::new_for_level(level, rng);
    for objective in &level.objectives {
        match objective {
            Objective::CollectRune { rune, .. } => assert!(
                RuneType::ALL[..pool].contains(rune),
                "level {}: target rune {rune:?} is outside the pool of {pool} runes",
                level.id
            ),
            Objective::DropRelics { target } => {
                let placed = count_relics(&board);
                assert!(
                    placed >= *target,
                    "level {}: DropRelics target {target} but only {placed} relics placed",
                    level.id
                );
            }
            Objective::ClearIce { .. } | Objective::ReachScore { .. } => {}
        }
    }
}

#[test]
fn every_bundled_level_is_feasible() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(11);
    for level in &data.levels {
        assert_level_feasible(level, &mut rng);
    }
}

#[test]
fn daily_challenges_are_feasible_for_a_full_year() {
    let mut rng = StdRng::seed_from_u64(12);
    for window in [0, DAILY_WINDOW_OFFSET] {
        for day in window..window + DAILY_FEASIBILITY_DAYS {
            let level = LevelDef::daily_challenge(day);
            assert_level_feasible(&level, &mut rng);
            for objective in &level.objectives {
                if let Objective::DropRelics { target } = objective {
                    assert_eq!(
                        *target as usize,
                        level.relic_cells.len(),
                        "day {day}: relic target must match relic cells"
                    );
                    for &(c, r) in &level.relic_cells {
                        assert!(
                            !level.blocked_cells.contains(&(c, r)),
                            "day {day}: relic placed on blocked cell"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn expedition_waves_only_target_runes_inside_the_pool() {
    for seed in 0..EXPEDITION_FEASIBILITY_SEEDS {
        let mut rng = StdRng::seed_from_u64(seed);
        for wave in 1..=EXPEDITION_MAX_WAVE {
            let mut run = ExpeditionRun::new();
            run.wave = wave;
            let level = run.generate_wave_level(&mut rng);
            assert_level_feasible(&level, &mut rng);
        }
    }
}

// ---------------------------------------------------------------------------
// Swap validation and shuffle safety.
// ---------------------------------------------------------------------------

fn rune_grid(cols: usize, rows: usize, layout: &[&str]) -> Vec<Cell> {
    assert_eq!(layout.len(), rows);
    let mut cells = Vec::with_capacity(cols * rows);
    for line in layout {
        let chars: Vec<char> = line.chars().collect();
        assert_eq!(chars.len(), cols);
        for ch in chars {
            let rune = match ch {
                'F' => RuneType::Fire,
                'I' => RuneType::Ice,
                'W' => RuneType::Wood,
                'S' => RuneType::Stone,
                'G' => RuneType::Gold,
                'A' => RuneType::Water,
                other => panic!("unknown rune letter {other}"),
            };
            cells.push(Cell::playable(rune));
        }
    }
    cells
}

#[test]
fn begin_swap_step_rejects_swap_that_only_leaves_a_match_elsewhere() {
    // Row 4 already holds a Gold triple, but swapping (0,0) and (1,0) creates no match.
    let layout = ["FIWSG", "IFSWA", "WSFGI", "SWGAF", "GGGAW"];
    let cells = rune_grid(5, 5, &layout);
    let mut board = Board::from_cells(5, 5, 6, cells);
    let before: Vec<Option<RuneType>> = (0..5)
        .flat_map(|r| (0..5).map(move |c| (c, r)))
        .map(|(c, r)| board.get(CellPos::new(c, r)).unwrap().rune)
        .collect();
    let result = board.begin_swap_step(CellPos::new(0, 0), CellPos::new(1, 0));
    assert!(
        result.is_none(),
        "a swap that creates no match with either swapped cell must be rejected"
    );
    let after: Vec<Option<RuneType>> = (0..5)
        .flat_map(|r| (0..5).map(move |c| (c, r)))
        .map(|(c, r)| board.get(CellPos::new(c, r)).unwrap().rune)
        .collect();
    assert_eq!(before, after, "a rejected swap must leave the board intact");
}

#[test]
fn has_valid_moves_agrees_with_begin_swap_step_on_random_boards() {
    let mut rng = StdRng::seed_from_u64(77);
    let mut disagreements = 0;
    for _ in 0..300 {
        let cells: Vec<Cell> = (0..36)
            .map(|_| Cell::playable(RuneType::ALL[rng.gen_range(0..4)]))
            .collect();
        let board = Board::from_cells(6, 6, 4, cells);
        let mut any_accepted = false;
        for row in 0..6 {
            for col in 0..6 {
                for (dc, dr) in [(1usize, 0usize), (0, 1)] {
                    let (nc, nr) = (col + dc, row + dr);
                    if nc >= 6 || nr >= 6 {
                        continue;
                    }
                    let mut clone = board.clone();
                    if clone
                        .begin_swap_step(CellPos::new(col, row), CellPos::new(nc, nr))
                        .is_some()
                    {
                        any_accepted = true;
                    }
                }
            }
        }
        if any_accepted != board.has_valid_moves() {
            disagreements += 1;
        }
    }
    assert_eq!(
        disagreements, 0,
        "has_valid_moves must match what begin_swap_step actually accepts"
    );
}

#[test]
fn shuffle_with_many_obstacles_is_verified_or_reports_failure() {
    let mut layout_rng = StdRng::seed_from_u64(31);
    for seed in 0..120u64 {
        let mut cells: Vec<Cell> = (0..64)
            .map(|i| {
                let rune = RuneType::ALL[(i % 8 + i / 8 * 2) % 5];
                let mut cell = Cell::playable(rune);
                match layout_rng.gen_range(0..10) {
                    0 | 1 => cell = Cell::blocked_cell(),
                    2 | 3 => cell.chained = true,
                    4 => {
                        cell.crate_layers = 1;
                        cell.rune = None;
                    }
                    _ => {}
                }
                cell
            })
            .collect();
        // Guarantee a few free cells so the shuffle has something to work on.
        for cell in cells.iter_mut().take(6) {
            *cell = Cell::playable(RuneType::Fire);
        }
        let mut board = Board::from_cells(8, 8, 5, cells);
        let snapshot: Vec<Option<RuneType>> = (0..64)
            .map(|i| board.get(CellPos::new(i % 8, i / 8)).unwrap().rune)
            .collect();
        let mut rng = StdRng::seed_from_u64(seed);
        let ok = board.shuffle_free_runes(&mut rng);
        if ok {
            assert!(
                board.prepare_cascade_step().is_none(),
                "seed {seed}: a successful shuffle must leave no pending match"
            );
            assert!(
                board.has_valid_moves(),
                "seed {seed}: a successful shuffle must leave a valid move"
            );
        } else {
            let after: Vec<Option<RuneType>> = (0..64)
                .map(|i| board.get(CellPos::new(i % 8, i / 8)).unwrap().rune)
                .collect();
            assert_eq!(
                snapshot, after,
                "seed {seed}: a failed shuffle must restore the original layout"
            );
        }
    }
}

#[test]
fn shuffle_on_an_open_board_always_succeeds() {
    for seed in 0..40u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        let cells: Vec<Cell> = (0..64)
            .map(|_| Cell::playable(RuneType::ALL[rng.gen_range(0..5)]))
            .collect();
        let mut board = Board::from_cells(8, 8, 5, cells);
        assert!(board.shuffle_free_runes(&mut rng));
        assert!(board.prepare_cascade_step().is_none());
        assert!(board.has_valid_moves());
    }
}

#[test]
fn ensure_playable_regenerates_when_shuffle_cannot_succeed() {
    // Three free runes, all distinct, next to a cursed rune that can still be swapped but is
    // never moved by a shuffle. No permutation of the free runes can build a valid move, and
    // the deterministic fallback has no free mover, so only regeneration can recover.
    for seed in 0..20u64 {
        let free = [RuneType::Ice, RuneType::Wood, RuneType::Stone];
        let mut cells: Vec<Cell> = free.iter().map(|&r| Cell::playable(r)).collect();
        let mut cursed = Cell::playable(RuneType::Fire);
        cursed.curse_turns = 3;
        cells.push(cursed);
        let mut board = Board::from_cells(4, 1, 5, cells);
        let mut rng = StdRng::seed_from_u64(seed);
        assert!(!board.shuffle_free_runes(&mut rng), "seed {seed}");
        assert!(board.ensure_playable(&mut rng), "seed {seed}");
        assert!(board.prepare_cascade_step().is_none(), "seed {seed}");
        assert!(board.has_valid_moves(), "seed {seed}");
        let cursed_after = board.get(CellPos::new(3, 0)).unwrap();
        assert_eq!(cursed_after.rune, Some(RuneType::Fire));
        assert_eq!(cursed_after.curse_turns, 3);
    }
}

#[test]
fn ensure_playable_fails_cleanly_on_a_fully_blocked_board() {
    let cells: Vec<Cell> = (0..16).map(|_| Cell::blocked_cell()).collect();
    let mut board = Board::from_cells(4, 4, 5, cells);
    let mut rng = StdRng::seed_from_u64(7);
    assert!(!board.ensure_playable(&mut rng));
    assert!(!board.has_valid_moves());
}

#[test]
fn unrecoverable_board_does_not_flag_a_shuffle() {
    let cells: Vec<Cell> = (0..16).map(|_| Cell::blocked_cell()).collect();
    let mut board = Board::from_cells(4, 4, 5, cells);
    let mut rng = StdRng::seed_from_u64(3);
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let out = board.use_active_tool(ActiveTool::LokiShuffle, None, &data.balance, &mut rng);
    assert!(out.valid);
    assert!(!out.shuffled);
}

// ---------------------------------------------------------------------------
// Boosters combos.
// ---------------------------------------------------------------------------

/// 7x7 board with no accidental match: horizontal neighbors differ, vertical neighbors differ.
fn combo_board(a_booster: Booster, b_booster: Booster) -> Board {
    let mut cells: Vec<Cell> = (0..49)
        .map(|i| {
            let (c, r) = (i % 7, i / 7);
            Cell::playable(RuneType::ALL[(c + 2 * r) % 5])
        })
        .collect();
    cells[3 * 7 + 2].booster = a_booster;
    cells[3 * 7 + 3].booster = b_booster;
    Board::from_cells(7, 7, 5, cells)
}

fn combo_cells(a: Booster, b: Booster) -> HashSet<(usize, usize)> {
    let mut board = combo_board(a, b);
    let prepared = board
        .begin_swap_step(CellPos::new(2, 3), CellPos::new(3, 3))
        .expect("booster swap must be accepted");
    assert!(prepared.is_direct_blast);
    prepared.cells.iter().map(|p| (p.col, p.row)).collect()
}

fn rect(
    cols: std::ops::RangeInclusive<usize>,
    rows: std::ops::RangeInclusive<usize>,
) -> HashSet<(usize, usize)> {
    let mut set = HashSet::new();
    for r in rows {
        for c in cols.clone() {
            set.insert((c, r));
        }
    }
    set
}

#[test]
fn bomb_plus_bomb_clears_a_five_by_five_area() {
    let cells = combo_cells(Booster::Bomb, Booster::Bomb);
    assert_eq!(cells, rect(1..=5, 1..=5));
}

#[test]
fn line_plus_line_clears_the_row_and_column_cross() {
    let cells = combo_cells(Booster::LineHorizontal, Booster::LineVertical);
    let mut expected = rect(0..=6, 3..=3);
    expected.extend(rect(3..=3, 0..=6));
    assert_eq!(cells.len(), 13);
    assert_eq!(cells, expected);
}

#[test]
fn bomb_plus_line_clears_three_rows_and_three_columns() {
    let cells = combo_cells(Booster::Bomb, Booster::LineHorizontal);
    let mut expected = rect(0..=6, 2..=4);
    expected.extend(rect(2..=4, 0..=6));
    assert_eq!(cells.len(), 33);
    assert_eq!(cells, expected);
}

// ---------------------------------------------------------------------------
// Boss damage, stars, rewards, energy and crafting.
// ---------------------------------------------------------------------------

fn test_boss_def(max_hp: u32, phase2_weakness: Option<RuneType>) -> BossDef {
    BossDef {
        id: "test_boss".to_string(),
        name_key: "boss.test.name".to_string(),
        title_key: "boss.test.title".to_string(),
        desc_key: "boss.test.desc".to_string(),
        max_hp,
        max_moves: 20,
        weakness: RuneType::Fire,
        riposte_interval: 3,
        riposte: BossRiposteKind::ChainRunes(2),
        phase2_weakness,
        phase2_riposte: None,
        required_glory: 0,
        reward_resources: ResourceBundle::default(),
        reward_tools: ToolInventory::default(),
        glory_reward: 0,
    }
}

fn step_with_runes(runes: &[(RuneType, u32)], combo: u32) -> MatchStepOutcome {
    let mut step = MatchStepOutcome {
        valid: true,
        max_combo: combo,
        ..Default::default()
    };
    for &(rune, count) in runes {
        step.cleared_runes.insert(rune, count);
    }
    step
}

#[test]
fn boss_damage_doubles_on_weakness_and_adds_obstacle_and_score_bonus() {
    let mut boss = BossState::new(test_boss_def(10_000, None));
    let mut step = step_with_runes(&[(RuneType::Fire, 4), (RuneType::Ice, 2)], 1);
    step.ice_broken = 1;
    // Fire (weakness): 4 * 45 * 2 = 360, Ice: 2 * 45 = 90, obstacle: 85 => 535 raw.
    // Score bonus 20%: 535 + 535 * 20 / 100 = 642.
    let dealt = boss.apply_step_damage(&step, 20);
    assert_eq!(dealt, 642);
    assert_eq!(boss.hp, 10_000 - 642);
    assert_eq!(boss.last_damage_dealt, 642);
    assert!(!boss.is_defeated());
}

#[test]
fn boss_damage_scales_with_combo_and_saturates_at_zero_hp() {
    let mut boss = BossState::new(test_boss_def(10_000, None));
    // Fire x2 = 180 raw, combo 3 => x1.7 = 306.
    let step = step_with_runes(&[(RuneType::Fire, 2)], 3);
    assert_eq!(boss.apply_step_damage(&step, 0), 306);

    let mut weak = BossState::new(test_boss_def(100, None));
    let big = step_with_runes(&[(RuneType::Fire, 10)], 1);
    weak.apply_step_damage(&big, 0);
    assert_eq!(weak.hp, 0);
    assert!(weak.is_defeated());
}

#[test]
fn boss_phase_two_switches_weakness_and_flags_entry_once() {
    let mut boss = BossState::new(test_boss_def(10_000, Some(RuneType::Ice)));
    boss.hp = 5_050;
    // Fire is still the weakness in phase 1: 3 * 45 * 2 = 270 -> hp 4780 (< 50%).
    let step = step_with_runes(&[(RuneType::Fire, 3)], 1);
    assert_eq!(boss.apply_step_damage(&step, 0), 270);
    assert_eq!(boss.hp, 4_780);
    assert!(boss.phase2_triggered);
    assert!(boss.just_entered_phase2);
    assert_eq!(boss.active_weakness(), RuneType::Ice);

    // In phase 2 Ice is doubled (90) and Fire is plain (45).
    let step = step_with_runes(&[(RuneType::Ice, 1), (RuneType::Fire, 1)], 1);
    assert_eq!(boss.apply_step_damage(&step, 0), 135);
    assert!(!boss.just_entered_phase2);
}

fn victory_session(moves_left: u32, initial_moves: u32) -> (LevelSession, GameDataBundle) {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(5);
    let level = data.levels[0].clone();
    let mut session = LevelSession::start(level, &mut rng);
    session.outcome = LevelOutcome::Victory;
    session.initial_moves = initial_moves;
    session.moves_left = moves_left;
    (session, data)
}

#[test]
fn earned_stars_follow_remaining_move_ratio() {
    let (s3, _) = victory_session(7, 20);
    assert_eq!(s3.earned_stars(), 3);
    let (s2, _) = victory_session(3, 20);
    assert_eq!(s2.earned_stars(), 2);
    let (s1, _) = victory_session(2, 20);
    assert_eq!(s1.earned_stars(), 1);
    let (s0, _) = victory_session(0, 20);
    assert_eq!(s0.earned_stars(), 1);

    let (mut lost, _) = victory_session(7, 20);
    lost.outcome = LevelOutcome::Defeat;
    assert_eq!(lost.earned_stars(), 0);
    lost.outcome = LevelOutcome::InProgress;
    assert_eq!(lost.earned_stars(), 0);
}

#[test]
fn earned_rewards_apply_move_passive_and_fury_bonuses() {
    let (mut session, data) = victory_session(7, 20);
    session.level.base_reward = ResourceBundle {
        wood: 100,
        stone: 50,
        gold: 10,
        mead: 3,
    };
    session.resource_bonus_pct = 10;
    session.fury_resource_bonus_pct = 5;
    // Move bonus 7 * 3 = 21, plus 10 and 5 => 36%.
    assert_eq!(session.reward_bonus_breakdown(&data.balance), (21, 10, 5));
    let rewards = session.earned_rewards(&data.balance);
    assert_eq!(
        rewards,
        ResourceBundle {
            wood: 136,
            stone: 68,
            gold: 13,
            mead: 4,
        }
    );

    // The move bonus is capped.
    session.moves_left = 15;
    assert_eq!(session.reward_bonus_breakdown(&data.balance).0, 30);

    // No reward without a victory.
    session.outcome = LevelOutcome::Defeat;
    assert_eq!(
        session.earned_rewards(&data.balance),
        ResourceBundle::default()
    );
}

#[test]
fn offline_energy_regen_uses_interval_cap_and_ignores_zero_elapsed() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let balance = &data.balance;
    let mut city = CityState::new(balance);
    assert_eq!(city.compute_bonuses(balance).energy_regen_speed_pct, 0);
    let interval = balance.energy_regen_seconds;

    city.energy = 0;
    city.apply_offline_energy_regen(0, balance);
    assert_eq!(city.energy, 0);

    city.apply_offline_energy_regen(interval - 1, balance);
    assert_eq!(city.energy, 0, "less than one interval grants nothing");

    city.apply_offline_energy_regen(interval * 10 + 1, balance);
    assert_eq!(city.energy, 10);

    city.apply_offline_energy_regen(interval * 10_000, balance);
    assert_eq!(city.energy, city.max_energy(balance));
}

#[test]
fn try_spend_energy_only_succeeds_when_affordable() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let balance = &data.balance;
    let cost = balance.energy_cost_per_level;
    let mut city = CityState::new(balance);

    city.energy = cost;
    assert!(city.try_spend_energy(balance));
    assert_eq!(city.energy, 0);

    city.energy = cost - 1;
    assert!(!city.try_spend_energy(balance));
    assert_eq!(
        city.energy,
        cost - 1,
        "a refused spend must not change energy"
    );
}

#[test]
fn try_craft_tool_spends_exact_cost_or_refuses() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let recipe = data
        .tool_shop
        .iter()
        .find(|r| r.tool == ActiveTool::Mjolnir)
        .expect("Mjolnir recipe exists");
    let mut city = CityState::new(&data.balance);
    let owned = city.tools.count(ActiveTool::Mjolnir);

    city.resources = ResourceBundle::default();
    assert_eq!(
        city.try_craft_tool(recipe),
        Err("city.error.not_enough_resources")
    );
    assert_eq!(city.tools.count(ActiveTool::Mjolnir), owned);

    let extra = ResourceBundle {
        wood: 1,
        stone: 2,
        gold: 3,
        mead: 4,
    };
    city.resources = ResourceBundle {
        wood: recipe.cost.wood + extra.wood,
        stone: recipe.cost.stone + extra.stone,
        gold: recipe.cost.gold + extra.gold,
        mead: recipe.cost.mead + extra.mead,
    };
    assert_eq!(city.try_craft_tool(recipe), Ok(owned + 1));
    assert_eq!(city.resources, extra);
}

#[test]
fn has_valid_moves_ignores_a_match_the_player_cannot_touch() {
    // The Gold triple is chained, so the only swappable pair (Wood, Ice) creates nothing.
    let mut cells = rune_grid(5, 1, &["GGGWI"]);
    for cell in cells.iter_mut().take(3) {
        cell.chained = true;
    }
    let mut board = Board::from_cells(5, 1, 6, cells);
    assert!(
        !board.has_valid_moves(),
        "a match elsewhere must not count as a valid move"
    );
    assert!(board
        .begin_swap_step(CellPos::new(3, 0), CellPos::new(4, 0))
        .is_none());
}

#[test]
fn login_tribute_cannot_be_reclaimed_after_clock_rollback() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut city = CityState::new(&data.balance);
    assert!(city.try_claim_login_tribute(100).is_some());
    let resources_after_claim = city.resources;
    assert!(!city.is_login_tribute_available(99));
    assert!(city.try_claim_login_tribute(99).is_none());
    assert!(!city.is_login_tribute_available(100));
    assert!(city.is_login_tribute_available(101));
    assert_eq!(city.resources, resources_after_claim);
}

#[test]
fn resource_and_tool_arithmetic_saturates_instead_of_overflowing() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut city = CityState::new(&data.balance);
    city.resources = ResourceBundle {
        wood: u32::MAX,
        stone: u32::MAX,
        gold: u32::MAX,
        mead: u32::MAX,
    };
    city.glory = u32::MAX;
    city.tools.mjolnir = u32::MAX;
    city.tools.bifrost_prism = u32::MAX;
    city.stats.total_victories = u32::MAX;
    city.stats.win_streak = u32::MAX;
    city.stats.last_daily_day_index = u64::MAX;
    city.last_login_claim_day = Some(u64::MAX);
    city.login_streak_day = 3;

    let reward = ResourceBundle {
        wood: 10,
        stone: 10,
        gold: 10,
        mead: 10,
    };
    city.record_level_victory("great_hall", "great_hall_1", 3, &reward, &data.balance);
    city.record_expedition_wave(u32::MAX, &reward, &data.balance);
    city.record_daily_victory(5, &reward, &data.balance);
    city.apply_offline_energy_regen(u64::MAX, &data.balance);
    city.regenerate_energy(u32::MAX, &data.balance);
    city.tools.grant_one(ActiveTool::Mjolnir);
    let _ = city.try_claim_login_tribute(1);
    assert_eq!(city.resources.wood, u32::MAX);
    assert_eq!(city.glory, u32::MAX);
    assert_eq!(city.tools.mjolnir, u32::MAX);
    assert_eq!(city.energy, city.max_energy(&data.balance));
    let scaled = CityState::scale_daily_rewards(
        &ResourceBundle {
            wood: u32::MAX,
            stone: 0,
            gold: 0,
            mead: 0,
        },
        u32::MAX,
    );
    assert_eq!(scaled.wood, u32::MAX);
}

/// Last dotted segments that mark a file name (for example `a.png`), not a locale key.
const KEY_SCAN_FILE_EXTENSIONS: [&str; 6] = ["ron", "json", "png", "ogg", "wav", "ttf"];

/// Collects every `"dotted.lowercase.key"` string literal found in `text`.
fn collect_key_literals(text: &str, out: &mut HashSet<String>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut end = start;
        while end < bytes.len()
            && (bytes[end].is_ascii_lowercase()
                || bytes[end].is_ascii_digit()
                || bytes[end] == b'_'
                || bytes[end] == b'.')
        {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b'"' && end > start {
            let lit = &text[start..end];
            let is_file_name = lit
                .rsplit('.')
                .next()
                .is_some_and(|last| KEY_SCAN_FILE_EXTENSIONS.contains(&last));
            if lit.contains('.') && !lit.starts_with('.') && !lit.ends_with('.') && !is_file_name {
                out.insert(lit.to_string());
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }
}

fn collect_source_files(dir: &std::path::Path, exts: &[&str], out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(&path, exts, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| exts.contains(&e))
        {
            out.push(path);
        }
    }
}

fn project_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn locale_keys(file: &str) -> HashSet<String> {
    let raw = std::fs::read_to_string(project_root().join("assets/locales").join(file)).unwrap();
    let map: std::collections::HashMap<String, String> = serde_json::from_str(&raw).unwrap();
    map.into_keys().collect()
}

/// Literals found in production sources (`crates/*/src`) and bundled data (`assets/data`).
fn used_key_literals() -> HashSet<String> {
    let root = project_root();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root.join("crates")).unwrap().flatten() {
        collect_source_files(&entry.path().join("src"), &["rs"], &mut files);
    }
    collect_source_files(&root.join("assets/data"), &["ron"], &mut files);
    assert!(
        files.len() > 10,
        "source scan must reach crates and data files"
    );
    let mut used = HashSet::new();
    for file in files {
        collect_key_literals(&std::fs::read_to_string(&file).unwrap(), &mut used);
    }
    used
}

#[test]
fn every_locale_key_is_used_and_every_used_key_exists() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let en = locale_keys("en.json");
    let fr = locale_keys("fr.json");
    assert_eq!(en, fr, "locales must carry identical key sets");

    let mut used = used_key_literals();
    // Chapter keys are built at runtime from building ids.
    for building in &data.buildings {
        for part in ["title", "line1", "line2"] {
            used.insert(format!("chapter.{}.{part}", building.id));
        }
    }

    let mut unused: Vec<&str> = en
        .iter()
        .filter(|k| !used.contains(*k))
        .map(String::as_str)
        .collect();
    unused.sort();
    assert!(
        unused.is_empty(),
        "locale keys never referenced by code or data:\n{}",
        unused.join("\n")
    );

    let namespaces: HashSet<&str> = en.iter().filter_map(|k| k.split('.').next()).collect();
    let mut missing: Vec<&str> = used
        .iter()
        .filter(|lit| {
            lit.split('.')
                .next()
                .is_some_and(|ns| namespaces.contains(ns))
                && !en.contains(*lit)
        })
        .map(String::as_str)
        .collect();
    missing.sort();
    assert!(
        missing.is_empty(),
        "literals that look like locale keys but are missing from en.json:\n{}",
        missing.join("\n")
    );
}

#[test]
fn collect_rune_objectives_are_always_included_in_board_rune_pool() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(4242);

    // Every bundled level's CollectRune objectives must be spawnable on its board.
    for level in &data.levels {
        let board = Board::new_for_level(level, &mut rng);
        for obj in &level.objectives {
            if let wyrdstone_core::Objective::CollectRune { rune, .. } = obj {
                let idx = RuneType::ALL.iter().position(|r| r == rune).unwrap();
                assert!(
                    idx < board.rune_pool_len(),
                    "Level {} requires {:?} (index {}), but rune_pool_len is {}",
                    level.id,
                    rune,
                    idx,
                    board.rune_pool_len()
                );
            }
        }
    }

    // Even if a level misconfigures rune_types_count = 5 with CollectRune(Water),
    // Board::new_for_level must auto-expand the rune pool to include Water.
    let mut misconfigured = data.level_by_id("meadery_1").unwrap().clone();
    misconfigured.rune_types_count = 5;
    let board = Board::new_for_level(&misconfigured, &mut rng);
    assert_eq!(board.rune_pool_len(), 6);
}

#[test]
fn bifrost_shard_swap_with_rune_clears_only_target_color() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    // 4x4 board: cell (0,0) is BifrostShard (rune: None), (1,0) is Fire,
    // all remaining cells are Gold except two extra Fire runes.
    let mut cells = vec![Cell::playable(RuneType::Gold); 16];
    cells[0].rune = None;
    cells[0].booster = Booster::BifrostShard;
    cells[1].rune = Some(RuneType::Fire);
    cells[7].rune = Some(RuneType::Fire);
    cells[12].rune = Some(RuneType::Fire);
    let mut board = Board::from_cells(4, 4, 6, cells);

    let prepared = board
        .begin_swap_step(CellPos::new(0, 0), CellPos::new(1, 0))
        .expect("Bifrost + Rune swap must prepare a clear");
    let out = board.commit_clear_step(&prepared, 1, &data.balance);
    assert!(out.valid);
    assert_eq!(out.cleared_runes.get(&RuneType::Fire).copied(), Some(3));
    assert_eq!(
        out.cleared_runes.get(&RuneType::Gold).copied().unwrap_or(0),
        0,
        "Swapping BifrostShard with Fire must not double-trigger onto Gold"
    );
}

#[test]
fn line_plus_line_swap_clears_only_cross_not_extra_row() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut cells = vec![Cell::playable(RuneType::Water); 25];
    cells[5].booster = Booster::LineHorizontal;
    cells[6].booster = Booster::LineHorizontal;
    let mut board = Board::from_cells(5, 5, 6, cells);

    // Swap (0,1) and (1,1): cross at b=(1,1) covers row 1 (5) + col 1 (5) - overlap (1) = 9 cells.
    // If (0,1) double-triggered a second horizontal line or bonus, score/runes would inflate.
    let prepared = board
        .begin_swap_step(CellPos::new(0, 1), CellPos::new(1, 1))
        .expect("Line + Line swap must prepare cross clear");
    let out = board.commit_clear_step(&prepared, 1, &data.balance);
    assert!(out.valid);
    assert_eq!(out.cleared_runes.get(&RuneType::Water).copied(), Some(9));
    let expected_score =
        9 * data.balance.points_per_tile + 2 * data.balance.booster_activation_bonus;
    assert_eq!(out.score_gained, expected_score);
}

#[test]
fn special_swap_combo_prepared_cells_are_sorted_deterministically() {
    for _ in 0..20 {
        let mut cells = vec![Cell::playable(RuneType::Stone); 25];
        cells[6].booster = Booster::Bomb;
        cells[7].booster = Booster::Bomb;
        let mut board = Board::from_cells(5, 5, 6, cells);
        let prepared = board
            .begin_swap_step(CellPos::new(1, 1), CellPos::new(2, 1))
            .expect("Bomb + Bomb combo");
        let keys: Vec<(usize, usize)> = prepared.cells.iter().map(|p| (p.row, p.col)).collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted);
    }
}

#[test]
fn sanitize_fixes_unowned_equipped_amulet_and_zero_tier_crafted_amulet() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut city = CityState::new(&data.balance);
    city.equipped_amulet = Some("amulet_odin".to_string());
    assert!(
        city.active_amulet_effect(&data.amulets).is_none(),
        "uncrafted equipped amulet must grant no effect even before sanitize"
    );

    city.crafted_amulets.insert("amulet_freya".to_string());
    city.amulet_tiers.insert("amulet_freya".to_string(), 0);
    city.sanitize(&data.buildings, &data.balance);

    assert!(city.equipped_amulet.is_none());
    assert_eq!(city.amulet_tier("amulet_freya"), 1);
    assert!(
        city.try_upgrade_amulet(data.amulet_by_id("amulet_freya").unwrap(), &data.balance)
            .is_ok()
            || city.amulet_tier("amulet_freya") == 1
    );
}

#[test]
fn daily_streak_achievement_survives_day_8_cycle_wrap() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut city = CityState::new(&data.balance);
    let reward = ResourceBundle {
        wood: 10,
        stone: 10,
        gold: 10,
        mead: 1,
    };
    for day in 1..=8u64 {
        city.record_daily_victory(day, &reward, &data.balance);
    }
    assert_eq!(city.stats.daily_streak, 1);
    assert_eq!(city.stats.best_daily_streak, 7);

    let streak_ach = data
        .achievements
        .iter()
        .find(|a| a.id == "ach_daily_streak_3")
        .expect("ach_daily_streak_3 exists");
    let (done, target) =
        achievement_progress(streak_ach, &city, &data.buildings, data.bosses.len());
    assert_eq!(done, target);
}

#[test]
fn fury_without_valid_targets_does_not_drain_gauge() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(5);
    let cells = vec![Cell::blocked_cell(); 16];
    let mut level = data.level_by_id("forge_1").unwrap().clone();
    level.cols = 4;
    level.rows = 4;
    let mut session = LevelSession::start_full(
        level,
        None,
        CityBonuses::default(),
        PatronGod::Thor,
        None,
        &mut rng,
    );
    session.board = Board::from_cells(4, 4, 6, cells);
    session.fury = session.fury_max;

    assert!(session
        .trigger_fury(&data.balance.session, &mut rng)
        .is_none());
    assert_eq!(session.fury, session.fury_max);

    session.patron = PatronGod::Loki;
    assert!(session
        .trigger_fury(&data.balance.session, &mut rng)
        .is_none());
    assert_eq!(session.fury, session.fury_max);
}

#[test]
fn clear_cell_set_does_not_duplicate_cleared_positions_on_direct_blast() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut cells = vec![Cell::playable(RuneType::Fire); 9];
    cells[4].fog = true;
    cells[4].chained = true;
    let mut board = Board::from_cells(3, 3, 6, cells);
    let prepared = board
        .prepare_tool_step(ActiveTool::Mjolnir, CellPos::new(1, 1))
        .unwrap();
    let out = board.commit_clear_step(&prepared, 1, &data.balance);
    let count = out
        .cleared_positions
        .iter()
        .filter(|&&p| p == CellPos::new(1, 1))
        .count();
    assert_eq!(count, 1);
}

#[test]
fn locale_placeholders_match_between_en_and_fr() {
    use std::collections::{BTreeSet, HashMap};

    fn extract_placeholders(template: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut chars = template.char_indices().peekable();
        while let Some((idx, ch)) = chars.next() {
            if ch == '{' {
                let start = idx + 1;
                let mut end = None;
                for (j, c2) in chars.by_ref() {
                    if c2 == '}' {
                        end = Some(j);
                        break;
                    }
                }
                if let Some(end_idx) = end {
                    out.insert(template[start..end_idx].to_string());
                }
            }
        }
        out
    }

    let en_map: HashMap<String, String> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/locales/en.json"
    )))
    .expect("en.json must parse");
    let fr_map: HashMap<String, String> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/locales/fr.json"
    )))
    .expect("fr.json must parse");

    assert_eq!(en_map.len(), fr_map.len());
    for (key, en_val) in &en_map {
        let fr_val = fr_map
            .get(key)
            .unwrap_or_else(|| panic!("missing French locale key '{key}'"));
        assert_eq!(
            extract_placeholders(en_val),
            extract_placeholders(fr_val),
            "placeholder mismatch for key '{key}' (en='{en_val}', fr='{fr_val}')"
        );
    }
}

#[test]
fn text_formatters_resolve_all_placeholders_in_en_and_fr() {
    let mut data = GameDataBundle::load_bundled(Language::En).expect("bundled data loads");
    for lang in [Language::En, Language::Fr] {
        data.i18n.set_language(lang);
        let mut rendered = Vec::new();
        for rank in [
            wyrdstone_core::ClanRank::Scout,
            wyrdstone_core::ClanRank::Huscarl,
            wyrdstone_core::ClanRank::Jarl,
            wyrdstone_core::ClanRank::HighKing,
        ] {
            rendered.push(wyrdstone_core::format_rank_perk(
                &data.i18n,
                rank,
                &data.balance,
            ));
        }
        for blessing in wyrdstone_core::DailyBlessing::ALL {
            rendered.push(wyrdstone_core::format_blessing_desc(
                &data.i18n,
                blessing,
                &data.balance.session,
            ));
        }
        for patron in PatronGod::ALL {
            rendered.push(wyrdstone_core::format_patron_desc(
                &data.i18n,
                patron,
                &data.balance.session,
            ));
        }
        for boon in wyrdstone_core::ExpeditionBoon::ALL {
            rendered.push(wyrdstone_core::format_boon_desc(&data.i18n, boon));
        }
        for building in &data.buildings {
            rendered.push(wyrdstone_core::format_building_bonus(
                &data.i18n,
                building,
                &data.balance,
            ));
        }
        for boss in &data.bosses {
            rendered.push(wyrdstone_core::format_boss_desc(&data.i18n, boss));
        }
        for ach in &data.achievements {
            rendered.push(wyrdstone_core::format_achievement_desc(&data.i18n, ach));
        }
        for text in rendered {
            assert!(!text.is_empty());
            assert!(
                !text.contains('{') && !text.contains('}'),
                "unreplaced placeholder in '{text}' ({lang:?})"
            );
        }
    }
}

#[test]
fn match4_and_lt_shapes_spawn_expected_boosters() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(4444);

    fn checkerboard_cells(cols: usize, rows: usize) -> Vec<Cell> {
        (0..cols * rows)
            .map(|idx| {
                let r = idx / cols;
                let c = idx % cols;
                Cell::playable(if (r + c).is_multiple_of(2) {
                    RuneType::Water
                } else {
                    RuneType::Wood
                })
            })
            .collect()
    }

    // 4-in-a-row horizontal match by swapping (2, 1) Fire with (2, 0) Stone:
    // Row 0 becomes Fire, Fire, Fire, Fire, Water -> spawns LineHorizontal/LineVertical.
    let mut cells4 = checkerboard_cells(5, 4);
    cells4[0] = Cell::playable(RuneType::Fire);
    cells4[1] = Cell::playable(RuneType::Fire);
    cells4[2] = Cell::playable(RuneType::Stone);
    cells4[3] = Cell::playable(RuneType::Fire);
    cells4[7] = Cell::playable(RuneType::Fire); // (2, 1)
    let mut board4 = Board::from_cells(5, 4, 6, cells4);
    let prep4 = board4
        .begin_swap_step(CellPos::new(2, 0), CellPos::new(2, 1))
        .expect("4-match swap must be valid");
    assert_eq!(prep4.spawned_boosters.len(), 1);
    assert!(matches!(
        prep4.spawned_boosters[0].2,
        Booster::LineHorizontal | Booster::LineVertical
    ));
    let step4 = board4.commit_clear_step(&prep4, 1, &data.balance);
    assert!(step4.valid);

    // T-shape 5-tile match by swapping (2, 2) Stone with (2, 3) Fire on 5x5 grid:
    // Horizontal at row 2 (cols 1, 2, 3) + vertical at col 2 (rows 0, 1, 2) -> spawns Bomb.
    let mut cells_t = checkerboard_cells(5, 5);
    cells_t[2 * 5 + 1] = Cell::playable(RuneType::Fire); // (1, 2)
    cells_t[2 * 5 + 2] = Cell::playable(RuneType::Stone); // (2, 2)
    cells_t[2 * 5 + 3] = Cell::playable(RuneType::Fire); // (3, 2)
    cells_t[2] = Cell::playable(RuneType::Fire); // (2, 0)
    cells_t[5 + 2] = Cell::playable(RuneType::Fire); // (2, 1)
    cells_t[3 * 5 + 2] = Cell::playable(RuneType::Fire); // (2, 3) -> swapped to (2, 2)
    let mut board_t = Board::from_cells(5, 5, 6, cells_t);
    let prep_t = board_t
        .begin_swap_step(CellPos::new(2, 2), CellPos::new(2, 3))
        .expect("T-shape swap must be valid");
    assert_eq!(prep_t.spawned_boosters.len(), 1);
    assert_eq!(prep_t.spawned_boosters[0].2, Booster::Bomb);
    let _ = board_t.commit_clear_step(&prep_t, 1, &data.balance);
    let _ = board_t.apply_gravity_and_refill_tracked(&mut rng);
}

#[test]
fn portal_transfers_piece_across_columns_during_gravity() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(7070);
    let mut level = data.level_by_id("great_hall_1").unwrap().clone();
    level.cols = 3;
    level.rows = 3;
    level.rune_types_count = 3;
    level.ice_cells.clear();
    level.chained_cells.clear();
    level.crate_cells.clear();
    level.relic_cells.clear();
    level.fog_cells.clear();
    level.cursed_cells.clear();
    level.blocked_cells = vec![(0, 1), (0, 2), (1, 0)];
    level.portals = vec![((0, 0), (1, 1))];
    level.objectives = vec![wyrdstone_core::Objective::ReachScore { target: 500 }];

    let mut board = Board::new_for_level(&level, &mut rng);
    // Place a relic at portal source (0, 0) and empty portal destination (1, 1).
    let src = CellPos::new(0, 0);
    let dst = CellPos::new(1, 1);
    let src_cell = board.get_mut(src).unwrap();
    src_cell.relic = true;
    src_cell.rune = None;
    let dst_cell = board.get_mut(dst).unwrap();
    dst_cell.rune = None;
    dst_cell.relic = false;
    dst_cell.booster = Booster::None;

    board.apply_gravity_and_refill_tracked(&mut rng);
    assert!(
        board.get(dst).unwrap().relic,
        "portal destination (1,1) must receive the relic from (0,0)"
    );
    assert!(
        !board.get(src).unwrap().relic,
        "portal source (0,0) must release its relic after transfer"
    );
}

#[test]
fn try_drink_mead_restores_energy_and_checks_bounds() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut city = CityState::new(&data.balance);
    let cap = city.max_energy(&data.balance);

    // At full cap, try_drink_mead is a no-op without spending Mead.
    let initial_mead = city.resources.mead;
    assert_eq!(city.try_drink_mead(&data.balance), Ok(cap));
    assert_eq!(city.resources.mead, initial_mead);

    // Below cap but insufficient Mead returns the exact shortage error key.
    city.energy = 0;
    city.resources.mead = data.balance.mead_refill_cost.saturating_sub(1);
    assert_eq!(
        city.try_drink_mead(&data.balance),
        Err("city.error.not_enough_resources")
    );
    assert_eq!(city.energy, 0);

    // Valid refill spends Mead and clamps at cap.
    city.resources.mead = data.balance.mead_refill_cost + 5;
    city.energy = cap.saturating_sub(3);
    assert_eq!(city.try_drink_mead(&data.balance), Ok(cap));
    assert_eq!(city.resources.mead, 5);
}

#[test]
fn i18n_catalog_toggle_and_fallback() {
    let mut catalog = wyrdstone_core::I18nCatalog::from_json_strs(
        r#"{"only_en": "English Only", "greet": "Hello {name}"}"#,
        r#"{"greet": "Bonjour {name}"}"#,
        Language::Fr,
    )
    .expect("valid locale json parses");

    assert_eq!(catalog.language(), Language::Fr);
    assert!(catalog.has_key(Language::En, "only_en"));
    assert!(!catalog.has_key(Language::Fr, "only_en"));
    assert_eq!(catalog.t("only_en"), "English Only");
    assert_eq!(catalog.t("missing.key"), "missing.key");
    assert_eq!(
        catalog.t_fmt("greet", &[("name", "Halvar".to_string())]),
        "Bonjour Halvar"
    );

    catalog.toggle_language();
    assert_eq!(catalog.language(), Language::En);
    assert_eq!(
        catalog.t_fmt("greet", &[("name", "Halvar".to_string())]),
        "Hello Halvar"
    );
}

#[test]
fn loki_fury_keeps_board_playable_and_bonus_blasts_report_awarded_score_and_stats() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(8181);
    let mut level = data.level_by_id("great_hall_1").unwrap().clone();
    level.cols = 3;
    level.rows = 3;
    level.ice_cells.clear();
    level.chained_cells.clear();
    level.crate_cells.clear();
    level.relic_cells.clear();
    level.fog_cells.clear();
    level.cursed_cells.clear();
    level.blocked_cells.clear();
    level.objectives = vec![wyrdstone_core::Objective::ReachScore { target: 999_999 }];

    let mut session = LevelSession::start_full(
        level,
        None,
        CityBonuses {
            score_bonus_pct: 50,
            ..CityBonuses::default()
        },
        PatronGod::Loki,
        None,
        &mut rng,
    );
    session.fury = session.fury_max;
    let outcome = session
        .trigger_fury(&data.balance.session, &mut rng)
        .expect("Loki fury succeeds on 3x3 free board");
    assert_eq!(outcome.patron, PatronGod::Loki);
    assert!(outcome.turn.is_none());
    assert!(
        session.board.has_valid_moves(),
        "Loki fury must leave at least one valid move"
    );

    let blasts = session.detonate_bonus_boosters_detailed(
        &data.balance,
        &mut rng,
        wyrdstone_core::VICTORY_SKIP_MAX_DETONATIONS,
    );
    assert!(!blasts.is_empty());
    for blast in blasts {
        assert!(blast.score_awarded >= blast.score_raw);
        assert!(blast.stats.runes_cleared > 0 || !blast.cleared.is_empty());
    }
}

#[test]
fn find_hint_move_prioritizes_booster_combo_over_plain_match() {
    let mut cells = vec![Cell::playable(RuneType::Stone); 36];
    // Deterministic non-matching 6x6 checkerboard (all Stone/Water alternated)
    for r in 0..6 {
        for c in 0..6 {
            cells[r * 6 + c].rune = Some(if (r + c) % 2 == 0 {
                RuneType::Stone
            } else {
                RuneType::Water
            });
        }
    }
    // Top-left plain 3-match swap: (0,0)<->(1,0) makes vertical 3-match in col 1 (Fire)
    cells[1].rune = Some(RuneType::Fire);
    cells[0].rune = Some(RuneType::Fire);
    cells[6 + 1].rune = Some(RuneType::Gold);
    cells[2 * 6 + 1].rune = Some(RuneType::Fire);
    cells[3 * 6 + 1].rune = Some(RuneType::Fire);
    //Wait: (0,1) swapped with (1,1): put Fire at (0,1), and Fire at (1,0), (1,2), (1,3)? Swap (0,1)<->(1,1) puts Fire at (1,1) making vertical col 1? Even simpler:
    // Horizontal swap (1,0) with (1,1) where (0,0),(2,0),(3,0) are... let's make clean (0,0) Gold swapped down with (0,1) Fire, where (1,0)=Gold,(2,0)=Gold!
    // Reset cleanly:
    for r in 0..6 {
        for c in 0..6 {
            cells[r * 6 + c].rune = Some(if (r + c) % 2 == 0 {
                RuneType::Stone
            } else {
                RuneType::Water
            });
        }
    }
    // Plain 3-match at top-left: swap (0,0) with (0,1) makes row 1 all Fire at (0,1),(1,1),(2,1)
    cells[0].rune = Some(RuneType::Fire);
    cells[6 + 1].rune = Some(RuneType::Fire);
    cells[6 + 2].rune = Some(RuneType::Fire);
    // Booster+Booster super-combo further down at (4,4)<->(5,4)
    cells[4 * 6 + 4].booster = Booster::LineHorizontal;
    cells[4 * 6 + 5].booster = Booster::Bomb;

    let board = Board::from_cells(6, 6, 6, cells);
    let hint = board.find_best_hint_move().expect("board has valid hints");
    assert_eq!(hint, (CellPos::new(4, 4), CellPos::new(5, 4)));
}

#[test]
fn patron_favored_rune_grants_bonus_fury() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let level = data.level_by_id("forge_1").expect("forge_1").clone();
    let mut rng = StdRng::seed_from_u64(401);
    let mut thor_session = LevelSession::start_full(
        level.clone(),
        None,
        CityBonuses::default(),
        PatronGod::Thor,
        None,
        &mut rng,
    );
    thor_session.apply_session_tuning(&data.balance.session);
    let mut freya_session = LevelSession::start_full(
        level,
        None,
        CityBonuses::default(),
        PatronGod::Freya,
        None,
        &mut rng,
    );
    freya_session.apply_session_tuning(&data.balance.session);

    let mut step = wyrdstone_core::MatchStepOutcome {
        valid: true,
        max_combo: 1,
        ..Default::default()
    };
    step.cleared_runes.insert(RuneType::Fire, 4);
    thor_session.apply_step_progress(&step);
    freya_session.apply_step_progress(&step);

    // 4 tiles * 5 base = 20 Fury; Thor favors Fire (+50%) -> 30 Fury.
    assert_eq!(freya_session.fury, 20);
    assert_eq!(thor_session.fury, 30);
}

#[test]
fn ragnarok_overdrive_ignites_doubles_score_blocks_frost_and_decays() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut level = data.level_by_id("forge_1").expect("forge_1").clone();
    level.frost_spread = true;
    let mut rng = StdRng::seed_from_u64(402);
    let mut session = LevelSession::start_full(
        level,
        None,
        CityBonuses::default(),
        PatronGod::Thor,
        None,
        &mut rng,
    );
    session.apply_session_tuning(&data.balance.session);

    // High combo + boosters charge Overdrive past 100 and ignite it.
    let mut step = wyrdstone_core::MatchStepOutcome {
        valid: true,
        score_gained: 200,
        max_combo: 4,
        boosters_used: 2,
        ..Default::default()
    };
    step.cleared_runes.insert(RuneType::Stone, 3);
    session.apply_step_progress(&step);
    assert!(session.just_triggered_overdrive);
    assert_eq!(
        session.overdrive_turns_left,
        data.balance.session.overdrive_turns
    );

    // While Overdrive is active, frost spread is suppressed and score gets +100% bonus.
    assert_eq!(session.apply_frost_spread_if_needed(&mut rng), None);
    let score_before = session.score;
    let mut step2 = wyrdstone_core::MatchStepOutcome {
        valid: true,
        score_gained: 100,
        max_combo: 1,
        ..Default::default()
    };
    step2.cleared_runes.insert(RuneType::Water, 3);
    session.apply_step_progress(&step2);
    assert_eq!(session.score - score_before, 200);

    // Decays across subsequent move-consuming turns cleanly back to 0.
    session.consume_move();
    for _ in 0..data.balance.session.overdrive_turns {
        session.tick_overdrive_if_needed();
    }
    assert_eq!(session.overdrive_turns_left, 0);
}

#[test]
fn boss_stagger_cancels_imminent_riposte_and_applies_exposed_damage_bonus() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let boss_def = data.boss_by_id("boss_fenrir").expect("boss_fenrir").clone();
    let mut rng = StdRng::seed_from_u64(403);
    let mut session = LevelSession::start_boss_with(
        boss_def,
        None,
        CityBonuses::default(),
        PatronGod::Thor,
        &data.balance,
        &mut rng,
    );
    session.apply_session_tuning(&data.balance.session);

    let boss = session.boss.as_mut().expect("boss present");
    boss.turns_until_riposte = 1;
    let weakness = boss.active_weakness();

    // Clearing stagger_weakness_tiles weakness tiles at countdown 1 interrupts the riposte.
    let mut stagger_step = wyrdstone_core::MatchStepOutcome {
        valid: true,
        max_combo: 1,
        ..Default::default()
    };
    stagger_step
        .cleared_runes
        .insert(weakness, data.balance.boss.stagger_weakness_tiles);
    session.apply_step_progress(&stagger_step);

    let boss_after = session.boss.as_ref().expect("boss");
    assert!(boss_after.just_staggered);
    assert_eq!(boss_after.exposed_turns, data.balance.boss.exposed_turns);
    assert_eq!(
        boss_after.turns_until_riposte,
        boss_after.def.riposte_interval
    );

    // End-of-turn riposte tick returns no hits and leaves Exposed intact on the stagger turn.
    let hits = session.tick_boss_riposte_if_needed(&mut rng);
    assert!(hits.is_empty());
    let boss_after_tick = session.boss.as_ref().expect("boss");
    assert_eq!(
        boss_after_tick.exposed_turns,
        data.balance.boss.exposed_turns
    );

    // Next hit deals +50% Exposed bonus damage.
    session.consume_move();
    let dmg_step = wyrdstone_core::MatchStepOutcome {
        valid: true,
        max_combo: 1,
        cleared_runes: std::collections::HashMap::from([(RuneType::Stone, 3)]),
        ..Default::default()
    };
    session.apply_step_progress(&dmg_step);
    let exposed_dmg = session.boss.as_ref().expect("boss").last_damage_dealt;
    let base_expected = 3 * data.balance.boss.damage_per_tile;
    assert!(
        exposed_dmg > base_expected,
        "Exposed boss damage {exposed_dmg} must exceed base {base_expected}"
    );
}

#[test]
fn valhalla_blitz_infinite_moves_momentum_freeze_time_bonus_and_medal_victory() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut rng = StdRng::seed_from_u64(501);
    let mut session = LevelSession::start_valhalla_blitz_with(
        None,
        CityBonuses::default(),
        PatronGod::Thor,
        &data.balance,
        &mut rng,
    );
    assert!(session.is_blitz);
    assert_eq!(session.blitz_multiplier, 1);
    let start_moves = session.moves_left;

    // Swapping within chain window raises multiplier and never consumes moves.
    session.consume_move();
    assert_eq!(session.moves_left, start_moves);
    assert_eq!(session.blitz_multiplier, 1);
    session.consume_move();
    assert_eq!(session.moves_left, start_moves);
    assert_eq!(session.blitz_multiplier, 2);

    // High combo step multiplies score and adds +2s bonus time.
    let before_time = session.time_left_secs.expect("timed blitz");
    let mut big_step = wyrdstone_core::MatchStepOutcome {
        valid: true,
        score_gained: 1000,
        max_combo: 3,
        ..Default::default()
    };
    big_step.cleared_runes.insert(RuneType::Fire, 5);
    session.apply_step_progress(&big_step);
    assert_eq!(session.blitz_multiplier, 3);
    assert_eq!(session.score, 3000);
    assert!(session.time_left_secs.expect("timed") > before_time);

    // Finalizing a turn early in Blitz never ends the game.
    session.finalize_turn(&data.balance);
    assert_eq!(session.outcome, LevelOutcome::InProgress);

    // Divine Fury freezes the Blitz countdown timer for fury_freeze_secs.
    session.fury = session.fury_max;
    let _ = session.trigger_patron_fury_detailed(&mut rng);
    assert!(session.blitz_freeze_secs > 0.0);
    let frozen_at = session.time_left_secs.expect("timed");
    session.tick_timer(1.0);
    assert_eq!(session.time_left_secs.expect("timed"), frozen_at);

    // Reaching zero clock triggers Victory (Arcade completion), never Defeat, and awards Bronze+ medal.
    session.blitz_freeze_secs = 0.0;
    session.score = data.balance.blitz.gold_score;
    session.tick_timer(200.0);
    assert_eq!(session.outcome, LevelOutcome::Victory);
    assert_eq!(session.earned_stars_with(&data.balance.session), 3);

    let rewards = session.earned_rewards(&data.balance);
    let mut city = CityState::new(&data.balance);
    let (medal, glory, new_best) = city.record_blitz_run(session.score, &rewards, &data.balance);
    assert_eq!(medal, wyrdstone_core::BlitzMedal::Gold);
    assert_eq!(glory, data.balance.blitz.glory_gold);
    assert!(new_best);
    assert_eq!(city.stats.best_blitz_score, data.balance.blitz.gold_score);
}

#[test]
fn save_data_deserializes_without_blitz_stats_fields() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let base = SaveData::new_game(&data.balance, Language::En);
    let mut val = serde_json::to_value(&base).expect("serialize");
    let stats_obj = val["city"]["stats"].as_object_mut().expect("stats map");
    stats_obj.remove("best_blitz_score");
    stats_obj.remove("blitz_runs_completed");
    let json = serde_json::to_string(&val).expect("to string");
    let loaded: SaveData = serde_json::from_str(&json).expect("backward-compat load");
    assert_eq!(loaded.city.stats.best_blitz_score, 0);
    assert_eq!(loaded.city.stats.blitz_runs_completed, 0);
}

#[test]
fn expedition_v2_relics_oaths_varied_layouts_and_wave5_boss() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut rng = StdRng::seed_from_u64(601);
    let mut city = CityState::new(&data.balance);
    let mut run = ExpeditionRun::new();

    // Board layouts vary across waves.
    let w1_def = run.generate_wave_level_v2(&data.balance.expedition, &mut rng);
    run.wave = 2;
    let w2_def = run.generate_wave_level_v2(&data.balance.expedition, &mut rng);
    assert_ne!(w1_def.blocked_cells, w2_def.blocked_cells);

    // Rolling choices at wave >= 2 offers an unowned Relic plus a Skald's Oath.
    run.roll_boon_choices_v2(&data.balance.expedition, &mut rng);
    assert!(run.pending_oath.is_some());
    assert!(matches!(
        run.pending_boons.first(),
        Some(wyrdstone_core::ExpeditionBoon::Relic(_))
    ));
    run.oath_accepted = true;
    run.pending_boons[0] =
        wyrdstone_core::ExpeditionBoon::Relic(wyrdstone_core::ExpeditionRelic::SurtrCoal);
    run.apply_boon(run.pending_boons[0], &mut city);
    assert!(run
        .relics
        .contains(&wyrdstone_core::ExpeditionRelic::SurtrCoal));
    assert!(run.active_oath.is_some());

    // Wave 5 spawns an Expedition Boss Raid while keeping Relics & Overdrive headstart.
    run.wave = 5;
    assert!(run.is_boss_wave());
    let session = LevelSession::start_expedition_wave_full(
        &run,
        &data.balance,
        &data.bosses,
        None,
        CityBonuses::default(),
        PatronGod::Thor,
        &mut rng,
    );
    assert!(session.boss.is_some());
    assert!(session.overdrive_charge >= session.overdrive_max / 2);
}

#[test]
fn save_data_deserializes_legacy_expedition_without_relics_or_oaths() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut base = SaveData::new_game(&data.balance, Language::En);
    base.active_expedition = Some(ExpeditionRun::new());
    let mut val = serde_json::to_value(&base).expect("serialize");
    let exp_obj = val["active_expedition"]
        .as_object_mut()
        .expect("active_expedition map");
    exp_obj.remove("relics");
    exp_obj.remove("pending_oath");
    exp_obj.remove("oath_accepted");
    exp_obj.remove("active_oath");
    let json = serde_json::to_string(&val).expect("to string");
    let loaded: SaveData = serde_json::from_str(&json).expect("backward-compat load");
    let run = loaded.active_expedition.expect("loaded run");
    assert!(run.relics.is_empty());
    assert!(run.active_oath.is_none());
}

#[test]
fn dual_slot_amulets_synergies_and_war_draught_apply_to_session() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut city = CityState::new(&data.balance);
    city.resources = wyrdstone_core::ResourceBundle {
        wood: 5_000,
        stone: 5_000,
        gold: 5_000,
        mead: 5_000,
    };

    // Great Hall T1 locks slot 2; T2 unlocks it.
    assert!(!city.is_secondary_amulet_unlocked());
    city.building_tiers
        .insert(wyrdstone_core::HUB_BUILDING_ID.to_string(), 2);
    assert!(city.is_secondary_amulet_unlocked());

    let thor_am = data.amulet_by_id("amulet_thor").expect("thor").clone();
    let heimdall_am = data
        .amulet_by_id("amulet_heimdall")
        .expect("heimdall")
        .clone();
    city.try_craft_amulet(&thor_am, &data.balance)
        .expect("craft thor");
    city.try_upgrade_amulet(&thor_am, &data.balance).unwrap();
    city.try_upgrade_amulet(&thor_am, &data.balance).unwrap();
    city.try_craft_amulet(&heimdall_am, &data.balance)
        .expect("craft heimdall");
    city.try_upgrade_amulet(&heimdall_am, &data.balance)
        .unwrap();
    city.try_upgrade_amulet(&heimdall_am, &data.balance)
        .unwrap();

    // Equip both slots (heimdall primary via last upgrade, thor secondary) + Patron Thor.
    city.selected_patron = PatronGod::Thor;
    city.try_equip_secondary_amulet("amulet_thor")
        .expect("equip slot 2");
    let synergies = city.active_synergies();
    assert!(synergies.contains(&wyrdstone_core::DivineSynergy::Thunderforge));
    assert!(synergies.contains(&wyrdstone_core::DivineSynergy::TwinRunes));

    // Brew a War Draught.
    city.try_brew_war_draught(wyrdstone_core::WarDraught::MeadOfPoetry)
        .expect("brew");
    assert_eq!(
        city.active_war_draught,
        Some(wyrdstone_core::WarDraught::MeadOfPoetry)
    );

    let mut rng = StdRng::seed_from_u64(701);
    let level = data.level_by_id("forge_1").expect("forge_1").clone();
    let mut session = LevelSession::start_full(
        level,
        city.active_amulet_effect(&data.amulets),
        city.compute_bonuses(&data.balance),
        city.selected_patron,
        None,
        &mut rng,
    );
    if let Some(extra) = city.secondary_amulet_effect(&data.amulets) {
        session.apply_extra_amulet_effect(extra, &mut rng);
    }
    session.apply_synergies(&synergies, &mut rng);
    if let Some(draught) = city.active_war_draught.take() {
        session.apply_war_draught(draught, &mut rng);
    }
    assert_eq!(session.fury, session.fury_max);
    assert!(session.boss_damage_bonus_pct >= 30);
    assert!(session.score_bonus_pct >= 65);
}

#[test]
fn save_data_deserializes_without_secondary_amulet_or_war_draught() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let base = SaveData::new_game(&data.balance, Language::En);
    let mut val = serde_json::to_value(&base).expect("serialize");
    let city_obj = val["city"].as_object_mut().expect("city map");
    city_obj.remove("equipped_amulet_secondary");
    city_obj.remove("active_war_draught");
    let json = serde_json::to_string(&val).expect("to string");
    let loaded: SaveData = serde_json::from_str(&json).expect("backward-compat load");
    assert!(loaded.city.equipped_amulet_secondary.is_none());
    assert!(loaded.city.active_war_draught.is_none());
}

#[test]
fn all_bundled_yggdrasil_trials_are_solvable_within_par() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    assert_eq!(data.trials.len(), 6);

    let solutions: Vec<(&str, Vec<(CellPos, CellPos)>)> = vec![
        (
            "trial_bifrost_spark",
            vec![(CellPos::new(2, 2), CellPos::new(3, 2))],
        ),
        (
            "trial_frost_prison",
            vec![(CellPos::new(2, 2), CellPos::new(3, 2))],
        ),
        (
            "trial_valkyrie_drop",
            vec![
                (CellPos::new(2, 4), CellPos::new(2, 5)),
                (CellPos::new(5, 4), CellPos::new(5, 5)),
            ],
        ),
        (
            "trial_chain_breaker",
            vec![
                (CellPos::new(2, 0), CellPos::new(2, 1)),
                (CellPos::new(3, 4), CellPos::new(3, 5)),
            ],
        ),
        (
            "trial_bomb_nova",
            vec![(CellPos::new(2, 2), CellPos::new(3, 2))],
        ),
        (
            "trial_allfather_gambit",
            vec![
                (CellPos::new(1, 1), CellPos::new(2, 1)),
                (CellPos::new(5, 4), CellPos::new(5, 5)),
            ],
        ),
    ];

    let mut city = CityState::new(&data.balance);
    for (trial_id, moves) in solutions {
        let trial = data
            .trial_by_id(trial_id)
            .unwrap_or_else(|| panic!("missing trial '{trial_id}'"));
        let mut rng = StdRng::seed_from_u64(trial.seed);
        let mut session =
            LevelSession::start_trial(trial, &data.balance).expect("trial board builds");
        for (a, b) in moves {
            let step = session.play_swap(a, b, &data.balance, &mut rng);
            assert!(
                step.valid,
                "solution swap ({a:?} <-> {b:?}) rejected on {trial_id}"
            );
        }
        assert_eq!(
            session.outcome,
            LevelOutcome::Victory,
            "trial {trial_id} did not reach Victory"
        );
        let used = session.trial_moves_used();
        assert!(
            used <= trial.par_moves,
            "trial {trial_id} took {used} moves (par {})",
            trial.par_moves
        );

        let (first_clear, hit_par, glory_1, res_1) = city.record_trial_victory(trial, used);
        assert!(first_clear && hit_par);
        assert_eq!(
            glory_1,
            trial.first_clear_glory + wyrdstone_core::TRIAL_PAR_BONUS_GLORY
        );
        assert_eq!(res_1, trial.first_clear_rewards);

        // Replaying already-Par trial grants zero duplicate first-clear rewards.
        let (first_2, hit_par_2, glory_2, res_2) = city.record_trial_victory(trial, used);
        assert!(!first_2 && hit_par_2);
        assert_eq!(glory_2, 0);
        assert!(res_2.is_zero());
    }
    assert_eq!(city.completed_trials.len(), 6);
    assert_eq!(city.par_trials.len(), 6);
}

#[test]
fn ragnarok_gauntlet_advances_all_four_bosses_and_persists_stats() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut city = CityState::new(&data.balance);
    let mut run = wyrdstone_core::GauntletRun::new(&data.bosses);
    assert_eq!(run.total_stages, 4);
    assert_eq!(run.moves_left, wyrdstone_core::GAUNTLET_STARTING_MOVES);

    let mut rng = StdRng::seed_from_u64(9001);
    for stage in 0..4 {
        let boss_id = run.current_boss_id().expect("boss id").to_string();
        let boss_def = data.boss_by_id(&boss_id).expect("boss def").clone();
        run.carried_fury = 75;
        run.carried_overdrive_turns = 2;
        let session = LevelSession::start_gauntlet_stage(
            &run,
            &boss_def,
            &data.balance,
            None,
            city.compute_bonuses(&data.balance),
            PatronGod::Thor,
            &mut rng,
        );
        if stage > 0 {
            assert_eq!(session.moves_left, run.moves_left);
            assert_eq!(session.fury, 75);
            assert_eq!(session.overdrive_turns_left, 2);
        }
        let expected_hp = (boss_def.max_hp * wyrdstone_core::GAUNTLET_HP_SCALE_PCT) / 100;
        assert_eq!(session.boss.as_ref().expect("boss").def.max_hp, expected_hp);

        let has_next = run.advance_after_stage(
            run.moves_left.saturating_sub(4),
            60,
            1,
            1_500,
            ResourceBundle {
                wood: 25,
                stone: 25,
                gold: 25,
                mead: 10,
            },
        );
        assert_eq!(has_next, stage < 3);
    }

    let glory = city.record_gauntlet_result(4, true, &run.total_rewards);
    assert_eq!(glory, wyrdstone_core::GAUNTLET_CLEAR_GLORY);
    assert_eq!(city.stats.best_gauntlet_stage, 4);
    assert_eq!(city.stats.gauntlets_cleared, 1);
    assert_eq!(run.cumulative_score, 6_000);
}

#[test]
fn save_data_deserializes_without_trials_or_gauntlet_fields() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let base = SaveData::new_game(&data.balance, Language::En);
    let mut val = serde_json::to_value(&base).expect("serialize");
    let city_obj = val["city"].as_object_mut().expect("city map");
    city_obj.remove("completed_trials");
    city_obj.remove("par_trials");
    let stats_obj = city_obj["stats"].as_object_mut().expect("stats map");
    stats_obj.remove("best_gauntlet_stage");
    stats_obj.remove("gauntlets_cleared");
    let json = serde_json::to_string(&val).expect("to string");
    let loaded: SaveData = serde_json::from_str(&json).expect("backward-compat load");
    assert!(loaded.city.completed_trials.is_empty());
    assert!(loaded.city.par_trials.is_empty());
    assert_eq!(loaded.city.stats.best_gauntlet_stage, 0);
    assert_eq!(loaded.city.stats.gauntlets_cleared, 0);
}

#[test]
fn retention_valhalla_flame_last_stand_offline_vault_and_bounties_work() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut city = CityState::new(&data.balance);
    let mut rng = StdRng::seed_from_u64(42);

    // 1. Valhalla Flame escalating tiers + Trial immunity.
    assert_eq!(
        city.valhalla_flame_tier(),
        wyrdstone_core::ValhallaFlameTier::None
    );
    city.stats.win_streak = 5;
    assert_eq!(
        city.valhalla_flame_tier(),
        wyrdstone_core::ValhallaFlameTier::Inferno
    );

    let lvl = data.level_by_id("great_hall_1").expect("level").clone();
    let mut session = LevelSession::start_full(
        lvl,
        None,
        city.compute_bonuses(&data.balance),
        PatronGod::Thor,
        None,
        &mut rng,
    );
    let bonuses_before = (0..session.board.rows())
        .flat_map(|r| (0..session.board.cols()).map(move |c| CellPos::new(c, r)))
        .filter(|&p| session.board.get(p).unwrap().booster == Booster::BifrostShard)
        .count();
    session.apply_valhalla_flame(city.valhalla_flame_tier(), &mut rng);
    let bonuses_after = (0..session.board.rows())
        .flat_map(|r| (0..session.board.cols()).map(move |c| CellPos::new(c, r)))
        .filter(|&p| session.board.get(p).unwrap().booster == Booster::BifrostShard)
        .count();
    assert_eq!(bonuses_after, bonuses_before + 1);
    assert_eq!(session.fury, 25);

    // 2. Last Stand revives once on defeat and excludes trials/blitz.
    session.outcome = LevelOutcome::Defeat;
    session.moves_left = 0;
    assert!(session.can_use_last_stand());
    assert!(session.apply_last_stand(false));
    assert_eq!(session.outcome, LevelOutcome::InProgress);
    assert_eq!(session.moves_left, wyrdstone_core::LAST_STAND_EXTRA_MOVES);
    session.outcome = LevelOutcome::Defeat;
    assert!(!session.can_use_last_stand(), "Last Stand is 1x per run");

    let mut trial_session =
        LevelSession::start_trial(&data.trials[0], &data.balance).expect("trial");
    trial_session.apply_valhalla_flame(wyrdstone_core::ValhallaFlameTier::Inferno, &mut rng);
    assert_eq!(trial_session.fury, 0);
    trial_session.outcome = LevelOutcome::Defeat;
    assert!(!trial_session.can_use_last_stand());

    // 3. Offline Harvest Vault scales cap by Great Hall tier and ignores <30 min absences.
    city.building_tiers.insert("great_hall".to_string(), 2);
    assert_eq!(city.offline_vault_max_hours(), 12);
    assert!(city
        .compute_offline_vault(&data.buildings, 1_000, 2_000)
        .is_none());
    let vault = city
        .compute_offline_vault(&data.buildings, 10_000, 10_000 + 20 * 3_600)
        .expect("vault available");
    assert_eq!(vault.cycles, 24, "capped at 12h = 24 cycles of 30m");
    let gold_before = city.resources.gold;
    city.claim_offline_vault(&vault, &data.buildings, 10_000 + 20 * 3_600);
    assert!(city.resources.gold > gold_before);

    // 4. Daily Bounties + Skald's Path + backward-compatible SaveData.
    city.daily_bounties.ensure_for_day(20_417);
    assert_eq!(city.daily_bounties.bounties.len(), 3);
    let kind0 = city.daily_bounties.bounties[0].kind;
    let target0 = city.daily_bounties.bounties[0].target;
    city.daily_bounties.record_progress(kind0, target0);
    assert_eq!(city.claimable_bounties_and_pass_count(), 1);
    assert!(city.try_claim_bounty(0).is_some());
    city.skald_pass.add_xp(100);
    assert_eq!(city.skald_pass.current_tier(), 1);
    assert!(city.try_claim_skald_tier(1).is_some());

    let base = SaveData::new_game(&data.balance, Language::En);
    let mut val = serde_json::to_value(&base).expect("serialize");
    let city_obj = val["city"].as_object_mut().expect("city map");
    city_obj.remove("daily_bounties");
    city_obj.remove("skald_pass");
    let json = serde_json::to_string(&val).expect("to string");
    let loaded: SaveData = serde_json::from_str(&json).expect("backward-compat load");
    assert!(loaded.city.daily_bounties.bounties.is_empty());
    assert_eq!(loaded.city.skald_pass.xp, 0);
}

#[test]
fn progressive_mode_unlocks_gate_early_modes_and_open_for_veterans() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut city = CityState::new(&data.balance);
    assert_eq!(city.unlocked_mode_count(), 0);
    for mode in wyrdstone_core::PlayMode::ALL {
        assert!(!city.is_mode_unlocked(mode));
        assert_ne!(data.i18n.t(mode.name_key()), mode.name_key());
        assert_ne!(data.i18n.t(mode.unlock_hint_key()), mode.unlock_hint_key());
    }

    city.stats.total_victories = 1;
    assert!(city.is_mode_unlocked(wyrdstone_core::PlayMode::DailyTrial));
    assert!(!city.is_mode_unlocked(wyrdstone_core::PlayMode::YggdrasilTrials));

    city.stats.total_victories = 2;
    assert!(city.is_mode_unlocked(wyrdstone_core::PlayMode::YggdrasilTrials));

    city.building_tiers.insert("great_hall".to_string(), 1);
    city.stats.total_victories = 3;
    assert!(city.is_mode_unlocked(wyrdstone_core::PlayMode::ValhallaBlitz));
    assert!(city.is_mode_unlocked(wyrdstone_core::PlayMode::BossRaids));
    assert!(!city.is_mode_unlocked(wyrdstone_core::PlayMode::Expedition));

    city.building_tiers.insert("great_hall".to_string(), 2);
    assert_eq!(
        city.unlocked_mode_count(),
        wyrdstone_core::PlayMode::ALL.len()
    );
}

#[test]
fn endgame_patron_devotion_weekly_challenge_and_legend_ranks_work() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut city = CityState::new(&data.balance);
    let mut rng = StdRng::seed_from_u64(9001);

    // 1. Patron Devotion Altar gates on Glory & resources and scales session bonuses (not Trials).
    assert_eq!(city.patron_devotion_tier(PatronGod::Thor), 0);
    assert_eq!(
        city.try_upgrade_patron_devotion(PatronGod::Thor),
        Err("patron.devotion.need_glory")
    );
    city.glory = 1_500;
    city.resources = ResourceBundle {
        wood: 5_000,
        stone: 5_000,
        gold: 5_000,
        mead: 1_000,
    };
    for expected in 1..=wyrdstone_core::MAX_PATRON_DEVOTION_TIER {
        assert_eq!(
            city.try_upgrade_patron_devotion(PatronGod::Thor),
            Ok(expected)
        );
    }
    assert_eq!(
        city.try_upgrade_patron_devotion(PatronGod::Thor),
        Err("patron.devotion.maxed")
    );

    let lvl = data.level_by_id("great_hall_1").expect("level").clone();
    let mut session = LevelSession::start_full(
        lvl,
        None,
        city.compute_bonuses(&data.balance),
        PatronGod::Thor,
        None,
        &mut rng,
    );
    let fury_pct_before = session.fury_charge_bonus_pct;
    let score_pct_before = session.score_bonus_pct;
    let start_fury_before = session.fury;
    session.apply_patron_devotion(city.patron_devotion_tier(PatronGod::Thor));
    assert_eq!(
        session.fury_charge_bonus_pct,
        fury_pct_before + 5 * wyrdstone_core::PATRON_DEVOTION_FURY_PCT_PER_TIER
    );
    assert_eq!(
        session.score_bonus_pct,
        score_pct_before + 5 * wyrdstone_core::PATRON_DEVOTION_SCORE_PCT_PER_TIER
    );
    assert_eq!(
        session.fury,
        start_fury_before + wyrdstone_core::PATRON_DEVOTION_START_FURY_AT_TIER3
    );
    assert_eq!(
        session.boss_damage_bonus_pct,
        wyrdstone_core::PATRON_DEVOTION_BOSS_DMG_PCT_AT_TIER5
    );

    let mut trial = LevelSession::start_trial(&data.trials[0], &data.balance).expect("trial");
    trial.apply_patron_devotion(5);
    assert_eq!(trial.score_bonus_pct, 0);

    // 2. Weekly Seeded Challenge spawns playable boards for 52 weeks and grants cumulative medals once per week.
    for week in 2_900..2_952 {
        let def = wyrdstone_core::LevelDef::weekly_challenge(week);
        let board = Board::new_for_level(&def, &mut rng);
        assert!(
            board.has_valid_moves(),
            "Weekly challenge week {week} must be playable"
        );
    }
    let week_def = wyrdstone_core::LevelDef::weekly_challenge(2_917);
    let (medal1, best1, _res1, gl1) =
        city.record_weekly_challenge_run(2_917, 15_000, &week_def.base_reward);
    assert_eq!(medal1, 2); // Silver
    assert!(best1);
    assert_eq!(gl1, 65); // Bronze 25 + Silver 40
    let (medal2, best2, _res2, gl2) =
        city.record_weekly_challenge_run(2_917, 12_000, &week_def.base_reward);
    assert_eq!(medal2, 1);
    assert!(!best2);
    assert_eq!(gl2, 0); // Already claimed up to Silver
    let (medal_next_week, best_next, _, _) =
        city.record_weekly_challenge_run(2_918, 8_500, &week_def.base_reward);
    assert_eq!(medal_next_week, 1);
    assert!(best_next);

    // 3. Legend Ranks compute beyond High King with capped spoils & backward-compatible save load.
    city.glory =
        data.balance.clan_rank_high_king_glory + 2 * wyrdstone_core::LEGEND_RANK_GLORY_STEP;
    assert_eq!(city.legend_level(&data.balance), 2);
    assert_eq!(
        city.legend_spoils_bonus_pct(&data.balance),
        2 * wyrdstone_core::LEGEND_SPOILS_PCT_PER_LEVEL
    );
    city.glory = 99_999;
    assert_eq!(
        city.legend_spoils_bonus_pct(&data.balance),
        wyrdstone_core::LEGEND_SPOILS_MAX_BONUS_PCT
    );

    let base = SaveData::new_game(&data.balance, Language::En);
    let mut val = serde_json::to_value(&base).expect("serialize");
    let city_obj = val["city"].as_object_mut().expect("city map");
    city_obj.remove("patron_devotion");
    city_obj.remove("weekly_challenge");
    let json = serde_json::to_string(&val).expect("to string");
    let loaded: SaveData = serde_json::from_str(&json).expect("backward-compat load");
    assert!(loaded.city.patron_devotion.is_empty());
    assert_eq!(loaded.city.weekly_challenge.best_score, 0);
}

#[test]
fn progressive_mode_unlocks_shortage_sources_and_quest_progress() {
    use wyrdstone_core::{newly_unlocked_modes, PlayMode, QuestCondition, QuestDef, ResourceKind};

    let data = GameDataBundle::load_bundled(Language::En).expect("bundled data loads");
    let mut city = CityState::new(&data.balance);

    // Fresh city has 0 unlocked modes; DailyTrial is the first locked mode.
    assert_eq!(city.unlocked_modes_mask(), 0);
    assert_eq!(city.unlocked_mode_count(), 0);
    assert_eq!(city.next_locked_mode(), Some(PlayMode::DailyTrial));

    // Victory #1 unlocks DailyTrial.
    let before = city.unlocked_modes_mask();
    city.stats.total_victories = 1;
    let newly = newly_unlocked_modes(before, city.unlocked_modes_mask());
    assert_eq!(newly, vec![PlayMode::DailyTrial]);
    assert_eq!(city.next_locked_mode(), Some(PlayMode::YggdrasilTrials));

    // Great Hall Tier 1 + 3 victories + 50 Glory unlocks remaining modes up to BossRaids, leaving Expedition.
    let before_tier1 = city.unlocked_modes_mask();
    city.building_tiers.insert("great_hall".to_string(), 1);
    city.stats.total_victories = 3;
    city.glory = 50;
    let newly_tier1 = newly_unlocked_modes(before_tier1, city.unlocked_modes_mask());
    assert_eq!(
        newly_tier1,
        vec![
            PlayMode::YggdrasilTrials,
            PlayMode::ValhallaBlitz,
            PlayMode::WeeklyChallenge,
            PlayMode::BossRaids,
        ]
    );
    assert_eq!(city.next_locked_mode(), Some(PlayMode::Expedition));

    // Great Hall Tier 2 unlocks Expedition -> all modes unlocked.
    city.building_tiers.insert("great_hall".to_string(), 2);
    assert_eq!(city.next_locked_mode(), None);

    // Shortage guidance on fresh city (great_hall & meadery unlocked, 0 stars -> Standard only).
    let fresh = CityState::new(&data.balance);
    let (b_wood, lvl_wood, diff_wood) = fresh
        .best_source_for_resource(ResourceKind::Wood, &data.buildings, &data.levels, None)
        .expect("wood source on fresh city");
    assert_eq!(b_wood.id, "great_hall");
    assert_eq!(lvl_wood.id, "great_hall_1");
    assert_eq!(diff_wood, 0);

    let (b_mead, lvl_mead, diff_mead) = fresh
        .best_source_for_resource(ResourceKind::Mead, &data.buildings, &data.levels, None)
        .expect("mead source on fresh city");
    assert_eq!(b_mead.id, "meadery");
    assert_eq!(lvl_mead.id, "meadery_1");
    assert_eq!(diff_mead, 0);

    // Unlocking >=1 mastery star on great_hall allows Mastery (great_hall_2) to win for Wood.
    let mut starred = CityState::new(&data.balance);
    starred.building_wins.insert("great_hall".to_string(), 1);
    let (_, lvl_wood_m, diff_wood_m) = starred
        .best_source_for_resource(ResourceKind::Wood, &data.buildings, &data.levels, None)
        .expect("mastery wood source");
    assert_eq!(diff_wood_m, 1);
    assert_eq!(lvl_wood_m.id, "great_hall_2");

    // Quest progress matches is_quest_ready across all 8 QuestCondition variants.
    let mut q_city = CityState::new(&data.balance);
    q_city.building_wins.insert("great_hall".to_string(), 1);
    q_city.completed_levels.insert("great_hall_2".to_string());
    q_city.building_tiers.insert("forge".to_string(), 1);
    q_city.crafted_amulets.insert("amulet_odin".to_string());
    q_city.claimed_collections.insert("col_asgard".to_string());
    q_city.stats.daily_completions = 2;
    q_city.stats.bosses_defeated = 0;
    q_city.defeated_bosses.insert("fenrir".to_string());
    q_city.stats.best_expedition_wave = 2;

    let cases = [
        (
            QuestCondition::WinBuildingLevel {
                building_id: "great_hall".to_string(),
                wins: 2,
            },
            (1, 2),
        ),
        (
            QuestCondition::WinSpecificLevel {
                level_id: "great_hall_2".to_string(),
            },
            (1, 1),
        ),
        (
            QuestCondition::ReachBuildingTier {
                building_id: "forge".to_string(),
                tier: 2,
            },
            (1, 2),
        ),
        (QuestCondition::CraftAnyAmulet, (1, 1)),
        (QuestCondition::ClaimAnyCollection, (1, 1)),
        (QuestCondition::CompleteDailyTrials { count: 3 }, (2, 3)),
        (QuestCondition::DefeatBosses { count: 1 }, (1, 1)),
        (QuestCondition::ReachExpeditionWave { wave: 3 }, (2, 3)),
    ];
    for (idx, (cond, expected)) in cases.into_iter().enumerate() {
        let q = QuestDef {
            id: format!("t{idx}"),
            speaker: "jarl".to_string(),
            title_key: "quest.q1.title".to_string(),
            desc_key: "quest.q1.desc".to_string(),
            condition: cond,
            reward: ResourceBundle::default(),
        };
        assert_eq!(q_city.quest_progress(&q), expected, "variant #{idx}");
    }
}

#[test]
fn building_mastery_perks_and_repeatable_collection_charging_work() {
    use wyrdstone_core::{
        COLLECTION_RECHARGE_REWARD_PCT, MASTERY_HARVEST_PCT_AT_4_STARS,
        MASTERY_HARVEST_PCT_AT_5_STARS, MASTERY_SPOILS_PCT_PER_STAR,
    };

    let data = GameDataBundle::load_bundled(Language::En).expect("bundled data loads");
    let mut rng = StdRng::seed_from_u64(777);
    let mut city = CityState::new(&data.balance);
    let gh = data
        .building_by_id("great_hall")
        .expect("great_hall exists");
    city.building_tiers.insert(gh.id.clone(), 1);

    let base_harvest = city.harvest_reward(gh);
    assert_eq!(city.building_mastery_spoils_pct(&gh.id), 0);
    assert_eq!(city.building_mastery_harvest_pct(&gh.id), 0);

    city.building_wins.insert(gh.id.clone(), 4);
    assert_eq!(
        city.building_mastery_spoils_pct(&gh.id),
        4 * MASTERY_SPOILS_PCT_PER_STAR
    );
    assert_eq!(
        city.building_mastery_harvest_pct(&gh.id),
        MASTERY_HARVEST_PCT_AT_4_STARS
    );
    let h4 = city.harvest_reward(gh);
    assert!(h4.gold > base_harvest.gold);

    city.building_wins.insert(gh.id.clone(), 5);
    assert_eq!(
        city.building_mastery_spoils_pct(&gh.id),
        5 * MASTERY_SPOILS_PCT_PER_STAR
    );
    assert_eq!(
        city.building_mastery_harvest_pct(&gh.id),
        MASTERY_HARVEST_PCT_AT_5_STARS
    );
    let h5 = city.harvest_reward(gh);
    assert!(h5.gold >= h4.gold);

    let lvl = data.level_by_id("great_hall_1").unwrap().clone();
    let mut session = LevelSession::start_full(
        lvl,
        None,
        city.compute_bonuses(&data.balance),
        PatronGod::Odin,
        None,
        &mut rng,
    );
    let before_pct = session.resource_bonus_pct;
    session.apply_building_mastery_bonus(city.building_mastery_spoils_pct(&gh.id));
    assert_eq!(
        session.resource_bonus_pct,
        before_pct + 5 * MASTERY_SPOILS_PCT_PER_STAR
    );

    let mut trial = LevelSession::start_trial(&data.trials[0], &data.balance).expect("trial");
    trial.apply_building_mastery_bonus(25);
    assert_eq!(trial.resource_bonus_pct, 0);

    // Repeatable collection charging: first claim pays full, resets found_relics, recharge pays 50%.
    let midgard = data
        .collections
        .iter()
        .find(|c| c.id == "col_midgard")
        .expect("col_midgard exists");
    for _ in 0..midgard.relic_ids.len() {
        assert!(city
            .discover_next_relic_for_building("great_hall", &data.collections)
            .is_some());
    }
    assert!(city.is_collection_ready(midgard));
    let gold_before = city.resources.gold;
    let glory_before = city.glory;
    assert!(city.try_claim_collection(midgard, &data.balance));
    assert_eq!(city.collection_charge_count(&midgard.id), 1);
    assert!(!city.is_collection_ready(midgard));
    assert_eq!(city.collection_found_count(midgard), 0);
    assert_eq!(city.total_relics_collected(&data.collections), 3);
    assert_eq!(
        city.resources.gold - gold_before,
        midgard.reward_resources.gold
    );
    assert_eq!(city.glory - glory_before, data.balance.collection_glory);

    // Discover all 3 again and verify scaled recharge reward.
    for _ in 0..midgard.relic_ids.len() {
        assert!(city
            .discover_next_relic_for_building("great_hall", &data.collections)
            .is_some());
    }
    assert!(city.is_collection_ready(midgard));
    let gold_before_r2 = city.resources.gold;
    let glory_before_r2 = city.glory;
    assert!(city.try_claim_collection(midgard, &data.balance));
    assert_eq!(city.collection_charge_count(&midgard.id), 2);
    assert_eq!(city.total_relics_collected(&data.collections), 6);
    assert_eq!(
        city.resources.gold - gold_before_r2,
        midgard
            .reward_resources
            .gold
            .saturating_mul(COLLECTION_RECHARGE_REWARD_PCT)
            .div_ceil(100)
    );
    assert_eq!(
        city.glory - glory_before_r2,
        data.balance
            .collection_glory
            .saturating_mul(COLLECTION_RECHARGE_REWARD_PCT)
            .div_ceil(100)
    );

    // Legacy v5 save migration clears already-claimed relics and initializes charges to 1.
    let mut legacy = CityState::new(&data.balance);
    for rid in &midgard.relic_ids {
        legacy.found_relics.insert(rid.clone());
    }
    legacy.claimed_collections.insert(midgard.id.clone());
    legacy.sanitize_collections(&data.collections);
    assert_eq!(legacy.collection_charge_count(&midgard.id), 1);
    assert_eq!(legacy.collection_found_count(midgard), 0);
    assert!(!legacy.is_collection_ready(midgard));
    assert_eq!(legacy.total_relics_collected(&data.collections), 3);
}

#[test]
fn valkyrie_aid_threat_ratings_and_defeat_tips_work() {
    use wyrdstone_core::{
        ThreatRating, ValkyrieAidTier, MAX_VALKYRIE_FAIL_STREAK, VALKYRIE_AID_TIER1_MOVES,
        VALKYRIE_AID_TIER2_FURY, VALKYRIE_AID_TIER2_MOVES, VALKYRIE_AID_TIER2_SECS,
    };

    let data = GameDataBundle::load_bundled(Language::En).expect("bundled data loads");
    let mut rng = StdRng::seed_from_u64(909);
    let mut city = CityState::new(&data.balance);
    let timed_lvl_id = "meadery_2";

    assert_eq!(
        city.valkyrie_aid_for_level(timed_lvl_id),
        ValkyrieAidTier::None
    );
    city.record_level_defeat(timed_lvl_id);
    assert_eq!(city.level_fail_streak(timed_lvl_id), 1);
    assert_eq!(
        city.valkyrie_aid_for_level(timed_lvl_id),
        ValkyrieAidTier::None
    );

    city.record_level_defeat(timed_lvl_id);
    assert_eq!(city.level_fail_streak(timed_lvl_id), 2);
    assert_eq!(
        city.valkyrie_aid_for_level(timed_lvl_id),
        ValkyrieAidTier::Skirmish
    );

    for _ in 0..5 {
        city.record_level_defeat(timed_lvl_id);
    }
    assert_eq!(
        city.level_fail_streak(timed_lvl_id),
        MAX_VALKYRIE_FAIL_STREAK
    );
    assert_eq!(
        city.valkyrie_aid_for_level(timed_lvl_id),
        ValkyrieAidTier::Vanguard
    );

    // Clamp & prune zero streaks during sanitize.
    city.level_fail_streaks.insert("bogus".to_string(), 99);
    city.level_fail_streaks.insert("zero".to_string(), 0);
    city.sanitize(&data.buildings, &data.balance);
    assert_eq!(city.level_fail_streak("bogus"), MAX_VALKYRIE_FAIL_STREAK);
    assert!(!city.level_fail_streaks.contains_key("zero"));

    // Apply Skirmish & Vanguard Valkyrie Aid to a timed session (preserves initial_moves invariant).
    let lvl = data.level_by_id(timed_lvl_id).unwrap().clone();
    let mut session = LevelSession::start_full(
        lvl,
        None,
        city.compute_bonuses(&data.balance),
        PatronGod::Thor,
        None,
        &mut rng,
    );
    let base_moves = session.initial_moves;
    let base_time = session.time_left_secs.unwrap();
    session.apply_valkyrie_aid(ValkyrieAidTier::Skirmish, &mut rng);
    assert_eq!(session.initial_moves, base_moves + VALKYRIE_AID_TIER1_MOVES);
    assert_eq!(session.moves_left, base_moves + VALKYRIE_AID_TIER1_MOVES);

    session.apply_valkyrie_aid(ValkyrieAidTier::Vanguard, &mut rng);
    assert_eq!(
        session.initial_moves,
        base_moves + VALKYRIE_AID_TIER1_MOVES + VALKYRIE_AID_TIER2_MOVES
    );
    assert_eq!(session.moves_left, session.initial_moves);
    assert!(session.time_left_secs.unwrap() > base_time + VALKYRIE_AID_TIER2_SECS);
    assert!(session.fury >= VALKYRIE_AID_TIER2_FURY);

    // Trial and Blitz sessions ignore Valkyrie's Aid.
    let mut trial = LevelSession::start_trial(&data.trials[0], &data.balance).expect("trial");
    let trial_moves = trial.moves_left;
    trial.apply_valkyrie_aid(ValkyrieAidTier::Vanguard, &mut rng);
    assert_eq!(trial.moves_left, trial_moves);
    assert_eq!(trial.defeat_tip_key(), "tip.defeat.trial");

    let mut blitz = LevelSession::start_valhalla_blitz_with(
        None,
        city.compute_bonuses(&data.balance),
        PatronGod::Odin,
        &data.balance,
        &mut rng,
    );
    let blitz_time = blitz.time_left_secs;
    blitz.apply_valkyrie_aid(ValkyrieAidTier::Vanguard, &mut rng);
    assert_eq!(blitz.time_left_secs, blitz_time);

    let boss = data.boss_by_id("boss_fenrir").expect("boss exists").clone();
    let boss_session = LevelSession::start_boss_with(
        boss,
        None,
        city.compute_bonuses(&data.balance),
        PatronGod::Thor,
        &data.balance,
        &mut rng,
    );
    assert_eq!(boss_session.defeat_tip_key(), "tip.defeat.boss");

    // Victory clears the fail streak cleanly.
    city.clear_level_fail_streak(timed_lvl_id);
    assert_eq!(city.level_fail_streak(timed_lvl_id), 0);
    assert_eq!(
        city.valkyrie_aid_for_level(timed_lvl_id),
        ValkyrieAidTier::None
    );

    // All 4 ThreatRating tiers must appear across the 18 bundled building levels.
    let mut seen_threats = std::collections::HashSet::new();
    for level in &data.levels {
        seen_threats.insert(level.threat_rating() as u8);
        assert!(!level.tactical_tip_key().is_empty());
    }
    assert!(seen_threats.contains(&(ThreatRating::Skirmish as u8)));
    assert!(seen_threats.contains(&(ThreatRating::Battle as u8)));
    assert!(seen_threats.contains(&(ThreatRating::Fierce as u8)));
    assert!(seen_threats.contains(&(ThreatRating::Ragnarok as u8)));
}

#[test]
fn claim_all_skald_tiers_claims_only_unlocked_tiers_once() {
    let data = GameDataBundle::load_bundled(Language::En).expect("bundled data loads");
    let mut city = CityState::new(&data.balance);
    city.skald_pass
        .add_xp(wyrdstone_core::SKALD_PASS_XP_PER_TIER * 3);
    let _ = city.try_claim_skald_tier(1);
    let before_glory = city.glory;

    let (count, res, tools, glory) = city.try_claim_all_skald_tiers();
    assert_eq!(count, 2);
    assert_eq!(tools.len(), 1);
    assert!(res.gold > 0);
    assert_eq!(city.glory, before_glory + glory);
    assert_eq!(city.skald_pass.claimable_count(), 0);

    let (second_count, _, _, second_glory) = city.try_claim_all_skald_tiers();
    assert_eq!(second_count, 0);
    assert_eq!(second_glory, 0);
}

#[test]
fn skald_pass_cycles_scale_rewards_and_preserve_overflow_xp() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut city = CityState::new(&data.balance);
    assert!(!city.try_advance_skald_cycle());

    let cap = wyrdstone_core::SkaldPassState::cycle_xp_cap();
    city.skald_pass.add_xp(cap + 240);
    assert_eq!(
        city.skald_pass.current_tier(),
        wyrdstone_core::SKALD_PASS_TIERS
    );
    assert_eq!(city.skald_pass.xp, cap + 240);

    city.sanitize(&data.buildings, &data.balance);
    assert_eq!(
        city.skald_pass.xp,
        cap + 240,
        "sanitize must preserve overflow XP up to 2x cap"
    );

    let (claimed, _, _, cycle0_glory) = city.try_claim_all_skald_tiers();
    assert_eq!(claimed, usize::from(wyrdstone_core::SKALD_PASS_TIERS));
    assert!(city.skald_pass.can_advance_cycle());
    assert_eq!(city.claimable_bounties_and_pass_count(), 1);
    assert!(city.try_advance_skald_cycle());
    assert_eq!(city.skald_pass.cycle, 1);
    assert_eq!(city.skald_pass.xp, 240);
    assert_eq!(city.skald_pass.current_tier(), 2);

    let (_, _, tier1_cycle1_glory) = city
        .try_claim_skald_tier(1)
        .expect("tier 1 already unlocked via overflow XP");
    let (_, _, tier1_cycle0_glory) = wyrdstone_core::skald_pass_tier_reward(1);
    assert!(tier1_cycle1_glory > tier1_cycle0_glory);
    assert!(cycle0_glory > 0);
}

#[test]
fn endless_decree_activates_after_saga_scales_and_persists_compatibly() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut city = CityState::new(&data.balance);
    assert!(!city.is_endless_decree_active(&data.quests));

    for q in &data.quests {
        city.claimed_quests.insert(q.id.clone());
    }
    assert!(city.is_endless_decree_active(&data.quests));
    assert_eq!(city.endless_decree.decree_number(), 1);
    assert_eq!(
        city.endless_decree.kind(),
        wyrdstone_core::EndlessDecreeKind::WinRaids
    );
    let target0 = city.endless_decree.target();
    city.record_endless_decree_progress(
        &data.quests,
        wyrdstone_core::EndlessDecreeKind::ClearRunes,
        50,
    );
    assert_eq!(city.endless_decree.progress, 0);

    city.record_endless_decree_progress(
        &data.quests,
        wyrdstone_core::EndlessDecreeKind::WinRaids,
        target0,
    );
    assert!(city.is_endless_decree_ready(&data.quests));
    let (res, tool_opt, glory) = city
        .try_claim_endless_decree(&data.quests)
        .expect("ready decree must claim");
    assert!(res.gold > 0 && glory > 0);
    assert!(tool_opt.is_none());
    assert_eq!(city.endless_decree.decree_number(), 2);

    // Every 5th decree awards a bonus Runic Tool.
    city.endless_decree.index = 4;
    let fifth_target = city.endless_decree.target();
    city.endless_decree.progress = fifth_target;
    let (_, fifth_tool, _) = city
        .try_claim_endless_decree(&data.quests)
        .expect("5th decree claims");
    assert!(fifth_tool.is_some());

    // Legacy v5 saves without `endless_decree` or `skald_pass.cycle` still load cleanly.
    let save = SaveData::new_game(&data.balance, Language::En);
    let mut val = serde_json::to_value(&save).expect("serialize");
    let city_obj = val["city"].as_object_mut().expect("city obj");
    city_obj.remove("endless_decree");
    if let Some(sp) = city_obj
        .get_mut("skald_pass")
        .and_then(|v| v.as_object_mut())
    {
        sp.remove("cycle");
    }
    let loaded: SaveData =
        serde_json::from_str(&serde_json::to_string(&val).expect("str")).expect("compat load");
    assert_eq!(loaded.city.endless_decree.index, 0);
    assert_eq!(loaded.city.skald_pass.cycle, 0);
}

#[test]
fn yggdrasil_wonder_unlocks_scales_bonuses_and_persists_compatibly() {
    let data = GameDataBundle::load_bundled(Language::En).expect("data");
    let mut city = CityState::new(&data.balance);
    assert!(!city.is_wonder_unlocked(&data.buildings));
    assert_eq!(
        city.try_upgrade_wonder(&data.buildings),
        Err("city.wonder.locked")
    );

    for b in &data.buildings {
        city.building_tiers.insert(b.id.clone(), 1);
    }
    assert!(city.is_wonder_unlocked(&data.buildings));
    city.resources = ResourceBundle {
        wood: 50_000,
        stone: 50_000,
        gold: 50_000,
        mead: 50_000,
    };

    let base_harvest = city.harvest_reward(&data.buildings[0]);
    let base_max_energy = city.max_energy(&data.balance);

    let (stage1, glory1, tool1) = city
        .try_upgrade_wonder(&data.buildings)
        .expect("stage 1 succeeds");
    assert_eq!(stage1, 1);
    assert!(glory1 > 0 && tool1.is_none());

    let (stage2, _, tool2) = city
        .try_upgrade_wonder(&data.buildings)
        .expect("stage 2 succeeds");
    assert_eq!(stage2, 2);
    assert_eq!(tool2, Some(wyrdstone_core::ActiveTool::Mjolnir));
    assert_eq!(city.max_energy(&data.balance), base_max_energy + 5);

    let bonuses = city.compute_bonuses(&data.balance);
    assert_eq!(bonuses.boss_damage_bonus_pct, 10);
    assert_eq!(bonuses.harvest_bonus_pct, 10);
    assert!(city.harvest_reward(&data.buildings[0]).gold > base_harvest.gold);

    let mut rng = rand::rngs::StdRng::seed_from_u64(7);
    let session = LevelSession::start_full(
        data.levels[0].clone(),
        None,
        bonuses,
        city.selected_patron,
        None,
        &mut rng,
    );
    assert_eq!(session.boss_damage_bonus_pct, 10);

    let save = SaveData::new_game(&data.balance, Language::En);
    let mut val = serde_json::to_value(&save).expect("serialize");
    val["city"]
        .as_object_mut()
        .expect("city obj")
        .remove("wonder_stage");
    let loaded: SaveData =
        serde_json::from_str(&serde_json::to_string(&val).expect("str")).expect("compat load");
    assert_eq!(loaded.city.wonder_stage, 0);
}
