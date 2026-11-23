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

//! Golden tests pinning the current gameplay numbers.
//!
//! These tests only use the legacy, signature-stable public APIs. They are frozen: later refactors
//! that move tunables into data must keep every assertion here passing unchanged.

use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::HashSet;
use wyrdstone_core::{
    ActiveTool, AmuletDef, AmuletEffect, Board, Booster, BossRiposteKind, BossState, CellPos,
    CityState, ClanRank, DailyBlessing, ExpeditionBoon, ExpeditionRun, GameDataBundle, Language,
    LevelDef, LevelOutcome, LevelSession, MatchStepOutcome, Objective, PatronGod, ResourceBundle,
    RuneType, ToolInventory, FURY_CHARGE_MAX,
};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Folds the RON serialization of a value into a running FNV-1a digest.
fn fold_digest<T: serde::Serialize>(digest: u64, value: &T) -> u64 {
    let text = ron::to_string(value).expect("value must serialize to RON");
    let mut hash = digest;
    for &b in text.as_bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    // Separator so concatenated values cannot collide by shifting boundaries.
    hash ^= 0xff;
    hash.wrapping_mul(FNV_PRIME)
}

fn data() -> GameDataBundle {
    GameDataBundle::load_bundled(Language::En).expect("bundled data must parse")
}

fn rb(wood: u32, stone: u32, gold: u32, mead: u32) -> ResourceBundle {
    ResourceBundle {
        wood,
        stone,
        gold,
        mead,
    }
}

fn zero_tools() -> ToolInventory {
    ToolInventory::default()
}

fn tools_delta(before: &ToolInventory, after: &ToolInventory) -> [i64; 6] {
    let f = |a: u32, b: u32| i64::from(b) - i64::from(a);
    [
        f(before.mjolnir, after.mjolnir),
        f(before.surtr_ember, after.surtr_ember),
        f(before.loki_shuffle, after.loki_shuffle),
        f(before.thor_bolt, after.thor_bolt),
        f(before.bifrost_prism, after.bifrost_prism),
        f(before.drakkar_charge, after.drakkar_charge),
    ]
}

// ---------------------------------------------------------------------------
// 1. Login tribute
// ---------------------------------------------------------------------------

#[test]
fn golden_login_tribute_table() {
    let expected: [(u8, ResourceBundle, Option<ActiveTool>); 7] = [
        (1, rb(35, 20, 30, 0), None),
        (2, rb(25, 40, 35, 10), None),
        (3, rb(30, 30, 50, 10), Some(ActiveTool::Mjolnir)),
        (4, rb(60, 55, 60, 15), None),
        (5, rb(50, 50, 75, 15), Some(ActiveTool::ThorBolt)),
        (6, rb(75, 75, 95, 25), None),
        (7, rb(100, 100, 140, 35), Some(ActiveTool::BifrostPrism)),
    ];
    for (day, res, tool) in expected {
        assert_eq!(
            CityState::login_tribute_reward(day),
            (res, tool),
            "login tribute day {day}"
        );
    }
    let day7 = CityState::login_tribute_reward(7);
    assert_eq!(
        CityState::login_tribute_reward(0),
        day7,
        "day 0 maps to day 7"
    );
    assert_eq!(
        CityState::login_tribute_reward(8),
        day7,
        "day 8 maps to day 7"
    );
    assert_eq!(CityState::login_tribute_reward(200), day7);
}

#[test]
fn golden_login_tribute_eight_consecutive_claims_cycle() {
    let d = data();
    let mut city = CityState::new(&d.balance);
    let mut days = Vec::new();
    for offset in 0..8u64 {
        let (day, _, _) = city
            .try_claim_login_tribute(100 + offset)
            .expect("each new day can be claimed");
        days.push(day);
    }
    assert_eq!(days, vec![1, 2, 3, 4, 5, 6, 7, 1]);
    // Claiming the same day twice is refused.
    assert!(city.try_claim_login_tribute(107).is_none());
}

#[test]
fn golden_login_tribute_claim_grants_resources_and_tool() {
    let d = data();
    let mut city = CityState::new(&d.balance);
    city.resources = ResourceBundle::default();
    city.tools = zero_tools();
    for offset in 0..3u64 {
        city.try_claim_login_tribute(50 + offset);
    }
    // Days 1..=3 summed: wood 35+25+30, stone 20+40+30, gold 30+35+50, mead 0+10+10.
    assert_eq!(city.resources, rb(90, 90, 115, 20));
    assert_eq!(city.tools.mjolnir, 1);
    assert_eq!(city.tools.thor_bolt, 0);
}

// ---------------------------------------------------------------------------
// 2. Daily reward scaling
// ---------------------------------------------------------------------------

#[test]
fn golden_scale_daily_rewards() {
    let base = rb(100, 100, 100, 100);
    for (streak, expected) in [(0u32, 100u32), (1, 100), (2, 110), (3, 120), (7, 160)] {
        assert_eq!(
            CityState::scale_daily_rewards(&base, streak),
            rb(expected, expected, expected, expected),
            "streak {streak}"
        );
    }
    assert_eq!(
        CityState::scale_daily_rewards(&rb(7, 7, 7, 7), 2),
        rb(7, 7, 7, 7),
        "integer truncation at small amounts"
    );
}

// ---------------------------------------------------------------------------
// 3. Daily victory streak rewards
// ---------------------------------------------------------------------------

#[test]
fn golden_record_daily_victory_streak_tools_and_glory() {
    let d = data();
    let mut city = CityState::new(&d.balance);
    city.tools = zero_tools();
    city.glory = 0;
    let zero = ResourceBundle::default();
    let mut results = Vec::new();
    for day_index in 10..18u64 {
        let before = city.tools;
        let glory_before = city.glory;
        let streak = city.record_daily_victory(day_index, &zero, &d.balance);
        results.push((
            streak,
            tools_delta(&before, &city.tools),
            city.glory - glory_before,
        ));
    }
    let expected: Vec<(u32, [i64; 6], u32)> = vec![
        (1, [0, 0, 0, 0, 1, 0], 35),
        (2, [0, 0, 0, 0, 1, 0], 35),
        (3, [0, 1, 0, 0, 1, 0], 35),
        (4, [0, 1, 0, 0, 1, 0], 35),
        (5, [0, 1, 0, 0, 1, 0], 35),
        (6, [0, 1, 0, 0, 1, 0], 35),
        (7, [0, 1, 0, 0, 1, 1], 35),
        (1, [0, 0, 0, 0, 1, 0], 35),
    ];
    assert_eq!(results, expected);
    assert_eq!(city.stats.daily_completions, 8);
}

