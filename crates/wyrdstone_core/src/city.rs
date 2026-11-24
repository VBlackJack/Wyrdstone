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

//! Fjord city-builder progression: resources, buildings, passive harvests, mastery, amulets, quests, and tools.

use crate::achievements::{is_achievement_unlocked, AchievementDef, ClanRank};
use crate::balance::{
    default_login_tribute_rewards, BalanceConfig, BuildingBonusKind, LoginTributeDay,
    DEFAULT_DAILY_STREAK_BONUS_PCT_PER_DAY,
};
use crate::board::ActiveTool;
use crate::boss::BossDef;
use crate::patron::PatronGod;
use crate::turn::ClearStatsDelta;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Maximum mastery star rank per building location.
pub const MAX_MASTERY_STARS: u8 = 5;
/// Maximum daily trial streak tier for bonus rewards.
pub const MAX_DAILY_STREAK_CYCLE: u32 = 7;
/// Maximum upgrade tier for Nordic Amulets.
pub const MAX_AMULET_TIER: u8 = 3;
/// Maximum star rating awarded for a level.
pub const MAX_LEVEL_STARS: u8 = 3;
/// Number of days in the Fjord Login Tribute cycle.
pub const LOGIN_TRIBUTE_DAYS: u8 = 7;
/// Identifier of the hub building whose tier gates every other building.
pub const HUB_BUILDING_ID: &str = "great_hall";
/// Fallback amulet tier step (percent) for resource bonus effects without an explicit `tier_step`.
pub const DEFAULT_AMULET_RESOURCE_PER_TIER: u32 = 12;
/// Fallback amulet tier step for extra moves effects without an explicit `tier_step`.
pub const DEFAULT_AMULET_MOVES_PER_TIER: u32 = 1;
/// Fallback amulet tier step (percent) for score bonus effects without an explicit `tier_step`.
pub const DEFAULT_AMULET_SCORE_PER_TIER: u32 = 15;
/// Fallback amulet tier step (percent) for extra time effects without an explicit `tier_step`.
pub const DEFAULT_AMULET_TIME_PER_TIER: u32 = 15;
/// Fallback amulet tier step for starting boosters effects without an explicit `tier_step`.
pub const DEFAULT_AMULET_BOOSTERS_PER_TIER: u32 = 1;

/// Bundle of village resources used for costs and rewards.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBundle {
    pub wood: u32,
    pub stone: u32,
    pub gold: u32,
    pub mead: u32,
}

impl ResourceBundle {
    pub fn can_afford(&self, cost: &ResourceBundle) -> bool {
        self.wood >= cost.wood
            && self.stone >= cost.stone
            && self.gold >= cost.gold
            && self.mead >= cost.mead
    }

    pub fn add(&mut self, other: &ResourceBundle) {
        self.wood = self.wood.saturating_add(other.wood);
        self.stone = self.stone.saturating_add(other.stone);
        self.gold = self.gold.saturating_add(other.gold);
        self.mead = self.mead.saturating_add(other.mead);
    }

    pub fn subtract(&mut self, cost: &ResourceBundle) {
        self.wood = self.wood.saturating_sub(cost.wood);
        self.stone = self.stone.saturating_sub(cost.stone);
        self.gold = self.gold.saturating_sub(cost.gold);
        self.mead = self.mead.saturating_sub(cost.mead);
    }

    pub fn is_zero(&self) -> bool {
        self.wood == 0 && self.stone == 0 && self.gold == 0 && self.mead == 0
    }
}

/// Passive buff granted by an equipped Nordic Amulet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmuletEffect {
    ResourceBonusPct(u32),
    ExtraMoves(u32),
    ScoreBonusPct(u32),
    StartingBoosters(u32),
    ExtraTimePct(u32),
}

/// Static Nordic Amulet definition loaded from `assets/data/amulets.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmuletDef {
    pub id: String,
    pub name_key: String,
    pub cost: ResourceBundle,
    pub effect: AmuletEffect,
    /// Effect gained per extra tier; `None` uses the named per-effect default.
    #[serde(default)]
    pub tier_step: Option<u32>,
}

/// Aggregate passive bonuses granted by all rebuilt village buildings and Clan Rank.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CityBonuses {
    pub extra_max_energy: u32,
    pub score_bonus_pct: u32,
    pub resource_bonus_pct: u32,
    pub extra_moves: u32,
    pub energy_regen_speed_pct: u32,
    pub fury_charge_bonus_pct: u32,
    pub starting_fury: u32,
    pub starting_boosters: u32,
}

/// Concrete perk granted each time a building is upgraded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpgradePerk {
    /// Grants one charge of the given Runic Tool.
    GrantTool(ActiveTool),
    /// Raises current endurance by the per-tier max endurance bonus (may exceed the cap).
    GainMaxEnergyBonus,
    /// Refills endurance up to the effective cap.
    RefillEnergy,
}

/// Static building definition loaded from `assets/data/buildings.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingDef {
    pub id: String,
    pub name_key: String,
    pub desc_key: String,
    pub bonus_key: String,
    pub max_tier: u32,
    pub requires_great_hall_tier: u32,
    pub level_id: String,
    #[serde(default)]
    pub mastery_level_id: Option<String>,
    #[serde(default)]
    pub legendary_level_id: Option<String>,
    pub map_x_pct: f32,
    pub map_y_pct: f32,
    /// Optional explicit pin left edge (percent of map width) in portrait layout.
    /// When absent the landscape position is projected and clamped.
    #[serde(default)]
    pub portrait_left_pct: Option<f32>,
    /// Optional explicit pin top edge (percent of map height) in portrait layout.
    #[serde(default)]
    pub portrait_top_pct: Option<f32>,
    pub upgrade_costs: Vec<ResourceBundle>,
    pub glory_per_tier: u32,
    #[serde(default)]
    pub harvest_per_tier: ResourceBundle,
    /// Perks applied on every upgrade, in order.
    #[serde(default)]
    pub upgrade_perks: Vec<UpgradePerk>,
}

