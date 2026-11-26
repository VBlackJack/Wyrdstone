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

//! Full-bleed painted Title screen with floating sculpted emblem, aurora ambience, and play entry.

use crate::audio::{toggle_mute, BgmTrack, SoundBank};
use crate::fjord_art::FjordTextures;
use crate::platform::{HudLayoutMode, SafeAreaInsets};
use crate::scroll::{PressArm, UiScrollMemory};
use crate::state::{AppState, GameContext};
use crate::theme::*;
use crate::ui_style::{
    button_shadow, display_font, gold_glow_shadow, panel_shadow, sliced_gold_button_image,
    sliced_panel_image, spawn_notice_toast, ToastLife, NOTICE_TOAST_DURATION_SECS,
};
use crate::vfx_art::VfxTextures;
use bevy::prelude::*;
#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode, WindowResized};
#[cfg(any(target_os = "android", target_arch = "wasm32"))]
use bevy::window::{PrimaryWindow, WindowResized};

const TITLE_SNOWFLAKE_COUNT: usize = 26;
const TITLE_CONTENT_GAP_PX: f32 = 10.0;
const TITLE_UTIL_BTN_MIN_WIDTH_PX: f32 = 132.0;

#[derive(Component)]
pub struct TitleRoot;

#[derive(Component)]
pub struct TitleLogoFloat;

#[derive(Component)]
pub struct TitleAurora {
    pub phase: f32,
    pub base_alpha: f32,
}

#[derive(Component)]
pub struct TitleSnowflake {
    pub base_x_pct: f32,
    pub base_y_pct: f32,
    pub x_pct: f32,
    pub y_pct: f32,
    pub speed_y: f32,
    pub phase: f32,
}

#[derive(Component, Clone, Copy)]
pub enum TitleAction {
    EnterFjord,
    NewSaga,
    ToggleLanguage,
    ToggleAudio,
    #[cfg(not(target_os = "android"))]
    ToggleFullscreen,
    #[cfg(target_arch = "wasm32")]
    EnterFullscreen(crate::platform::FullscreenOrientation),
}

fn title_is_fullscreen(
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))] window: Option<&Window>,
) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        crate::platform::is_web_fullscreen()
    }
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        window.is_some_and(|w| !matches!(w.mode, WindowMode::Windowed))
    }
    #[cfg(target_os = "android")]
    {
        false
    }
}

/// Spawns the Title screen hierarchy.
pub fn setup_title_ui(
    mut commands: Commands,
    ctx: Res<GameContext>,
    fjord_art: Res<FjordTextures>,
    vfx: Res<VfxTextures>,
    sound_bank: Res<SoundBank>,
    safe_area: Res<SafeAreaInsets>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let win_opt = windows.get_single().ok();
    let hud_mode = win_opt
        .map(|w| HudLayoutMode::from_viewport(w.width(), w.height()))
        .unwrap_or_default();
    let is_fullscreen = title_is_fullscreen(
        #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
        win_opt,
    );
    spawn_title_tree(
        &mut commands,
        &ctx,
        &fjord_art,
        &vfx,
        sound_bank.muted,
        *safe_area,
        is_fullscreen,
        hud_mode,
    );
}

