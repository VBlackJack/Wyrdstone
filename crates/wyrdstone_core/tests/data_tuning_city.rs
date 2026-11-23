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

//! Equivalence tests: the data-driven city progression matches the legacy default numbers.

use wyrdstone_core::achievements::AchievementCondition;
use wyrdstone_core::city::{validate_city_content, HUB_BUILDING_ID};
use wyrdstone_core::{CityState, ClanRank, GameDataBundle, Language, ResourceBundle};

fn bundle() -> GameDataBundle {
    GameDataBundle::load_bundled(Language::En).expect("bundled data must parse")
}

fn dump<T: serde::Serialize>(value: &T) -> String {
    ron::to_string(value).expect("value must serialize")
}

#[test]
fn login_tribute_reward_with_matches_legacy() {
    let d = bundle();
    for day in 0..=12u8 {
        assert_eq!(
            dump(&CityState::login_tribute_reward_with(day, &d.balance)),
            dump(&CityState::login_tribute_reward(day)),
            "day {day}"
        );
    }
}

#[test]
fn login_tribute_claims_with_match_legacy() {
    let d = bundle();
    let mut legacy = CityState::new(&d.balance);
    let mut data_driven = CityState::new(&d.balance);
    for offset in 0..16u64 {
        let today = 100 + offset;
        assert_eq!(
            dump(&data_driven.try_claim_login_tribute_with(today, &d.balance)),
            dump(&legacy.try_claim_login_tribute(today)),
            "claim {offset}"
        );
        assert_eq!(dump(&data_driven.resources), dump(&legacy.resources));
        assert_eq!(dump(&data_driven.tools), dump(&legacy.tools));
    }
}

#[test]
fn scale_daily_rewards_with_matches_legacy() {
    let d = bundle();
    let base = ResourceBundle {
        wood: 140,
        stone: 140,
        gold: 180,
        mead: 40,
    };
    for streak in 0..=10u32 {
        assert_eq!(
            dump(&CityState::scale_daily_rewards_with(
                &base, streak, &d.balance
            )),
            dump(&CityState::scale_daily_rewards(&base, streak)),
            "streak {streak}"
        );
    }
}

#[test]
fn clan_rank_perks_with_match_const_fns() {
    let d = bundle();
    for rank in [
        ClanRank::Scout,
        ClanRank::Huscarl,
        ClanRank::Jarl,
        ClanRank::HighKing,
    ] {
        assert_eq!(
            rank.resource_bonus_pct_with(&d.balance),
            rank.resource_bonus_pct()
        );
        assert_eq!(rank.starting_fury_with(&d.balance), rank.starting_fury());
    }
}

#[test]
fn amulet_tier_step_matches_default_steps() {
    let d = bundle();
    for amulet in &d.amulets {
        let mut stripped = amulet.clone();
        stripped.tier_step = None;
        for tier in 0..=3u8 {
            assert_eq!(
                dump(&CityState::scaled_amulet_effect_for_tier(amulet, tier)),
                dump(&CityState::scaled_amulet_effect_for_tier(&stripped, tier)),
                "{} tier {tier}",
                amulet.id
            );
        }
    }
}

#[test]
fn bundled_city_content_validates() {
    let d = bundle();
    assert!(validate_city_content(&d.buildings, &d.amulets, &d.achievements).is_empty());
}

#[test]
fn broken_city_content_is_reported() {
    let d = bundle();

    let no_hub: Vec<_> = d
        .buildings
        .iter()
        .filter(|b| b.id != HUB_BUILDING_ID)
        .cloned()
        .collect();
    let problems = validate_city_content(&no_hub, &d.amulets, &d.achievements);
    assert!(
        problems.iter().any(|p| p.contains(HUB_BUILDING_ID)),
        "{problems:?}"
    );

    let mut achievements = d.achievements.clone();
    achievements[0].condition = None;
    let problems = validate_city_content(&d.buildings, &d.amulets, &achievements);
    assert!(
        problems.iter().any(|p| p.contains(&achievements[0].id)),
        "{problems:?}"
    );

    let mut achievements = d.achievements.clone();
    achievements[1].condition = Some(AchievementCondition::BestCombo(0));
    let problems = validate_city_content(&d.buildings, &d.amulets, &achievements);
    assert!(
        problems.iter().any(|p| p.contains(&achievements[1].id)),
        "{problems:?}"
    );
}