#[test]
fn golden_record_daily_victory_scales_rewards_by_streak() {
    let d = data();
    let mut city = CityState::new(&d.balance);
    city.resources = ResourceBundle::default();
    let reward = rb(100, 100, 100, 100);
    city.record_daily_victory(10, &reward, &d.balance);
    assert_eq!(city.resources, rb(100, 100, 100, 100));
    city.record_daily_victory(11, &reward, &d.balance);
    assert_eq!(city.resources, rb(210, 210, 210, 210));
    city.record_daily_victory(12, &reward, &d.balance);
    assert_eq!(city.resources, rb(330, 330, 330, 330));
}

// ---------------------------------------------------------------------------
// 4. Starting boosters from the win streak
// ---------------------------------------------------------------------------

#[test]
fn golden_starting_boosters_by_win_streak() {
    let d = data();
    let mut city = CityState::new(&d.balance);
    for (streak, expected) in [(0u32, 0u32), (1, 0), (2, 1), (10, 1)] {
        city.stats.win_streak = streak;
        assert_eq!(
            city.compute_bonuses(&d.balance).starting_boosters,
            expected,
            "win streak {streak}"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Clan ranks
// ---------------------------------------------------------------------------

#[test]
fn golden_clan_rank_perks_by_glory() {
    let d = data();
    let mut city = CityState::new(&d.balance);
    let cases: [(u32, ClanRank, u32, u32); 7] = [
        (0, ClanRank::Scout, 0, 0),
        (99, ClanRank::Scout, 0, 0),
        (100, ClanRank::Huscarl, 5, 10),
        (279, ClanRank::Huscarl, 5, 10),
        (280, ClanRank::Jarl, 10, 20),
        (599, ClanRank::Jarl, 10, 20),
        (600, ClanRank::HighKing, 15, 30),
    ];
    for (glory, rank, spoils, fury) in cases {
        city.glory = glory;
        assert_eq!(
            city.clan_rank_with_balance(&d.balance),
            rank,
            "glory {glory}"
        );
        assert_eq!(
            city.clan_rank(),
            rank,
            "default thresholds at glory {glory}"
        );
        let bonuses = city.compute_bonuses(&d.balance);
        assert_eq!(
            bonuses.resource_bonus_pct, spoils,
            "spoils at glory {glory}"
        );
        assert_eq!(bonuses.starting_fury, fury, "fury at glory {glory}");
        assert_eq!(rank.resource_bonus_pct(), spoils);
        assert_eq!(rank.starting_fury(), fury);
    }
}

// ---------------------------------------------------------------------------
// 6. Building bonuses
// ---------------------------------------------------------------------------

#[test]
fn golden_building_bonuses_by_tier() {
    let d = data();
    for tier in 1..=3u32 {
        let mut city = CityState::new(&d.balance);
        city.building_tiers.insert("great_hall".to_string(), tier);
        assert_eq!(city.compute_bonuses(&d.balance).extra_max_energy, 10 * tier);
        assert_eq!(city.max_energy(&d.balance), 50 + 10 * tier);

        let mut city = CityState::new(&d.balance);
        city.building_tiers.insert("forge".to_string(), tier);
        assert_eq!(city.compute_bonuses(&d.balance).score_bonus_pct, 10 * tier);

        let mut city = CityState::new(&d.balance);
        city.building_tiers
            .insert("longship_dock".to_string(), tier);
        assert_eq!(
            city.compute_bonuses(&d.balance).resource_bonus_pct,
            15 * tier
        );

        let mut city = CityState::new(&d.balance);
        city.building_tiers.insert("odin_shrine".to_string(), tier);
        assert_eq!(city.compute_bonuses(&d.balance).extra_moves, tier);

        let mut city = CityState::new(&d.balance);
        city.building_tiers.insert("meadery".to_string(), tier);
        assert_eq!(
            city.compute_bonuses(&d.balance).energy_regen_speed_pct,
            20 * tier
        );

        let mut city = CityState::new(&d.balance);
        city.building_tiers.insert("watchtower".to_string(), tier);
        assert_eq!(
            city.compute_bonuses(&d.balance).fury_charge_bonus_pct,
            15 * tier
        );
    }
}

#[test]
fn golden_unknown_building_grants_no_bonus() {
    let d = data();
    let mut city = CityState::new(&d.balance);
    city.building_tiers.insert("foo".to_string(), 3);
    let bonuses = city.compute_bonuses(&d.balance);
    assert_eq!(
        bonuses,
        CityState::new(&d.balance).compute_bonuses(&d.balance)
    );
    assert_eq!(bonuses.extra_max_energy, 0);
    assert_eq!(bonuses.score_bonus_pct, 0);
    assert_eq!(bonuses.resource_bonus_pct, 0);
    assert_eq!(bonuses.extra_moves, 0);
    assert_eq!(bonuses.energy_regen_speed_pct, 0);
    assert_eq!(bonuses.fury_charge_bonus_pct, 0);
}

// ---------------------------------------------------------------------------
// 7. Upgrade perks
// ---------------------------------------------------------------------------

fn rich_city(d: &GameDataBundle) -> CityState {
    let mut city = CityState::new(&d.balance);
    city.resources = rb(100_000, 100_000, 100_000, 100_000);
    city.tools = zero_tools();
    city.building_tiers.insert("great_hall".to_string(), 3);
    city
}

fn upgrade_tool_delta(building_id: &str) -> ([i64; 6], u32, u32, u32) {
    let d = data();
    let def = d.building_by_id(building_id).expect("building exists");
    let mut city = rich_city(&d);
    if building_id == "great_hall" {
        city.building_tiers.remove("great_hall");
    }
    city.energy = 10;
    let before = city.tools;
    let glory_before = city.glory;
    city.try_upgrade_building(def, &d.balance)
        .expect("upgrade must succeed with rich resources");
    (
        tools_delta(&before, &city.tools),
        city.glory - glory_before,
        def.glory_per_tier,
        city.energy,
    )
}

#[test]
fn golden_upgrade_perks_tool_grants_and_glory() {
    let cases: [(&str, [i64; 6]); 6] = [
        ("great_hall", [0, 0, 0, 0, 0, 0]),
        ("forge", [1, 0, 0, 1, 0, 0]),
        ("longship_dock", [0, 0, 0, 0, 0, 0]),
        ("odin_shrine", [0, 0, 0, 0, 1, 1]),
        ("meadery", [0, 0, 0, 0, 0, 0]),
        ("watchtower", [0, 0, 1, 0, 1, 0]),
    ];
    for (id, expected) in cases {
        let (delta, glory_gain, glory_per_tier, _) = upgrade_tool_delta(id);
        assert_eq!(delta, expected, "tool deltas for {id}");
        assert_eq!(glory_gain, glory_per_tier, "glory for {id}");
        assert!(glory_per_tier > 0);
    }
}

#[test]
fn golden_upgrade_perk_great_hall_energy_exceeds_cap() {
    let d = data();
    let def = d.building_by_id("great_hall").unwrap();
    let mut city = CityState::new(&d.balance);
    city.resources = rb(100_000, 100_000, 100_000, 100_000);
    city.energy = 200;
    city.try_upgrade_building(def, &d.balance).unwrap();
    assert_eq!(city.energy, 210, "great hall adds 10 endurance with no cap");
    assert!(city.energy > city.max_energy(&d.balance));
}

#[test]
fn golden_upgrade_perk_meadery_refills_to_cap() {
    let d = data();
    let def = d.building_by_id("meadery").unwrap();
    let mut city = rich_city(&d);
    city.energy = 10;
    city.try_upgrade_building(def, &d.balance).unwrap();
    assert_eq!(city.energy, city.max_energy(&d.balance));
    assert_eq!(city.energy, 80);

    // Above the cap the meadery never lowers endurance.
    let mut city = rich_city(&d);
    city.energy = 500;
    city.try_upgrade_building(def, &d.balance).unwrap();
    assert_eq!(city.energy, 500);
}

#[test]
fn golden_upgrade_locked_building_is_refused() {
    let d = data();
    let def = d.building_by_id("forge").unwrap();
    let mut city = CityState::new(&d.balance);
    city.resources = rb(100_000, 100_000, 100_000, 100_000);
    assert_eq!(
        city.try_upgrade_building(def, &d.balance),
        Err("city.panel.building.locked")
    );
}

// ---------------------------------------------------------------------------
// 8. Mastery ladder
// ---------------------------------------------------------------------------

#[test]
fn golden_mastery_star_tool_ladder() {
    let d = data();
    let mut city = CityState::new(&d.balance);
    city.tools = zero_tools();
    city.glory = 0;
    let zero = ResourceBundle::default();
    let mut deltas = Vec::new();
    for _ in 0..6 {
        let before = city.tools;
        let glory_before = city.glory;
        city.record_level_victory("forge", "forge_1", 3, &zero, &d.balance);
        deltas.push((tools_delta(&before, &city.tools), city.glory - glory_before));
    }
    let expected = vec![
        ([1, 0, 0, 0, 0, 0], 10), // Mjolnir
        ([0, 1, 0, 0, 0, 0], 10), // Surtr Ember
        ([0, 0, 0, 1, 0, 0], 10), // Thor Bolt
        ([0, 0, 0, 0, 1, 0], 10), // Bifrost Prism
        ([0, 0, 0, 0, 0, 1], 10), // Drakkar Charge
        ([0, 0, 0, 0, 0, 0], 10), // ladder exhausted
    ];
    assert_eq!(deltas, expected);
    assert_eq!(city.building_mastery_stars("forge"), 5);
    assert_eq!(city.stats.total_victories, 6);
    assert_eq!(city.stats.win_streak, 6);
}

// ---------------------------------------------------------------------------
// 9. Expedition glory
// ---------------------------------------------------------------------------

#[test]
fn golden_record_expedition_wave_glory() {
    let d = data();
    for (wave, gain) in [(1u32, 15u32), (4, 30)] {
        let mut city = CityState::new(&d.balance);
        city.glory = 0;
        city.record_expedition_wave(wave, &ResourceBundle::default(), &d.balance);
        assert_eq!(city.glory, gain, "wave {wave}");
        assert_eq!(city.stats.best_expedition_wave, wave);
    }
}

// ---------------------------------------------------------------------------
// 10. Amulet scaling
// ---------------------------------------------------------------------------

fn amulet<'a>(d: &'a GameDataBundle, id: &str) -> &'a AmuletDef {
    d.amulet_by_id(id).expect("amulet exists")
}

#[test]
fn golden_amulet_effect_scaling_by_tier() {
    let d = data();
    let cases: [(&str, [AmuletEffect; 4]); 5] = [
        (
            "amulet_freya",
            [
                AmuletEffect::ResourceBonusPct(25),
                AmuletEffect::ResourceBonusPct(25),
                AmuletEffect::ResourceBonusPct(37),
                AmuletEffect::ResourceBonusPct(49),
            ],
        ),
        (
            "amulet_heimdall",
            [
                AmuletEffect::ExtraMoves(3),
                AmuletEffect::ExtraMoves(3),
                AmuletEffect::ExtraMoves(4),
                AmuletEffect::ExtraMoves(5),
            ],
        ),
        (
            "amulet_thor",
            [
                AmuletEffect::ScoreBonusPct(20),
                AmuletEffect::ScoreBonusPct(20),
                AmuletEffect::ScoreBonusPct(35),
                AmuletEffect::ScoreBonusPct(50),
            ],
        ),
        (
            "amulet_odin",
            [
                AmuletEffect::StartingBoosters(1),
                AmuletEffect::StartingBoosters(1),
                AmuletEffect::StartingBoosters(2),
                AmuletEffect::StartingBoosters(3),
            ],
        ),
        (
            "amulet_vidar",
            [
                AmuletEffect::ExtraTimePct(25),
                AmuletEffect::ExtraTimePct(25),
                AmuletEffect::ExtraTimePct(40),
                AmuletEffect::ExtraTimePct(55),
            ],
        ),
    ];
    for (id, effects) in cases {
        for (tier, expected) in effects.into_iter().enumerate() {
            assert_eq!(
                CityState::scaled_amulet_effect_for_tier(amulet(&d, id), tier as u8),
                expected,
                "{id} tier {tier}"
            );
        }
    }
}

#[test]
fn golden_amulet_next_cost_is_base_times_tier_plus_one() {
    let d = data();
    let def = amulet(&d, "amulet_freya");
    let base = def.cost;
    let mut city = CityState::new(&d.balance);
    for tier in 0..=3u8 {
        city.amulet_tiers.insert(def.id.clone(), tier);
        let expected = if tier >= 3 {
            None
        } else {
            let m = u32::from(tier) + 1;
            Some(rb(
                base.wood * m,
                base.stone * m,
                base.gold * m,
                base.mead * m,
            ))
        };
        assert_eq!(city.amulet_next_cost(def), expected, "tier {tier}");
    }
    assert_eq!(base, rb(40, 40, 90, 20));
}

// ---------------------------------------------------------------------------
// 11. Achievements
// ---------------------------------------------------------------------------

const TOTAL_BOSSES: usize = 4;

/// Returns the ids of every feat that can currently be claimed, in data order.
fn claimable_ids(city: &CityState, d: &GameDataBundle, total_bosses: usize) -> Vec<String> {
    let ids: Vec<String> = d
        .achievements
        .iter()
        .filter(|a| {
            let mut probe = city.clone();
            probe.try_claim_achievement(&a.id, &d.achievements, &d.buildings, total_bosses)
        })
        .map(|a| a.id.clone())
        .collect();
    assert_eq!(
        city.claimable_achievement_count(&d.achievements, &d.buildings, total_bosses),
        ids.len(),
        "claimable_achievement_count must agree with try_claim_achievement"
    );
    ids
}

fn fresh_city(d: &GameDataBundle) -> CityState {
    CityState::new(&d.balance)
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn golden_achievements_fresh_city_has_nothing() {
    let d = data();
    assert!(claimable_ids(&fresh_city(&d), &d, TOTAL_BOSSES).is_empty());
}

#[test]
fn golden_achievement_total_victories_thresholds() {
    let d = data();
    for (victories, expected) in [
        (0u32, ids(&[])),
        (1, ids(&["ach_first_win"])),
        (9, ids(&["ach_first_win"])),
        (10, ids(&["ach_first_win", "ach_veteran"])),
    ] {
        let mut city = fresh_city(&d);
        city.stats.total_victories = victories;
        assert_eq!(
            claimable_ids(&city, &d, TOTAL_BOSSES),
            expected,
            "victories {victories}"
        );
    }
}

#[test]
fn golden_achievement_best_combo_thresholds() {
    let d = data();
    for (combo, expected) in [
        (3u32, ids(&[])),
        (4, ids(&["ach_combo4"])),
        (5, ids(&["ach_combo4", "ach_combo5"])),
    ] {
        let mut city = fresh_city(&d);
        city.stats.best_combo = combo;
        assert_eq!(
            claimable_ids(&city, &d, TOTAL_BOSSES),
            expected,
            "combo {combo}"
        );
    }
}

#[test]
fn golden_achievement_runes_cleared_thresholds() {
    let d = data();
    for (runes, expected) in [
        (499u64, ids(&[])),
        (500, ids(&["ach_runes_500"])),
        (1999, ids(&["ach_runes_500"])),
        (2000, ids(&["ach_runes_500", "ach_runes_2000"])),
    ] {
        let mut city = fresh_city(&d);
        city.stats.total_runes_cleared = runes;
        assert_eq!(
            claimable_ids(&city, &d, TOTAL_BOSSES),
            expected,
            "runes {runes}"
        );
    }
}

#[test]
fn golden_achievement_first_boss_by_counter_or_set() {
    let d = data();
    let mut city = fresh_city(&d);
    city.stats.bosses_defeated = 1;
    assert_eq!(
        claimable_ids(&city, &d, TOTAL_BOSSES),
        ids(&["ach_first_boss"])
    );

    let mut city = fresh_city(&d);
    city.defeated_bosses.insert("boss_fenrir".to_string());
    assert_eq!(
        claimable_ids(&city, &d, TOTAL_BOSSES),
        ids(&["ach_first_boss"])
    );
}

#[test]
fn golden_achievement_all_bosses() {
    let d = data();
    let boss_ids: Vec<String> = d.bosses.iter().map(|b| b.id.clone()).collect();
    assert_eq!(boss_ids.len(), TOTAL_BOSSES);

    let mut city = fresh_city(&d);
    for id in boss_ids.iter().take(TOTAL_BOSSES - 1) {
        city.defeated_bosses.insert(id.clone());
    }
    assert_eq!(
        claimable_ids(&city, &d, TOTAL_BOSSES),
        ids(&["ach_first_boss"])
    );

    city.defeated_bosses
        .insert(boss_ids[TOTAL_BOSSES - 1].clone());
    assert_eq!(
        claimable_ids(&city, &d, TOTAL_BOSSES),
        ids(&["ach_first_boss", "ach_all_bosses"])
    );

    // A boss total of zero can never be satisfied.
    assert_eq!(claimable_ids(&city, &d, 0), ids(&["ach_first_boss"]));
    let mut probe = city.clone();
    assert!(!probe.try_claim_achievement("ach_all_bosses", &d.achievements, &d.buildings, 0));
}

#[test]
fn golden_achievement_expedition_waves() {
    let d = data();
    for (wave, expected) in [
        (2u32, ids(&[])),
        (3, ids(&["ach_expedition_3"])),
        (4, ids(&["ach_expedition_3"])),
        (5, ids(&["ach_expedition_3", "ach_expedition_5"])),
    ] {
        let mut city = fresh_city(&d);
        city.stats.best_expedition_wave = wave;
        assert_eq!(
            claimable_ids(&city, &d, TOTAL_BOSSES),
            expected,
            "wave {wave}"
        );
    }
}

#[test]
fn golden_achievement_master_builder_needs_every_building() {
    let d = data();
    let mut city = fresh_city(&d);
    for b in d.buildings.iter().take(d.buildings.len() - 1) {
        city.building_tiers.insert(b.id.clone(), 1);
    }
    assert!(claimable_ids(&city, &d, TOTAL_BOSSES).is_empty());
    let last = d.buildings.last().unwrap();
    city.building_tiers.insert(last.id.clone(), 1);
    assert_eq!(
        claimable_ids(&city, &d, TOTAL_BOSSES),
        ids(&["ach_master_builder"])
    );
}

#[test]
fn golden_achievement_daily_streak() {
    let d = data();
    for (streak, expected) in [
        (2u32, ids(&[])),
        (3, ids(&["ach_daily_streak_3"])),
        (7, ids(&["ach_daily_streak_3"])),
    ] {
        let mut city = fresh_city(&d);
        city.stats.daily_streak = streak;
        assert_eq!(
            claimable_ids(&city, &d, TOTAL_BOSSES),
            expected,
            "streak {streak}"
        );
    }
}

#[test]
fn golden_achievement_claim_pays_reward_and_glory_once() {
    let d = data();
    let def = d
        .achievements
        .iter()
        .find(|a| a.id == "ach_first_win")
        .unwrap();
    let mut city = fresh_city(&d);
    city.stats.total_victories = 1;
    city.resources = ResourceBundle::default();
    city.glory = 0;
    assert!(city.try_claim_achievement(
        "ach_first_win",
        &d.achievements,
        &d.buildings,
        TOTAL_BOSSES
    ));
    assert_eq!(city.resources, def.reward);
    assert_eq!(city.glory, def.glory);
    assert!(!city.try_claim_achievement(
        "ach_first_win",
        &d.achievements,
        &d.buildings,
        TOTAL_BOSSES
    ));
    assert_eq!(city.glory, def.glory, "no double payout");
}

#[test]
fn golden_achievement_unknown_id_never_unlocks() {
    let d = data();
    let mut city = fresh_city(&d);
    city.stats.total_victories = 1000;
    city.stats.best_combo = 1000;
    assert!(!city.try_claim_achievement(
        "ach_does_not_exist",
        &d.achievements,
        &d.buildings,
        TOTAL_BOSSES
    ));
    assert!(city.claimed_achievements.is_empty());
}

// ---------------------------------------------------------------------------
// 12 / 13 / 14. Level session: stars, blessings, patron fury
// ---------------------------------------------------------------------------

fn session_on(level_id: &str, seed: u64) -> LevelSession {
    let d = data();
    let level = d.level_by_id(level_id).expect("level exists").clone();
    let mut rng = StdRng::seed_from_u64(seed);
    LevelSession::start(level, &mut rng)
}

#[test]
fn golden_earned_stars() {
    let mut session = session_on("forge_1", 1);
    session.outcome = LevelOutcome::Victory;
    for (moves_left, initial, expected) in [
        (3u32, 10u32, 3u8),
        (29, 100, 2),
        (12, 100, 2),
        (11, 100, 1),
        (0, 10, 1),
        (0, 0, 1),
    ] {
        session.initial_moves = initial;
        session.moves_left = moves_left;
        assert_eq!(
            session.earned_stars(),
            expected,
            "moves_left {moves_left} of {initial}"
        );
    }
    session.outcome = LevelOutcome::Defeat;
    assert_eq!(session.earned_stars(), 0);
}

#[test]
fn golden_daily_blessings() {
    for blessing in DailyBlessing::ALL {
        let mut session = session_on("forge_1", 2);
        let fury = session.fury_charge_bonus_pct;
        let res = session.resource_bonus_pct;
        let boss = session.boss_damage_bonus_pct;
        let initial = session.initial_moves;
        let left = session.moves_left;
        session.apply_daily_blessing(blessing);
        assert_eq!(session.daily_blessing, Some(blessing));
        let (df, dr, db, di, dl) = (
            session.fury_charge_bonus_pct - fury,
            session.resource_bonus_pct - res,
            session.boss_damage_bonus_pct - boss,
            session.initial_moves - initial,
            session.moves_left - left,
        );
        let expected = match blessing {
            DailyBlessing::OdinFavor => (20, 0, 0, 0, 0),
            DailyBlessing::NjordBreeze => (0, 15, 0, 0, 0),
            DailyBlessing::FenrirEclipse => (0, 0, 25, 0, 0),
            DailyBlessing::IdunnGrace => (0, 0, 0, 1, 1),
        };
        assert_eq!((df, dr, db, di, dl), expected, "{blessing:?}");
    }
}

#[test]
fn golden_blessing_for_day_cycles_through_all() {
    for day in 0..12u64 {
        assert_eq!(
            DailyBlessing::for_day(day),
            DailyBlessing::ALL[(day % 4) as usize]
        );
    }
}

fn count_boosters(board: &Board, booster: Booster) -> usize {
    let mut n = 0;
    for row in 0..board.rows() {
        for col in 0..board.cols() {
            if board.get(CellPos::new(col, row)).unwrap().booster == booster {
                n += 1;
            }
        }
    }
    n
}

fn fury_session(patron: PatronGod, seed: u64) -> (LevelSession, StdRng) {
    let d = data();
    let level = d.level_by_id("forge_1").unwrap().clone();
    assert!(!level.ice_cells.is_empty());
    let mut rng = StdRng::seed_from_u64(seed);
    let mut session =
        LevelSession::start_full(level, None, Default::default(), patron, None, &mut rng);
    session.fury = FURY_CHARGE_MAX;
    (session, rng)
}

#[test]
fn golden_patron_fury_requires_full_gauge() {
    let (mut session, mut rng) = fury_session(PatronGod::Freya, 5);
    session.fury = FURY_CHARGE_MAX - 1;
    let (clear, spawned) = session.trigger_patron_fury_detailed(&mut rng);
    assert!(clear.is_none());
    assert!(spawned.is_empty());
    assert_eq!(session.fury_resource_bonus_pct, 0);
}

#[test]
fn golden_patron_fury_freya_adds_spoils() {
    let (mut session, mut rng) = fury_session(PatronGod::Freya, 5);
    let (clear, spawned) = session.trigger_patron_fury_detailed(&mut rng);
    assert_eq!(session.fury_resource_bonus_pct, 25);
    assert_eq!(session.fury, 0);
    assert!(spawned.is_empty());
    let clear = clear.expect("freya clears the most frequent rune");
    assert!(clear.is_direct_blast);
    assert!(!clear.cells.is_empty());
}

#[test]
fn golden_patron_fury_odin_adds_moves_and_one_bomb() {
    let (mut session, mut rng) = fury_session(PatronGod::Odin, 5);
    let moves_before = session.moves_left;
    let bombs_before = count_boosters(&session.board, Booster::Bomb);
    let (clear, spawned) = session.trigger_patron_fury_detailed(&mut rng);
    assert!(clear.is_none());
    assert_eq!(session.moves_left, moves_before + 2);
    assert_eq!(spawned.len(), 1);
    assert_eq!(
        count_boosters(&session.board, Booster::Bomb),
        bombs_before + 1
    );
}

#[test]
fn golden_patron_fury_thor_clears_up_to_four_cells() {
    let (mut session, mut rng) = fury_session(PatronGod::Thor, 5);
    let mut candidates = 0usize;
    for row in 0..session.board.rows() {
        for col in 0..session.board.cols() {
            let c = session.board.get(CellPos::new(col, row)).unwrap();
            if c.blocked || c.relic {
                continue;
            }
            if c.crate_layers > 0
                || c.chained
                || c.ice_layers > 0
                || c.curse_turns > 0
                || c.rune.is_some()
            {
                candidates += 1;
            }
        }
    }
    assert!(candidates > 4);
    let (clear, spawned) = session.trigger_patron_fury_detailed(&mut rng);
    assert!(spawned.is_empty());
    let clear = clear.expect("thor strikes");
    assert_eq!(clear.cells.len(), candidates.min(4));
    assert_eq!(clear.cells.len(), 4);
    let unique: HashSet<CellPos> = clear.cells.iter().copied().collect();
    assert_eq!(unique.len(), 4);
}

#[test]
fn golden_patron_fury_loki_spawns_two_boosters() {
    let (mut session, mut rng) = fury_session(PatronGod::Loki, 5);
    let (clear, spawned) = session.trigger_patron_fury_detailed(&mut rng);
    assert!(clear.is_none());
    assert_eq!(spawned.len(), 2);
    for pos in &spawned {
        assert_ne!(session.board.get(*pos).unwrap().booster, Booster::None);
    }
}

// ---------------------------------------------------------------------------
// 15. Daily challenge generator
// ---------------------------------------------------------------------------

const DAILY_DIGEST_DAYS: u64 = 730;
const DAILY_DIGEST_EXPECTED: u64 = 0x86c6_95ab_627e_8f48;

#[test]
fn golden_daily_challenge_digest() {
    let mut digest = FNV_OFFSET;
    for day in 0..=DAILY_DIGEST_DAYS {
        digest = fold_digest(digest, &LevelDef::daily_challenge(day));
    }
    assert_eq!(
        digest, DAILY_DIGEST_EXPECTED,
        "daily digest = {digest:#018x}"
    );
}

#[test]
fn golden_daily_challenge_pins() {
    let day0 = LevelDef::daily_challenge(0);
    assert_eq!(day0.id, "daily_0");
    assert_eq!(
        day0.objectives,
        vec![
            Objective::ReachScore { target: 5381 },
            Objective::CollectRune {
                rune: RuneType::Water,
                target: 24
            }
        ]
    );
    assert_eq!(day0.max_moves, 28);
    assert_eq!(day0.time_limit_seconds, Some(115));
    assert!(!day0.frost_spread);
    assert_eq!(day0.blocked_cells, vec![(0, 3), (0, 4), (7, 3), (7, 4)]);
    assert!(day0.relic_cells.is_empty());

    let day1 = LevelDef::daily_challenge(1);
    assert_eq!(
        day1.objectives,
        vec![
            Objective::ClearIce { target: 8 },
            Objective::ReachScore { target: 5000 }
        ]
    );
    assert_eq!(day1.max_moves, 25);
    assert_eq!(day1.time_limit_seconds, None);
    assert_eq!(day1.portals.len(), 2);
    assert_eq!(day1.blocked_cells, vec![(0, 0), (7, 0), (0, 7), (7, 7)]);

    let day100 = LevelDef::daily_challenge(100);
    assert_eq!(
        day100.objectives,
        vec![
            Objective::ReachScore { target: 5086 },
            Objective::CollectRune {
                rune: RuneType::Gold,
                target: 26
            }
        ]
    );
    assert_eq!(day100.max_moves, 27);
    assert_eq!(day100.time_limit_seconds, Some(100));
    assert_eq!(day100.blocked_cells, vec![(0, 3), (0, 4), (7, 3), (7, 4)]);

    let day20000 = LevelDef::daily_challenge(20000);
    assert_eq!(
        day20000.objectives,
        vec![
            Objective::DropRelics { target: 2 },
            Objective::ClearIce { target: 8 }
        ]
    );
    assert_eq!(day20000.max_moves, 26);
    assert_eq!(day20000.relic_cells, vec![(2, 0), (5, 0)]);
    assert!(!day20000.frost_spread);

    for day in [0u64, 1, 100, 20000] {
        let level = LevelDef::daily_challenge(day);
        assert_eq!(level.base_reward, rb(140, 140, 180, 40));
        assert_eq!(level.building_id, "daily_trial");
        assert_eq!((level.cols, level.rows, level.rune_types_count), (8, 8, 6));
    }
}

#[test]
fn golden_daily_challenge_covers_every_archetype_and_frost() {
    let mut glacial = 0;
    let mut relic = 0;
    let mut blitz = 0;
    let mut portal = 0;
    let mut frost_on = 0;
    for day in 0..=DAILY_DIGEST_DAYS {
        let level = LevelDef::daily_challenge(day);
        match (&level.objectives[0], level.time_limit_seconds.is_some()) {
            (Objective::DropRelics { .. }, _) => relic += 1,
            (_, true) => blitz += 1,
            _ if !level.portals.is_empty() => portal += 1,
            _ => glacial += 1,
        }
        if level.frost_spread {
            frost_on += 1;
        }
        assert!((24..=28).contains(&level.max_moves));
    }
    assert!(glacial > 0 && relic > 0 && blitz > 0 && portal > 0);
    assert!(frost_on > 0 && frost_on < glacial);
}

// ---------------------------------------------------------------------------
// 16. Expedition
// ---------------------------------------------------------------------------

const EXPEDITION_DIGEST_EXPECTED: u64 = 0x52bf_fa44_bd90_8f7b;
const BOON_DIGEST_EXPECTED: u64 = 0xca63_fe59_8bfc_d2e5;

fn run_at(wave: u32, extra_moves: u32) -> ExpeditionRun {
    let mut run = ExpeditionRun::new();
    run.wave = wave;
    run.extra_moves = extra_moves;
    run
}

#[test]
fn golden_expedition_wave_digest() {
    let mut digest = FNV_OFFSET;
    for wave in 1..=8u32 {
        for extra in [0u32, 20] {
            for seed in 0..20u64 {
                let mut rng = StdRng::seed_from_u64(seed);
                let level = run_at(wave, extra).generate_wave_level(&mut rng);
                digest = fold_digest(digest, &level);
            }
        }
    }
    assert_eq!(
        digest, EXPEDITION_DIGEST_EXPECTED,
        "expedition digest = {digest:#018x}"
    );
}

#[test]
fn golden_expedition_wave_pins() {
    let mut rng = StdRng::seed_from_u64(7);
    let w1 = run_at(1, 0).generate_wave_level(&mut rng);
    assert_eq!(w1.rune_types_count, 5);
    assert_eq!(w1.max_moves, 22);
    assert_eq!(w1.building_id, "expedition");
    assert_eq!(w1.id, "expedition_w1");
    assert!(w1.chained_cells.is_empty());
    assert!(!w1.frost_spread);

    let w3 = run_at(3, 0).generate_wave_level(&mut rng);
    assert_eq!(w3.rune_types_count, 6);
    assert_eq!(
        w3.objectives,
        vec![
            Objective::ClearIce { target: 12 },
            Objective::ReachScore { target: 5700 }
        ]
    );
    assert_eq!(w3.crate_cells.len(), 2);

    let w6 = run_at(6, 0).generate_wave_level(&mut rng);
    assert_eq!(w6.cursed_cells, vec![(2, 2, 6), (5, 5, 6)]);
    assert!(w6.frost_spread);
    assert_eq!(w6.max_moves, 22);

    for extra in [0u32, 3, 14, 15, 20] {
        let level = run_at(6, extra).generate_wave_level(&mut rng);
        assert_eq!(level.max_moves, (22 + extra).min(36), "extra {extra}");
    }

    for w in 1..=8u32 {
        let level = run_at(w, 0).generate_wave_level(&mut rng);
        assert_eq!(
            level.base_reward,
            rb(50 + 20 * w, 45 + 20 * w, 65 + 25 * w, 12 + 5 * w),
            "reward wave {w}"
        );
    }
}

#[test]
fn golden_expedition_boon_choices() {
    let mut digest = FNV_OFFSET;
    for seed in 0..32u64 {
        let mut run = ExpeditionRun::new();
        let mut rng = StdRng::seed_from_u64(seed);
        run.roll_boon_choices(&mut rng);
        assert_eq!(run.pending_boons.len(), 3);
        for (i, boon) in run.pending_boons.iter().enumerate() {
            assert!(ExpeditionBoon::ALL.contains(boon));
            for other in &run.pending_boons[i + 1..] {
                assert_ne!(boon, other, "boons must be distinct (seed {seed})");
            }
        }
        digest = fold_digest(digest, &run.pending_boons);
    }
    assert_eq!(digest, BOON_DIGEST_EXPECTED, "boon digest = {digest:#018x}");
}

#[test]
fn golden_expedition_apply_boon_effects() {
    let d = data();
    let mut city = CityState::new(&d.balance);
    city.tools.mjolnir = 0;

    let mut run = ExpeditionRun::new();
    run.pending_boons = vec![ExpeditionBoon::StartingBooster];
    run.apply_boon(ExpeditionBoon::ExtraMoves(3), &mut city);
    assert_eq!((run.extra_moves, run.wave), (3, 2));
    assert!(run.pending_boons.is_empty());

    run.apply_boon(ExpeditionBoon::ScoreBoostPct(25), &mut city);
    assert_eq!((run.score_bonus_pct, run.wave), (25, 3));

    run.apply_boon(ExpeditionBoon::GrantMjolnir(2), &mut city);
    assert_eq!((city.tools.mjolnir, run.wave), (2, 4));

    run.apply_boon(ExpeditionBoon::StartingBooster, &mut city);
    assert_eq!((run.starting_boosters, run.wave), (1, 5));

    let mut headstart = Vec::new();
    for _ in 0..3 {
        run.apply_boon(ExpeditionBoon::FuryHeadstart(50), &mut city);
        headstart.push(run.fury_headstart);
    }
    assert_eq!(headstart, vec![50, 100, 100]);
    assert_eq!(run.wave, 8);
}

#[test]
fn golden_expedition_boon_pool_order() {
    assert_eq!(
        ExpeditionBoon::ALL,
        [
            ExpeditionBoon::ExtraMoves(3),
            ExpeditionBoon::ScoreBoostPct(25),
            ExpeditionBoon::GrantMjolnir(2),
            ExpeditionBoon::StartingBooster,
            ExpeditionBoon::FuryHeadstart(50),
        ]
    );
}

// ---------------------------------------------------------------------------
// 17. Bosses
// ---------------------------------------------------------------------------

const BOSS_DIGEST_EXPECTED: u64 = 0x855e_7c07_4c34_e9fa;

#[test]
fn golden_boss_level_defs_digest() {
    let d = data();
    let mut digest = FNV_OFFSET;
    for boss in &d.bosses {
        digest = fold_digest(digest, &boss.to_level_def());
    }
    assert_eq!(digest, BOSS_DIGEST_EXPECTED, "boss digest = {digest:#018x}");
}

#[test]
fn golden_boss_level_obstacle_counts() {
    let d = data();
    // (ice, chains, crates, fog, blocked) per riposte kind.
    let expected = [
        ("boss_fenrir", (4usize, 4usize, 0usize, 0usize, 4usize)),
        ("boss_jormungandr", (8, 2, 0, 0, 4)),
        ("boss_surtr", (4, 2, 2, 0, 4)),
        ("boss_hel", (4, 2, 0, 6, 4)),
    ];
    for (id, counts) in expected {
        let boss = d.boss_by_id(id).expect("boss exists");
        let level = boss.to_level_def();
        assert_eq!(
            (
                level.ice_cells.len(),
                level.chained_cells.len(),
                level.crate_cells.len(),
                level.fog_cells.len(),
                level.blocked_cells.len()
            ),
            counts,
            "{id}"
        );
        assert_eq!(level.building_id, "boss_raid");
        assert_eq!((level.cols, level.rows, level.rune_types_count), (8, 8, 6));
        assert_eq!(level.max_moves, boss.max_moves);
        assert_eq!(
            level.objectives,
            vec![Objective::ReachScore {
                target: boss.max_hp
            }]
        );
    }
}

fn step_with(rune: RuneType, count: u32, combo: u32, obstacles: u32) -> MatchStepOutcome {
    let mut step = MatchStepOutcome {
        valid: true,
        max_combo: combo,
        ice_broken: obstacles,
        ..Default::default()
    };
    if count > 0 {
        step.cleared_runes.insert(rune, count);
    }
    step
}

#[test]
fn golden_boss_damage_formula() {
    let d = data();
    let def = d.boss_by_id("boss_fenrir").unwrap().clone();
    let weakness = def.weakness;

    // 3 weakness tiles, combo x3: 3 * 45 * 2 = 270, * (1 + 2 * 0.35) = 459.
    let mut boss = BossState::new(def.clone());
    let dealt = boss.apply_step_damage(&step_with(weakness, 3, 3, 0), 0);
    assert_eq!(dealt, 459);
    assert_eq!(boss.hp, def.max_hp - 459);
    assert_eq!(boss.last_damage_dealt, 459);

    // 2 obstacles, no tiles: 2 * 85 = 170.
    let mut boss = BossState::new(def.clone());
    let dealt = boss.apply_step_damage(&step_with(weakness, 0, 1, 2), 0);
    assert_eq!(dealt, 170);

    // Non-weakness tiles deal base damage only.
    let mut boss = BossState::new(def.clone());
    let off_rune = RuneType::ALL
        .iter()
        .copied()
        .find(|r| *r != weakness)
        .unwrap();
    assert_eq!(
        boss.apply_step_damage(&step_with(off_rune, 4, 1, 0), 0),
        180
    );

    // Score bonus percentage applies after rounding: 459 + 459 * 25 / 100 = 573.
    let mut boss = BossState::new(def);
    assert_eq!(
        boss.apply_step_damage(&step_with(weakness, 3, 3, 0), 25),
        573
    );
}

#[test]
fn golden_boss_phase_two_boundary() {
    let d = data();
    let def = d.boss_by_id("boss_fenrir").unwrap().clone();
    assert_eq!(def.max_hp, 4500);
    let mut boss = BossState::new(def);
    assert!(!boss.is_phase2());
    boss.hp = 2250;
    assert!(!boss.is_phase2(), "exactly half is not phase 2");
    boss.hp = 2249;
    assert!(boss.is_phase2());
    boss.hp = 0;
    assert!(!boss.is_phase2(), "a defeated boss is not in phase 2");
}

#[test]
fn golden_boss_phase_two_entry_flag() {
    let d = data();
    let def = d.boss_by_id("boss_fenrir").unwrap().clone();
    let mut boss = BossState::new(def);
    boss.hp = 2300;
    // 100 weakness-free obstacles-free tiles: deal enough to cross below half.
    let off_rune = RuneType::ALL
        .iter()
        .copied()
        .find(|r| *r != boss.def.weakness)
        .unwrap();
    boss.apply_step_damage(&step_with(off_rune, 2, 1, 0), 0); // 90 damage: 2210 < 2250
    assert!(boss.phase2_triggered);
    assert!(boss.just_entered_phase2);
    boss.apply_step_damage(&step_with(off_rune, 1, 1, 0), 0);
    assert!(!boss.just_entered_phase2);
}

#[test]
fn golden_boss_curse_riposte_sets_five_turns() {
    let d = data();
    let boss_def = d.boss_by_id("boss_hel").unwrap();
    assert_eq!(boss_def.riposte, BossRiposteKind::CurseRunes(2));
    let level = boss_def.to_level_def();
    let mut rng = StdRng::seed_from_u64(11);
    let mut board = Board::new_for_level(&level, &mut rng);
    let hit = board.apply_boss_riposte(BossRiposteKind::CurseRunes(2), &mut rng);
    assert_eq!(hit.len(), 2);
    for pos in hit {
        assert_eq!(board.get(pos).unwrap().curse_turns, 5);
    }
}

#[test]
fn golden_boss_riposte_interval_countdown() {
    let d = data();
    let def = d.boss_by_id("boss_fenrir").unwrap().clone();
    assert_eq!(def.riposte_interval, 3);
    let level = def.to_level_def();
    let mut rng = StdRng::seed_from_u64(13);
    let mut board = Board::new_for_level(&level, &mut rng);
    let mut boss = BossState::new(def);
    assert_eq!(boss.turns_until_riposte, 3);
    assert!(boss.tick_turn_and_riposte(&mut board, &mut rng).is_empty());
    assert_eq!(boss.turns_until_riposte, 2);
    assert!(boss.tick_turn_and_riposte(&mut board, &mut rng).is_empty());
    assert_eq!(boss.turns_until_riposte, 1);
    let hit = boss.tick_turn_and_riposte(&mut board, &mut rng);
    assert_eq!(hit.len(), 2, "ChainRunes(2) chains two cells");
    assert_eq!(
        boss.turns_until_riposte, 3,
        "countdown resets to the interval"
    );
    for pos in hit {
        assert!(board.get(pos).unwrap().chained);
    }
}
