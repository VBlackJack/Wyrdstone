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
mod platform;
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
#[cfg(all(target_os = "android", not(target_arch = "wasm32")))]
use bevy::window::MonitorSelection;
#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
use bevy::window::{MonitorSelection, WindowMode};
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::{WindowPosition, WindowResizeConstraints, WindowResolution};
use state::{tick_energy_regen, AppState, GameContext};
use theme::COLOR_BG_DEEP;
#[cfg(not(target_arch = "wasm32"))]
use theme::{WINDOW_DEFAULT_HEIGHT, WINDOW_DEFAULT_WIDTH, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};

/// Android entry point invoked by `android-activity` / `cargo-apk`.
#[cfg(target_os = "android")]
#[bevy_main]
fn main() {
    run_app();
}

#[cfg(target_os = "windows")]
const ENV_WGPU_BACKEND: &str = "WGPU_BACKEND";
#[cfg(target_os = "windows")]
const ENV_WGPU_DX12_COMPILER: &str = "WGPU_DX12_COMPILER";

/// Configures and launches the Wyrdstone Bevy application.
pub fn run_app() {
    let ctx = GameContext::initialize();
    let window_title = ctx.data.i18n.t("app.title.name").to_string();
    let mut app = App::new();

    #[cfg(not(target_arch = "wasm32"))]
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: window_title,
            resolution: WindowResolution::new(WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_HEIGHT),
            position: WindowPosition::Centered(MonitorSelection::Primary),
            resize_constraints: WindowResizeConstraints {
                min_width: WINDOW_MIN_WIDTH,
                min_height: WINDOW_MIN_HEIGHT,
                ..default()
            },
            resizable: true,
            ..default()
        }),
        ..default()
    };
    #[cfg(target_arch = "wasm32")]
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: window_title,
            canvas: Some("#wyrdstone-canvas".into()),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: true,
            ..default()
        }),
        ..default()
    };

    // Default to DX12 with FXC and single-threaded rendering on Windows to avoid
    // NVIDIA Vulkan swapchain freezes during window resize/unmaximize, while still
    // honoring explicit WGPU_BACKEND / WGPU_DX12_COMPILER environment overrides.
    #[cfg(target_os = "windows")]
    let plugins = {
        use bevy::render::settings::{Backends, Dx12Compiler, RenderCreation, WgpuSettings};
        use bevy::render::RenderPlugin;

        let mut wgpu_settings = WgpuSettings::default();
        if std::env::var_os(ENV_WGPU_BACKEND).is_none() {
            wgpu_settings.backends = Some(Backends::DX12);
        }
        if std::env::var_os(ENV_WGPU_DX12_COMPILER).is_none() {
            wgpu_settings.dx12_shader_compiler = Dx12Compiler::Fxc;
        }

        DefaultPlugins
            .set(window_plugin)
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(wgpu_settings),
                ..default()
            })
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
    };
    #[cfg(all(not(target_os = "windows"), not(target_arch = "wasm32")))]
    let plugins = DefaultPlugins.set(window_plugin);
    #[cfg(target_arch = "wasm32")]
    let plugins = DefaultPlugins
        .set(bevy::asset::AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..default()
        })
        .set(window_plugin);

    app.add_plugins(plugins)
        .insert_resource(ClearColor(COLOR_BG_DEEP))
        .insert_resource(ctx)
        .init_resource::<platform::SafeAreaInsets>()
        .add_event::<platform::HapticEvent>()
        .init_resource::<scroll::UiScrollMemory>()
        .init_state::<AppState>();
    ui_style::install_box_shadow_cull_guard(&mut app);

    app.add_systems(
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
    .add_systems(PreUpdate, map_back_key_to_escape.after(InputSystem));
    #[cfg(target_os = "android")]
    {
        app.add_systems(
            Startup,
            platform::sync_android_safe_area.before(enter_title_after_boot),
        )
        .add_systems(
            PreUpdate,
            platform::sync_android_safe_area.before(InputSystem),
        );
    }
    app.add_systems(
        Update,
        (
            tick_energy_regen,
            handle_global_shortcuts,
            audio::handle_app_lifecycle,
            scroll::handle_ui_scroll,
            ui_style::tick_screen_fades,
            ui_style::animate_button_feedback,
            platform::handle_haptic_events,
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
            city_ui::update_city_hover_tooltip,
            city_ui::sync_map_building_layout,
            city_ui::animate_fjord_harvest_orbs,
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
    .add_systems(OnExit(AppState::LevelResult), result_ui::cleanup_result_ui);
    app.run();
}

fn setup_camera(
    mut commands: Commands,
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))] winit_windows: NonSend<
        bevy::winit::WinitWindows,
    >,
) {
    commands.spawn(Camera2d);
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
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
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))] mut windows: Query<
        &mut Window,
        With<bevy::window::PrimaryWindow>,
    >,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        audio::toggle_mute(&mut ctx, &mut sound_bank, &mut bgm_sinks);
    }
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    if keys.just_pressed(KeyCode::F11) {
        if let Ok(mut window) = windows.get_single_mut() {
            window.mode = match window.mode {
                WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                _ => WindowMode::Windowed,
            };
        }
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

#[cfg(test)]
mod system_param_tests {
    use super::*;

    fn assert_system_valid<M>(sys: impl IntoSystem<(), (), M>) {
        let mut world = World::new();
        let mut system = IntoSystem::into_system(sys);
        system.initialize(&mut world);
    }

    #[test]
    fn all_registered_systems_initialize_without_b0001_conflicts() {
        // Shared Update & PreUpdate systems
        assert_system_valid(enter_title_after_boot);
        assert_system_valid(map_back_key_to_escape);
        assert_system_valid(tick_energy_regen);
        assert_system_valid(handle_global_shortcuts);
        assert_system_valid(audio::handle_app_lifecycle);
        assert_system_valid(scroll::handle_ui_scroll);
        assert_system_valid(ui_style::tick_screen_fades);
        assert_system_valid(ui_style::animate_button_feedback);
        assert_system_valid(platform::handle_haptic_events);

        // Title systems
        assert_system_valid(title_ui::setup_title_ui);
        assert_system_valid(title_ui::handle_title_buttons);
        assert_system_valid(title_ui::animate_title_ambience);
        assert_system_valid(title_ui::cleanup_title_ui);

        // Fjord City systems
        assert_system_valid(city_ui::setup_city_ui);
        assert_system_valid(city_ui::handle_city_buttons);
        assert_system_valid(city_ui::update_city_hover_tooltip);
        assert_system_valid(city_ui::sync_map_building_layout);
        assert_system_valid(city_ui::animate_fjord_harvest_orbs);
        assert_system_valid(city_ui::animate_city_ambience);
        assert_system_valid(city_ui::cleanup_city_ui);

        // Match-3 systems
        assert_system_valid(match3_scene::setup_match3_scene);
        assert_system_valid(match3_scene::handle_match3_hud_buttons);
        assert_system_valid(match3_scene::handle_board_pointer_input);
        assert_system_valid(match3_scene::tick_board_animation);
        assert_system_valid(match3_scene::sync_match3_visuals);
        assert_system_valid(match3_scene::animate_particles);
        assert_system_valid(match3_scene::cleanup_match3_scene);

        // Level Result systems
        assert_system_valid(result_ui::setup_result_ui);
        assert_system_valid(result_ui::handle_result_buttons);
        assert_system_valid(result_ui::animate_result_stars);
        assert_system_valid(result_ui::cleanup_result_ui);
    }
}