/// Completion requirement for a Saga quest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestCondition {
    WinBuildingLevel { building_id: String, wins: u32 },
    WinSpecificLevel { level_id: String },
    ReachBuildingTier { building_id: String, tier: u32 },
    CraftAnyAmulet,
    ClaimAnyCollection,
    CompleteDailyTrials { count: u32 },
    DefeatBosses { count: u32 },
    ReachExpeditionWave { wave: u32 },
}

/// Static Saga quest loaded from `assets/data/quests.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestDef {
    pub id: String,
    #[serde(default = "default_speaker")]
    pub speaker: String,
    pub title_key: String,
    pub desc_key: String,
    pub condition: QuestCondition,
    pub reward: ResourceBundle,
}

fn default_speaker() -> String {
    "jarl".to_string()
}

/// Static Nine Realms relic collection set loaded from `assets/data/collections.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionDef {
    pub id: String,
    pub name_key: String,
    pub relic_ids: Vec<String>,
    pub relic_name_keys: Vec<String>,
    #[serde(default)]
    pub source_building_ids: Vec<String>,
    pub reward_resources: ResourceBundle,
    pub reward_tools: ToolInventory,
}

/// Static crafting recipe for forging active tools in the Fjord.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRecipeDef {
    pub tool: ActiveTool,
    pub name_key: String,
    pub desc_key: String,
    pub cost: ResourceBundle,
}

/// Player inventory of the six active Match-3 tools.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInventory {
    pub mjolnir: u32,
    pub surtr_ember: u32,
    pub loki_shuffle: u32,
    #[serde(default)]
    pub thor_bolt: u32,
    #[serde(default)]
    pub bifrost_prism: u32,
    #[serde(default)]
    pub drakkar_charge: u32,
}

impl ToolInventory {
    pub fn count(&self, tool: ActiveTool) -> u32 {
        match tool {
            ActiveTool::Mjolnir => self.mjolnir,
            ActiveTool::SurtrEmber => self.surtr_ember,
            ActiveTool::LokiShuffle => self.loki_shuffle,
            ActiveTool::ThorBolt => self.thor_bolt,
            ActiveTool::BifrostPrism => self.bifrost_prism,
            ActiveTool::DrakkarCharge => self.drakkar_charge,
        }
    }

    pub fn add(&mut self, other: &ToolInventory) {
        self.mjolnir = self.mjolnir.saturating_add(other.mjolnir);
        self.surtr_ember = self.surtr_ember.saturating_add(other.surtr_ember);
        self.loki_shuffle = self.loki_shuffle.saturating_add(other.loki_shuffle);
        self.thor_bolt = self.thor_bolt.saturating_add(other.thor_bolt);
        self.bifrost_prism = self.bifrost_prism.saturating_add(other.bifrost_prism);
        self.drakkar_charge = self.drakkar_charge.saturating_add(other.drakkar_charge);
    }

    pub fn grant_one(&mut self, tool: ActiveTool) {
        match tool {
            ActiveTool::Mjolnir => self.mjolnir = self.mjolnir.saturating_add(1),
            ActiveTool::SurtrEmber => self.surtr_ember = self.surtr_ember.saturating_add(1),
            ActiveTool::LokiShuffle => self.loki_shuffle = self.loki_shuffle.saturating_add(1),
            ActiveTool::ThorBolt => self.thor_bolt = self.thor_bolt.saturating_add(1),
            ActiveTool::BifrostPrism => self.bifrost_prism = self.bifrost_prism.saturating_add(1),
            ActiveTool::DrakkarCharge => {
                self.drakkar_charge = self.drakkar_charge.saturating_add(1)
            }
        }
    }

    pub fn consume(&mut self, tool: ActiveTool) -> bool {
        let slot = match tool {
            ActiveTool::Mjolnir => &mut self.mjolnir,
            ActiveTool::SurtrEmber => &mut self.surtr_ember,
            ActiveTool::LokiShuffle => &mut self.loki_shuffle,
            ActiveTool::ThorBolt => &mut self.thor_bolt,
            ActiveTool::BifrostPrism => &mut self.bifrost_prism,
            ActiveTool::DrakkarCharge => &mut self.drakkar_charge,
        };
        if *slot > 0 {
            *slot -= 1;
            true
        } else {
            false
        }
    }
}

/// Persistent lifetime statistics for the Clan Chronicles screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClanStats {
    pub total_victories: u32,
    pub total_runes_cleared: u64,
    pub best_combo: u32,
    pub best_score: u32,
    pub last_daily_day_index: u64,
    #[serde(default)]
    pub daily_streak: u32,
    #[serde(default)]
    pub best_daily_streak: u32,
    #[serde(default)]
    pub daily_completions: u32,
    #[serde(default)]
    pub bosses_defeated: u32,
    #[serde(default)]
    pub best_expedition_wave: u32,
    #[serde(default)]
    pub win_streak: u32,
    #[serde(default)]
    pub best_win_streak: u32,
}

impl ClanStats {
    /// Applies the lifetime statistics earned by one committed clear step.
    pub fn record_clear(&mut self, delta: ClearStatsDelta) {
        self.total_runes_cleared = self.total_runes_cleared.saturating_add(delta.runes_cleared);
        self.best_combo = self.best_combo.max(delta.combo);
    }
}

