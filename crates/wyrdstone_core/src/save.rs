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

use crate::achievements::AchievementDef;
use crate::balance::BalanceConfig;
use crate::boss::BossDef;
use crate::city::{AmuletDef, BuildingDef, CityState, CollectionDef, QuestDef, ToolRecipeDef};
use crate::expedition::ExpeditionRun;
use crate::i18n::{I18nCatalog, Language};
use crate::level::LevelDef;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Current save format version for forward-safe migrations.
pub const SAVE_SCHEMA_VERSION: u32 = 5;

/// Default background music volume (0.0..=1.0).
pub const DEFAULT_BGM_VOLUME: f32 = 0.22;
/// Default sound effects volume (0.0..=1.0).
pub const DEFAULT_SFX_VOLUME: f32 = 0.55;

/// Maximum number of read attempts before a save file is considered unreadable.
pub const SAVE_READ_MAX_ATTEMPTS: u32 = 3;
/// Delay between two read attempts, absorbing transient file locks (antivirus, sync tools).
pub const SAVE_READ_RETRY_DELAY_MS: u64 = 80;

/// Result of a recovery-aware save load (see [`SaveData::load_with_recovery`]).
#[derive(Debug, Clone)]
pub enum LoadOutcome {
    /// The primary save file was read successfully.
    Loaded(Box<SaveData>),
    /// The primary file was unreadable, preserved as a timestamped copy, and the `.bak` was used.
    RecoveredFromBackup(Box<SaveData>),
    /// The primary file was unreadable and preserved at the given path; no usable `.bak` existed.
    FreshAfterBackup(PathBuf),
    /// The primary file exists but cannot be read, and it could not be preserved either.
    /// The caller must not overwrite it. When the `.bak` copy is readable it is provided so the
    /// session can start from it (writes stay suspended).
    ReadOnly {
        reason: String,
        backup: Option<Box<SaveData>>,
    },
    /// No save file exists.
    NotFound,
}

/// What a read-only session should do when it retries to preserve an unreadable save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOnlyResolution {
    /// The primary file is readable and valid now (a transient lock ended): keep the session
    /// read-only, since writing would overwrite the player's real progress.
    KeepReadOnly,
    /// The primary file is still unreadable and could not be preserved: keep waiting.
    StillBlocked,
    /// The primary file is genuinely unparsable (a copy was kept) or gone: writing is safe.
    Lift,
}

/// Non-fatal details of a successful save.
#[derive(Debug, Clone, Default)]
pub struct SaveReport {
    /// Set when the `.bak` rotation failed while the primary file was still written correctly.
    pub backup_rotation_error: Option<String>,
}

fn default_bgm_volume() -> f32 {
    DEFAULT_BGM_VOLUME
}

fn default_sfx_volume() -> f32 {
    DEFAULT_SFX_VOLUME
}

const BUNDLED_EN_JSON: &str = include_str!("../../../assets/locales/en.json");
const BUNDLED_FR_JSON: &str = include_str!("../../../assets/locales/fr.json");
const BUNDLED_BALANCE_RON: &str = include_str!("../../../assets/data/balance.ron");
const BUNDLED_BUILDINGS_RON: &str = include_str!("../../../assets/data/buildings.ron");
const BUNDLED_LEVELS_RON: &str = include_str!("../../../assets/data/levels.ron");
const BUNDLED_QUESTS_RON: &str = include_str!("../../../assets/data/quests.ron");
const BUNDLED_AMULETS_RON: &str = include_str!("../../../assets/data/amulets.ron");
const BUNDLED_COLLECTIONS_RON: &str = include_str!("../../../assets/data/collections.ron");
const BUNDLED_TOOL_SHOP_RON: &str = include_str!("../../../assets/data/tool_shop.ron");
const BUNDLED_BOSSES_RON: &str = include_str!("../../../assets/data/bosses.ron");
const BUNDLED_ACHIEVEMENTS_RON: &str = include_str!("../../../assets/data/achievements.ron");

