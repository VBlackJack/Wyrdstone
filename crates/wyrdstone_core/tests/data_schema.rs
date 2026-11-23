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

//! Schema compatibility tests: a `balance.ron` written before the tuning fields existed must keep
//! loading, and its serde defaults must equal the values written explicitly in the bundled file.

use wyrdstone_core::balance::{BalanceConfig, BuildingBonusKind};
use wyrdstone_core::{ActiveTool, ExpeditionBoon, GameDataBundle, Language, ResourceBundle};

/// The `balance.ron` text as it was before the gameplay tuning fields were added.
const LEGACY_BALANCE_RON: &str = r#"// Copyright 2026 Julien Bombled - Apache 2.0
BalanceConfig(
    default_board_cols: 8,
    default_board_rows: 8,
    points_per_tile: 50,
    combo_multiplier_step: 0.5,
    booster_activation_bonus: 250,
    ice_break_bonus: 120,
    chain_break_bonus: 150,
    crate_break_bonus: 140,
    relic_drop_bonus: 350,
    remaining_move_bonus: 200,
    max_energy: 50,
    energy_cost_per_level: 5,
    energy_regen_seconds: 15,
    mead_refill_cost: 10,
    mead_refill_energy: 15,
    starting_wood: 40,
    starting_stone: 30,
    starting_gold: 50,
    starting_mead: 10,
    starting_mjolnir: 3,
    starting_surtr_ember: 2,
    starting_loki_shuffle: 3,
    starting_thor_bolt: 2,
    starting_bifrost_prism: 1,
    starting_drakkar_charge: 1,
    swap_anim_seconds: 0.18,
    clear_anim_seconds: 0.28,
    fall_anim_seconds: 0.24,
    swipe_threshold_px: 24.0,
    great_hall_energy_per_tier: 10,
    forge_score_pct_per_tier: 10,
    dock_resource_pct_per_tier: 15,
    shrine_moves_per_tier: 1,
    meadery_regen_pct_per_tier: 20,
    move_reward_bonus_pct_per_move: 3,
    move_reward_bonus_cap_pct: 30,
    victory_glory: 10,
    quest_glory: 15,
    amulet_glory: 20,
    daily_glory: 35,
    collection_glory: 40,
    harvest_interval_seconds: 120,
    watchtower_fury_pct_per_tier: 15,
    clan_rank_huscarl_glory: 100,
    clan_rank_jarl_glory: 280,
    clan_rank_high_king_glory: 600,
)
"#;

fn ron_of(config: &BalanceConfig) -> String {
    ron::to_string(config).expect("balance config serializes")
}

fn bundled_balance() -> BalanceConfig {
    GameDataBundle::load_bundled(Language::En)
        .expect("bundled data must parse")
        .balance
}

#[test]
fn legacy_balance_file_still_parses() {
    BalanceConfig::from_ron_str(LEGACY_BALANCE_RON).expect("legacy balance.ron must parse");
}

#[test]
fn serde_defaults_equal_bundled_values() {
    let legacy = BalanceConfig::from_ron_str(LEGACY_BALANCE_RON).unwrap();
    let bundled = bundled_balance();
    assert_eq!(
        ron_of(&legacy),
        ron_of(&bundled),
        "defaults for missing fields must match the values written in balance.ron"
    );
}

#[test]
fn defaults_pin_current_numbers() {
    let legacy = BalanceConfig::from_ron_str(LEGACY_BALANCE_RON).unwrap();

    assert_eq!(legacy.login_tribute_rewards.len(), 7);
    assert_eq!(
        legacy.login_tribute_rewards[2].resources,
        ResourceBundle {
            wood: 30,
            stone: 30,
            gold: 50,
            mead: 10
        }
    );
    assert_eq!(
        legacy.login_tribute_rewards[2].tool,
        Some(ActiveTool::Mjolnir)
    );
    assert_eq!(legacy.login_tribute_rewards[0].tool, None);

    assert_eq!(legacy.building_bonuses.len(), 6);
    assert_eq!(legacy.building_bonuses[0].building_id, "great_hall");
    assert_eq!(
        legacy.building_bonuses[0].kind,
        BuildingBonusKind::ExtraMaxEnergy
    );

    assert_eq!(legacy.win_streak_booster_threshold, 2);
    assert_eq!(legacy.win_streak_starting_boosters, 1);
    assert_eq!(legacy.clan_rank_huscarl_resource_bonus_pct, 5);
    assert_eq!(legacy.clan_rank_high_king_starting_fury, 30);
    assert_eq!(legacy.mastery_star_tools.len(), 5);
    assert_eq!(legacy.daily_streak_bonus_pct_per_day, 10);
    assert_eq!(legacy.daily_streak_tool_rewards.len(), 3);
    assert_eq!(legacy.expedition_glory_per_wave, 5);

    assert_eq!(legacy.session.three_star_moves_ratio, 0.30);
    assert_eq!(legacy.session.two_star_moves_ratio, 0.12);
    assert_eq!(legacy.session.thor_fury_strike_cells, 4);
    assert_eq!(legacy.session.loki_fury_boosters, 2);

    assert_eq!(legacy.daily.max_moves_range, (24, 28));
    assert_eq!(legacy.daily.glacial_frost_chance, 0.5);
    assert_eq!(legacy.daily.relic_count_range, (2, 3));

    assert_eq!(legacy.expedition.wave_one_rune_types, 5);
    assert_eq!(legacy.expedition.boons, ExpeditionBoon::ALL.to_vec());
    assert_eq!(legacy.expedition.boon_choices, 3);
    assert_eq!(legacy.expedition.curse_turns, 6);

    assert_eq!(legacy.boss.damage_per_tile, 45);
    assert_eq!(legacy.boss.combo_damage_step, 0.35);
    assert_eq!(legacy.boss.curse_turns, 5);
}

#[test]
fn bundled_float_thresholds_equal_literals() {
    let bundled = bundled_balance();
    assert_eq!(bundled.session.three_star_moves_ratio, 0.30_f32);
    assert_eq!(bundled.session.two_star_moves_ratio, 0.12_f32);
    assert_eq!(bundled.boss.combo_damage_step, 0.35_f32);
    assert_eq!(bundled.daily.glacial_frost_chance, 0.5_f64);
}

#[test]
fn partial_nested_tables_fall_back_to_defaults() {
    let patched = LEGACY_BALANCE_RON.replace(
        "clan_rank_high_king_glory: 600,",
        "clan_rank_high_king_glory: 600,\n    boss: (damage_per_tile: 99),",
    );
    let config = BalanceConfig::from_ron_str(&patched).expect("partial nested table parses");
    assert_eq!(config.boss.damage_per_tile, 99);
    assert_eq!(config.boss.damage_per_obstacle, 85);
    assert_eq!(config.boss.phase2_hp_pct, 50);
}
