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
    LevelOutcome, LevelSession, ResourceBundle, SaveData, HUB_BUILDING_ID,
};

/// File name of the primary save inside the platform data directory.
const SAVE_FILE_NAME: &str = "wyrdstone_save.json";
/// Notice shown when the primary save was unreadable and the `.bak` copy was restored.
const NOTICE_RECOVERED_FROM_BACKUP: &str = "save.notice.recovered_from_backup";
/// Notice shown when the save is unreadable and cannot be copied, so saving is suspended.
pub const NOTICE_SAVE_READ_ONLY: &str = "save.warning.read_only";
/// Notice shown when the save was written by a newer game version, so saving is suspended.
pub const NOTICE_SAVE_NEWER_VERSION: &str = "save.warning.newer_version";
/// Notice shown when an unreadable save was preserved and a fresh city was started.
const NOTICE_UNREADABLE_BACKED_UP: &str = "save.notice.unreadable_backed_up";

/// Notice shown when the export is refused because no progress was loaded.
const NOTICE_BACKUP_READ_ONLY: &str = "city.panel.backup.read_only";

/// Why the save is read-only, which decides the notice shown and how it may be lifted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOnlyCause {
    /// The save file exists but could not be read, and could not be preserved either.
    Unreadable,
    /// The save file was written by a newer game version.
    NewerVersion,
}

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
    pub last_glory_reward: u32,
    pub last_blitz_medal: Option<wyrdstone_core::BlitzMedal>,
    pub last_blitz_new_best: bool,
    pub last_defeat_out_of_time: bool,
    pub open_inspector_on_city_enter: bool,
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
    /// Why `save_read_only` is set (never serialized); `None` while saving is allowed.
    pub save_read_only_cause: Option<ReadOnlyCause>,
    /// True when the read-only session started from a readable `.bak` (progress was loaded), so
    /// its state is trustworthy enough to export.
    save_read_only_has_progress: bool,
    /// True once the primary file is known to be good this session (loaded or written by us),
    /// which is the only time its content may be rotated into the `.bak` copy.
    save_file_verified: bool,
    /// True once the unreadable primary was copied aside this session, so no further
    /// `.corrupt-<unix_secs>` copy is made while resolving the read-only state.
    primary_copied_aside: bool,
    /// True once a newer-version `.bak` was preserved by `prepare_replace` this session.
    newer_backup_preserved: bool,
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
        let mut save_read_only_cause = None;
        let mut save_read_only_has_progress = false;
        let mut save_file_verified = false;
        let mut primary_copied_aside = false;
        let mut newer_backup_preserved = false;
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
                primary_copied_aside = true;
                status_notice_key = Some(NOTICE_UNREADABLE_BACKED_UP.to_string());
                // Writes resume on a fresh city, and the second write would rotate over a
                // newer-version `.bak`: keep it first, or stay read-only.
                if let Some(path) = save_path.as_deref() {
                    if SaveData::must_preserve_backup(path) {
                        match SaveData::prepare_replace(path, false) {
                            Ok(_) => newer_backup_preserved = true,
                            Err(err) => {
                                error!("Wyrdstone newer backup could not be preserved: {err}");
                                save_read_only = true;
                                save_read_only_cause = Some(ReadOnlyCause::Unreadable);
                                status_notice_key = Some(NOTICE_SAVE_READ_ONLY.to_string());
                            }
                        }
                    }
                }
                None
            }
            Some(LoadOutcome::ReadOnly { reason, backup }) => {
                warn!("Wyrdstone save is unreadable and could not be preserved: {reason}");
                has_unreadable_save = true;
                save_read_only = true;
                save_read_only_cause = Some(ReadOnlyCause::Unreadable);
                status_notice_key = Some(NOTICE_SAVE_READ_ONLY.to_string());
                // A readable `.bak` starts the session; writes stay suspended.
                save_read_only_has_progress = backup.is_some();
                backup.map(|saved| *saved)
            }
            Some(LoadOutcome::NewerVersion { found, backup }) => {
                warn!("Wyrdstone save was written by a newer version (schema {found}); read-only");
                has_unreadable_save = true;
                save_read_only = true;
                save_read_only_cause = Some(ReadOnlyCause::NewerVersion);
                status_notice_key = Some(NOTICE_SAVE_NEWER_VERSION.to_string());
                // A readable older `.bak` starts the session; writes stay suspended.
                save_read_only_has_progress = backup.is_some();
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
            .unwrap_or_else(|| HUB_BUILDING_ID.to_string());
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
            last_glory_reward: 0,
            last_blitz_medal: None,
            last_blitz_new_best: false,
            last_defeat_out_of_time: false,
            open_inspector_on_city_enter: false,
            last_discovered_relic_key: None,
            status_notice_key,
            celebration_message: None,
            celebration_timer_secs: 0.0,
            pending_dialogue: None,
            energy_regen_timer: Timer::from_seconds(regen_secs, TimerMode::Repeating),
            energy_regen_anchor_unix_secs: regen_anchor,
            save_path,
            save_read_only,
            save_read_only_cause,
            save_read_only_has_progress,
            save_file_verified,
            primary_copied_aside,
            newer_backup_preserved,
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

    /// Notice key describing why saving is suspended, `None` when saving is allowed.
    pub fn save_read_only_notice(&self) -> Option<&'static str> {
        if !self.save_read_only {
            return None;
        }
        Some(read_only_notice(self.save_read_only_cause))
    }

    /// Persists current progression and audio settings to disk atomically.
    ///
    /// While the existing save is unreadable (`save_read_only`), nothing is written until the
    /// unreadable file has been copied aside.
    pub fn persist(&mut self) {
        if let Err(err) = self.persist_checked() {
            warn!("Wyrdstone save write skipped or failed: {err}");
        }
    }

    fn persist_checked(&mut self) -> Result<(), String> {
        let Some(path) = self.save_path.clone() else {
            return Ok(());
        };
        if self.save_read_only {
            if self.save_read_only_cause == Some(ReadOnlyCause::NewerVersion) {
                self.status_notice_key = self.save_read_only_notice().map(str::to_string);
                return Err("newer-version save is read-only".to_string());
            }
            // A newer-version `.bak` must be copied aside before anything can replace it. Do it
            // before resolving: resolving copies an unreadable primary aside, and a failure after
            // that would repeat the copy on every later persist.
            if !self.newer_backup_preserved {
                if let Err(err) = SaveData::prepare_replace(&path, false) {
                    error!("Wyrdstone newer backup could not be preserved: {err}");
                    self.status_notice_key = self.save_read_only_notice().map(str::to_string);
                    return Err(err);
                }
                self.newer_backup_preserved = true;
            }
            // Re-check the primary file first: a transient lock may have ended, in which case
            // the readable save must not be overwritten by this session's progress. An
            // unreadable primary already copied aside is not examined (and copied) again.
            let (resolution, copied_aside) = if self.primary_copied_aside {
                (ReadOnlyResolution::Lift, true)
            } else {
                SaveData::resolve_read_only_detailed(&path)
            };
            match resolution {
                ReadOnlyResolution::KeepReadOnly => {
                    warn!(
                        "Wyrdstone save is readable again; session stays read-only until restart"
                    );
                    self.status_notice_key = self.save_read_only_notice().map(str::to_string);
                    return Err("save is read-only".to_string());
                }
                ReadOnlyResolution::StillBlocked => {
                    warn!("Wyrdstone save write skipped, unreadable save not preserved");
                    return Err("unreadable save still blocked".to_string());
                }
                ReadOnlyResolution::Lift => {
                    info!("Wyrdstone unreadable save resolved, resuming writes");
                    if copied_aside {
                        self.status_notice_key = Some(NOTICE_UNREADABLE_BACKED_UP.to_string());
                    } else if matches!(
                        self.status_notice_key.as_deref(),
                        Some(NOTICE_SAVE_READ_ONLY | NOTICE_SAVE_NEWER_VERSION)
                    ) {
                        // The file vanished: only the read-only notice is obsolete.
                        self.status_notice_key = None;
                    }
                    self.save_read_only = false;
                    self.save_read_only_cause = None;
                    self.save_read_only_has_progress = false;
                }
            }
        }
        let snapshot = self.build_save_snapshot();
        match snapshot.save_with_options(&path, self.save_file_verified) {
            Ok(report) => {
                self.save_file_verified = true;
                if has_saved_progress(&self.city, self.tutorial_step) {
                    self.has_existing_save = true;
                }
                if let Some(err) = report.backup_rotation_error {
                    warn!("Wyrdstone save backup rotation error: {err}");
                }
                Ok(())
            }
            Err(err) => {
                warn!("Wyrdstone save write error: {err}");
                Err(err)
            }
        }
    }

    /// Ends the read-only state after an explicit player action that replaces the save.
    ///
    /// Every newer-version file the replacement would destroy (the primary, and a `.bak` that
    /// the next rotation would overwrite) is first copied aside by
    /// [`SaveData::prepare_replace`]; when a copy fails the read-only state is kept (the notice
    /// is returned as the error) so nothing is overwritten. An unreadable primary is copied
    /// aside on a best-effort basis: an explicit player reset or import proceeds even if that
    /// copy fails (the failure is logged).
    fn lift_save_read_only(&mut self) -> Result<(), &'static str> {
        if !self.save_read_only {
            return Ok(());
        }
        if let Some(path) = &self.save_path {
            let primary_is_newer = self.save_read_only_cause == Some(ReadOnlyCause::NewerVersion);
            match SaveData::prepare_replace(path, primary_is_newer) {
                Ok(copies) => {
                    for copy in &copies {
                        info!(
                            "Wyrdstone newer-version save preserved at {}",
                            copy.display()
                        );
                    }
                }
                Err(err) => {
                    error!("Wyrdstone newer-version save could not be preserved: {err}");
                    self.status_notice_key = Some(NOTICE_SAVE_NEWER_VERSION.to_string());
                    return Err(NOTICE_SAVE_NEWER_VERSION);
                }
            }
            if !primary_is_newer {
                if let Err(err) = SaveData::backup_unreadable(path) {
                    error!(
                        "Wyrdstone unreadable save could not be preserved before replacing it: {err}"
                    );
                }
            }
        }
        self.save_read_only = false;
        self.save_read_only_cause = None;
        self.save_read_only_has_progress = false;
        Ok(())
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

    /// Exports an atomic JSON backup to `wyrdstone_backup.json` (external app storage on
    /// Android, sibling of the save file on desktop) and returns the destination path string.
    pub fn export_backup(&self) -> Result<String, &'static str> {
        if self.save_read_only && !self.save_read_only_has_progress {
            // Read-only without a readable `.bak`: no saved progress was loaded, so there is
            // nothing trustworthy to export. A session started from a `.bak` may export it.
            return Err(NOTICE_BACKUP_READ_ONLY);
        }
        let Some(backup_path) = resolve_backup_path(self.save_path.as_deref()) else {
            warn!("Wyrdstone backup export unavailable: no accessible backup directory");
            return Err("city.panel.backup.error");
        };
        let snapshot = self.build_save_snapshot();
        snapshot.save_to_path(&backup_path).map_err(|e| {
            warn!("Wyrdstone backup export failed: {e}");
            "city.panel.backup.error"
        })?;
        Ok(backup_path.display().to_string())
    }

    /// Imports `wyrdstone_backup.json`, restoring full player state and persisting to primary save.
    pub fn import_backup(&mut self) -> Result<(), &'static str> {
        let Some(backup_path) = resolve_backup_path(self.save_path.as_deref()) else {
            warn!("Wyrdstone backup import unavailable: no accessible backup directory");
            return Err("city.panel.backup.error");
        };
        if !backup_path.exists() {
            return Err("city.panel.backup.missing");
        }
        let saved = match SaveData::load_from_path(&backup_path) {
            Ok(Some(s)) => s,
            Ok(None) => return Err("city.panel.backup.missing"),
            Err(e) => {
                warn!("Wyrdstone backup import failed: {e}");
                return Err("city.panel.backup.error");
            }
        };
        self.lift_save_read_only()?;
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
        self.selected_building_id = self
            .data
            .buildings
            .first()
            .map(|b| b.id.clone())
            .unwrap_or_else(|| HUB_BUILDING_ID.to_string());
        self.selected_difficulty = 0;
        self.confirm_reset = false;
        self.active_session = None;
        self.active_level_id = None;
        self.armed_tool = None;
        self.last_outcome = None;
        self.last_glory_reward = 0;
        self.last_blitz_medal = None;
        self.last_blitz_new_best = false;
        self.last_defeat_out_of_time = false;
        self.open_inspector_on_city_enter = false;
        self.pending_dialogue = None;
        self.energy_regen_anchor_unix_secs = now_unix_secs();
        self.has_existing_save = true;
        self.persist_checked().map_err(|e| {
            warn!("Wyrdstone backup import persist failed: {e}");
            "city.panel.backup.error"
        })?;
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
    ///
    /// Returns `false` (nothing changed, notice set) when a newer-version save could not be
    /// preserved first.
    pub fn reset_saga(&mut self) -> bool {
        if self.lift_save_read_only().is_err() {
            return false;
        }
        self.city = CityState::new(&self.data.balance);
        self.selected_building_id = self
            .data
            .buildings
            .first()
            .map(|b| b.id.clone())
            .unwrap_or_else(|| HUB_BUILDING_ID.to_string());
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
        self.last_glory_reward = 0;
        self.last_blitz_medal = None;
        self.last_blitz_new_best = false;
        self.last_defeat_out_of_time = false;
        self.open_inspector_on_city_enter = false;
        self.last_discovered_relic_key = None;
        self.status_notice_key = None;
        self.celebration_message = None;
        self.celebration_timer_secs = 0.0;
        self.pending_dialogue = None;
        self.energy_regen_anchor_unix_secs = now_unix_secs();
        self.has_existing_save = true;
        self.persist();
        true
    }

    fn apply_loadout_extras(&mut self, session: &mut LevelSession) -> bool {
        if let Some(extra) = self.city.secondary_amulet_effect(&self.data.amulets) {
            session.apply_extra_amulet_effect(extra, &mut self.rng);
        }
        let synergies = self.city.active_synergies();
        if !synergies.is_empty() {
            session.apply_synergies(&synergies, &mut self.rng);
        }
        if let Some(draught) = self.city.active_war_draught.take() {
            session.apply_war_draught(draught, &mut self.rng);
            return true;
        }
        false
    }

    /// Launches today's deterministic Daily Trial board (free of Endurance cost once per day).
    pub fn start_daily_trial(&mut self) -> Result<(), &'static str> {
        let day_idx = today_day_index();
        let level_def =
            wyrdstone_core::LevelDef::daily_challenge_with(day_idx, &self.data.balance.daily);
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
        session.apply_session_tuning(&self.data.balance.session);
        session
            .apply_daily_blessing_with(DailyBlessing::for_day(day_idx), &self.data.balance.session);
        let consumed_draught = self.apply_loadout_extras(&mut session);
        self.active_level_id = Some(format!("daily_{day_idx}"));
        self.active_session = Some(session);
        self.active_modal = ActiveModal::None;
        self.armed_tool = None;
        self.status_notice_key = None;
        if consumed_draught {
            self.persist();
        }
        Ok(())
    }

    /// Launches the 75-second Valhalla Blitz arcade score-attack mode.
    pub fn start_valhalla_blitz(&mut self) -> Result<(), &'static str> {
        if !self.city.try_spend_energy(&self.data.balance) {
            return Err("city.error.not_enough_energy");
        }
        let amulet = self.city.active_amulet_effect(&self.data.amulets);
        let city_bonuses = self.city.compute_bonuses(&self.data.balance);
        let mut session = LevelSession::start_valhalla_blitz_with(
            amulet,
            city_bonuses,
            self.city.selected_patron,
            &self.data.balance,
            &mut self.rng,
        );
        session.apply_daily_blessing_with(self.today_blessing(), &self.data.balance.session);
        self.apply_loadout_extras(&mut session);
        self.active_level_id = Some(wyrdstone_core::BLITZ_BUILDING_ID.to_string());
        self.active_session = Some(session);
        self.active_modal = ActiveModal::None;
        self.armed_tool = None;
        self.status_notice_key = None;
        self.persist();
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
        session.apply_session_tuning(&self.data.balance.session);
        session.apply_daily_blessing_with(self.today_blessing(), &self.data.balance.session);
        self.apply_loadout_extras(&mut session);
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
            return Err("city.panel.raids.locked_notice");
        }
        if !self.city.try_spend_energy(&self.data.balance) {
            return Err("city.error.not_enough_energy");
        }
        let amulet = self.city.active_amulet_effect(&self.data.amulets);
        let city_bonuses = self.city.compute_bonuses(&self.data.balance);
        let mut session = LevelSession::start_boss_with(
            boss_def,
            amulet,
            city_bonuses,
            self.city.selected_patron,
            &self.data.balance,
            &mut self.rng,
        );
        session.apply_session_tuning(&self.data.balance.session);
        session.apply_daily_blessing_with(self.today_blessing(), &self.data.balance.session);
        self.apply_loadout_extras(&mut session);
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
            let mut session = LevelSession::start_expedition_wave_full(
                &run,
                &self.data.balance,
                &self.data.bosses,
                amulet,
                city_bonuses,
                self.city.selected_patron,
                &mut self.rng,
            );
            session.apply_daily_blessing_with(self.today_blessing(), &self.data.balance.session);
            let consumed_draught = self.apply_loadout_extras(&mut session);
            self.active_level_id = Some(session.level.id.clone());
            self.active_session = Some(session);
            self.active_modal = ActiveModal::None;
            self.armed_tool = None;
            self.status_notice_key = None;
            if consumed_draught {
                self.persist();
            }
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
        let mut session = LevelSession::start_expedition_wave_full(
            &run,
            &self.data.balance,
            &self.data.bosses,
            amulet,
            city_bonuses,
            self.city.selected_patron,
            &mut self.rng,
        );
        session.apply_daily_blessing_with(self.today_blessing(), &self.data.balance.session);
        self.apply_loadout_extras(&mut session);
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
        let mut session = LevelSession::start_expedition_wave_full(
            &run,
            &self.data.balance,
            &self.data.bosses,
            amulet,
            city_bonuses,
            self.city.selected_patron,
            &mut self.rng,
        );
        session.apply_daily_blessing_with(self.today_blessing(), &self.data.balance.session);
        self.apply_loadout_extras(&mut session);
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

