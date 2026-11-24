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

//! Structural validation of the bundled game data: dangling references, duplicate identifiers and
//! out-of-range tuning values are reported together so a data mistake is fixed in a single pass.

use crate::balance::BalanceConfig;
use crate::board::RuneType;
use crate::boss::BOSS_RAID_BUILDING_ID;
use crate::city::{
    validate_city_content, QuestCondition, LOGIN_TRIBUTE_DAYS, MAX_DAILY_STREAK_CYCLE,
};
use crate::expedition::EXPEDITION_BUILDING_ID;
use crate::level::DAILY_TRIAL_BUILDING_ID;
use crate::save::GameDataBundle;
use std::collections::HashSet;

/// Identifiers that levels and collections may use as a building source without being a village
/// building: the synthetic Daily Trial, Boss Raid and Expedition boards.
pub const RESERVED_BUILDING_IDS: [&str; 3] = [
    DAILY_TRIAL_BUILDING_ID,
    BOSS_RAID_BUILDING_ID,
    EXPEDITION_BUILDING_ID,
];

/// Smallest number of rune types a board can be played with.
const MIN_RUNE_TYPES: usize = 3;

/// Collects every problem found in the bundle, or returns `Ok(())` when the data is consistent.
pub fn validate_bundle(b: &GameDataBundle) -> Result<(), String> {
    let mut problems: Vec<String> = Vec::new();
    check_unique_ids(b, &mut problems);
    check_references(b, &mut problems);
    check_balance(&b.balance, b, &mut problems);
    problems.extend(validate_city_content(
        &b.buildings,
        &b.amulets,
        &b.achievements,
    ));
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

fn check_unique<'a, I>(list_name: &str, ids: I, problems: &mut Vec<String>)
where
    I: IntoIterator<Item = &'a str>,
{
    let mut seen: HashSet<&str> = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            problems.push(format!("{list_name}: duplicate id '{id}'"));
        }
    }
}

fn check_unique_ids(b: &GameDataBundle, problems: &mut Vec<String>) {
    check_unique(
        "buildings",
        b.buildings.iter().map(|x| x.id.as_str()),
        problems,
    );
    check_unique("levels", b.levels.iter().map(|x| x.id.as_str()), problems);
    check_unique("quests", b.quests.iter().map(|x| x.id.as_str()), problems);
    check_unique("amulets", b.amulets.iter().map(|x| x.id.as_str()), problems);
    check_unique(
        "collections",
        b.collections.iter().map(|x| x.id.as_str()),
        problems,
    );
    check_unique("bosses", b.bosses.iter().map(|x| x.id.as_str()), problems);
    check_unique(
        "achievements",
        b.achievements.iter().map(|x| x.id.as_str()),
        problems,
    );
}

fn check_references(b: &GameDataBundle, problems: &mut Vec<String>) {
    let building_ids: HashSet<&str> = b.buildings.iter().map(|x| x.id.as_str()).collect();
    let level_ids: HashSet<&str> = b.levels.iter().map(|x| x.id.as_str()).collect();
    let is_known_source =
        |id: &str| building_ids.contains(id) || RESERVED_BUILDING_IDS.contains(&id);

    for level in &b.levels {
        if !is_known_source(&level.building_id) {
            problems.push(format!(
                "level '{}': unknown building_id '{}'",
                level.id, level.building_id
            ));
        }
    }

    for building in &b.buildings {
        if RESERVED_BUILDING_IDS.contains(&building.id.as_str()) {
            problems.push(format!(
                "building '{}': collides with a reserved building id",
                building.id
            ));
        }
        let linked = [
            Some(building.level_id.as_str()),
            building.mastery_level_id.as_deref(),
            building.legendary_level_id.as_deref(),
        ];
        for level_id in linked.into_iter().flatten() {
            if !level_ids.contains(level_id) {
                problems.push(format!(
                    "building '{}': unknown level id '{level_id}'",
                    building.id
                ));
            }
        }
        if building.upgrade_costs.len() < building.max_tier as usize {
            problems.push(format!(
                "building '{}': {} upgrade costs for max_tier {}",
                building.id,
                building.upgrade_costs.len(),
                building.max_tier
            ));
        }
    }

    for quest in &b.quests {
        match &quest.condition {
            QuestCondition::WinBuildingLevel { building_id, .. }
            | QuestCondition::ReachBuildingTier { building_id, .. } => {
                if !building_ids.contains(building_id.as_str()) {
                    problems.push(format!(
                        "quest '{}': unknown building_id '{building_id}'",
                        quest.id
                    ));
                }
            }
            QuestCondition::WinSpecificLevel { level_id } => {
                if !level_ids.contains(level_id.as_str()) {
                    problems.push(format!(
                        "quest '{}': unknown level_id '{level_id}'",
                        quest.id
                    ));
                }
            }
            QuestCondition::CraftAnyAmulet
            | QuestCondition::ClaimAnyCollection
            | QuestCondition::CompleteDailyTrials { .. }
            | QuestCondition::DefeatBosses { .. }
            | QuestCondition::ReachExpeditionWave { .. } => {}
        }
    }

    for col in &b.collections {
        for source in &col.source_building_ids {
            if !is_known_source(source) {
                problems.push(format!(
                    "collection '{}': unknown source building id '{source}'",
                    col.id
                ));
            }
        }
        if col.relic_ids.len() != col.relic_name_keys.len() {
            problems.push(format!(
                "collection '{}': {} relic ids but {} relic name keys",
                col.id,
                col.relic_ids.len(),
                col.relic_name_keys.len()
            ));
        }
    }
}

