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
use rand::SeedableRng;
use wyrdstone_core::save::{DEFAULT_BGM_VOLUME, DEFAULT_SFX_VOLUME, SAVE_SCHEMA_VERSION};
use wyrdstone_core::{
    ActiveTool, AmuletEffect, Board, Booster, Cell, CellPos, CityBonuses, CityState, ClanRank,
    ExpeditionBoon, ExpeditionRun, GameDataBundle, Language, LevelDef, LevelOutcome, LevelSession,
    PatronGod, QuestCondition, ResourceBundle, RuneType, SaveData, ACHIEVEMENTS, FURY_CHARGE_MAX,
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
            | QuestCondition::CompleteDailyTrials { .. } => {}
        }
    }

    for a in &data.amulets {
        assert_ne!(data.i18n.t(&a.name_key), a.name_key);
        assert_ne!(data.i18n.t(&a.desc_key), a.desc_key);
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
        muted: true,
        bgm_volume: DEFAULT_BGM_VOLUME,
        sfx_volume: DEFAULT_SFX_VOLUME,
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
    let collected = city.try_collect_harvest("great_hall", 1000, &data.balance);
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

#[test]
fn atomic_save_and_corrupt_backup() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let dir = std::env::temp_dir().join("wyrdstone_save_test");
    let path = dir.join("test_save.json");
    let _ = std::fs::remove_dir_all(&dir);

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

    let _ = std::fs::remove_dir_all(&dir);
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
        Some(AmuletEffect::StartingBooster),
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
    assert_eq!(data.bosses.len(), 3);
    for boss in &data.bosses {
        assert_ne!(data.i18n.t(&boss.name_key), boss.name_key);
        assert_ne!(data.i18n.t(&boss.title_key), boss.title_key);
        assert_ne!(data.i18n.t(&boss.desc_key), boss.desc_key);
    }
    for god in PatronGod::ALL {
        assert_ne!(data.i18n.t(god.name_key()), god.name_key());
        assert_ne!(data.i18n.t(god.desc_key()), god.desc_key());
    }
    for ach in ACHIEVEMENTS {
        assert_ne!(data.i18n.t(ach.title_key), ach.title_key);
        assert_ne!(data.i18n.t(ach.desc_key), ach.desc_key);
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
    assert!(city.has_any_claimable_achievement(&data.buildings, data.bosses.len()));
    assert!(city.try_claim_achievement("ach_first_boss", &data.buildings, data.bosses.len()));
    assert!(!city.try_claim_achievement("ach_first_boss", &data.buildings, data.bosses.len()));
}
