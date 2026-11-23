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
use std::time::{SystemTime, UNIX_EPOCH};
use wyrdstone_core::save::{DEFAULT_BGM_VOLUME, DEFAULT_SFX_VOLUME};
use wyrdstone_core::{
    ActiveTool, CityState, DailyBlessing, ExpeditionRun, GameDataBundle, Language, LevelOutcome,
    LevelSession, ResourceBundle, SaveData,
};

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
    Help,
    Raids,
    Patrons,
    Achievements,
    ExpeditionBoons,
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
    pub audio_muted: bool,
    pub bgm_volume: f32,
    pub sfx_volume: f32,
    pub colorblind_mode: bool,
    pub fast_animations: bool,
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
    pub energy_regen_timer: Timer,
    pub save_path: Option<PathBuf>,
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
        let mut tutorial_step = 0u8;
        let mut active_expedition = None;

        let loaded_save = save_path
            .as_ref()
            .and_then(|p| match SaveData::load_from_path(p) {
                Ok(opt) => opt,
                Err(err) => {
                    warn!("Wyrdstone save load error: {err}");
                    None
                }
            });

        let (city, has_existing_save) = match loaded_save {
            Some(mut saved) => {
                data.i18n.set_language(saved.language);
                audio_muted = saved.muted;
                bgm_volume = saved.bgm_volume.clamp(0.0, 1.0);
                sfx_volume = saved.sfx_volume.clamp(0.0, 1.0);
                colorblind_mode = saved.colorblind_mode;
                fast_animations = saved.fast_animations;
                active_expedition = saved.active_expedition;
                tutorial_step = if saved.city.stats.total_victories > 0 && saved.tutorial_step == 0
                {
                    3
                } else {
                    saved.tutorial_step
                };
                let now = now_unix_secs();
                if saved.last_saved_unix_secs > 0 && now > saved.last_saved_unix_secs {
                    saved.city.apply_offline_energy_regen(
                        now - saved.last_saved_unix_secs,
                        &data.balance,
                    );
                } else if saved.last_saved_unix_secs == 0
                    && saved.city.energy < data.balance.energy_cost_per_level
                {
                    let cap = saved.city.max_energy(&data.balance);
                    saved.city.energy = cap;
                }
                (saved.city, true)
            }
            None => (CityState::new(&data.balance), false),
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
            audio_muted,
            bgm_volume,
            sfx_volume,
            colorblind_mode,
            fast_animations,
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
            status_notice_key: None,
            celebration_message: None,
            celebration_timer_secs: 0.0,
            energy_regen_timer: Timer::from_seconds(regen_secs, TimerMode::Repeating),
            save_path,
            rng: StdRng::from_entropy(),
        }
    }

    /// Toggles an exclusive modal overlay.
    pub fn toggle_modal(&mut self, modal: ActiveModal) {
        self.active_modal = if self.active_modal == modal {
            ActiveModal::None
        } else {
            modal
        };
    }

    /// Persists current progression and audio settings to disk atomically.
    pub fn persist(&mut self) {
        self.has_existing_save = true;
        if let Some(path) = &self.save_path {
            let snapshot = self.build_save_snapshot();
            if let Err(err) = snapshot.save_to_path(path) {
                warn!("Wyrdstone save write error: {err}");
            }
        }
    }

    fn build_save_snapshot(&self) -> SaveData {
        SaveData {
            schema_version: wyrdstone_core::save::SAVE_SCHEMA_VERSION,
            language: self.data.i18n.language(),
            last_saved_unix_secs: now_unix_secs(),
            muted: self.audio_muted,
            bgm_volume: self.bgm_volume,
            sfx_volume: self.sfx_volume,
            colorblind_mode: self.colorblind_mode,
            fast_animations: self.fast_animations,
            tutorial_step: self.tutorial_step,
            active_expedition: self.active_expedition.clone(),
            city: self.city.clone(),
        }
    }

    /// Exports an atomic JSON backup to `wyrdstone_backup.json` next to the primary save file.
    pub fn export_backup(&self) -> Result<(), &'static str> {
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
            .map_err(|_| "city.panel.backup.error")
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
        self.tutorial_step = saved.tutorial_step;
        self.active_expedition = saved.active_expedition;
        self.city = saved.city;
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
        self.active_expedition = None;
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
        self.active_expedition = None;
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
        self.active_expedition = None;
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
            .map(|dirs| dirs.data_local_dir().join("wyrdstone_save.json"))
    }
    #[cfg(target_os = "android")]
    {
        Some(PathBuf::from(
            "/data/data/com.julienbombled.wyrdstone/files/wyrdstone_save.json",
        ))
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

/// Regenerates endurance periodically, sped up by the Mead Hall tier.
pub fn tick_energy_regen(time: Res<Time>, mut ctx: ResMut<GameContext>) {
    let speed_mult = 1.0
        + (ctx
            .city
            .compute_bonuses(&ctx.data.balance)
            .energy_regen_speed_pct as f32
            / 100.0);
    let scaled_delta = time.delta().mul_f32(speed_mult);
    if ctx.energy_regen_timer.tick(scaled_delta).just_finished() {
        let balance = ctx.data.balance.clone();
        let prev = ctx.city.energy;
        ctx.city.regenerate_energy(1, &balance);
        if ctx.city.energy != prev {
            ctx.persist();
        }
    }
}