/// All static game definitions loaded once at startup.
#[derive(Debug, Clone)]
pub struct GameDataBundle {
    pub balance: BalanceConfig,
    pub buildings: Vec<BuildingDef>,
    pub levels: Vec<LevelDef>,
    pub quests: Vec<QuestDef>,
    pub amulets: Vec<AmuletDef>,
    pub collections: Vec<CollectionDef>,
    pub tool_shop: Vec<ToolRecipeDef>,
    pub bosses: Vec<BossDef>,
    pub achievements: Vec<AchievementDef>,
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
        let collections: Vec<CollectionDef> = ron::from_str(BUNDLED_COLLECTIONS_RON)
            .map_err(|e| format!("Failed to parse collections.ron: {e}"))?;
        let tool_shop: Vec<ToolRecipeDef> = ron::from_str(BUNDLED_TOOL_SHOP_RON)
            .map_err(|e| format!("Failed to parse tool_shop.ron: {e}"))?;
        let bosses: Vec<BossDef> = ron::from_str(BUNDLED_BOSSES_RON)
            .map_err(|e| format!("Failed to parse bosses.ron: {e}"))?;
        let achievements: Vec<AchievementDef> = ron::from_str(BUNDLED_ACHIEVEMENTS_RON)
            .map_err(|e| format!("Failed to parse achievements.ron: {e}"))?;
        let i18n = I18nCatalog::from_json_strs(BUNDLED_EN_JSON, BUNDLED_FR_JSON, initial_language)
            .map_err(|e| format!("Failed to parse locales: {e}"))?;

        Ok(Self {
            balance,
            buildings,
            levels,
            quests,
            amulets,
            collections,
            tool_shop,
            bosses,
            achievements,
            i18n,
        })
    }

    /// Finds a boss definition by its unique identifier.
    pub fn boss_by_id(&self, id: &str) -> Option<&BossDef> {
        self.bosses.iter().find(|b| b.id == id)
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
    #[serde(default)]
    pub last_saved_unix_secs: u64,
    /// Wall-clock instant from which the next endurance point is being earned.
    #[serde(default)]
    pub energy_regen_anchor_unix_secs: u64,
    #[serde(default)]
    pub muted: bool,
    #[serde(default = "default_bgm_volume")]
    pub bgm_volume: f32,
    #[serde(default = "default_sfx_volume")]
    pub sfx_volume: f32,
    #[serde(default)]
    pub colorblind_mode: bool,
    #[serde(default)]
    pub fast_animations: bool,
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default)]
    pub tutorial_step: u8,
    #[serde(default)]
    pub active_expedition: Option<ExpeditionRun>,
    pub city: CityState,
}