/// Mutable player progression in the Wyrdstone Fjord.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityState {
    pub resources: ResourceBundle,
    pub energy: u32,
    pub glory: u32,
    pub tools: ToolInventory,
    pub building_tiers: HashMap<String, u32>,
    pub building_wins: HashMap<String, u32>,
    pub claimed_quests: HashSet<String>,
    #[serde(default)]
    pub crafted_amulets: HashSet<String>,
    #[serde(default)]
    pub amulet_tiers: HashMap<String, u8>,
    #[serde(default)]
    pub equipped_amulet: Option<String>,
    #[serde(default)]
    pub found_relics: HashSet<String>,
    #[serde(default)]
    pub claimed_collections: HashSet<String>,
    #[serde(default)]
    pub stats: ClanStats,
    #[serde(default)]
    pub completed_levels: HashSet<String>,
    #[serde(default)]
    pub level_best_stars: HashMap<String, u8>,
    #[serde(default)]
    pub last_harvest_unix_secs: HashMap<String, u64>,
    #[serde(default)]
    pub selected_patron: PatronGod,
    #[serde(default)]
    pub defeated_bosses: HashSet<String>,
    #[serde(default)]
    pub claimed_achievements: HashSet<String>,
    #[serde(default)]
    pub login_streak_day: u8,
    #[serde(default)]
    pub last_login_claim_day: Option<u64>,
    #[serde(default)]
    pub seen_chapters: HashSet<String>,
}

impl CityState {
    /// Initializes a brand-new village from balance defaults.
    pub fn new(balance: &BalanceConfig) -> Self {
        Self {
            resources: ResourceBundle {
                wood: balance.starting_wood,
                stone: balance.starting_stone,
                gold: balance.starting_gold,
                mead: balance.starting_mead,
            },
            energy: balance.max_energy,
            glory: 0,
            tools: ToolInventory {
                mjolnir: balance.starting_mjolnir,
                surtr_ember: balance.starting_surtr_ember,
                loki_shuffle: balance.starting_loki_shuffle,
                thor_bolt: balance.starting_thor_bolt,
                bifrost_prism: balance.starting_bifrost_prism,
                drakkar_charge: balance.starting_drakkar_charge,
            },
            building_tiers: HashMap::new(),
            building_wins: HashMap::new(),
            claimed_quests: HashSet::new(),
            crafted_amulets: HashSet::new(),
            amulet_tiers: HashMap::new(),
            equipped_amulet: None,
            found_relics: HashSet::new(),
            claimed_collections: HashSet::new(),
            stats: ClanStats::default(),
            completed_levels: HashSet::new(),
            level_best_stars: HashMap::new(),
            last_harvest_unix_secs: HashMap::new(),
            selected_patron: PatronGod::default(),
            defeated_bosses: HashSet::new(),
            claimed_achievements: HashSet::new(),
            login_streak_day: 0,
            last_login_claim_day: None,
            seen_chapters: HashSet::new(),
        }
    }

    /// Clamps values that a tampered or corrupted save could push outside their legal range:
    /// building tiers, amulet ownership/tiers, star ratings, streak counters and endurance.
    pub fn sanitize(&mut self, buildings: &[BuildingDef], balance: &BalanceConfig) {
        for def in buildings {
            if let Some(tier) = self.building_tiers.get_mut(&def.id) {
                *tier = (*tier).min(def.max_tier);
            }
        }
        self.amulet_tiers
            .retain(|id, tier| *tier > 0 || self.crafted_amulets.contains(id));
        for tier in self.amulet_tiers.values_mut() {
            *tier = (*tier).clamp(1, MAX_AMULET_TIER);
        }
        for id in &self.crafted_amulets {
            self.amulet_tiers.entry(id.clone()).or_insert(1);
        }
        for id in self.amulet_tiers.keys() {
            self.crafted_amulets.insert(id.clone());
        }
        if self
            .equipped_amulet
            .as_deref()
            .is_some_and(|id| !self.crafted_amulets.contains(id))
        {
            self.equipped_amulet = None;
        }
        for stars in self.level_best_stars.values_mut() {
            *stars = (*stars).min(MAX_LEVEL_STARS);
        }
        self.login_streak_day = self.login_streak_day.min(LOGIN_TRIBUTE_DAYS);
        self.stats.daily_streak = self.stats.daily_streak.min(MAX_DAILY_STREAK_CYCLE);
        self.stats.best_daily_streak = self.stats.best_daily_streak.max(self.stats.daily_streak);
        self.energy = self.energy.min(self.max_energy(balance));
    }

    /// Returns true if the 7-day Fjord Login Tribute has not yet been claimed today.
    pub fn is_login_tribute_available(&self, today_day_index: u64) -> bool {
        self.last_login_claim_day
            .is_none_or(|last| today_day_index > last)
    }

    /// Returns which day (1..=7) of the 7-day Fjord Login Tribute is next to claim.
    pub fn next_login_tribute_day(&self, today_day_index: u64) -> u8 {
        match self.last_login_claim_day {
            Some(prev) if prev == today_day_index => {
                self.login_streak_day.clamp(1, LOGIN_TRIBUTE_DAYS)
            }
            Some(prev) if prev.saturating_add(1) == today_day_index => {
                (self.login_streak_day % LOGIN_TRIBUTE_DAYS) + 1
            }
            _ => 1,
        }
    }

    /// Returns the escalating reward bundle and optional Runic Tool for a given login day (1..=7).
    pub fn login_tribute_reward(day: u8) -> (ResourceBundle, Option<ActiveTool>) {
        Self::tribute_from(day, &default_login_tribute_rewards())
    }

    /// Returns the login tribute reward of a day (1..=7) from the balance table; days outside the
    /// table use its last entry.
    pub fn login_tribute_reward_with(
        day: u8,
        balance: &BalanceConfig,
    ) -> (ResourceBundle, Option<ActiveTool>) {
        Self::tribute_from(day, &balance.login_tribute_rewards)
    }

    /// Picks the table entry of a day (1..=7); days outside the table use its last entry.
    fn tribute_from(day: u8, table: &[LoginTributeDay]) -> (ResourceBundle, Option<ActiveTool>) {
        let entry = usize::from(day)
            .checked_sub(1)
            .and_then(|idx| table.get(idx))
            .or_else(|| table.last());
        entry.map_or((ResourceBundle::default(), None), |e| (e.resources, e.tool))
    }

