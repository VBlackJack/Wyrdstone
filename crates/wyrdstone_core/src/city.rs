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

//! Fjord city-builder progression: resources, buildings, mastery stars, amulets, quests, and tools.

use crate::balance::BalanceConfig;
use crate::board::ActiveTool;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Maximum mastery star rank per building location.
pub const MAX_MASTERY_STARS: u8 = 5;

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
        self.wood += other.wood;
        self.stone += other.stone;
        self.gold += other.gold;
        self.mead += other.mead;
    }

    pub fn subtract(&mut self, cost: &ResourceBundle) {
        self.wood = self.wood.saturating_sub(cost.wood);
        self.stone = self.stone.saturating_sub(cost.stone);
        self.gold = self.gold.saturating_sub(cost.gold);
        self.mead = self.mead.saturating_sub(cost.mead);
    }
}

/// Passive buff granted by an equipped Nordic Amulet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmuletEffect {
    ResourceBonusPct(u32),
    ExtraMoves(u32),
    ScoreBonusPct(u32),
}

/// Static Nordic Amulet definition loaded from `assets/data/amulets.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmuletDef {
    pub id: String,
    pub name_key: String,
    pub desc_key: String,
    pub cost: ResourceBundle,
    pub effect: AmuletEffect,
}

/// Static building definition loaded from `assets/data/buildings.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingDef {
    pub id: String,
    pub name_key: String,
    pub desc_key: String,
    pub max_tier: u32,
    pub requires_great_hall_tier: u32,
    pub level_id: String,
    #[serde(default)]
    pub mastery_level_id: Option<String>,
    pub upgrade_costs: Vec<ResourceBundle>,
    pub glory_per_tier: u32,
}

/// Completion requirement for a Saga quest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestCondition {
    WinBuildingLevel { building_id: String, wins: u32 },
    ReachBuildingTier { building_id: String, tier: u32 },
    CraftAnyAmulet,
}

/// Static Saga quest loaded from `assets/data/quests.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestDef {
    pub id: String,
    pub title_key: String,
    pub desc_key: String,
    pub condition: QuestCondition,
    pub reward: ResourceBundle,
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
    pub equipped_amulet: Option<String>,
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
            equipped_amulet: None,
        }
    }

    pub fn building_tier(&self, building_id: &str) -> u32 {
        self.building_tiers.get(building_id).copied().unwrap_or(0)
    }

    pub fn building_wins(&self, building_id: &str) -> u32 {
        self.building_wins.get(building_id).copied().unwrap_or(0)
    }

    pub fn building_mastery_stars(&self, building_id: &str) -> u8 {
        self.building_wins(building_id).min(MAX_MASTERY_STARS as u32) as u8
    }

    /// Returns true if a building is unlocked based on Great Hall tier.
    pub fn is_building_unlocked(&self, def: &BuildingDef) -> bool {
        self.building_tier("great_hall") >= def.requires_great_hall_tier
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

    /// Attempts to upgrade a building, deducting resources and granting clan glory.
    pub fn try_upgrade_building(&mut self, def: &BuildingDef) -> Result<u32, &'static str> {
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
        let new_tier = self.building_tier(&def.id) + 1;
        self.building_tiers.insert(def.id.clone(), new_tier);
        self.glory += def.glory_per_tier;
        if def.id == "forge" {
            self.tools.mjolnir += 1;
            self.tools.thor_bolt += 1;
        }
        Ok(new_tier)
    }

    /// Crafts a Nordic Amulet and equips it immediately.
    pub fn try_craft_amulet(&mut self, amulet: &AmuletDef) -> Result<(), &'static str> {
        if self.crafted_amulets.contains(&amulet.id) {
            self.equipped_amulet = Some(amulet.id.clone());
            return Ok(());
        }
        if !self.resources.can_afford(&amulet.cost) {
            return Err("city.error.not_enough_resources");
        }
        self.resources.subtract(&amulet.cost);
        self.crafted_amulets.insert(amulet.id.clone());
        self.equipped_amulet = Some(amulet.id.clone());
        self.glory += 20;
        Ok(())
    }

    /// Equips an already crafted amulet.
    pub fn equip_amulet(&mut self, amulet_id: &str) -> bool {
        if self.crafted_amulets.contains(amulet_id) {
            self.equipped_amulet = Some(amulet_id.to_string());
            true
        } else {
            false
        }
    }

    /// Returns the passive effect of the currently equipped amulet, if any.
    pub fn active_amulet_effect(&self, amulets: &[AmuletDef]) -> Option<AmuletEffect> {
        let id = self.equipped_amulet.as_deref()?;
        amulets.iter().find(|a| a.id == id).map(|a| a.effect)
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

    /// Restores endurance up to the configured cap.
    pub fn regenerate_energy(&mut self, amount: u32, balance: &BalanceConfig) {
        self.energy = (self.energy + amount).min(balance.max_energy);
    }

    /// Records a Match-3 victory, granting earned rewards and mastery progress.
    pub fn record_level_victory(&mut self, building_id: &str, rewards: &ResourceBundle) {
        self.resources.add(rewards);
        *self.building_wins.entry(building_id.to_string()).or_insert(0) += 1;
        self.glory += 10;
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
            QuestCondition::ReachBuildingTier { building_id, tier } => {
                self.building_tier(building_id) >= *tier
            }
            QuestCondition::CraftAnyAmulet => !self.crafted_amulets.is_empty(),
        }
    }

    /// Returns the first unclaimed Saga quest.
    pub fn active_quest<'a>(&self, quests: &'a [QuestDef]) -> Option<&'a QuestDef> {
        quests.iter().find(|q| !self.claimed_quests.contains(&q.id))
    }

    /// Claims a completed quest's reward.
    pub fn try_claim_quest(&mut self, quest: &QuestDef) -> bool {
        if !self.is_quest_ready(quest) {
            return false;
        }
        self.claimed_quests.insert(quest.id.clone());
        self.resources.add(&quest.reward);
        self.glory += 15;
        true
    }
}