/// Cleans up Title screen entities on state exit.
pub fn cleanup_title_ui(
    mut commands: Commands,
    mut ctx: ResMut<GameContext>,
    roots: Query<Entity, With<TitleRoot>>,
) {
    // A notice shown on the Title screen has been seen: do not repeat it on the hub. The
    // read-only save warning stays until the player restarts.
    if !ctx.save_read_only {
        ctx.status_notice_key = None;
    }
    for entity in &roots {
        commands.entity(entity).try_despawn_recursive();
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_title_tree(
    commands: &mut Commands,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    muted: bool,
    safe: SafeAreaInsets,
    is_fullscreen: bool,
    hud_mode: HudLayoutMode,
) {
    let i18n = &ctx.data.i18n;
    let is_phone_landscape = hud_mode.is_phone_landscape();
    let is_mobile = hud_mode.is_mobile();
    #[cfg(target_os = "android")]
    let _ = is_fullscreen;
    let play_label = if ctx.has_existing_save {
        i18n.t("app.title.button.continue")
    } else {
        i18n.t("app.title.button.play")
    };
    let sound_label = if muted {
        i18n.t("app.audio.off")
    } else {
        i18n.t("app.audio.on")
    };

    commands
        .spawn((
            TitleRoot,
            ImageNode::new(fjord_art.fjord_map.clone()),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .with_children(|bg| {
            // Soft textured Aurora Borealis ribbons drifting across the night sky.
            let auroras = [
                (
                    4.0,
                    2.0,
                    56.0,
                    18.0,
                    Color::srgba(0.20, 0.94, 0.72, 0.34),
                    0.0,
                ),
                (
                    38.0,
                    4.0,
                    58.0,
                    16.0,
                    Color::srgba(0.32, 0.80, 0.98, 0.28),
                    2.1,
                ),
                (
                    18.0,
                    10.0,
                    46.0,
                    13.0,
                    Color::srgba(0.58, 0.42, 0.98, 0.22),
                    4.0,
                ),
            ];
            for (left, top, w, h, color, phase) in auroras {
                bg.spawn((
                    TitleAurora {
                        phase,
                        base_alpha: color.alpha(),
                    },
                    ImageNode::new(vfx.aurora_ribbon.clone()).with_color(color),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(left),
                        top: Val::Percent(top),
                        width: Val::Percent(w),
                        height: Val::Percent(h),
                        ..default()
                    },
                    PickingBehavior::IGNORE,
                ));
            }

            // Gentle textured snow drifting over the fjord.
            for i in 0..TITLE_SNOWFLAKE_COUNT {
                let x_pct = ((i * 37 + 11) % 98) as f32;
                let y_pct = ((i * 53 + 7) % 98) as f32;
                let size = 4.0 + (i % 4) as f32 * 2.2;
                bg.spawn((
                    TitleSnowflake {
                        base_x_pct: x_pct,
                        base_y_pct: y_pct,
                        x_pct,
                        y_pct,
                        speed_y: 3.2 + (i % 5) as f32 * 1.1,
                        phase: i as f32 * 0.9,
                    },
                    ImageNode::new(vfx.soft_orb.clone())
                        .with_color(Color::srgba(0.92, 0.97, 1.0, 0.55)),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(x_pct),
                        top: Val::Percent(y_pct),
                        width: Val::Px(size),
                        height: Val::Px(size),
                        ..default()
                    },
                    PickingBehavior::IGNORE,
                ));
            }

            // Subtle dark vignette overlay for crisp typography contrast.
            bg.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(if is_phone_landscape {
                        6.0
                    } else {
                        TITLE_CONTENT_GAP_PX
                    }),
                    padding: UiRect {
                        left: Val::Px(if is_mobile { 10.0 } else { 14.0 } + safe.left),
                        right: Val::Px(if is_mobile { 10.0 } else { 14.0 } + safe.right),
                        top: Val::Px(if is_mobile { 8.0 } else { 12.0 } + safe.top),
                        bottom: Val::Px(if is_mobile { 8.0 } else { 12.0 } + safe.bottom),
                    },
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.03, 0.06, 0.11, 0.46)),
            ))
            .with_children(|parent| {
                // Floating transparent sculpted WYRDSTONE emblem directly over the fjord sky.
                parent
                    .spawn((
                        TitleLogoFloat,
                        Node {
                            max_width: Val::Percent(92.0),
                            max_height: if is_phone_landscape {
                                Val::Vh(18.0)
                            } else {
                                Val::Vh(24.0)
                            },
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            flex_shrink: 1.0,
                            margin: UiRect::bottom(Val::Px(2.0)),
                            ..default()
                        },
                    ))
                    .with_children(|hero| {
                        hero.spawn((
                            ImageNode::new(fjord_art.logo.clone()),
                            Node {
                                width: Val::Px(if is_phone_landscape { 270.0 } else { 520.0 }),
                                max_width: Val::Percent(100.0),
                                max_height: if is_phone_landscape {
                                    Val::Vh(18.0)
                                } else {
                                    Val::Vh(24.0)
                                },
                                flex_shrink: 1.0,
                                aspect_ratio: Some(16.0 / 9.0),
                                ..default()
                            },
                        ));
                    });

                // Sleek carved Nordic subtitle & lore plaque (9-slice stone & bronze frame).
                parent
                    .spawn((
                        sliced_panel_image(vfx),
                        Node {
                            max_width: Val::Percent(92.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(4.0),
                            padding: if is_phone_landscape {
                                UiRect::axes(Val::Px(14.0), Val::Px(7.0))
                            } else if is_mobile {
                                UiRect::axes(Val::Px(16.0), Val::Px(10.0))
                            } else {
                                UiRect::axes(Val::Px(24.0), Val::Px(12.0))
                            },
                            ..default()
                        },
                        panel_shadow(),
                    ))
                    .with_children(|plaque| {
                        plaque.spawn((
                            Text::new(i18n.t("app.title.subtitle")),
                            display_font(
                                fjord_art,
                                if is_phone_landscape {
                                    FONT_BODY
                                } else {
                                    FONT_HEADING + 2.0
                                },
                            ),
                            TextColor(COLOR_ACCENT_GOLD),
                            TextLayout::new_with_justify(JustifyText::Center),
                        ));
                        plaque.spawn((
                            Text::new(i18n.t("app.title.footer")),
                            TextFont {
                                font_size: if is_mobile { FONT_SMALL } else { FONT_BODY },
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                            TextLayout::new_with_justify(JustifyText::Center),
                        ));
                    });

                // Primary sculpted forged-gold Play button (9-slice).
                parent
                    .spawn((
                        Button,
                        TitleAction::EnterFjord,
                        sliced_gold_button_image(vfx),
                        Node {
                            width: Val::Px(if is_phone_landscape { 260.0 } else { 320.0 }),
                            max_width: Val::Percent(92.0),
                            height: Val::Px(if is_phone_landscape { 46.0 } else { 58.0 }),
                            border: UiRect::all(Val::Px(2.5)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor(Color::NONE),
                        BorderRadius::all(Val::Px(12.0)),
                        gold_glow_shadow(),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(play_label),
                            display_font(
                                fjord_art,
                                if is_phone_landscape {
                                    FONT_BODY + 1.0
                                } else {
                                    FONT_HEADING + 1.0
                                },
                            ),
                            TextColor(COLOR_TEXT_DARK),
                        ));
                    });

                // Utility buttons row (Language + Audio + Optional New Saga)
                parent
                    .spawn(Node {
                        column_gap: Val::Px(if is_phone_landscape { 8.0 } else { 12.0 }),
                        row_gap: Val::Px(if is_phone_landscape { 6.0 } else { 8.0 }),
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        let mut util_btns: Vec<(&str, TitleAction, Color)> = vec![
                            (
                                i18n.t("app.title.button.language"),
                                TitleAction::ToggleLanguage,
                                Color::srgba(0.10, 0.14, 0.22, 0.90),
                            ),
                            (
                                sound_label,
                                TitleAction::ToggleAudio,
                                Color::srgba(0.10, 0.14, 0.22, 0.90),
                            ),
                        ];
                        if ctx.has_existing_save {
                            let (lbl, col) = if ctx.confirm_reset {
                                (
                                    i18n.t("app.title.button.confirm_reset"),
                                    COLOR_ACCENT_CRIMSON,
                                )
                            } else {
                                (
                                    i18n.t("app.title.button.new_saga"),
                                    Color::srgba(0.24, 0.12, 0.14, 0.92),
                                )
                            };
                            util_btns.push((lbl, TitleAction::NewSaga, col));
                        }
                        #[cfg(target_arch = "wasm32")]
                        if crate::platform::is_fullscreen_supported() {
                            if crate::platform::is_orientation_lock_supported() {
                                util_btns.push((
                                    i18n.t("app.display.fullscreen_portrait"),
                                    TitleAction::EnterFullscreen(
                                        crate::platform::FullscreenOrientation::Portrait,
                                    ),
                                    Color::srgba(0.10, 0.14, 0.22, 0.90),
                                ));
                                util_btns.push((
                                    i18n.t("app.display.fullscreen_landscape"),
                                    TitleAction::EnterFullscreen(
                                        crate::platform::FullscreenOrientation::Landscape,
                                    ),
                                    Color::srgba(0.10, 0.14, 0.22, 0.90),
                                ));
                                if is_fullscreen {
                                    util_btns.push((
                                        i18n.t("app.display.windowed"),
                                        TitleAction::ToggleFullscreen,
                                        COLOR_ACCENT_TEAL,
                                    ));
                                }
                            } else {
                                let fs_lbl = if is_fullscreen {
                                    i18n.t("app.display.windowed")
                                } else {
                                    i18n.t("app.display.fullscreen")
                                };
                                util_btns.push((
                                    fs_lbl,
                                    TitleAction::ToggleFullscreen,
                                    if is_fullscreen {
                                        COLOR_ACCENT_TEAL
                                    } else {
                                        Color::srgba(0.10, 0.14, 0.22, 0.90)
                                    },
                                ));
                            }
                        }
                        #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
                        if crate::platform::is_fullscreen_supported() {
                            let fs_lbl = if is_fullscreen {
                                i18n.t("app.display.windowed")
                            } else {
                                i18n.t("app.display.fullscreen")
                            };
                            util_btns.push((
                                fs_lbl,
                                TitleAction::ToggleFullscreen,
                                if is_fullscreen {
                                    COLOR_ACCENT_TEAL
                                } else {
                                    Color::srgba(0.10, 0.14, 0.22, 0.90)
                                },
                            ));
                        }
                        for (label, act, bg_col) in util_btns {
                            row.spawn((
                                Button,
                                act,
                                Node {
                                    min_width: Val::Px(if is_mobile {
                                        112.0
                                    } else {
                                        TITLE_UTIL_BTN_MIN_WIDTH_PX
                                    }),
                                    min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                    padding: if is_phone_landscape {
                                        UiRect::axes(Val::Px(10.0), Val::Px(6.0))
                                    } else {
                                        UiRect::axes(Val::Px(14.0), Val::Px(8.0))
                                    },
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.5)),
                                    ..default()
                                },
                                BackgroundColor(bg_col),
                                BorderColor(COLOR_PANEL_BORDER),
                                BorderRadius::all(Val::Px(10.0)),
                                button_shadow(),
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(label),
                                    TextFont {
                                        font_size: if is_mobile { FONT_SMALL } else { FONT_BODY },
                                        ..default()
                                    },
                                    TextColor(title_util_text_color(bg_col)),
                                    TextLayout::new_with_justify(JustifyText::Center),
                                ));
                            });
                        }
                    });
            });

            // Save and recovery notices (restored from backup, read-only, unreadable save).
            if let Some(notice_key) = &ctx.status_notice_key {
                let life = (!ctx.save_read_only).then_some(NOTICE_TOAST_DURATION_SECS);
                spawn_notice_toast(bg, i18n.t(notice_key), life, safe.bottom);
            }
        });
}

