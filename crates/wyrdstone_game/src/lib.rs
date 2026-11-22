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

//! Wyrdstone Bevy application crate supporting both Windows desktop and Android.

mod audio;
mod city_ui;
mod fjord_art;
mod match3_scene;
mod result_ui;
mod runes_art;
mod state;
mod theme;
mod title_ui;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use state::{tick_energy_regen, AppState, GameContext};
use theme::{COLOR_BG_DEEP, WINDOW_DEFAULT_HEIGHT, WINDOW_DEFAULT_WIDTH};

/// Android entry point invoked by `android-activity` / `cargo-apk`.
#[cfg(target_os = "android")]
#[bevy_main]
fn main() {
    run_app();
}

/// Configures and launches the Wyrdstone Bevy application.
pub fn run_app() {
    let ctx = GameContext::initialize();
    let window_title = ctx.data.i18n.t("app.title.name").to_string();
    let mut app = App::new();

    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: window_title,
            resolution: WindowResolution::new(WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_HEIGHT),
            resizable: true,
            ..default()
        }),
        ..default()
    };

    app.add_plugins(DefaultPlugins.set(window_plugin))
        .insert_resource(ClearColor(COLOR_BG_DEEP))
        .insert_resource(ctx)
        .init_state::<AppState>()
        .add_systems(
            PreStartup,
            (
                setup_camera,
                runes_art::init_rune_textures,
                fjord_art::init_fjord_textures,
                audio::init_sound_bank,
            ),
        )
        .add_systems(Update, tick_energy_regen)
        // Title screen
        .add_systems(OnEnter(AppState::Title), title_ui::setup_title_ui)
        .add_systems(
            Update,
            title_ui::handle_title_buttons.run_if(in_state(AppState::Title)),
        )
        .add_systems(OnExit(AppState::Title), title_ui::cleanup_title_ui)
        // Fjord City hub
        .add_systems(OnEnter(AppState::FjordCity), city_ui::setup_city_ui)
        .add_systems(
            Update,
            city_ui::handle_city_buttons.run_if(in_state(AppState::FjordCity)),
        )
        .add_systems(OnExit(AppState::FjordCity), city_ui::cleanup_city_ui)
        // Match-3 board scene
        .add_systems(OnEnter(AppState::Match3), match3_scene::setup_match3_scene)
        .add_systems(
            Update,
            (
                match3_scene::handle_match3_hud_buttons,
                match3_scene::handle_board_pointer_input,
                match3_scene::tick_board_animation,
                match3_scene::sync_match3_visuals,
                match3_scene::animate_particles,
            )
                .chain()
                .run_if(in_state(AppState::Match3)),
        )
        .add_systems(OnExit(AppState::Match3), match3_scene::cleanup_match3_scene)
        // Level Result modal
        .add_systems(OnEnter(AppState::LevelResult), result_ui::setup_result_ui)
        .add_systems(
            Update,
            result_ui::handle_result_buttons.run_if(in_state(AppState::LevelResult)),
        )
        .add_systems(OnExit(AppState::LevelResult), result_ui::cleanup_result_ui)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