fn check_range<T: PartialOrd + std::fmt::Display>(
    name: &str,
    range: (T, T),
    problems: &mut Vec<String>,
) {
    if range.0 > range.1 {
        problems.push(format!(
            "balance.{name}: lower bound {} exceeds upper bound {}",
            range.0, range.1
        ));
    }
}

fn check_balance(bal: &BalanceConfig, b: &GameDataBundle, problems: &mut Vec<String>) {
    // Building bonus table.
    let mut bound: HashSet<&str> = HashSet::new();
    for binding in &bal.building_bonuses {
        if !b.buildings.iter().any(|x| x.id == binding.building_id) {
            problems.push(format!(
                "balance.building_bonuses: unknown building id '{}'",
                binding.building_id
            ));
        }
        if !bound.insert(binding.building_id.as_str()) {
            problems.push(format!(
                "balance.building_bonuses: duplicate building id '{}'",
                binding.building_id
            ));
        }
    }

    // Login tribute.
    if bal.login_tribute_rewards.len() != usize::from(LOGIN_TRIBUTE_DAYS) {
        problems.push(format!(
            "balance.login_tribute_rewards: {} entries, expected {LOGIN_TRIBUTE_DAYS}",
            bal.login_tribute_rewards.len()
        ));
    }

    // Clan rank thresholds.
    let thresholds = [
        0,
        bal.clan_rank_huscarl_glory,
        bal.clan_rank_jarl_glory,
        bal.clan_rank_high_king_glory,
    ];
    if thresholds.windows(2).any(|w| w[0] >= w[1]) {
        problems.push(format!(
            "balance.clan_rank_*_glory: thresholds must be strictly ascending, got {thresholds:?}"
        ));
    }

    // Star ratios.
    let s = &bal.session;
    if !(0.0..=1.0).contains(&s.two_star_moves_ratio)
        || !(0.0..=1.0).contains(&s.three_star_moves_ratio)
        || s.two_star_moves_ratio > s.three_star_moves_ratio
    {
        problems.push(format!(
            "balance.session: star ratios must satisfy 0 <= two ({}) <= three ({}) <= 1",
            s.two_star_moves_ratio, s.three_star_moves_ratio
        ));
    }

    // Ranges.
    let d = &bal.daily;
    check_range(
        "daily.glacial_collect_range",
        d.glacial_collect_range,
        problems,
    );
    check_range("daily.relic_count_range", d.relic_count_range, problems);
    check_range("daily.blitz_time_range", d.blitz_time_range, problems);
    check_range("daily.blitz_score_range", d.blitz_score_range, problems);
    check_range("daily.blitz_collect_range", d.blitz_collect_range, problems);
    check_range("daily.max_moves_range", d.max_moves_range, problems);
    if !(0.0..=1.0).contains(&d.glacial_frost_chance) {
        problems.push(format!(
            "balance.daily.glacial_frost_chance: {} is not a probability",
            d.glacial_frost_chance
        ));
    }

    // Expedition.
    let e = &bal.expedition;
    if !(MIN_RUNE_TYPES..=RuneType::ALL.len()).contains(&e.wave_one_rune_types) {
        problems.push(format!(
            "balance.expedition.wave_one_rune_types: {} outside {MIN_RUNE_TYPES}..={}",
            e.wave_one_rune_types,
            RuneType::ALL.len()
        ));
    }
    if e.boon_choices < 1 || e.boon_choices > e.boons.len() {
        problems.push(format!(
            "balance.expedition.boon_choices: {} outside 1..={}",
            e.boon_choices,
            e.boons.len()
        ));
    }

    // Mastery ladder and daily streak tools.
    if bal.mastery_star_tools.is_empty() {
        problems.push("balance.mastery_star_tools: must not be empty".to_string());
    }
    for reward in &bal.daily_streak_tool_rewards {
        if reward.min_streak < 1 || reward.min_streak > MAX_DAILY_STREAK_CYCLE {
            problems.push(format!(
                "balance.daily_streak_tool_rewards: min_streak {} outside 1..={MAX_DAILY_STREAK_CYCLE}",
                reward.min_streak
            ));
        }
    }
}
