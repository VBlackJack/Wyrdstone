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

//! Tests for the structural validation of the bundled game data.

use wyrdstone_core::balance::{BuildingBonusBinding, BuildingBonusKind};
use wyrdstone_core::validate::validate_bundle;
use wyrdstone_core::{GameDataBundle, Language};

fn bundle() -> GameDataBundle {
    GameDataBundle::load_bundled(Language::En).expect("bundled data must parse")
}

#[test]
fn bundled_data_passes_validation() {
    if let Err(problems) = validate_bundle(&bundle()) {
        panic!("bundled data must validate:\n{problems}");
    }
}

#[test]
fn renamed_building_id_is_reported_by_name() {
    let mut b = bundle();
    let target = b
        .buildings
        .iter_mut()
        .find(|x| x.id == "forge")
        .expect("forge exists");
    target.id = "forge_renamed".to_string();
    let err = validate_bundle(&b).expect_err("a dangling building reference must fail");
    assert!(err.contains("forge"), "message must name the id: {err}");
}

#[test]
fn duplicate_building_id_is_reported() {
    let mut b = bundle();
    let dup = b.buildings[0].clone();
    b.buildings.push(dup);
    let err = validate_bundle(&b).expect_err("duplicate ids must fail");
    assert!(err.contains("duplicate id 'great_hall'"), "{err}");
}

#[test]
fn level_with_unknown_building_is_reported() {
    let mut b = bundle();
    b.levels[0].building_id = "nowhere".to_string();
    let err = validate_bundle(&b).expect_err("unknown building must fail");
    assert!(err.contains("nowhere"), "{err}");
}

#[test]
fn login_tribute_with_six_days_fails() {
    let mut b = bundle();
    b.balance.login_tribute_rewards.pop();
    let err = validate_bundle(&b).expect_err("six tribute days must fail");
    assert!(err.contains("login_tribute_rewards"), "{err}");
}

#[test]
fn inverted_move_range_fails() {
    let mut b = bundle();
    b.balance.daily.max_moves_range = (30, 20);
    let err = validate_bundle(&b).expect_err("an inverted range must fail");
    assert!(err.contains("max_moves_range"), "{err}");
}

#[test]
fn unknown_building_bonus_entry_fails() {
    let mut b = bundle();
    b.balance.building_bonuses.push(BuildingBonusBinding {
        building_id: "nope".to_string(),
        kind: BuildingBonusKind::ExtraMoves,
    });
    let err = validate_bundle(&b).expect_err("a bonus for an unknown building must fail");
    assert!(err.contains("nope"), "{err}");
}

#[test]
fn duplicate_building_bonus_entry_fails() {
    let mut b = bundle();
    let first = b.balance.building_bonuses[0].clone();
    b.balance.building_bonuses.push(first);
    let err = validate_bundle(&b).expect_err("a duplicate bonus binding must fail");
    assert!(err.contains("duplicate building id"), "{err}");
}

#[test]
fn non_ascending_clan_rank_thresholds_fail() {
    let mut b = bundle();
    b.balance.clan_rank_jarl_glory = b.balance.clan_rank_huscarl_glory;
    let err = validate_bundle(&b).expect_err("equal thresholds must fail");
    assert!(err.contains("strictly ascending"), "{err}");
}

#[test]
fn star_ratios_out_of_order_fail() {
    let mut b = bundle();
    b.balance.session.two_star_moves_ratio = 0.5;
    b.balance.session.three_star_moves_ratio = 0.3;
    assert!(validate_bundle(&b).is_err());
}

#[test]
fn expedition_limits_fail_when_out_of_range() {
    let mut b = bundle();
    b.balance.expedition.wave_one_rune_types = 2;
    assert!(validate_bundle(&b).is_err());

    let mut b = bundle();
    b.balance.expedition.wave_one_rune_types = 7;
    assert!(validate_bundle(&b).is_err());

    let mut b = bundle();
    b.balance.expedition.boon_choices = 0;
    assert!(validate_bundle(&b).is_err());

    let mut b = bundle();
    b.balance.expedition.boon_choices = b.balance.expedition.boons.len() + 1;
    assert!(validate_bundle(&b).is_err());
}

#[test]
fn empty_mastery_ladder_fails() {
    let mut b = bundle();
    b.balance.mastery_star_tools.clear();
    let err = validate_bundle(&b).expect_err("an empty ladder must fail");
    assert!(err.contains("mastery_star_tools"), "{err}");
}

#[test]
fn daily_streak_tool_threshold_out_of_cycle_fails() {
    let mut b = bundle();
    b.balance.daily_streak_tool_rewards[0].min_streak = 0;
    assert!(validate_bundle(&b).is_err());

    let mut b = bundle();
    b.balance.daily_streak_tool_rewards[0].min_streak = 8;
    assert!(validate_bundle(&b).is_err());
}

#[test]
fn collection_relic_lists_must_align() {
    let mut b = bundle();
    b.collections[0].relic_name_keys.pop();
    let err = validate_bundle(&b).expect_err("misaligned relic lists must fail");
    assert!(err.contains("relic"), "{err}");
}

#[test]
fn all_problems_are_collected_in_one_pass() {
    let mut b = bundle();
    b.balance.daily.max_moves_range = (30, 20);
    b.balance.login_tribute_rewards.pop();
    b.balance.mastery_star_tools.clear();
    let err = validate_bundle(&b).expect_err("several problems must fail");
    assert!(err.lines().count() >= 3, "{err}");
}
