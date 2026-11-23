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

//! Application states and shared game context resource.

use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::path::PathBuf;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use wyrdstone_core::save::{
    LoadOutcome, ReadOnlyResolution, DEFAULT_BGM_VOLUME, DEFAULT_SFX_VOLUME,
};
use wyrdstone_core::{
    ActiveTool, BalanceConfig, CityState, DailyBlessing, ExpeditionRun, GameDataBundle, Language,
    LevelOutcome, LevelSession, ResourceBundle, SaveData,
};

/// File name of the primary save inside the platform data directory.
const SAVE_FILE_NAME: &str = "wyrdstone_save.json";
/// Notice shown when the primary save was unreadable and the `.bak` copy was restored.
const NOTICE_RECOVERED_FROM_BACKUP: &str = "save.notice.recovered_from_backup";
/// Notice shown when the save is unreadable and cannot be copied, so saving is suspended.
pub const NOTICE_SAVE_READ_ONLY: &str = "save.warning.read_only";
/// Notice shown when the save was written by a newer game version, so saving is suspended.
const NOTICE_SAVE_NEWER_VERSION: &str = "save.warning.newer_version";
/// Notice shown when an unreadable save was preserved and a fresh city was started.
const NOTICE_UNREADABLE_BACKED_UP: &str = "save.notice.unreadable_backed_up";

/// Top-level scene states.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Boot,
    Title,
    FjordCity,
    Match3,
    LevelResult,
}

/// Exclusive active modal overlay across Fjord City and Match-3 scenes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveModal {
    #[default]
    None,
    Amulets,
    Collections,
    Shop,
    Stats,
    Settings,
    Help,
    Raids,
    Patrons,
    Achievements,
    ExpeditionBoons,
    LoginCalendar,
    ChapterDialogue,
}

/// Illustrated Saga chapter milestone dialogue shown when rebuilding key Fjord landmarks.
#[derive(Debug, Clone)]
pub struct ChapterDialogue {
    pub speaker: String,
    pub title_key: String,
    pub line1_key: String,
    pub line2_key: String,
}

/// Shared mutable state across all Wyrdstone scenes.
#[derive(Resource)]
pub struct GameContext {
    pub data: GameDataBundle,
    pub city: CityState,
    pub has_existing_save: bool,
    pub selected_building_id: String,
    pub selected_difficulty: u8,
    pub active_modal: ActiveModal,
    pub confirm_reset: bool,
    pub confirm_import_backup: bool,
    pub confirm_end_expedition: bool,
    pub audio_muted: bool,
    pub bgm_volume: f32,
    pub sfx_volume: f32,
    pub colorblind_mode: bool,
    pub fast_animations: bool,
    pub reduced_motion: bool,
    pub tutorial_step: u8,
    pub active_session: Option<LevelSession>,
    pub active_level_id: Option<String>,
    pub active_expedition: Option<ExpeditionRun>,
    pub armed_tool: Option<ActiveTool>,
    pub last_outcome: Option<LevelOutcome>,
    pub last_rewards: ResourceBundle,
    pub last_score: u32,
    pub last_stars: u8,
    pub last_daily_streak: Option<u32>,
    pub last_discovered_relic_key: Option<String>,
    pub status_notice_key: Option<String>,
    pub celebration_message: Option<String>,
    pub celebration_timer_secs: f32,
    pub pending_dialogue: Option<ChapterDialogue>,
    pub energy_regen_timer: Timer,
    /// Wall-clock instant from which the next endurance point is being earned.
    pub energy_regen_anchor_unix_secs: u64,
    pub save_path: Option<PathBuf>,
    /// True while an existing save file could not be read or preserved: writing is suspended
    /// so the unreadable file is never overwritten.
    pub save_read_only: bool,
    /// True once the primary file is known to be good this session (loaded or written by us),
    /// which is the only time its content may be rotated into the `.bak` copy.
    save_file_verified: bool,
    pub rng: StdRng,
}

