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
                if matches!(name_str.as_ref(), ".git" | ".claude" | "target" | "local") {
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
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
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
