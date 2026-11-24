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
use crate::board::{ActiveTool, RuneType};
use crate::boss::BOSS_RAID_BUILDING_ID;
use crate::city::{
    validate_city_content, QuestCondition, LOGIN_TRIBUTE_DAYS, MAX_DAILY_STREAK_CYCLE,
};
use crate::expedition::{EXPEDITION_BUILDING_ID, MAX_EXPEDITION_ICE_CELLS};
use crate::i18n::Language;
use crate::level::{LevelDef, Objective, DAILY_TRIAL_BUILDING_ID};
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
/// Maximum allowed phase-2 HP percentage threshold.
const MAX_PERCENT: u32 = 100;

/// Collects every problem found in the bundle, or returns `Ok(())` when the data is consistent.
pub fn validate_bundle(b: &GameDataBundle) -> Result<(), String> {
    let mut problems: Vec<String> = Vec::new();
    check_unique_ids(b, &mut problems);
    check_references(b, &mut problems);
    check_levels(&b.levels, &mut problems);
    check_bosses(b, &mut problems);
    check_tool_shop(b, &mut problems);
    check_balance(&b.balance, b, &mut problems);
    check_i18n_keys(b, &mut problems);
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
        if col.relic_ids.is_empty() {
            problems.push(format!(
                "collection '{}': relic_ids must not be empty",
                col.id
            ));
        }
        if col.relic_ids.len() != col.relic_name_keys.len() {
            problems.push(format!(
                "collection '{}': {} relic ids but {} relic name keys",
                col.id,
                col.relic_ids.len(),
                col.relic_name_keys.len()
            ));
        }
        check_unique(
            &format!("collection '{}' relic_ids", col.id),
            col.relic_ids.iter().map(String::as_str),
            problems,
        );
    }
}

fn check_levels(levels: &[LevelDef], problems: &mut Vec<String>) {
    for level in levels {
        if level.cols == 0 || level.rows == 0 {
            problems.push(format!(
                "level '{}': board dimensions {}x{} must be positive",
                level.id, level.cols, level.rows
            ));
        }
        if level.max_moves == 0 {
            problems.push(format!("level '{}': max_moves must be positive", level.id));
        }
        if !(MIN_RUNE_TYPES..=RuneType::ALL.len()).contains(&level.rune_types_count) {
            problems.push(format!(
                "level '{}': rune_types_count {} outside {MIN_RUNE_TYPES}..={}",
                level.id,
                level.rune_types_count,
                RuneType::ALL.len()
            ));
        }
        if level.objectives.is_empty() {
            problems.push(format!(
                "level '{}': objectives must not be empty",
                level.id
            ));
        }
        let check_coord = |kind: &str, c: usize, r: usize, probs: &mut Vec<String>| {
            if c >= level.cols || r >= level.rows {
                probs.push(format!(
                    "level '{}': {kind} ({c}, {r}) out of {}x{} bounds",
                    level.id, level.cols, level.rows
                ));
            }
        };
        for &(c, r, _) in &level.ice_cells {
            check_coord("ice_cell", c, r, problems);
        }
        for &(c, r) in &level.chained_cells {
            check_coord("chained_cell", c, r, problems);
        }
        for &(c, r, _) in &level.crate_cells {
            check_coord("crate_cell", c, r, problems);
        }
        for &(c, r) in &level.relic_cells {
            check_coord("relic_cell", c, r, problems);
        }
        for &(c, r) in &level.fog_cells {
            check_coord("fog_cell", c, r, problems);
        }
        for &(c, r, _) in &level.cursed_cells {
            check_coord("cursed_cell", c, r, problems);
        }
        for &(c, r) in &level.blocked_cells {
            check_coord("blocked_cell", c, r, problems);
        }
        for &((sc, sr), (dc, dr)) in &level.portals {
            check_coord("portal source", sc, sr, problems);
            check_coord("portal destination", dc, dr, problems);
        }
        let pool_len = level.rune_types_count.min(RuneType::ALL.len());
        let active_pool = &RuneType::ALL[..pool_len];
        for obj in &level.objectives {
            match obj {
                Objective::CollectRune { rune, target } => {
                    if *target == 0 {
                        problems.push(format!(
                            "level '{}': CollectRune target must be positive",
                            level.id
                        ));
                    }
                    if !active_pool.contains(rune) {
                        problems.push(format!(
                            "level '{}': CollectRune({rune:?}) is outside rune_types_count {}",
                            level.id, level.rune_types_count
                        ));
                    }
                }
                Objective::ClearIce { target } => {
                    if *target == 0 {
                        problems.push(format!(
                            "level '{}': ClearIce target must be positive",
                            level.id
                        ));
                    } else if !level.frost_spread && (*target as usize) > level.ice_cells.len() {
                        problems.push(format!(
                            "level '{}': ClearIce target {} exceeds {} placed ice cells",
                            level.id,
                            target,
                            level.ice_cells.len()
                        ));
                    }
                }
                Objective::DropRelics { target } => {
                    if *target == 0 {
                        problems.push(format!(
                            "level '{}': DropRelics target must be positive",
                            level.id
                        ));
                    } else if (*target as usize) > level.relic_cells.len() {
                        problems.push(format!(
                            "level '{}': DropRelics target {} exceeds {} placed relics",
                            level.id,
                            target,
                            level.relic_cells.len()
                        ));
                    }
                }
                Objective::ReachScore { target } => {
                    if *target == 0 {
                        problems.push(format!(
                            "level '{}': ReachScore target must be positive",
                            level.id
                        ));
                    }
                }
            }
        }
    }
}