/// Chooses readable foreground contrast for Title utility buttons.
fn title_util_text_color(bg_col: Color) -> Color {
    if bg_col == COLOR_ACCENT_TEAL {
        COLOR_TEXT_ON_ACCENT
    } else {
        COLOR_TEXT_PRIMARY
    }
}

/// Animates the floating WYRDSTONE emblem, aurora ribbons, and snowfall on the Title screen via GPU transforms.
#[allow(clippy::type_complexity)]
pub fn animate_title_ambience(
    time: Res<Time>,
    ctx: Res<GameContext>,
    parent_nodes: Query<
        &ComputedNode,
        (
            Without<TitleLogoFloat>,
            Without<TitleAurora>,
            Without<TitleSnowflake>,
        ),
    >,
    mut logos: Query<
        (&ComputedNode, &mut Transform),
        (
            With<TitleLogoFloat>,
            Without<TitleAurora>,
            Without<TitleSnowflake>,
        ),
    >,
    mut auroras: Query<
        (&TitleAurora, &Parent, &mut Transform, &mut ImageNode),
        (Without<TitleLogoFloat>, Without<TitleSnowflake>),
    >,
    mut flakes: Query<
        (&mut TitleSnowflake, &Parent, &mut Transform),
        (Without<TitleLogoFloat>, Without<TitleAurora>),
    >,
) {
    if ctx.reduced_motion {
        return;
    }
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    let float_px = (elapsed * 1.6).sin() * 5.0;
    for (computed, mut tf) in &mut logos {
        let scale_factor = computed.inverse_scale_factor().max(0.01).recip();
        tf.translation.y += float_px * scale_factor;
    }

    for (aurora, parent, mut tf, mut img) in &mut auroras {
        let drift = (elapsed * 0.32 + aurora.phase).sin() * 4.5;
        let pw = parent_nodes
            .get(parent.get())
            .map(|n| n.size().x)
            .unwrap_or(0.0);
        tf.translation.x += drift * 0.01 * pw;
        let shimmer = 0.72 + 0.28 * (elapsed * 0.78 + aurora.phase * 1.4).cos();
        img.color = img.color.with_alpha(aurora.base_alpha * shimmer);
    }

    for (mut flake, parent, mut tf) in &mut flakes {
        flake.y_pct += flake.speed_y * dt;
        if flake.y_pct > 102.0 {
            flake.y_pct = -3.0;
        }
        let sway = (elapsed * 1.1 + flake.phase).sin() * 1.4;
        let target_x = (flake.x_pct + sway).clamp(0.0, 99.0);
        let psize = parent_nodes
            .get(parent.get())
            .map(|n| n.size())
            .unwrap_or(Vec2::ZERO);
        tf.translation.x += (target_x - flake.base_x_pct) * 0.01 * psize.x;
        tf.translation.y += (flake.y_pct - flake.base_y_pct) * 0.01 * psize.y;
    }
}

type TitleButtonQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Interaction,
        &'static TitleAction,
        &'static mut BackgroundColor,
        Option<&'static mut ImageNode>,
        Option<&'static Children>,
    ),
    Changed<Interaction>,
>;

type TitleFocusQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static TitleAction, &'static mut BorderColor), Without<ToastLife>>;

/// Handles clicks and keyboard shortcuts (`Tab`/`Arrows`, `Enter`/`Space`, `Escape`) on Title.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn handle_title_buttons(
    mut commands: Commands,
    time: Res<Time>,
    mut toasts: Query<(Entity, &mut ToastLife)>,
    keys: Res<ButtonInput<KeyCode>>,
    mut ctx: ResMut<GameContext>,
    fjord_art: Res<FjordTextures>,
    vfx: Res<VfxTextures>,
    mut sound_bank: ResMut<SoundBank>,
    mut bgm_sinks: Query<&mut AudioSink, With<BgmTrack>>,
    mut next_state: ResMut<NextState<AppState>>,
    roots: Query<Entity, With<TitleRoot>>,
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))] mut windows: Query<
        &mut Window,
        With<PrimaryWindow>,
    >,
    #[cfg(any(target_os = "android", target_arch = "wasm32"))] windows: Query<
        &Window,
        With<PrimaryWindow>,
    >,
    (
        touches,
        scroll_mem,
        safe_area,
        mut press_arm,
        mut focus_idx,
        mut prev_fullscreen,
        mut prev_hud_mode,
        mut resize_events,
    ): (
        Res<Touches>,
        Res<UiScrollMemory>,
        Res<SafeAreaInsets>,
        Local<PressArm>,
        Local<Option<usize>>,
        Local<Option<bool>>,
        Local<Option<HudLayoutMode>>,
        EventReader<WindowResized>,
    ),
    mut buttons: TitleButtonQuery,
    mut focus_buttons: TitleFocusQuery,
    mut text_colors: Query<&mut TextColor>,
) {
    let mut hud_mode_now = prev_hud_mode.unwrap_or_else(|| {
        windows
            .get_single()
            .map(|w| HudLayoutMode::from_viewport(w.width(), w.height()))
            .unwrap_or_default()
    });
    for ev in resize_events.read() {
        if ev.width.is_finite() && ev.height.is_finite() && ev.width >= 100.0 && ev.height >= 100.0
        {
            hud_mode_now = HudLayoutMode::from_viewport(ev.width, ev.height);
        }
    }
    let mode_changed = match *prev_hud_mode {
        None => {
            *prev_hud_mode = Some(hud_mode_now);
            false
        }
        Some(prev) if prev != hud_mode_now => {
            *prev_hud_mode = Some(hud_mode_now);
            true
        }
        _ => false,
    };

    let fs_now = {
        #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
        {
            title_is_fullscreen(windows.get_single().ok())
        }
        #[cfg(not(all(not(target_os = "android"), not(target_arch = "wasm32"))))]
        {
            title_is_fullscreen()
        }
    };
    let fs_changed = match *prev_fullscreen {
        None => {
            *prev_fullscreen = Some(fs_now);
            false
        }
        Some(prev) if prev != fs_now => {
            *prev_fullscreen = Some(fs_now);
            true
        }
        _ => false,
    };

    for (entity, mut life) in &mut toasts {
        life.0 -= time.delta_secs();
        if life.0 <= 0.0 {
            commands.entity(entity).try_despawn_recursive();
            if !ctx.save_read_only {
                ctx.status_notice_key = None;
            }
        }
    }

    if fs_changed || mode_changed {
        for entity in &roots {
            commands.entity(entity).try_despawn_recursive();
        }
        spawn_title_tree(
            &mut commands,
            &ctx,
            &fjord_art,
            &vfx,
            sound_bank.muted,
            *safe_area,
            fs_now,
            hud_mode_now,
        );
        return;
    }

    if keys.just_pressed(KeyCode::Escape) && ctx.confirm_reset {
        ctx.confirm_reset = false;
        *focus_idx = None;
        for entity in &roots {
            commands.entity(entity).try_despawn_recursive();
        }
        spawn_title_tree(
            &mut commands,
            &ctx,
            &fjord_art,
            &vfx,
            sound_bank.muted,
            *safe_area,
            fs_now,
            hud_mode_now,
        );
        return;
    }

    let focus_step: Option<isize> = if keys.just_pressed(KeyCode::Tab) {
        if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            Some(-1)
        } else {
            Some(1)
        }
    } else if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::ArrowRight) {
        Some(1)
    } else if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::ArrowLeft) {
        Some(-1)
    } else {
        None
    };

    if let Some(step) = focus_step {
        let count = focus_buttons.iter().count();
        if count > 0 {
            let next = match *focus_idx {
                Some(cur) => ((cur as isize + step).rem_euclid(count as isize)) as usize,
                None => {
                    if step >= 0 {
                        0
                    } else {
                        count - 1
                    }
                }
            };
            *focus_idx = Some(next);
            for (idx, (_, action, mut border)) in focus_buttons.iter_mut().enumerate() {
                if idx == next {
                    border.0 = COLOR_ACCENT_GOLD_HOVER;
                } else {
                    border.0 = match action {
                        TitleAction::EnterFjord => Color::NONE,
                        _ => COLOR_PANEL_BORDER,
                    };
                }
            }
        }
        return;
    }

    let mut triggered_actions: Vec<TitleAction> = Vec::new();
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        let chosen = focus_idx
            .and_then(|idx| focus_buttons.iter().nth(idx).map(|(_, act, _)| *act))
            .unwrap_or(TitleAction::EnterFjord);
        triggered_actions.push(chosen);
    }

    if sound_bank.is_changed() && !sound_bank.is_added() {
        for entity in &roots {
            commands.entity(entity).try_despawn_recursive();
        }
        spawn_title_tree(
            &mut commands,
            &ctx,
            &fjord_art,
            &vfx,
            sound_bank.muted,
            *safe_area,
            fs_now,
            hud_mode_now,
        );
        return;
    }

    // Actions fire on release so a touch that starts a scroll never triggers a button.
    let tapped = press_arm.completed_taps(
        buttons.iter().map(|(entity, i, ..)| (entity, *i)),
        &touches,
        &scroll_mem,
    );
    for (entity, _, action, ..) in &buttons {
        if tapped.contains(&entity) {
            triggered_actions.push(*action);
        }
    }
    if let Some(action) = triggered_actions.into_iter().next() {
        match action {
            TitleAction::EnterFjord => {
                ctx.confirm_reset = false;
                *focus_idx = None;
                next_state.set(AppState::FjordCity);
                return;
            }
            TitleAction::NewSaga => {
                if ctx.confirm_reset {
                    if ctx.reset_saga() {
                        *focus_idx = None;
                        next_state.set(AppState::FjordCity);
                    } else {
                        // The newer save could not be preserved: stay here, notice is set.
                        ctx.confirm_reset = false;
                        *focus_idx = None;
                        for entity in &roots {
                            commands.entity(entity).try_despawn_recursive();
                        }
                        spawn_title_tree(
                            &mut commands,
                            &ctx,
                            &fjord_art,
                            &vfx,
                            sound_bank.muted,
                            *safe_area,
                            fs_now,
                            hud_mode_now,
                        );
                    }
                } else {
                    ctx.confirm_reset = true;
                    *focus_idx = None;
                    for entity in &roots {
                        commands.entity(entity).try_despawn_recursive();
                    }
                    spawn_title_tree(
                        &mut commands,
                        &ctx,
                        &fjord_art,
                        &vfx,
                        sound_bank.muted,
                        *safe_area,
                        fs_now,
                        hud_mode_now,
                    );
                }
                return;
            }
            TitleAction::ToggleLanguage => {
                ctx.confirm_reset = false;
                ctx.data.i18n.toggle_language();
                ctx.persist();
                for entity in &roots {
                    commands.entity(entity).try_despawn_recursive();
                }
                spawn_title_tree(
                    &mut commands,
                    &ctx,
                    &fjord_art,
                    &vfx,
                    sound_bank.muted,
                    *safe_area,
                    fs_now,
                    hud_mode_now,
                );
                return;
            }
            TitleAction::ToggleAudio => {
                ctx.confirm_reset = false;
                // The title rebuilds on the next frame through `sound_bank` change detection.
                toggle_mute(&mut ctx, &mut sound_bank, &mut bgm_sinks);
                return;
            }
            #[cfg(not(target_os = "android"))]
            TitleAction::ToggleFullscreen => {
                ctx.confirm_reset = false;
                #[cfg(target_arch = "wasm32")]
                {
                    crate::platform::toggle_web_fullscreen();
                    let next_fs = crate::platform::is_web_fullscreen();
                    *prev_fullscreen = Some(next_fs);
                    for entity in &roots {
                        commands.entity(entity).try_despawn_recursive();
                    }
                    spawn_title_tree(
                        &mut commands,
                        &ctx,
                        &fjord_art,
                        &vfx,
                        sound_bank.muted,
                        *safe_area,
                        next_fs,
                        hud_mode_now,
                    );
                }
                #[cfg(not(target_arch = "wasm32"))]
                if let Ok(mut win) = windows.get_single_mut() {
                    let next_mode = match win.mode {
                        WindowMode::Windowed => {
                            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
                        }
                        _ => WindowMode::Windowed,
                    };
                    let next_fs = !matches!(next_mode, WindowMode::Windowed);
                    win.mode = next_mode;
                    *prev_fullscreen = Some(next_fs);
                    for entity in &roots {
                        commands.entity(entity).try_despawn_recursive();
                    }
                    spawn_title_tree(
                        &mut commands,
                        &ctx,
                        &fjord_art,
                        &vfx,
                        sound_bank.muted,
                        *safe_area,
                        next_fs,
                        hud_mode_now,
                    );
                }
                return;
            }
            #[cfg(target_arch = "wasm32")]
            TitleAction::EnterFullscreen(ori) => {
                ctx.confirm_reset = false;
                crate::platform::enter_web_fullscreen(ori);
                let next_fs = crate::platform::is_web_fullscreen();
                *prev_fullscreen = Some(next_fs);
                for entity in &roots {
                    commands.entity(entity).try_despawn_recursive();
                }
                spawn_title_tree(
                    &mut commands,
                    &ctx,
                    &fjord_art,
                    &vfx,
                    sound_bank.muted,
                    *safe_area,
                    next_fs,
                    hud_mode_now,
                );
                return;
            }
        }
    }

    for (_, interaction, action, mut bg, img_opt, children_opt) in &mut buttons {
        match *interaction {
            Interaction::Pressed => {}
            Interaction::Hovered => match action {
                TitleAction::EnterFjord => {
                    if let Some(mut img) = img_opt {
                        img.color = Color::srgb(1.0, 0.96, 0.86);
                    }
                }
                TitleAction::NewSaga => {
                    *bg = BackgroundColor(if ctx.confirm_reset {
                        COLOR_ACCENT_CRIMSON_HOVER
                    } else {
                        Color::srgba(0.42, 0.16, 0.20, 0.96)
                    });
                }
                TitleAction::ToggleLanguage | TitleAction::ToggleAudio => {
                    *bg = BackgroundColor(COLOR_ACCENT_TEAL);
                    if let Some(children) = children_opt {
                        for &child in children {
                            if let Ok(mut tc) = text_colors.get_mut(child) {
                                tc.0 = COLOR_TEXT_ON_ACCENT;
                            }
                        }
                    }
                }
                #[cfg(not(target_os = "android"))]
                TitleAction::ToggleFullscreen => {
                    *bg = BackgroundColor(COLOR_ACCENT_TEAL);
                    if let Some(children) = children_opt {
                        for &child in children {
                            if let Ok(mut tc) = text_colors.get_mut(child) {
                                tc.0 = COLOR_TEXT_ON_ACCENT;
                            }
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                TitleAction::EnterFullscreen(_) => {
                    *bg = BackgroundColor(COLOR_ACCENT_TEAL);
                    if let Some(children) = children_opt {
                        for &child in children {
                            if let Ok(mut tc) = text_colors.get_mut(child) {
                                tc.0 = COLOR_TEXT_ON_ACCENT;
                            }
                        }
                    }
                }
            },
            Interaction::None => match action {
                TitleAction::EnterFjord => {
                    *bg = BackgroundColor(Color::NONE);
                    if let Some(mut img) = img_opt {
                        img.color = Color::WHITE;
                    }
                }
                TitleAction::NewSaga => {
                    *bg = BackgroundColor(if ctx.confirm_reset {
                        COLOR_ACCENT_CRIMSON
                    } else {
                        Color::srgba(0.24, 0.12, 0.14, 0.92)
                    });
                }
                TitleAction::ToggleLanguage | TitleAction::ToggleAudio => {
                    *bg = BackgroundColor(Color::srgba(0.10, 0.14, 0.22, 0.90));
                    if let Some(children) = children_opt {
                        for &child in children {
                            if let Ok(mut tc) = text_colors.get_mut(child) {
                                tc.0 = COLOR_TEXT_PRIMARY;
                            }
                        }
                    }
                }
                #[cfg(not(target_os = "android"))]
                TitleAction::ToggleFullscreen => {
                    let idle_bg = if fs_now {
                        COLOR_ACCENT_TEAL
                    } else {
                        Color::srgba(0.10, 0.14, 0.22, 0.90)
                    };
                    *bg = BackgroundColor(idle_bg);
                    if let Some(children) = children_opt {
                        for &child in children {
                            if let Ok(mut tc) = text_colors.get_mut(child) {
                                tc.0 = title_util_text_color(idle_bg);
                            }
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                TitleAction::EnterFullscreen(_) => {
                    *bg = BackgroundColor(Color::srgba(0.10, 0.14, 0.22, 0.90));
                    if let Some(children) = children_opt {
                        for &child in children {
                            if let Ok(mut tc) = text_colors.get_mut(child) {
                                tc.0 = COLOR_TEXT_PRIMARY;
                            }
                        }
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_utility_button_text_color_ensures_wcag_contrast_on_teal() {
        assert_eq!(
            title_util_text_color(COLOR_ACCENT_TEAL),
            COLOR_TEXT_ON_ACCENT
        );
        assert_eq!(
            title_util_text_color(Color::srgba(0.10, 0.14, 0.22, 0.90)),
            COLOR_TEXT_PRIMARY
        );
    }
}
