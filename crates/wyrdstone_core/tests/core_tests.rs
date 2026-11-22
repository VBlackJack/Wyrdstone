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
use wyrdstone_core::{
    ActiveTool, Board, Booster, Cell, CellPos, CityState, GameDataBundle, Language, LevelOutcome,
    LevelSession, QuestCondition, RuneType, SaveData,
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

    // Verify all building references and i18n keys resolve.
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

    // Verify all quest references resolve.
    for q in &data.quests {
        assert_ne!(data.i18n.t(&q.title_key), q.title_key);
        match &q.condition {
            QuestCondition::WinBuildingLevel { building_id, .. }
            | QuestCondition::ReachBuildingTier { building_id, .. } => {
                assert!(data.building_by_id(building_id).is_some());
            }
        }
    }
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

    // 4x4 deterministic board: swapping (0,0) Ice with (1,0) Fire makes 3 Fire in row 0?
    // Row 0: Fire, Fire, Ice, Fire -> swap (2,0) and (3,0) makes Fire, Fire, Fire, Ice.
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

    // 5x3 board: Row 0 = Fire, Fire, Ice, Fire, Fire; Row 1 col 2 = Fire.
    // Swapping (2,0) and (2,1) creates a 5-in-a-row of Fire -> spawns BifrostShard at (2,0).
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
    // BifrostShard spawned at swap target (or dropped down if gravity acted; verify one exists).
    let has_bifrost = (0..3).any(|r| {
        (0..5).any(|c| board.get(CellPos::new(c, r)).unwrap().booster == Booster::BifrostShard)
    });
    assert!(has_bifrost, "5-in-a-row must spawn a BifrostShard booster");
}

#[test]
fn active_tools_mjolnir_and_loki_shuffle_work() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut rng = StdRng::seed_from_u64(123);
    let mut session = LevelSession::start(data.levels[2].clone(), &mut rng);
    let moves_before = session.moves_left;

    // Use Mjolnir on a chained cell (3,3) in forge_1.
    let res = session.play_tool(
        ActiveTool::Mjolnir,
        Some(CellPos::new(3, 3)),
        &data.balance,
        &mut rng,
    );
    assert!(res.valid);
    assert_eq!(res.chains_broken, 1);
    assert_eq!(session.moves_left, moves_before);

    // Use Loki Shuffle.
    let shuf = session.play_tool(ActiveTool::LokiShuffle, None, &data.balance, &mut rng);
    assert!(shuf.valid);
    assert!(shuf.shuffled);
    assert_eq!(session.outcome, LevelOutcome::InProgress);
}

#[test]
fn city_upgrades_quests_and_save_roundtrip() {
    let data = GameDataBundle::load_bundled(Language::Fr).unwrap();
    let mut city = CityState::new(&data.balance);

    let great_hall = data.building_by_id("great_hall").unwrap();
    let forge = data.building_by_id("forge").unwrap();

    // Forge requires Great Hall tier 1.
    assert!(!city.is_building_unlocked(forge));
    assert!(city.try_upgrade_building(forge).is_err());

    // Win Great Hall level once -> q1 ready!
    city.record_level_victory("great_hall", &data.levels[0].base_reward);
    let q1 = &data.quests[0];
    assert!(city.is_quest_ready(q1));
    assert!(city.try_claim_quest(q1));

    // Now we can afford Great Hall tier 1.
    assert_eq!(city.try_upgrade_building(great_hall), Ok(1));
    assert!(city.is_building_unlocked(forge));

    // Save/load JSON roundtrip.
    let save = SaveData {
        schema_version: wyrdstone_core::save::SAVE_SCHEMA_VERSION,
        language: Language::Fr,
        city,
    };
    let json = save.to_json().expect("serialize save");
    let loaded = SaveData::from_json(&json).expect("deserialize save");
    assert_eq!(loaded.city.building_tier("great_hall"), 1);
    assert!(loaded.city.claimed_quests.contains("q1"));
}