    /// Claims today's 7-day Fjord Login Tribute if available, granting resources and optional tool.
    pub fn try_claim_login_tribute(
        &mut self,
        today_day_index: u64,
    ) -> Option<(u8, ResourceBundle, Option<ActiveTool>)> {
        self.claim_login_tribute_from(today_day_index, &default_login_tribute_rewards())
    }

    /// Claims today's Fjord Login Tribute using the balance reward table.
    pub fn try_claim_login_tribute_with(
        &mut self,
        today_day_index: u64,
        balance: &BalanceConfig,
    ) -> Option<(u8, ResourceBundle, Option<ActiveTool>)> {
        self.claim_login_tribute_from(today_day_index, &balance.login_tribute_rewards)
    }

    /// Claims today's Fjord Login Tribute from an explicit reward table.
    fn claim_login_tribute_from(
        &mut self,
        today_day_index: u64,
        table: &[LoginTributeDay],
    ) -> Option<(u8, ResourceBundle, Option<ActiveTool>)> {
        if !self.is_login_tribute_available(today_day_index) {
            return None;
        }
        let day = self.next_login_tribute_day(today_day_index);
        let (res, tool) = Self::tribute_from(day, table);
        self.resources.add(&res);
        if let Some(t) = tool {
            self.tools.grant_one(t);
        }
        self.login_streak_day = day;
        self.last_login_claim_day = Some(today_day_index);
        Some((day, res, tool))
    }

    /// Returns the player's current Viking Clan Rank based on total Glory.
    pub fn clan_rank(&self) -> ClanRank {
        ClanRank::from_glory(self.glory)
    }

    /// Returns the player's current Viking Clan Rank using externalized balance thresholds.
    pub fn clan_rank_with_balance(&self, balance: &BalanceConfig) -> ClanRank {
        ClanRank::from_glory_with_balance(self.glory, balance)
    }

    pub fn building_tier(&self, building_id: &str) -> u32 {
        self.building_tiers.get(building_id).copied().unwrap_or(0)
    }

    pub fn building_wins(&self, building_id: &str) -> u32 {
        self.building_wins.get(building_id).copied().unwrap_or(0)
    }

    pub fn building_mastery_stars(&self, building_id: &str) -> u8 {
        self.building_wins(building_id)
            .min(MAX_MASTERY_STARS as u32) as u8
    }

    pub fn level_stars(&self, level_id: &str) -> u8 {
        self.level_best_stars.get(level_id).copied().unwrap_or(0)
    }

    /// Returns true if a building is unlocked based on Great Hall tier.
    pub fn is_building_unlocked(&self, def: &BuildingDef) -> bool {
        self.building_tier(HUB_BUILDING_ID) >= def.requires_great_hall_tier
    }

