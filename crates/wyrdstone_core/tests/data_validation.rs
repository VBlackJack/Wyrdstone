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

#[test]
fn load_bundled_runs_validation_and_succeeds() {
    GameDataBundle::load_bundled(Language::En).expect("bundled data must load and validate");
    GameDataBundle::load_bundled(Language::Fr).expect("bundled data must load and validate");
}

#[test]
fn reserved_ids_come_from_the_building_constants() {
    use wyrdstone_core::validate::RESERVED_BUILDING_IDS;
    use wyrdstone_core::{BOSS_RAID_BUILDING_ID, DAILY_TRIAL_BUILDING_ID, EXPEDITION_BUILDING_ID};
    assert_eq!(
        RESERVED_BUILDING_IDS,
        [
            DAILY_TRIAL_BUILDING_ID,
            BOSS_RAID_BUILDING_ID,
            EXPEDITION_BUILDING_ID
        ]
    );
}

#[test]
fn building_colliding_with_a_reserved_id_fails() {
    use wyrdstone_core::BOSS_RAID_BUILDING_ID;
    let mut b = bundle();
    let mut clone = b.buildings[1].clone();
    clone.id = BOSS_RAID_BUILDING_ID.to_string();
    b.buildings.push(clone);
    let err = validate_bundle(&b).expect_err("a reserved id used by a building must fail");
    assert!(err.contains("reserved building id"), "{err}");
    assert!(err.contains(BOSS_RAID_BUILDING_ID), "{err}");
}

#[test]
fn missing_hub_building_is_reported_by_city_validation() {
    use wyrdstone_core::HUB_BUILDING_ID;
    let mut b = bundle();
    b.buildings.retain(|x| x.id != HUB_BUILDING_ID);
    let err = validate_bundle(&b).expect_err("a missing hub must fail");
    assert!(err.contains(HUB_BUILDING_ID), "{err}");
}

/// Minimal balance document: only the required fields, every tunable falls back to serde.
const MINIMAL_BALANCE_RON: &str = "(default_board_cols: 0, default_board_rows: 0, \
points_per_tile: 0, combo_multiplier_step: 0.0, booster_activation_bonus: 0, ice_break_bonus: 0, \
chain_break_bonus: 0, crate_break_bonus: 0, relic_drop_bonus: 0, remaining_move_bonus: 0, \
max_energy: 0, energy_cost_per_level: 0, energy_regen_seconds: 0, mead_refill_cost: 0, \
mead_refill_energy: 0, starting_wood: 0, starting_stone: 0, starting_gold: 0, starting_mead: 0, \
starting_mjolnir: 0, starting_surtr_ember: 0, starting_loki_shuffle: 0, starting_thor_bolt: 0, \
starting_bifrost_prism: 0, starting_drakkar_charge: 0, swap_anim_seconds: 0.0, \
clear_anim_seconds: 0.0, fall_anim_seconds: 0.0, swipe_threshold_px: 0.0)";

#[test]
fn minimal_balance_document_yields_tuning_and_serde_defaults() {
    use wyrdstone_core::balance::{BossTuning, DailyTuning, ExpeditionTuning, SessionTuning};
    use wyrdstone_core::BalanceConfig;
    let parsed = BalanceConfig::from_ron_str(MINIMAL_BALANCE_RON).expect("minimal document parses");
    assert_eq!(parsed.session, SessionTuning::default());
    assert_eq!(parsed.daily, DailyTuning::default());
    assert_eq!(parsed.expedition, ExpeditionTuning::default());
    assert_eq!(parsed.boss, BossTuning::default());
    // Top-level serde defaults are the shipped values, not zero.
    assert!(!parsed.login_tribute_rewards.is_empty());
    assert!(!parsed.mastery_star_tools.is_empty());
    assert!(!parsed.building_bonuses.is_empty());
    assert!(parsed.victory_glory > 0);
    assert!(parsed.harvest_interval_seconds > 0);
}

#[test]
fn zero_relic_count_lower_bound_is_reported() {
    let mut b = bundle();
    b.balance.daily.relic_count_range.0 = 0;
    let err = validate_bundle(&b).expect_err("a zero relic lower bound must fail");
    assert!(err.contains("daily.relic_count_range"), "{err}");
}

#[test]
fn zero_expedition_score_wave_interval_is_reported() {
    let mut b = bundle();
    b.balance.expedition.score_wave_interval = 0;
    let err = validate_bundle(&b).expect_err("a zero score wave interval must fail");
    assert!(err.contains("expedition.score_wave_interval"), "{err}");
}

#[test]
fn validate_bundle_rejects_out_of_bounds_level_and_invalid_boss() {
    let mut b = bundle();
    b.levels[0].ice_cells.push((99, 99, 1));
    b.bosses[0].riposte_interval = 0;
    b.balance.expedition.ice_cap = 20;
    let err =
        validate_bundle(&b).expect_err("out of bounds level and zero boss interval must fail");
    assert!(err.contains("out of"), "{err}");
    assert!(err.contains("riposte_interval"), "{err}");
    assert!(err.contains("ice_cap"), "{err}");
}