fn resolve_backup_path(save_path: Option<&std::path::Path>) -> Option<PathBuf> {
    #[cfg(not(target_os = "android"))]
    {
        save_path.map(|p| p.with_file_name("wyrdstone_backup.json"))
    }
    #[cfg(target_os = "android")]
    {
        let _ = save_path;
        bevy::window::ANDROID_APP
            .get()
            .and_then(|app| app.external_data_path())
            .map(|dir| dir.join("wyrdstone_backup.json"))
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

/// Maps a read-only cause to its notice key (unknown cause falls back to the unreadable one).
fn read_only_notice(cause: Option<ReadOnlyCause>) -> &'static str {
    match cause {
        Some(ReadOnlyCause::NewerVersion) => NOTICE_SAVE_NEWER_VERSION,
        Some(ReadOnlyCause::Unreadable) | None => NOTICE_SAVE_READ_ONLY,
    }
}

/// True when a loaded save holds real progress, so the title offers Continue instead of Play.
fn has_saved_progress(city: &CityState, tutorial_step: u8) -> bool {
    tutorial_step > 0
        || city.stats.total_victories > 0
        || !city.building_tiers.is_empty()
        || !city.completed_levels.is_empty()
}

/// Regeneration speed as a percentage (100 = base rate), raised by the Mead Hall tier.
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
    let now = now_unix_secs();
    let max_energy = ctx.city.max_energy(&ctx.data.balance);
    if ctx.city.energy >= max_energy {
        if ctx.energy_regen_anchor_unix_secs != now || !ctx.energy_regen_timer.elapsed().is_zero() {
            let unflagged = ctx.bypass_change_detection();
            unflagged.energy_regen_anchor_unix_secs = now;
            unflagged.energy_regen_timer.reset();
        }
        return;
    }
    let regen_secs = ctx.data.balance.energy_regen_seconds;
    if now < ctx.energy_regen_anchor_unix_secs || regen_secs == 0 {
        if ctx.energy_regen_anchor_unix_secs != now || !ctx.energy_regen_timer.elapsed().is_zero() {
            let unflagged = ctx.bypass_change_detection();
            unflagged.energy_regen_anchor_unix_secs = now;
            unflagged.energy_regen_timer.reset();
        }
        return;
    }
    let speed_pct = regen_speed_pct(&ctx.city, &ctx.data.balance);
    let would_gain = (now - ctx.energy_regen_anchor_unix_secs) * speed_pct / 100 / regen_secs > 0;
    if !would_gain {
        let duration = ctx.energy_regen_timer.duration();
        let elapsed_real = now.saturating_sub(ctx.energy_regen_anchor_unix_secs) as f32;
        let elapsed_scaled = elapsed_real * speed_pct as f32 / 100.0;
        let target_elapsed = Duration::from_secs_f32(elapsed_scaled).min(duration);
        if (ctx.energy_regen_timer.elapsed_secs() - target_elapsed.as_secs_f32()).abs() > 0.01 {
            ctx.bypass_change_detection()
                .energy_regen_timer
                .set_elapsed(target_elapsed);
        }
        return;
    }

    let ctx = &mut *ctx;
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
        city.building_tiers.insert(HUB_BUILDING_ID.to_string(), 1);
        assert!(has_saved_progress(&city, 0));
    }

    #[test]
    fn completed_levels_count_as_progress() {
        let mut city = fresh_city();
        city.completed_levels.insert("lvl_1".to_string());
        assert!(has_saved_progress(&city, 0));
    }
}