    /// Returns the cost of the next upgrade tier, if not yet maxed.
    pub fn next_upgrade_cost<'a>(&self, def: &'a BuildingDef) -> Option<&'a ResourceBundle> {
        let current = self.building_tier(&def.id) as usize;
        if current >= def.max_tier as usize {
            None
        } else {
            def.upgrade_costs.get(current)
        }
    }

    /// Computes aggregate passive bonuses granted by all rebuilt buildings and Clan Rank.
    pub fn compute_bonuses(&self, balance: &BalanceConfig) -> CityBonuses {
        let rank = self.clan_rank_with_balance(balance);
        let mut bonuses = CityBonuses {
            starting_fury: rank.starting_fury_with(balance),
            starting_boosters: if self.stats.win_streak >= balance.win_streak_booster_threshold {
                balance.win_streak_starting_boosters
            } else {
                0
            },
            ..CityBonuses::default()
        };
        for binding in &balance.building_bonuses {
            let tier = self.building_tier(&binding.building_id);
            let (slot, per_tier) = match binding.kind {
                BuildingBonusKind::ExtraMaxEnergy => (
                    &mut bonuses.extra_max_energy,
                    balance.great_hall_energy_per_tier,
                ),
                BuildingBonusKind::ScoreBonusPct => (
                    &mut bonuses.score_bonus_pct,
                    balance.forge_score_pct_per_tier,
                ),
                BuildingBonusKind::ResourceBonusPct => (
                    &mut bonuses.resource_bonus_pct,
                    balance.dock_resource_pct_per_tier,
                ),
                BuildingBonusKind::ExtraMoves => {
                    (&mut bonuses.extra_moves, balance.shrine_moves_per_tier)
                }
                BuildingBonusKind::EnergyRegenSpeedPct => (
                    &mut bonuses.energy_regen_speed_pct,
                    balance.meadery_regen_pct_per_tier,
                ),
                BuildingBonusKind::FuryChargeBonusPct => (
                    &mut bonuses.fury_charge_bonus_pct,
                    balance.watchtower_fury_pct_per_tier,
                ),
            };
            *slot = slot.saturating_add(tier.saturating_mul(per_tier));
        }
        bonuses.resource_bonus_pct = bonuses
            .resource_bonus_pct
            .saturating_add(rank.resource_bonus_pct_with(balance));
        bonuses
    }

    /// Increments the active win streak and updates the lifetime best streak.
    fn bump_win_streak(&mut self) {
        self.stats.win_streak = self.stats.win_streak.saturating_add(1);
        self.stats.best_win_streak = self.stats.best_win_streak.max(self.stats.win_streak);
    }

    /// Resets the active win streak on defeat or retreat.
    pub fn reset_win_streak(&mut self) {
        self.stats.win_streak = 0;
    }

    /// Returns effective max endurance including Great Hall bonus.
    pub fn max_energy(&self, balance: &BalanceConfig) -> u32 {
        balance
            .max_energy
            .saturating_add(self.compute_bonuses(balance).extra_max_energy)
    }

    /// Returns passive harvest output for a rebuilt building based on its RON `harvest_per_tier` and tier.
    pub fn harvest_reward(&self, def: &BuildingDef) -> ResourceBundle {
        let tier = self.building_tier(&def.id);
        if tier == 0 {
            return ResourceBundle::default();
        }
        ResourceBundle {
            wood: def.harvest_per_tier.wood * tier,
            stone: def.harvest_per_tier.stone * tier,
            gold: def.harvest_per_tier.gold * tier,
            mead: def.harvest_per_tier.mead * tier,
        }
    }

    /// Returns remaining seconds until a rebuilt building's next passive harvest (`Some(0)` when ready, `None` if ruined).
    pub fn harvest_remaining_secs(
        &self,
        building_id: &str,
        now_secs: u64,
        balance: &BalanceConfig,
    ) -> Option<u64> {
        if self.building_tier(building_id) == 0 || balance.harvest_interval_seconds == 0 {
            return None;
        }
        let last = self
            .last_harvest_unix_secs
            .get(building_id)
            .copied()
            .unwrap_or(0);
        let elapsed = now_secs.saturating_sub(last);
        Some(balance.harvest_interval_seconds.saturating_sub(elapsed))
    }

    /// Returns true when a rebuilt building has a passive resource harvest ready to collect.
    pub fn is_harvest_ready(
        &self,
        building_id: &str,
        now_secs: u64,
        balance: &BalanceConfig,
    ) -> bool {
        self.harvest_remaining_secs(building_id, now_secs, balance) == Some(0)
    }

    /// Collects a building's ready passive resource harvest and resets its timer.
    pub fn try_collect_harvest(
        &mut self,
        def: &BuildingDef,
        now_secs: u64,
        balance: &BalanceConfig,
    ) -> Option<ResourceBundle> {
        if !self.is_harvest_ready(&def.id, now_secs, balance) {
            return None;
        }
        let reward = self.harvest_reward(def);
        if reward.is_zero() {
            return None;
        }
        self.resources.add(&reward);
        self.last_harvest_unix_secs.insert(def.id.clone(), now_secs);
        Some(reward)
    }

    /// Attempts to upgrade a building, deducting resources and granting clan glory and concrete perks.
    pub fn try_upgrade_building(
        &mut self,
        def: &BuildingDef,
        balance: &BalanceConfig,
    ) -> Result<u32, &'static str> {
        if !self.is_building_unlocked(def) {
            return Err("city.panel.building.locked");
        }
        let Some(cost) = self.next_upgrade_cost(def).copied() else {
            return Err("city.button.max_level");
        };
        if !self.resources.can_afford(&cost) {
            return Err("city.error.not_enough_resources");
        }
        self.resources.subtract(&cost);
        let new_tier = self.building_tier(&def.id).saturating_add(1);
        self.building_tiers.insert(def.id.clone(), new_tier);
        self.glory = self.glory.saturating_add(def.glory_per_tier);
        for perk in &def.upgrade_perks {
            match *perk {
                UpgradePerk::GrantTool(tool) => self.tools.grant_one(tool),
                UpgradePerk::GainMaxEnergyBonus => {
                    self.energy = self
                        .energy
                        .saturating_add(balance.great_hall_energy_per_tier);
                }
                UpgradePerk::RefillEnergy => {
                    let cap = self.max_energy(balance);
                    self.energy = self.energy.max(cap);
                }
            }
        }
        Ok(new_tier)
    }

    /// Crafts one charge of an active tool using village resources.
    pub fn try_craft_tool(&mut self, recipe: &ToolRecipeDef) -> Result<u32, &'static str> {
        if !self.resources.can_afford(&recipe.cost) {
            return Err("city.error.not_enough_resources");
        }
        self.resources.subtract(&recipe.cost);
        self.tools.grant_one(recipe.tool);
        Ok(self.tools.count(recipe.tool))
    }

    /// Returns the current tier (0 = uncrafted, 1..=3 = crafted/upgraded) of an amulet.
    pub fn amulet_tier(&self, amulet_id: &str) -> u8 {
        if let Some(&t) = self.amulet_tiers.get(amulet_id) {
            return t.min(MAX_AMULET_TIER);
        }
        if self.crafted_amulets.contains(amulet_id) {
            1
        } else {
            0
        }
    }

    /// Returns the resource cost to craft (tier 0->1) or upgrade (tier 1->2, 2->3) an amulet.
    pub fn amulet_next_cost(&self, amulet: &AmuletDef) -> Option<ResourceBundle> {
        let tier = self.amulet_tier(&amulet.id);
        if tier >= MAX_AMULET_TIER {
            return None;
        }
        let mult = (tier as u32) + 1;
        Some(ResourceBundle {
            wood: amulet.cost.wood * mult,
            stone: amulet.cost.stone * mult,
            gold: amulet.cost.gold * mult,
            mead: amulet.cost.mead * mult,
        })
    }

    /// Crafts a Nordic Amulet (tier 1) or equips it if already crafted.
    pub fn try_craft_amulet(
        &mut self,
        amulet: &AmuletDef,
        balance: &BalanceConfig,
    ) -> Result<(), &'static str> {
        if self.crafted_amulets.contains(&amulet.id) {
            self.equipped_amulet = Some(amulet.id.clone());
            return Ok(());
        }
        if !self.resources.can_afford(&amulet.cost) {
            return Err("city.error.not_enough_resources");
        }
        self.resources.subtract(&amulet.cost);
        self.crafted_amulets.insert(amulet.id.clone());
        self.amulet_tiers.insert(amulet.id.clone(), 1);
        self.equipped_amulet = Some(amulet.id.clone());
        self.glory = self.glory.saturating_add(balance.amulet_glory);
        Ok(())
    }

    /// Upgrades an already crafted amulet to the next tier (up to `MAX_AMULET_TIER`).
    pub fn try_upgrade_amulet(
        &mut self,
        amulet: &AmuletDef,
        balance: &BalanceConfig,
    ) -> Result<u8, &'static str> {
        let current = self.amulet_tier(&amulet.id);
        if current == 0 {
            self.try_craft_amulet(amulet, balance)?;
            return Ok(1);
        }
        let Some(cost) = self.amulet_next_cost(amulet) else {
            return Err("city.button.max_level");
        };
        if !self.resources.can_afford(&cost) {
            return Err("city.error.not_enough_resources");
        }
        self.resources.subtract(&cost);
        let next = (current + 1).min(MAX_AMULET_TIER);
        self.amulet_tiers.insert(amulet.id.clone(), next);
        self.equipped_amulet = Some(amulet.id.clone());
        self.glory = self.glory.saturating_add(balance.amulet_glory);
        Ok(next)
    }

    /// Scales an amulet's base effect to an explicit tier (`1..=3`).
    pub fn scaled_amulet_effect_for_tier(def: &AmuletDef, tier: u8) -> AmuletEffect {
        let extra_tiers = tier.max(1).saturating_sub(1) as u32;
        let step = |default: u32| def.tier_step.unwrap_or(default);
        match def.effect {
            AmuletEffect::ResourceBonusPct(base) => AmuletEffect::ResourceBonusPct(
                base + extra_tiers * step(DEFAULT_AMULET_RESOURCE_PER_TIER),
            ),
            AmuletEffect::ExtraMoves(base) => {
                AmuletEffect::ExtraMoves(base + extra_tiers * step(DEFAULT_AMULET_MOVES_PER_TIER))
            }
            AmuletEffect::ScoreBonusPct(base) => AmuletEffect::ScoreBonusPct(
                base + extra_tiers * step(DEFAULT_AMULET_SCORE_PER_TIER),
            ),
            AmuletEffect::ExtraTimePct(base) => {
                AmuletEffect::ExtraTimePct(base + extra_tiers * step(DEFAULT_AMULET_TIME_PER_TIER))
            }
            AmuletEffect::StartingBoosters(base) => AmuletEffect::StartingBoosters(
                base + extra_tiers * step(DEFAULT_AMULET_BOOSTERS_PER_TIER),
            ),
        }
    }

    /// Scales an amulet's base effect by its current tier (`1..=3`).
    pub fn scaled_amulet_effect(&self, def: &AmuletDef) -> AmuletEffect {
        Self::scaled_amulet_effect_for_tier(def, self.amulet_tier(&def.id).max(1))
    }

    /// Returns the passive effect of the currently equipped amulet, scaled by its tier (1..=3).
    pub fn active_amulet_effect(&self, amulets: &[AmuletDef]) -> Option<AmuletEffect> {
        let id = self.equipped_amulet.as_deref()?;
        if self.amulet_tier(id) == 0 {
            return None;
        }
        let def = amulets.iter().find(|a| a.id == id)?;
        Some(self.scaled_amulet_effect(def))
    }

    /// Consumes endurance to launch a Match-3 level.
    pub fn try_spend_energy(&mut self, balance: &BalanceConfig) -> bool {
        if self.energy >= balance.energy_cost_per_level {
            self.energy -= balance.energy_cost_per_level;
            true
        } else {
            false
        }
    }

    /// Restores endurance up to the effective cap (including Great Hall bonus).
    pub fn regenerate_energy(&mut self, amount: u32, balance: &BalanceConfig) {
        let cap = self.max_energy(balance);
        self.energy = self.energy.saturating_add(amount).min(cap);
    }

    /// Drinks Mead to immediately restore Endurance so players are never soft-locked.
    pub fn try_drink_mead(&mut self, balance: &BalanceConfig) -> Result<u32, &'static str> {
        let cap = self.max_energy(balance);
        if self.energy >= cap {
            return Ok(self.energy);
        }
        if self.resources.mead < balance.mead_refill_cost {
            return Err("city.error.not_enough_resources");
        }
        self.resources.mead -= balance.mead_refill_cost;
        self.energy = self
            .energy
            .saturating_add(balance.mead_refill_energy)
            .min(cap);
        Ok(self.energy)
    }

    /// Applies offline endurance regeneration based on elapsed seconds since last save.
    pub fn apply_offline_energy_regen(&mut self, elapsed_secs: u64, balance: &BalanceConfig) {
        if balance.energy_regen_seconds == 0 || elapsed_secs == 0 {
            return;
        }
        let speed_pct = 100 + self.compute_bonuses(balance).energy_regen_speed_pct as u64;
        let effective_secs = elapsed_secs.saturating_mul(speed_pct) / 100;
        let gained =
            u32::try_from(effective_secs / balance.energy_regen_seconds).unwrap_or(u32::MAX);
        if gained > 0 {
            self.regenerate_energy(gained, balance);
        }
    }

    /// Records a Match-3 victory, granting earned rewards, mastery progress, stars, and a free tool per new star.
    pub fn record_level_victory(
        &mut self,
        building_id: &str,
        level_id: &str,
        stars: u8,
        rewards: &ResourceBundle,
        balance: &BalanceConfig,
    ) {
        self.resources.add(rewards);
        self.completed_levels.insert(level_id.to_string());
        let entry = self
            .level_best_stars
            .entry(level_id.to_string())
            .or_insert(0);
        *entry = (*entry).max(stars.clamp(1, MAX_LEVEL_STARS));

        let prev_stars = self.building_mastery_stars(building_id);
        let wins = self
            .building_wins
            .entry(building_id.to_string())
            .or_insert(0);
        *wins = wins.saturating_add(1);
        let new_stars = self.building_mastery_stars(building_id);
        if new_stars > prev_stars {
            let ladder = &balance.mastery_star_tools;
            let rung = usize::from(new_stars).saturating_sub(1);
            if let Some(&tool) = ladder.get(rung).or_else(|| ladder.last()) {
                self.tools.grant_one(tool);
            }
        }
        self.glory = self.glory.saturating_add(balance.victory_glory);
        self.stats.total_victories = self.stats.total_victories.saturating_add(1);
        self.bump_win_streak();
    }

    /// Returns true when today's Daily Trial has not yet been completed.
    pub fn is_daily_available(&self, day_index: u64) -> bool {
        self.stats.last_daily_day_index < day_index
    }

    /// Applies the default Daily Trial streak multiplier (`+10%` per consecutive day beyond day 1).
    pub fn scale_daily_rewards(rewards: &ResourceBundle, streak: u32) -> ResourceBundle {
        Self::scale_by_pct(rewards, streak, DEFAULT_DAILY_STREAK_BONUS_PCT_PER_DAY)
    }

    /// Applies the Daily Trial streak multiplier using the balance percent per extra streak day.
    pub fn scale_daily_rewards_with(
        rewards: &ResourceBundle,
        streak: u32,
        balance: &BalanceConfig,
    ) -> ResourceBundle {
        Self::scale_by_pct(rewards, streak, balance.daily_streak_bonus_pct_per_day)
    }

    /// Scales a reward bundle by `pct_per_day` percent for each streak day beyond the first.
    fn scale_by_pct(rewards: &ResourceBundle, streak: u32, pct_per_day: u32) -> ResourceBundle {
        let streak_bonus_pct = u128::from(streak.saturating_sub(1)) * u128::from(pct_per_day);
        let scale = |base: u32| -> u32 {
            let bonus = u128::from(base) * streak_bonus_pct / 100;
            u32::try_from(u128::from(base) + bonus).unwrap_or(u32::MAX)
        };
        ResourceBundle {
            wood: scale(rewards.wood),
            stone: scale(rewards.stone),
            gold: scale(rewards.gold),
            mead: scale(rewards.mead),
        }
    }

    /// Marks today's Daily Trial as completed, updating streak and granting escalating rewards.
    pub fn record_daily_victory(
        &mut self,
        day_index: u64,
        rewards: &ResourceBundle,
        balance: &BalanceConfig,
    ) -> u32 {
        let new_streak = if self.stats.last_daily_day_index > 0
            && self.stats.last_daily_day_index.saturating_add(1) == day_index
        {
            (self.stats.daily_streak % MAX_DAILY_STREAK_CYCLE).saturating_add(1)
        } else {
            1
        };
        let scaled_rewards = Self::scale_daily_rewards_with(rewards, new_streak, balance);
        self.resources.add(&scaled_rewards);
        self.stats.last_daily_day_index = day_index;
        self.stats.daily_streak = new_streak;
        self.stats.best_daily_streak = self.stats.best_daily_streak.max(new_streak);
        self.stats.daily_completions = self.stats.daily_completions.saturating_add(1);
        self.stats.total_victories = self.stats.total_victories.saturating_add(1);
        for reward in &balance.daily_streak_tool_rewards {
            if new_streak >= reward.min_streak {
                self.tools.grant_one(reward.tool);
            }
        }
        self.glory = self.glory.saturating_add(balance.daily_glory);
        self.bump_win_streak();
        new_streak
    }

    /// Evaluates whether a quest condition is currently satisfied.
    pub fn is_quest_ready(&self, quest: &QuestDef) -> bool {
        if self.claimed_quests.contains(&quest.id) {
            return false;
        }
        match &quest.condition {
            QuestCondition::WinBuildingLevel { building_id, wins } => {
                self.building_wins(building_id) >= *wins
            }
            QuestCondition::WinSpecificLevel { level_id } => {
                self.completed_levels.contains(level_id)
            }
            QuestCondition::ReachBuildingTier { building_id, tier } => {
                self.building_tier(building_id) >= *tier
            }
            QuestCondition::CraftAnyAmulet => !self.crafted_amulets.is_empty(),
            QuestCondition::ClaimAnyCollection => !self.claimed_collections.is_empty(),
            QuestCondition::CompleteDailyTrials { count } => self.stats.daily_completions >= *count,
            QuestCondition::DefeatBosses { count } => {
                self.stats.bosses_defeated >= *count || self.defeated_bosses.len() as u32 >= *count
            }
            QuestCondition::ReachExpeditionWave { wave } => {
                self.stats.best_expedition_wave >= *wave
            }
        }
    }

    /// Returns the first unclaimed Saga quest.
    pub fn active_quest<'a>(&self, quests: &'a [QuestDef]) -> Option<&'a QuestDef> {
        quests.iter().find(|q| !self.claimed_quests.contains(&q.id))
    }

    /// Claims a completed quest's reward.
    pub fn try_claim_quest(&mut self, quest: &QuestDef, balance: &BalanceConfig) -> bool {
        if !self.is_quest_ready(quest) {
            return false;
        }
        self.claimed_quests.insert(quest.id.clone());
        self.resources.add(&quest.reward);
        self.glory = self.glory.saturating_add(balance.quest_glory);
        true
    }

    /// Discovers the next undiscovered relic preferring collections tied to `building_id`,
    /// falling back to any undiscovered relic so progress never stalls.
    pub fn discover_next_relic_for_building(
        &mut self,
        building_id: &str,
        collections: &[CollectionDef],
    ) -> Option<String> {
        for col in collections {
            let matches_building = col.source_building_ids.is_empty()
                || col.source_building_ids.iter().any(|b| b == building_id);
            if !matches_building {
                continue;
            }
            for (idx, rid) in col.relic_ids.iter().enumerate() {
                if !self.found_relics.contains(rid) {
                    self.found_relics.insert(rid.clone());
                    return col.relic_name_keys.get(idx).cloned();
                }
            }
        }
        self.discover_next_relic(collections)
    }

    /// Discovers the next undiscovered relic across all collections, returning its i18n name key.
    pub fn discover_next_relic(&mut self, collections: &[CollectionDef]) -> Option<String> {
        for col in collections {
            for (idx, rid) in col.relic_ids.iter().enumerate() {
                if !self.found_relics.contains(rid) {
                    self.found_relics.insert(rid.clone());
                    return col.relic_name_keys.get(idx).cloned();
                }
            }
        }
        None
    }

    /// Counts how many relics in a collection have been discovered.
    pub fn collection_found_count(&self, col: &CollectionDef) -> usize {
        col.relic_ids
            .iter()
            .filter(|rid| self.found_relics.contains(*rid))
            .count()
    }

    /// Returns true if every relic in a collection is found and the chest is not yet claimed.
    pub fn is_collection_ready(&self, col: &CollectionDef) -> bool {
        !self.claimed_collections.contains(&col.id)
            && self.collection_found_count(col) == col.relic_ids.len()
    }

    /// Returns the number of collection chests ready to claim.
    pub fn claimable_collection_count(&self, collections: &[CollectionDef]) -> usize {
        collections
            .iter()
            .filter(|c| self.is_collection_ready(c))
            .count()
    }

    /// Claims a completed Nine Realms collection chest (resources + active tools + glory).
    pub fn try_claim_collection(&mut self, col: &CollectionDef, balance: &BalanceConfig) -> bool {
        if !self.is_collection_ready(col) {
            return false;
        }
        self.claimed_collections.insert(col.id.clone());
        self.resources.add(&col.reward_resources);
        self.tools.add(&col.reward_tools);
        self.glory = self.glory.saturating_add(balance.collection_glory);
        true
    }

    /// Collects all ready passive building tributes at once.
    pub fn collect_all_harvests(
        &mut self,
        buildings: &[BuildingDef],
        now_secs: u64,
        balance: &BalanceConfig,
    ) -> (usize, ResourceBundle) {
        let mut count = 0usize;
        let mut total = ResourceBundle::default();
        for b in buildings {
            if let Some(r) = self.try_collect_harvest(b, now_secs, balance) {
                count += 1;
                total.add(&r);
            }
        }
        (count, total)
    }

    /// Records a Mythological Boss Raid victory using the boss's base reward bundle.
    pub fn record_boss_victory(&mut self, boss: &BossDef) {
        self.record_boss_victory_with_rewards(boss, &boss.reward_resources);
    }

    /// Records a Mythological Boss Raid victory with scaled spoils (Freya Fury, Amulets, Dock, Rank).
    pub fn record_boss_victory_with_rewards(&mut self, boss: &BossDef, rewards: &ResourceBundle) {
        self.resources.add(rewards);
        self.tools.add(&boss.reward_tools);
        self.glory = self.glory.saturating_add(boss.glory_reward);
        self.defeated_bosses.insert(boss.id.clone());
        self.stats.bosses_defeated = self.stats.bosses_defeated.saturating_add(1);
        self.stats.total_victories = self.stats.total_victories.saturating_add(1);
        self.bump_win_streak();
    }

    /// Records an Endless Drakkar Expedition wave victory and updates best wave record.
    pub fn record_expedition_wave(
        &mut self,
        wave: u32,
        rewards: &ResourceBundle,
        balance: &BalanceConfig,
    ) {
        self.resources.add(rewards);
        self.stats.best_expedition_wave = self.stats.best_expedition_wave.max(wave);
        self.stats.total_victories = self.stats.total_victories.saturating_add(1);
        self.bump_win_streak();
        self.glory = self.glory.saturating_add(
            balance
                .victory_glory
                .saturating_add(wave.saturating_mul(balance.expedition_glory_per_wave)),
        );
    }

    /// Returns the number of unlocked Skalds' Feats ready to claim.
    pub fn claimable_achievement_count(
        &self,
        achievements: &[AchievementDef],
        buildings: &[BuildingDef],
        total_bosses: usize,
    ) -> usize {
        achievements
            .iter()
            .filter(|ach| {
                !self.claimed_achievements.contains(&ach.id)
                    && is_achievement_unlocked(ach, self, buildings, total_bosses)
            })
            .count()
    }

    /// Returns true if at least one Skald's Feat is unlocked and ready to claim.
    pub fn has_any_claimable_achievement(
        &self,
        achievements: &[AchievementDef],
        buildings: &[BuildingDef],
        total_bosses: usize,
    ) -> bool {
        self.claimable_achievement_count(achievements, buildings, total_bosses) > 0
    }

    /// Claims a completed Skald's Feat reward.
    pub fn try_claim_achievement(
        &mut self,
        ach_id: &str,
        achievements: &[AchievementDef],
        buildings: &[BuildingDef],
        total_bosses: usize,
    ) -> bool {
        if self.claimed_achievements.contains(ach_id) {
            return false;
        }
        let Some(ach) = achievements.iter().find(|a| a.id == ach_id) else {
            return false;
        };
        if !is_achievement_unlocked(ach, self, buildings, total_bosses) {
            return false;
        }
        self.claimed_achievements.insert(ach_id.to_string());
        self.resources.add(&ach.reward);
        self.glory = self.glory.saturating_add(ach.glory);
        true
    }
}

/// Checks structural invariants of the village content definitions and returns every problem found.
pub fn validate_city_content(
    buildings: &[BuildingDef],
    amulets: &[AmuletDef],
    achievements: &[AchievementDef],
) -> Vec<String> {
    let mut problems = Vec::new();
    if !buildings.iter().any(|b| b.id == HUB_BUILDING_ID) {
        problems.push(format!(
            "buildings: hub building '{HUB_BUILDING_ID}' is missing"
        ));
    }
    for amulet in amulets {
        if amulet.tier_step == Some(0) {
            problems.push(format!("amulets: '{}' has a zero tier_step", amulet.id));
        }
    }
    for ach in achievements {
        match ach.condition {
            None => problems.push(format!("achievements: '{}' has no condition", ach.id)),
            Some(condition) => {
                if condition.threshold() == Some(0) {
                    problems.push(format!(
                        "achievements: '{}' has a threshold below 1",
                        ach.id
                    ));
                }
            }
        }
    }
    problems
}