impl SaveData {
    /// Creates a fresh save from balance defaults.
    pub fn new_game(balance: &BalanceConfig, language: Language) -> Self {
        Self {
            schema_version: SAVE_SCHEMA_VERSION,
            language,
            last_saved_unix_secs: 0,
            energy_regen_anchor_unix_secs: 0,
            muted: false,
            bgm_volume: DEFAULT_BGM_VOLUME,
            sfx_volume: DEFAULT_SFX_VOLUME,
            colorblind_mode: false,
            fast_animations: false,
            reduced_motion: false,
            tutorial_step: 0,
            active_expedition: None,
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
    /// Atomically writes save data to disk without rotating the `.bak` copy.
    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        self.save_with_options(path, false).map(|_| ())
    }

    /// Atomically writes save data: temporary file, flush to disk, rename, then directory sync
    /// on Unix. When `rotate_backup` is set, the current primary file is first copied to `.bak`;
    /// callers must only request it when that file is known to be good.
    pub fn save_with_options(
        &self,
        path: &Path,
        rotate_backup: bool,
    ) -> Result<SaveReport, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create save directory: {e}"))?;
        }
        let json = self
            .to_json()
            .map_err(|e| format!("Failed to serialize save: {e}"))?;
        let tmp_path = path.with_extension("json.tmp");
        write_synced(&tmp_path, json.as_bytes())
            .map_err(|e| format!("Failed to write temporary save file: {e}"))?;
        let mut report = SaveReport::default();
        if rotate_backup {
            report.backup_rotation_error = rotate_backup_copy(path).err();
        }
        std::fs::rename(&tmp_path, path)
            .map_err(|e| format!("Failed to commit atomic save rename: {e}"))?;
        sync_parent_dir(path)?;
        Ok(report)
    }

    /// Returns the `.bak` companion path of a primary save file.
    pub fn backup_path(path: &Path) -> PathBuf {
        path.with_extension("json.bak")
    }

    /// Copies an unreadable primary file to a timestamped `.corrupt-<unix_secs>` sibling.
    /// Returns `Ok(None)` when the file no longer exists.
    pub fn backup_unreadable(path: &Path) -> Result<Option<PathBuf>, String> {
        if let Ok(false) = path.try_exists() {
            return Ok(None);
        }
        let dest = path.with_extension(format!("json.corrupt-{}", unix_secs()));
        std::fs::copy(path, &dest)
            .map_err(|e| format!("Failed to copy unreadable save to {}: {e}", dest.display()))?;
        Ok(Some(dest))
    }

    /// Loads a save with recovery: retries transient read errors, preserves an unreadable file
    /// as a timestamped copy, then falls back to the `.bak` copy.
    pub fn load_with_recovery(path: &Path) -> LoadOutcome {
        if let Ok(false) = path.try_exists() {
            return LoadOutcome::NotFound;
        }
        let failure = match read_with_retries(path) {
            Ok(raw) => match Self::from_json(&raw) {
                Ok(parsed) => return LoadOutcome::Loaded(Box::new(parsed.normalized())),
                Err(e) => format!("Failed to parse save file: {e}"),
            },
            Err(e) => format!("Failed to read save file: {e}"),
        };
        match Self::backup_unreadable(path) {
            Err(copy_err) => LoadOutcome::ReadOnly {
                reason: format!("{failure}; {copy_err}"),
                backup: Self::read_backup(path),
            },
            Ok(None) => LoadOutcome::NotFound,
            Ok(Some(copied)) => match Self::read_backup(path) {
                Some(parsed) => LoadOutcome::RecoveredFromBackup(parsed),
                None => LoadOutcome::FreshAfterBackup(copied),
            },
        }
    }

    /// Reads and parses the `.bak` companion of `path`, if usable.
    fn read_backup(path: &Path) -> Option<Box<SaveData>> {
        read_with_retries(&Self::backup_path(path))
            .ok()
            .and_then(|raw| Self::from_json(&raw).ok())
            .map(|parsed| Box::new(parsed.normalized()))
    }

    /// Decides how a read-only session proceeds, re-checking the primary file first so that a
    /// save that became readable again (transient lock released) is never overwritten.
    pub fn resolve_read_only(path: &Path) -> ReadOnlyResolution {
        match Self::load_with_recovery(path) {
            LoadOutcome::Loaded(_) => ReadOnlyResolution::KeepReadOnly,
            LoadOutcome::ReadOnly { .. } => ReadOnlyResolution::StillBlocked,
            LoadOutcome::RecoveredFromBackup(_)
            | LoadOutcome::FreshAfterBackup(_)
            | LoadOutcome::NotFound => ReadOnlyResolution::Lift,
        }
    }

    fn normalized(mut self) -> Self {
        self.bgm_volume = self.bgm_volume.clamp(0.0, 1.0);
        self.sfx_volume = self.sfx_volume.clamp(0.0, 1.0);
        self.schema_version = SAVE_SCHEMA_VERSION;
        self
    }

    /// Loads save data from disk if it exists, backing up corrupted files to `.corrupt`.
    pub fn load_from_path(path: &Path) -> Result<Option<Self>, String> {
        if !path.exists() {
            return Ok(None);
        }
        let raw =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read save file: {e}"))?;
        match Self::from_json(&raw) {
            Ok(parsed) => Ok(Some(parsed.normalized())),
            Err(e) => {
                let corrupt_path = path.with_extension("json.corrupt");
                let _ = std::fs::copy(path, &corrupt_path);
                Err(format!(
                    "Failed to parse save file (backed up to {}): {e}",
                    corrupt_path.display()
                ))
            }
        }
    }
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Reads a file as UTF-8, retrying transient I/O errors a few times.
fn read_with_retries(path: &Path) -> std::io::Result<String> {
    let mut attempt = 1;
    loop {
        match std::fs::read_to_string(path) {
            Ok(raw) => return Ok(raw),
            Err(e)
                if attempt >= SAVE_READ_MAX_ATTEMPTS
                    || matches!(
                        e.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidData
                    ) =>
            {
                return Err(e)
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(SAVE_READ_RETRY_DELAY_MS));
                attempt += 1;
            }
        }
    }
}

/// Creates `path`, writes all bytes and flushes them to stable storage.
fn write_synced(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Copies the current primary file (if any) to `.bak` through a temporary file and a rename.
fn rotate_backup_copy(path: &Path) -> Result<(), String> {
    if let Ok(false) = path.try_exists() {
        return Ok(());
    }
    let backup = SaveData::backup_path(path);
    let tmp = path.with_extension("json.bak.tmp");
    std::fs::copy(path, &tmp).map_err(|e| format!("Failed to copy save to backup: {e}"))?;
    std::fs::rename(&tmp, &backup).map_err(|e| format!("Failed to commit backup rename: {e}"))
}

/// Flushes the parent directory entry so the rename survives a power loss (Unix only;
/// directories cannot be opened on Windows).
#[cfg(unix)]
fn sync_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::File::open(parent)
            .and_then(|dir| dir.sync_all())
            .map_err(|e| format!("Failed to sync save directory: {e}"))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn sync_parent_dir(_path: &Path) -> Result<(), String> {
    Ok(())
}