impl GameContext {
    /// Initializes game data and loads any existing local save file.
    pub fn initialize() -> Self {
        let mut data = GameDataBundle::load_bundled(Language::Fr)
            .expect("Bundled Wyrdstone data must be valid");
        let save_path = resolve_save_path();

        let mut audio_muted = false;
        let mut bgm_volume = DEFAULT_BGM_VOLUME;
        let mut sfx_volume = DEFAULT_SFX_VOLUME;
        let mut colorblind_mode = false;
        let mut fast_animations = false;
        let mut reduced_motion = false;
        let mut tutorial_step = 0u8;
        let mut active_expedition = None;

        let mut save_read_only = false;
        let mut save_file_verified = false;
        let mut status_notice_key = None;
        let mut has_unreadable_save = false;
        let loaded_save = match save_path.as_deref().map(SaveData::load_with_recovery) {
            Some(LoadOutcome::Loaded(saved)) => {
                save_file_verified = true;
                Some(*saved)
            }
            Some(LoadOutcome::RecoveredFromBackup(saved)) => {
                warn!("Wyrdstone save was unreadable; restored from backup copy");
                has_unreadable_save = true;
                status_notice_key = Some(NOTICE_RECOVERED_FROM_BACKUP.to_string());
                Some(*saved)
            }
            Some(LoadOutcome::FreshAfterBackup(copied)) => {
                warn!(
                    "Wyrdstone save was unreadable; preserved at {}",
                    copied.display()
                );
                has_unreadable_save = true;
                status_notice_key = Some(NOTICE_UNREADABLE_BACKED_UP.to_string());
                None
            }
            Some(LoadOutcome::ReadOnly { reason, backup }) => {
                warn!("Wyrdstone save is unreadable and could not be preserved: {reason}");
                has_unreadable_save = true;
                save_read_only = true;
                status_notice_key = Some(NOTICE_SAVE_READ_ONLY.to_string());
                // A readable `.bak` starts the session; writes stay suspended.
                backup.map(|saved| *saved)
            }
            Some(LoadOutcome::NewerVersion { found, backup }) => {
                warn!("Wyrdstone save was written by a newer version (schema {found}); read-only");
                has_unreadable_save = true;
                save_read_only = true;
                status_notice_key = Some(NOTICE_SAVE_NEWER_VERSION.to_string());
                // A readable older `.bak` starts the session; writes stay suspended.
                backup.map(|saved| *saved)
            }
            Some(LoadOutcome::NotFound) | None => None,
        };

        let now = now_unix_secs();
        let (city, has_existing_save, regen_anchor) = match loaded_save {
            Some(mut saved) => {
                data.i18n.set_language(saved.language);
                audio_muted = saved.muted;
                bgm_volume = saved.bgm_volume.clamp(0.0, 1.0);
                sfx_volume = saved.sfx_volume.clamp(0.0, 1.0);
                colorblind_mode = saved.colorblind_mode;
                fast_animations = saved.fast_animations;
                reduced_motion = saved.reduced_motion;
                active_expedition = saved.active_expedition;
                tutorial_step = if saved.city.stats.total_victories > 0 && saved.tutorial_step == 0
                {
                    3
                } else {
                    saved.tutorial_step
                };
                let mut anchor = now;
                if saved.energy_regen_anchor_unix_secs > 0 || saved.last_saved_unix_secs > 0 {
                    anchor = if saved.energy_regen_anchor_unix_secs > 0 {
                        saved.energy_regen_anchor_unix_secs
                    } else {
                        saved.last_saved_unix_secs
                    };
                    let speed_pct = regen_speed_pct(&saved.city, &data.balance);
                    advance_energy_regen(
                        &mut saved.city,
                        &mut anchor,
                        now,
                        speed_pct,
                        &data.balance,
                    );
                } else if saved.city.energy < data.balance.energy_cost_per_level {
                    let cap = saved.city.max_energy(&data.balance);
                    saved.city.energy = cap;
                }
                let progressed = has_saved_progress(&saved.city, tutorial_step);
                (saved.city, progressed || has_unreadable_save, anchor)
            }
            None => (CityState::new(&data.balance), has_unreadable_save, now),
        };

        let default_bldg = data
            .buildings
            .first()
            .map(|b| b.id.clone())
            .unwrap_or_else(|| "great_hall".to_string());
        let regen_secs = data.balance.energy_regen_seconds as f32;

        Self {
            data,
            city,
            has_existing_save,
            selected_building_id: default_bldg,
            selected_difficulty: 0,
            active_modal: ActiveModal::None,
            confirm_reset: false,
            confirm_import_backup: false,
            confirm_end_expedition: false,
            audio_muted,
            bgm_volume,
            sfx_volume,
            colorblind_mode,
            fast_animations,
            reduced_motion,
            tutorial_step,
            active_session: None,
            active_level_id: None,
            active_expedition,
            armed_tool: None,
            last_outcome: None,
            last_rewards: ResourceBundle::default(),
            last_score: 0,
            last_stars: 0,
            last_daily_streak: None,
            last_discovered_relic_key: None,
            status_notice_key,
            celebration_message: None,
            celebration_timer_secs: 0.0,
            pending_dialogue: None,
            energy_regen_timer: Timer::from_seconds(regen_secs, TimerMode::Repeating),
            energy_regen_anchor_unix_secs: regen_anchor,
            save_path,
            save_read_only,
            save_file_verified,
            rng: StdRng::from_entropy(),
        }
    }