#[cfg(test)]
mod read_only_notice_tests {
    use super::*;

    #[test]
    fn each_cause_maps_to_its_notice() {
        assert_eq!(
            read_only_notice(Some(ReadOnlyCause::NewerVersion)),
            NOTICE_SAVE_NEWER_VERSION
        );
        assert_eq!(
            read_only_notice(Some(ReadOnlyCause::Unreadable)),
            NOTICE_SAVE_READ_ONLY
        );
        assert_eq!(read_only_notice(None), NOTICE_SAVE_READ_ONLY);
    }

    struct TempSaveDir {
        path: PathBuf,
    }

    impl TempSaveDir {
        fn new(tag: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "wyrdstone_state_test_{}_{}_{}",
                tag,
                std::process::id(),
                unique
            ));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }
    }

    impl Drop for TempSaveDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn persist_checked_refuses_newer_version_without_touching_disk() {
        let dir = TempSaveDir::new("newer_version");
        let save_file = dir.path.join("save.json");
        let original = r#"{"version":9999}"#;
        std::fs::write(&save_file, original).expect("write newer save");

        let data = GameDataBundle::load_bundled(Language::En).expect("bundled data loads");
        let city = CityState::new(&data.balance);
        let mut ctx = GameContext {
            data,
            city,
            has_existing_save: true,
            selected_building_id: HUB_BUILDING_ID.to_string(),
            selected_difficulty: 0,
            active_modal: ActiveModal::None,
            confirm_reset: false,
            confirm_import_backup: false,
            confirm_end_expedition: false,
            audio_muted: false,
            bgm_volume: DEFAULT_BGM_VOLUME,
            sfx_volume: DEFAULT_SFX_VOLUME,
            colorblind_mode: false,
            fast_animations: false,
            reduced_motion: false,
            tutorial_step: 0,
            active_session: None,
            active_level_id: None,
            active_expedition: None,
            armed_tool: None,
            last_outcome: None,
            last_rewards: ResourceBundle::default(),
            last_score: 0,
            last_stars: 0,
            last_daily_streak: None,
            last_glory_reward: 0,
            last_blitz_medal: None,
            last_blitz_new_best: false,
            last_defeat_out_of_time: false,
            open_inspector_on_city_enter: false,
            last_discovered_relic_key: None,
            status_notice_key: None,
            celebration_message: None,
            celebration_timer_secs: 0.0,
            pending_dialogue: None,
            energy_regen_timer: Timer::from_seconds(60.0, TimerMode::Repeating),
            energy_regen_anchor_unix_secs: 0,
            save_path: Some(save_file.clone()),
            save_read_only: true,
            save_read_only_cause: Some(ReadOnlyCause::NewerVersion),
            save_read_only_has_progress: false,
            save_file_verified: false,
            primary_copied_aside: false,
            newer_backup_preserved: false,
            rng: StdRng::seed_from_u64(42),
        };

        let err = ctx
            .persist_checked()
            .expect_err("newer-version save must refuse persistence");
        assert!(err.contains("newer-version"));
        assert_eq!(
            ctx.status_notice_key.as_deref(),
            Some(NOTICE_SAVE_NEWER_VERSION)
        );
        assert_eq!(
            std::fs::read_to_string(&save_file).expect("read save"),
            original
        );
    }
}

