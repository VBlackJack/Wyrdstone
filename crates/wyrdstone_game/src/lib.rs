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

mod art_util;
mod audio;
mod city_ui;
mod fjord_art;
mod match3_scene;
mod result_ui;
mod runes_art;
mod scroll;
mod state;
mod theme;
mod title_ui;
mod ui_style;
mod vfx_art;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::{ButtonState, InputSystem};
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

    let mut default_plugins = DefaultPlugins
        .set(window_plugin)
        .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(
    );
    #[cfg(target_os = "windows")]
    {
        default_plugins = default_plugins.set(bevy::render::RenderPlugin {
            render_creation: bevy::render::settings::RenderCreation::Automatic(
                bevy::render::settings::WgpuSettings {
                    backends: Some(bevy::render::settings::Backends::DX12),
                    ..default()
                },
            ),
            ..default()
        });
    }

    app.add_plugins(default_plugins)
        .insert_resource(ClearColor(COLOR_BG_DEEP))
        .insert_resource(ctx)
        .init_resource::<scroll::UiScrollMemory>()
        .init_state::<AppState>()
        .add_systems(
            PreStartup,
            (
                setup_camera,
                runes_art::init_rune_textures,
                vfx_art::init_vfx_textures,
                fjord_art::init_fjord_textures,
                audio::init_sound_bank,
            ),
        )
        .add_systems(Startup, enter_title_after_boot)
        .add_systems(PreUpdate, map_back_key_to_escape.after(InputSystem))
        .add_systems(
            Update,
            (
                tick_energy_regen,
                handle_global_shortcuts,
                audio::handle_app_lifecycle,
                scroll::handle_ui_scroll,
                ui_style::tick_screen_fades,
                ui_style::animate_button_feedback,
            ),
        )
        // Title screen
        .add_systems(
            OnEnter(AppState::Title),
            (title_ui::setup_title_ui, ui_style::spawn_screen_fade),
        )
        .add_systems(
            Update,
            title_ui::handle_title_buttons.run_if(in_state(AppState::Title)),
        )
        .add_systems(
            PostUpdate,
            title_ui::animate_title_ambience
                .after(bevy::ui::UiSystem::Layout)
                .before(bevy::transform::TransformSystem::TransformPropagate)
                .run_if(in_state(AppState::Title)),
        )
        .add_systems(OnExit(AppState::Title), title_ui::cleanup_title_ui)
        // Fjord City hub
        .add_systems(
            OnEnter(AppState::FjordCity),
            (city_ui::setup_city_ui, ui_style::spawn_screen_fade),
        )
        .add_systems(
            Update,
            (
                city_ui::handle_city_buttons,
                city_ui::sync_map_building_layout,
            )
                .chain()
                .run_if(in_state(AppState::FjordCity)),
        )
        .add_systems(
            PostUpdate,
            city_ui::animate_city_ambience
                .after(bevy::ui::UiSystem::Layout)
                .before(bevy::transform::TransformSystem::TransformPropagate)
                .run_if(in_state(AppState::FjordCity)),
        )
        .add_systems(OnExit(AppState::FjordCity), city_ui::cleanup_city_ui)
        // Match-3 board scene
        .add_systems(
            OnEnter(AppState::Match3),
            (
                match3_scene::setup_match3_scene,
                audio::switch_bgm_to_match3,
                ui_style::spawn_screen_fade,
            ),
        )
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
        .add_systems(
            OnExit(AppState::Match3),
            (
                match3_scene::cleanup_match3_scene,
                audio::switch_bgm_to_fjord,
            ),
        )
        // Level Result modal
        .add_systems(
            OnEnter(AppState::LevelResult),
            (result_ui::setup_result_ui, ui_style::spawn_screen_fade),
        )
        .add_systems(
            Update,
            (
                result_ui::handle_result_buttons,
                result_ui::animate_result_stars,
            )
                .run_if(in_state(AppState::LevelResult)),
        )
        .add_systems(OnExit(AppState::LevelResult), result_ui::cleanup_result_ui)
        .run();
}

fn setup_camera(
    mut commands: Commands,
    #[cfg(not(target_os = "android"))] mut windows: Query<
        &mut Window,
        With<bevy::window::PrimaryWindow>,
    >,
    #[cfg(not(target_os = "android"))] winit_windows: NonSend<bevy::winit::WinitWindows>,
) {
    commands.spawn(Camera2d);
    #[cfg(not(target_os = "android"))]
    {
        if let Ok(mut window) = windows.get_single_mut() {
            window.set_maximized(true);
        }
        let (rgba, w, h) = fjord_art::app_icon_rgba();
        if let Ok(icon) = winit::window::Icon::from_rgba(rgba, w, h) {
            for winit_win in winit_windows.windows.values() {
                winit_win.set_window_icon(Some(icon.clone()));
            }
        }
    }
}

fn enter_title_after_boot(mut next_state: ResMut<NextState<AppState>>) {
    next_state.set(AppState::Title);
}

fn handle_global_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut ctx: ResMut<GameContext>,
    mut sound_bank: ResMut<audio::SoundBank>,
    mut bgm_sinks: Query<&mut AudioSink, With<audio::BgmTrack>>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        audio::toggle_mute(&mut ctx, &mut sound_bank, &mut bgm_sinks);
    }
}

/// Maps the Android system Back key (reported as a logical browser-back key with no physical
/// key code) onto `KeyCode::Escape`, mirroring both press and release so Escape never sticks.
/// On Android, Back from the Title screen (with no pending reset confirmation) saves and exits.
fn map_back_key_to_escape(
    mut events: EventReader<KeyboardInput>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    #[cfg(target_os = "android")] state: Res<State<AppState>>,
    #[cfg(target_os = "android")] mut ctx: ResMut<GameContext>,
) {
    for event in events.read() {
        if !matches!(event.logical_key, Key::BrowserBack | Key::GoBack) {
            continue;
        }
        match event.state {
            ButtonState::Pressed => {
                if event.repeat {
                    continue;
                }
                #[cfg(target_os = "android")]
                if *state.get() == AppState::Title && !ctx.confirm_reset {
                    // Returning from android_main would leave the process alive with a stale
                    // activity handle, so terminate it explicitly after saving.
                    ctx.persist();
                    std::process::exit(0);
                }
                keys.press(KeyCode::Escape);
            }
            ButtonState::Released => keys.release(KeyCode::Escape),
        }
    }
}