    /// Toggles an exclusive modal overlay and clears transient confirmation flags.
    pub fn toggle_modal(&mut self, modal: ActiveModal) {
        self.active_modal = if self.active_modal == modal {
            ActiveModal::None
        } else {
            modal
        };
        self.confirm_import_backup = false;
        self.confirm_end_expedition = false;
        self.status_notice_key = None;
    }

    /// Persists current progression and audio settings to disk atomically.
    ///
    /// While the existing save is unreadable (`save_read_only`), nothing is written until the
    /// unreadable file has been copied aside.
    pub fn persist(&mut self) {
        let Some(path) = self.save_path.clone() else {
            return;
        };
        if self.save_read_only {
            // Re-check the primary file first: a transient lock may have ended, in which case
            // the readable save must not be overwritten by this session's progress.
            match SaveData::resolve_read_only(&path) {
                ReadOnlyResolution::KeepReadOnly => {
                    warn!(
                        "Wyrdstone save is readable again; session stays read-only until restart"
                    );
                    self.status_notice_key = Some(NOTICE_SAVE_READ_ONLY.to_string());
                    return;
                }
                ReadOnlyResolution::StillBlocked => {
                    warn!("Wyrdstone save write skipped, unreadable save not preserved");
                    return;
                }
                ReadOnlyResolution::Lift => {
                    info!("Wyrdstone unreadable save preserved, resuming writes");
                    self.save_read_only = false;
                    self.status_notice_key = Some(NOTICE_UNREADABLE_BACKED_UP.to_string());
                }
            }
        }
        let snapshot = self.build_save_snapshot();
        match snapshot.save_with_options(&path, self.save_file_verified) {
            Ok(report) => {
                self.save_file_verified = true;
                if let Some(err) = report.backup_rotation_error {
                    warn!("Wyrdstone save backup rotation error: {err}");
                }
            }
            Err(err) => warn!("Wyrdstone save write error: {err}"),
        }
    }

    /// Ends the read-only state after an explicit player action that replaces the save,
    /// still trying to keep a copy of the unreadable file first.
    fn lift_save_read_only(&mut self) {
        if !self.save_read_only {
            return;
        }
        if let Some(path) = &self.save_path {
            if let Err(err) = SaveData::backup_unreadable(path) {
                warn!(
                    "Wyrdstone unreadable save could not be preserved before replacing it: {err}"
                );
            }
        }
        self.save_read_only = false;
    }