#[cfg(test)]
mod energy_regen_tests {
    use super::*;

    fn test_balance() -> BalanceConfig {
        BalanceConfig::from_ron_str(include_str!("../../../assets/data/balance.ron"))
            .expect("bundled balance parses")
    }

    #[test]
    fn wall_clock_regen_preserves_partial_tick_and_respects_cap() {
        let balance = test_balance();
        let mut city = CityState::new(&balance);
        city.energy = 10;
        let cap = city.max_energy(&balance);
        let regen_secs = balance.energy_regen_seconds;

        let mut anchor = 1_000;
        // Elapsed = 2.5 ticks at 100% speed -> gains 2 points and advances anchor by 2 ticks.
        let gained = advance_energy_regen(
            &mut city,
            &mut anchor,
            1_000 + regen_secs * 2 + regen_secs / 2,
            100,
            &balance,
        );
        assert_eq!(gained, 2);
        assert_eq!(city.energy, 12);
        assert_eq!(anchor, 1_000 + regen_secs * 2);

        // Filling past cap snaps anchor to `now`.
        let now = anchor + regen_secs * 1_000;
        let gained_to_cap = advance_energy_regen(&mut city, &mut anchor, now, 200, &balance);
        assert_eq!(gained_to_cap, cap - 12);
        assert_eq!(city.energy, cap);
        assert_eq!(anchor, now);
    }
}
