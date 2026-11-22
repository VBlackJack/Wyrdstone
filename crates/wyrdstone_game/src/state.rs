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
use wyrdstone_core::{
    ActiveTool, CityState, GameDataBundle, Language, LevelOutcome, LevelSession, ResourceBundle,
    SaveData,
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

/// Shared mutable state across all Wyrdstone scenes.
#[derive(Resource)]
pub struct GameContext {
    pub data: GameDataBundle,
    pub city: CityState,
    pub has_existing_save: bool,
    pub selected_building_id: String,
    pub amulets_modal_open: bool,
    pub collections_modal_open: bool,
    pub help_modal_open: bool,
    pub confirm_reset: bool,
    pub active_session: Option<LevelSession>,
    pub active_level_id: Option<String>,
    pub armed_tool: Option<ActiveTool>,
    pub last_outcome: Option<LevelOutcome>,
    pub last_rewards: ResourceBundle,
    pub last_score: u32,
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

        let (city, has_existing_save) = match save_path
            .as_ref()
            .and_then(|p| SaveData::load_from_path(p).ok().flatten())
        {
            Some(mut saved) => {
                data.i18n.set_language(saved.language);
                let now = now_unix_secs();
                if saved.last_saved_unix_secs > 0 && now > saved.last_saved_unix_secs {
                    saved
                        .city
                        .apply_offline_energy_regen(now - saved.last_saved_unix_secs, &data.balance);
                } else if saved.last_saved_unix_secs == 0
                    && saved.city.energy < data.balance.energy_cost_per_level
                {
                    // Legacy save before offline regen existed: restore full endurance once.
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
            amulets_modal_open: false,
            collections_modal_open: false,
            help_modal_open: false,
            confirm_reset: false,
            active_session: None,
            active_level_id: None,
            armed_tool: None,
            last_outcome: None,
            last_rewards: ResourceBundle::default(),
            last_score: 0,
            last_discovered_relic_key: None,
            status_notice_key: None,
            celebration_message: None,
            celebration_timer_secs: 0.0,
            energy_regen_timer: Timer::from_seconds(regen_secs, TimerMode::Repeating),
            save_path,
            rng: StdRng::from_entropy(),
        }
    }

    /// Persists current progression to disk when a save path is available.
    pub fn persist(&mut self) {
        self.has_existing_save = true;
        if let Some(path) = &self.save_path {
            let snapshot = SaveData {
                schema_version: wyrdstone_core::save::SAVE_SCHEMA_VERSION,
                language: self.data.i18n.language(),
                last_saved_unix_secs: now_unix_secs(),
                city: self.city.clone(),
            };
            let _ = snapshot.save_to_path(path);
        }
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
        self.amulets_modal_open = false;
        self.collections_modal_open = false;
        self.help_modal_open = false;
        self.confirm_reset = false;
        self.active_session = None;
        self.last_discovered_relic_key = None;
        self.celebration_message = None;
        self.celebration_timer_secs = 0.0;
        self.persist();
    }

    /// Launches a Match-3 level by ID, spending endurance and applying amulet + village bonuses.
    pub fn start_level(&mut self, level_id: &str) -> Result<(), &'static str> {
        if !self.city.try_spend_energy(&self.data.balance) {
            return Err("city.error.not_enough_energy");
        }
        let Some(level_def) = self.data.level_by_id(level_id).cloned() else {
            return Err("city.error.not_enough_energy");
        };
        let amulet = self.city.active_amulet_effect(&self.data.amulets);
        let city_bonuses = self.city.compute_bonuses();
        let session =
            LevelSession::start_with_bonuses(level_def, amulet, city_bonuses, &mut self.rng);
        self.active_level_id = Some(level_id.to_string());
        self.active_session = Some(session);
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

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Regenerates endurance periodically, sped up by the Mead Hall tier.
pub fn tick_energy_regen(time: Res<Time>, mut ctx: ResMut<GameContext>) {
    let speed_mult = 1.0 + (ctx.city.compute_bonuses().energy_regen_speed_pct as f32 / 100.0);
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