    fn build_save_snapshot(&self) -> SaveData {
        SaveData {
            schema_version: wyrdstone_core::save::SAVE_SCHEMA_VERSION,
            language: self.data.i18n.language(),
            last_saved_unix_secs: now_unix_secs(),
            energy_regen_anchor_unix_secs: self.energy_regen_anchor_unix_secs,
            muted: self.audio_muted,
            bgm_volume: self.bgm_volume,
            sfx_volume: self.sfx_volume,
            colorblind_mode: self.colorblind_mode,
            fast_animations: self.fast_animations,
            reduced_motion: self.reduced_motion,
            tutorial_step: self.tutorial_step,
            active_expedition: self.active_expedition.clone(),
            city: self.city.clone(),
        }
    }

    /// Exports an atomic JSON backup to `wyrdstone_backup.json` next to the primary save file
    /// and returns the destination path string on success.
    pub fn export_backup(&self) -> Result<String, &'static str> {
        let Some(backup_path) = self
            .save_path
            .as_ref()
            .map(|p| p.with_file_name("wyrdstone_backup.json"))
        else {
            return Err("city.panel.backup.error");
        };
        let snapshot = self.build_save_snapshot();
        snapshot
            .save_to_path(&backup_path)
            .map_err(|_| "city.panel.backup.error")?;
        Ok(backup_path.display().to_string())
    }

    /// Imports `wyrdstone_backup.json`, restoring full player state and persisting to primary save.
    pub fn import_backup(&mut self) -> Result<(), &'static str> {
        let Some(backup_path) = self
            .save_path
            .as_ref()
            .map(|p| p.with_file_name("wyrdstone_backup.json"))
        else {
            return Err("city.panel.backup.error");
        };
        if !backup_path.exists() {
            return Err("city.panel.backup.missing");
        }
        let saved = match SaveData::load_from_path(&backup_path) {
            Ok(Some(s)) => s,
            Ok(None) => return Err("city.panel.backup.missing"),
            Err(_) => return Err("city.panel.backup.error"),
        };
        self.data.i18n.set_language(saved.language);
        self.audio_muted = saved.muted;
        self.bgm_volume = if saved.bgm_volume.is_finite() {
            saved.bgm_volume.clamp(0.0, 1.0)
        } else {
            DEFAULT_BGM_VOLUME
        };
        self.sfx_volume = if saved.sfx_volume.is_finite() {
            saved.sfx_volume.clamp(0.0, 1.0)
        } else {
            DEFAULT_SFX_VOLUME
        };
        self.colorblind_mode = saved.colorblind_mode;
        self.fast_animations = saved.fast_animations;
        self.reduced_motion = saved.reduced_motion;
        self.tutorial_step = saved.tutorial_step;
        self.active_expedition = saved.active_expedition;
        self.city = saved.city;
        self.energy_regen_anchor_unix_secs = now_unix_secs();
        self.lift_save_read_only();
        self.has_existing_save = true;
        self.persist();
        Ok(())
    }

    /// Scales animation durations by 1.5x when Fast Animation Mode is enabled.
    pub fn anim_duration(&self, base_secs: f32) -> f32 {
        if self.fast_animations {
            base_secs / 1.5
        } else {
            base_secs
        }
    }

    /// Returns today's deterministic UTC Daily Fjord Blessing.
    pub fn today_blessing(&self) -> DailyBlessing {
        DailyBlessing::for_day(today_day_index())
    }

    /// Resets player progression to a brand-new Saga and persists immediately.
    pub fn reset_saga(&mut self) {
        self.lift_save_read_only();
        self.city = CityState::new(&self.data.balance);
        self.selected_building_id = self
            .data
            .buildings
            .first()
            .map(|b| b.id.clone())
            .unwrap_or_else(|| "great_hall".to_string());
        self.selected_difficulty = 0;
        self.active_modal = ActiveModal::None;
        self.confirm_reset = false;
        self.tutorial_step = 0;
        self.active_session = None;
        self.active_level_id = None;
        self.active_expedition = None;
        self.last_outcome = None;
        self.last_stars = 0;
        self.last_daily_streak = None;
        self.last_discovered_relic_key = None;
        self.status_notice_key = None;
        self.celebration_message = None;
        self.celebration_timer_secs = 0.0;
        self.pending_dialogue = None;
        self.energy_regen_anchor_unix_secs = now_unix_secs();
        self.has_existing_save = true;
        self.persist();
    }

    /// Launches today's deterministic Daily Trial board (free of Endurance cost once per day).
    pub fn start_daily_trial(&mut self) -> Result<(), &'static str> {
        let day_idx = today_day_index();
        let level_def = wyrdstone_core::LevelDef::daily_challenge(day_idx);
        let amulet = self.city.active_amulet_effect(&self.data.amulets);
        let city_bonuses = self.city.compute_bonuses(&self.data.balance);
        let mut session = LevelSession::start_full(
            level_def,
            amulet,
            city_bonuses,
            self.city.selected_patron,
            None,
            &mut self.rng,
        );
        session.apply_daily_blessing(DailyBlessing::for_day(day_idx));
        self.active_level_id = Some(format!("daily_{day_idx}"));
        self.active_session = Some(session);
        self.active_modal = ActiveModal::None;
        self.armed_tool = None;
        self.status_notice_key = None;
        Ok(())
    }

    /// Launches a Match-3 level by ID, verifying existence before spending endurance.
    pub fn start_level(&mut self, level_id: &str) -> Result<(), &'static str> {
        let Some(level_def) = self.data.level_by_id(level_id).cloned() else {
            return Err("city.error.level_not_found");
        };
        if !self.city.try_spend_energy(&self.data.balance) {
            return Err("city.error.not_enough_energy");
        }
        let amulet = self.city.active_amulet_effect(&self.data.amulets);
        let city_bonuses = self.city.compute_bonuses(&self.data.balance);
        let mut session = LevelSession::start_full(
            level_def,
            amulet,
            city_bonuses,
            self.city.selected_patron,
            None,
            &mut self.rng,
        );
        session.apply_daily_blessing(self.today_blessing());
        self.active_level_id = Some(level_id.to_string());
        self.active_session = Some(session);
        self.active_modal = ActiveModal::None;
        self.armed_tool = None;
        self.status_notice_key = None;
        self.persist();
        Ok(())
    }

    /// Launches a Mythological Boss Raid encounter by boss ID.
    pub fn start_boss_raid(&mut self, boss_id: &str) -> Result<(), &'static str> {
        let Some(boss_def) = self.data.boss_by_id(boss_id).cloned() else {
            return Err("city.error.level_not_found");
        };
        if self.city.glory < boss_def.required_glory {
            return Err("city.panel.building.locked");
        }
        if !self.city.try_spend_energy(&self.data.balance) {
            return Err("city.error.not_enough_energy");
        }
        let amulet = self.city.active_amulet_effect(&self.data.amulets);
        let city_bonuses = self.city.compute_bonuses(&self.data.balance);
        let mut session = LevelSession::start_boss(
            boss_def,
            amulet,
            city_bonuses,
            self.city.selected_patron,
            &mut self.rng,
        );
        session.apply_daily_blessing(self.today_blessing());
        self.active_level_id = Some(boss_id.to_string());
        self.active_session = Some(session);
        self.active_modal = ActiveModal::None;
        self.armed_tool = None;
        self.status_notice_key = None;
        self.persist();
        Ok(())
    }

    /// Starts a fresh Endless Drakkar Expedition from Wave 1, or resumes a saved run without extra energy.
    pub fn start_or_resume_expedition(&mut self) -> Result<bool, &'static str> {
        if let Some(run) = self.active_expedition.clone() {
            if !run.pending_boons.is_empty() {
                self.active_modal = ActiveModal::ExpeditionBoons;
                return Ok(false);
            }
            let amulet = self.city.active_amulet_effect(&self.data.amulets);
            let city_bonuses = self.city.compute_bonuses(&self.data.balance);
            let mut session = LevelSession::start_expedition_wave(
                &run,
                amulet,
                city_bonuses,
                self.city.selected_patron,
                &mut self.rng,
            );
            session.apply_daily_blessing(self.today_blessing());
            self.active_level_id = Some(session.level.id.clone());
            self.active_session = Some(session);
            self.active_modal = ActiveModal::None;
            self.armed_tool = None;
            self.status_notice_key = None;
            return Ok(true);
        }
        self.start_expedition().map(|()| true)
    }

    /// Starts a fresh Endless Drakkar Expedition from Wave 1.
    pub fn start_expedition(&mut self) -> Result<(), &'static str> {
        if !self.city.try_spend_energy(&self.data.balance) {
            return Err("city.error.not_enough_energy");
        }
        let run = ExpeditionRun::new();
        let amulet = self.city.active_amulet_effect(&self.data.amulets);
        let city_bonuses = self.city.compute_bonuses(&self.data.balance);
        let mut session = LevelSession::start_expedition_wave(
            &run,
            amulet,
            city_bonuses,
            self.city.selected_patron,
            &mut self.rng,
        );
        session.apply_daily_blessing(self.today_blessing());
        self.active_level_id = Some(session.level.id.clone());
        self.active_expedition = Some(run);
        self.active_session = Some(session);
        self.active_modal = ActiveModal::None;
        self.armed_tool = None;
        self.status_notice_key = None;
        self.persist();
        Ok(())
    }

    /// Applies the chosen roguelite boon and launches the next Expedition wave (no extra endurance).
    pub fn continue_expedition_with_boon(&mut self, boon_idx: usize) -> Result<(), &'static str> {
        let Some(mut run) = self.active_expedition.take() else {
            return Err("city.error.level_not_found");
        };
        let Some(&boon) = run.pending_boons.get(boon_idx) else {
            self.active_expedition = Some(run);
            return Err("city.error.level_not_found");
        };
        run.apply_boon(boon, &mut self.city);
        let amulet = self.city.active_amulet_effect(&self.data.amulets);
        let city_bonuses = self.city.compute_bonuses(&self.data.balance);
        let mut session = LevelSession::start_expedition_wave(
            &run,
            amulet,
            city_bonuses,
            self.city.selected_patron,
            &mut self.rng,
        );
        session.apply_daily_blessing(self.today_blessing());
        self.active_level_id = Some(session.level.id.clone());
        self.active_expedition = Some(run);
        self.active_session = Some(session);
        self.active_modal = ActiveModal::None;
        self.armed_tool = None;
        self.status_notice_key = None;
        self.persist();
        Ok(())
    }
}

