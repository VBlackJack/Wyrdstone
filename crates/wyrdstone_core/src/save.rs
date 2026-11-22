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

//! Bundled game data loader and persistent JSON save schema.

use crate::balance::BalanceConfig;
use crate::city::{AmuletDef, BuildingDef, CityState, QuestDef};
use crate::i18n::{I18nCatalog, Language};
use crate::level::LevelDef;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Current save format version for forward-safe migrations.
pub const SAVE_SCHEMA_VERSION: u32 = 2;

const BUNDLED_EN_JSON: &str = include_str!("../../../assets/locales/en.json");
const BUNDLED_FR_JSON: &str = include_str!("../../../assets/locales/fr.json");
const BUNDLED_BALANCE_RON: &str = include_str!("../../../assets/data/balance.ron");
const BUNDLED_BUILDINGS_RON: &str = include_str!("../../../assets/data/buildings.ron");
const BUNDLED_LEVELS_RON: &str = include_str!("../../../assets/data/levels.ron");
const BUNDLED_QUESTS_RON: &str = include_str!("../../../assets/data/quests.ron");
const BUNDLED_AMULETS_RON: &str = include_str!("../../../assets/data/amulets.ron");

/// All static game definitions loaded once at startup.
#[derive(Debug, Clone)]
pub struct GameDataBundle {
    pub balance: BalanceConfig,
    pub buildings: Vec<BuildingDef>,
    pub levels: Vec<LevelDef>,
    pub quests: Vec<QuestDef>,
    pub amulets: Vec<AmuletDef>,
    pub i18n: I18nCatalog,
}

impl GameDataBundle {
    /// Loads the bundled assets embedded into the binary (identical on Windows and Android).
    pub fn load_bundled(initial_language: Language) -> Result<Self, String> {
        let balance = BalanceConfig::from_ron_str(BUNDLED_BALANCE_RON)
            .map_err(|e| format!("Failed to parse balance.ron: {e}"))?;
        let buildings: Vec<BuildingDef> = ron::from_str(BUNDLED_BUILDINGS_RON)
            .map_err(|e| format!("Failed to parse buildings.ron: {e}"))?;
        let levels: Vec<LevelDef> = ron::from_str(BUNDLED_LEVELS_RON)
            .map_err(|e| format!("Failed to parse levels.ron: {e}"))?;
        let quests: Vec<QuestDef> = ron::from_str(BUNDLED_QUESTS_RON)
            .map_err(|e| format!("Failed to parse quests.ron: {e}"))?;
        let amulets: Vec<AmuletDef> = ron::from_str(BUNDLED_AMULETS_RON)
            .map_err(|e| format!("Failed to parse amulets.ron: {e}"))?;
        let i18n = I18nCatalog::from_json_strs(BUNDLED_EN_JSON, BUNDLED_FR_JSON, initial_language)
            .map_err(|e| format!("Failed to parse locales: {e}"))?;

        Ok(Self {
            balance,
            buildings,
            levels,
            quests,
            amulets,
            i18n,
        })
    }

    /// Finds a level definition by its unique identifier.
    pub fn level_by_id(&self, id: &str) -> Option<&LevelDef> {
        self.levels.iter().find(|lvl| lvl.id == id)
    }

    /// Finds a building definition by its unique identifier.
    pub fn building_by_id(&self, id: &str) -> Option<&BuildingDef> {
        self.buildings.iter().find(|b| b.id == id)
    }

    /// Finds an amulet definition by its unique identifier.
    pub fn amulet_by_id(&self, id: &str) -> Option<&AmuletDef> {
        self.amulets.iter().find(|a| a.id == id)
    }
}

/// Serializable player progression snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub schema_version: u32,
    pub language: Language,
    pub city: CityState,
}

impl SaveData {
    /// Creates a fresh save from balance defaults.
    pub fn new_game(balance: &BalanceConfig, language: Language) -> Self {
        Self {
            schema_version: SAVE_SCHEMA_VERSION,
            language,
            city: CityState::new(balance),
        }
    }

    /// Serializes save data to pretty JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes save data from JSON.
    pub fn from_json(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }

    /// Writes save data to disk when filesystem storage is available.
    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create save directory: {e}"))?;
        }
        let json = self
            .to_json()
            .map_err(|e| format!("Failed to serialize save: {e}"))?;
        std::fs::write(path, json).map_err(|e| format!("Failed to write save file: {e}"))
    }

    /// Loads save data from disk if it exists.
    pub fn load_from_path(path: &Path) -> Result<Option<Self>, String> {
        if !path.exists() {
            return Ok(None);
        }
        let raw =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read save file: {e}"))?;
        let parsed =
            Self::from_json(&raw).map_err(|e| format!("Failed to parse save file: {e}"))?;
        Ok(Some(parsed))
    }
}