fn check_bosses(b: &GameDataBundle, problems: &mut Vec<String>) {
    for boss in &b.bosses {
        if boss.max_hp == 0 {
            problems.push(format!("boss '{}': max_hp must be positive", boss.id));
        }
        if boss.max_moves == 0 {
            problems.push(format!("boss '{}': max_moves must be positive", boss.id));
        }
        if boss.riposte_interval == 0 {
            problems.push(format!(
                "boss '{}': riposte_interval must be positive",
                boss.id
            ));
        }
    }
}

fn check_tool_shop(b: &GameDataBundle, problems: &mut Vec<String>) {
    let mut seen: HashSet<ActiveTool> = HashSet::new();
    for recipe in &b.tool_shop {
        if !seen.insert(recipe.tool) {
            problems.push(format!("tool_shop: duplicate tool '{:?}'", recipe.tool));
        }
    }
}

fn check_i18n_keys(b: &GameDataBundle, problems: &mut Vec<String>) {
    let mut check_key = |owner: &str, key: &str| {
        for lang in [Language::En, Language::Fr] {
            if !b.i18n.has_key(lang, key) {
                problems.push(format!("{owner}: missing {lang:?} locale key '{key}'"));
            }
        }
    };
    for def in &b.buildings {
        let owner = format!("building '{}'", def.id);
        check_key(&owner, &def.name_key);
        check_key(&owner, &def.desc_key);
        check_key(&owner, &def.bonus_key);
    }
    for q in &b.quests {
        let owner = format!("quest '{}'", q.id);
        check_key(&owner, &q.title_key);
        check_key(&owner, &q.desc_key);
    }
    for a in &b.amulets {
        let owner = format!("amulet '{}'", a.id);
        check_key(&owner, &a.name_key);
        check_key(&owner, &a.desc_key);
    }
    for c in &b.collections {
        let owner = format!("collection '{}'", c.id);
        check_key(&owner, &c.name_key);
        for rk in &c.relic_name_keys {
            check_key(&owner, rk);
        }
    }
    for boss in &b.bosses {
        let owner = format!("boss '{}'", boss.id);
        check_key(&owner, &boss.name_key);
        check_key(&owner, &boss.title_key);
        check_key(&owner, &boss.desc_key);
    }
    for ach in &b.achievements {
        let owner = format!("achievement '{}'", ach.id);
        check_key(&owner, &ach.title_key);
        check_key(&owner, &ach.desc_key);
    }
    for tool in &b.tool_shop {
        let owner = format!("tool_shop '{:?}'", tool.tool);
        check_key(&owner, &tool.name_key);
        check_key(&owner, &tool.desc_key);
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
    if d.relic_count_range.0 < 1 {
        problems.push(format!(
            "balance.daily.relic_count_range: lower bound {} must be at least 1",
            d.relic_count_range.0
        ));
    }
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
    if e.ice_cap > MAX_EXPEDITION_ICE_CELLS {
        problems.push(format!(
            "balance.expedition.ice_cap: {} exceeds {MAX_EXPEDITION_ICE_CELLS} grid cells",
            e.ice_cap
        ));
    }
    if e.score_wave_interval < 1 {
        problems.push(format!(
            "balance.expedition.score_wave_interval: {} must be at least 1",
            e.score_wave_interval
        ));
    }
    if e.boon_choices < 1 || e.boon_choices > e.boons.len() {
        problems.push(format!(
            "balance.expedition.boon_choices: {} outside 1..={}",
            e.boon_choices,
            e.boons.len()
        ));
    }

    // Core stamina and multipliers.
    if bal.max_energy == 0 {
        problems.push("balance.max_energy: must be positive".to_string());
    }
    if bal.energy_regen_seconds == 0 {
        problems.push("balance.energy_regen_seconds: must be positive".to_string());
    }
    if !bal.combo_multiplier_step.is_finite() || bal.combo_multiplier_step < 0.0 {
        problems.push(format!(
            "balance.combo_multiplier_step: {} must be finite and non-negative",
            bal.combo_multiplier_step
        ));
    }
    if bal.boss.phase2_hp_pct > MAX_PERCENT {
        problems.push(format!(
            "balance.boss.phase2_hp_pct: {} exceeds {MAX_PERCENT}",
            bal.boss.phase2_hp_pct
        ));
    }
    if bal.boss.weakness_multiplier < 1 {
        problems.push(format!(
            "balance.boss.weakness_multiplier: {} must be at least 1",
            bal.boss.weakness_multiplier
        ));
    }
    if !bal.boss.combo_damage_step.is_finite() || bal.boss.combo_damage_step < 0.0 {
        problems.push(format!(
            "balance.boss.combo_damage_step: {} must be finite and non-negative",
            bal.boss.combo_damage_step
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