fn resolve_save_path() -> Option<PathBuf> {
    #[cfg(not(target_os = "android"))]
    {
        directories::ProjectDirs::from("com", "julienbombled", "Wyrdstone")
            .map(|dirs| dirs.data_local_dir().join(SAVE_FILE_NAME))
    }
    #[cfg(target_os = "android")]
    {
        bevy::window::ANDROID_APP
            .get()
            .and_then(|app| app.internal_data_path())
            .map(|dir| dir.join(SAVE_FILE_NAME))
    }
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Returns the current UTC day number for Daily Trial tracking.
pub fn today_day_index() -> u64 {
    now_unix_secs() / 86_400
}

/// Regeneration speed as a percentage (100 = base rate), raised by the Mead Hall tier.
/// True when a loaded save holds real progress, so the title offers Continue instead of Play.
fn has_saved_progress(city: &CityState, tutorial_step: u8) -> bool {
    tutorial_step > 0
        || city.stats.total_victories > 0
        || !city.building_tiers.is_empty()
        || !city.completed_levels.is_empty()
}

fn regen_speed_pct(city: &CityState, balance: &BalanceConfig) -> u64 {
    100 + u64::from(city.compute_bonuses(balance).energy_regen_speed_pct)
}

/// Credits endurance earned between `anchor` and `now` (wall clock, so time spent in the
/// background counts) and moves the anchor forward by the time actually consumed, keeping the
/// remainder of a partial point. Returns the endurance gained.
fn advance_energy_regen(
    city: &mut CityState,
    anchor: &mut u64,
    now: u64,
    speed_pct: u64,
    balance: &BalanceConfig,
) -> u32 {
    let regen_secs = balance.energy_regen_seconds;
    let cap = city.max_energy(balance);
    if city.energy >= cap || now < *anchor || regen_secs == 0 {
        *anchor = now;
        return 0;
    }
    let gained = (now - *anchor) * speed_pct / 100 / regen_secs;
    if gained == 0 {
        return 0;
    }
    let before = city.energy;
    city.regenerate_energy(u32::try_from(gained).unwrap_or(u32::MAX), balance);
    if city.energy >= cap {
        *anchor = now;
    } else {
        *anchor += (gained * regen_secs * 100).div_ceil(speed_pct);
    }
    city.energy - before
}

/// Regenerates endurance from the wall clock, sped up by the Mead Hall tier.
///
/// The countdown timer shown by the city header is kept in sync in scaled units (its duration
/// is one regeneration period at base speed; the header divides the remainder by the speed).
pub fn tick_energy_regen(mut ctx: ResMut<GameContext>) {
    let ctx = &mut *ctx;
    let now = now_unix_secs();
    let speed_pct = regen_speed_pct(&ctx.city, &ctx.data.balance);
    let gained = advance_energy_regen(
        &mut ctx.city,
        &mut ctx.energy_regen_anchor_unix_secs,
        now,
        speed_pct,
        &ctx.data.balance,
    );
    let duration = ctx.energy_regen_timer.duration();
    if ctx.city.energy >= ctx.city.max_energy(&ctx.data.balance) {
        ctx.energy_regen_timer.reset();
    } else {
        let elapsed_real = now.saturating_sub(ctx.energy_regen_anchor_unix_secs) as f32;
        let elapsed_scaled = elapsed_real * speed_pct as f32 / 100.0;
        ctx.energy_regen_timer
            .set_elapsed(Duration::from_secs_f32(elapsed_scaled).min(duration));
    }
    if gained > 0 {
        ctx.persist();
    }
}

#[cfg(test)]
mod progress_tests {
    use super::*;

    fn fresh_city() -> CityState {
        let balance = BalanceConfig::from_ron_str(include_str!("../../../assets/data/balance.ron"))
            .expect("bundled balance parses");
        CityState::new(&balance)
    }

    #[test]
    fn fresh_city_has_no_progress() {
        assert!(!has_saved_progress(&fresh_city(), 0));
    }

    #[test]
    fn tutorial_step_counts_as_progress() {
        assert!(has_saved_progress(&fresh_city(), 1));
    }

    #[test]
    fn victories_count_as_progress() {
        let mut city = fresh_city();
        city.stats.total_victories = 1;
        assert!(has_saved_progress(&city, 0));
    }

    #[test]
    fn built_tiers_count_as_progress() {
        let mut city = fresh_city();
        city.building_tiers.insert("great_hall".to_string(), 1);
        assert!(has_saved_progress(&city, 0));
    }

    #[test]
    fn completed_levels_count_as_progress() {
        let mut city = fresh_city();
        city.completed_levels.insert("lvl_1".to_string());
        assert!(has_saved_progress(&city, 0));
    }
}
