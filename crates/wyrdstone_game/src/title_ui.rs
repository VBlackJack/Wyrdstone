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
use crate::scroll::{PressArm, UiScrollMemory};
use crate::state::{AppState, GameContext};
use crate::theme::*;
use crate::ui_style::{
    button_shadow, display_font, gold_glow_shadow, panel_shadow, sliced_gold_button_image,
    sliced_panel_image, spawn_notice_toast, ToastLife, NOTICE_TOAST_DURATION_SECS,
};
use crate::vfx_art::VfxTextures;
use bevy::prelude::*;

const TITLE_SNOWFLAKE_COUNT: usize = 26;

#[derive(Component)]
pub struct TitleRoot;

#[derive(Component)]
pub struct TitleLogoFloat;

#[derive(Component)]
pub struct TitleAurora {
    pub base_left_pct: f32,
    pub phase: f32,
    pub base_alpha: f32,
}

#[derive(Component)]
pub struct TitleSnowflake {
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
}

/// Spawns the Title screen hierarchy.
pub fn setup_title_ui(
    mut commands: Commands,
    ctx: Res<GameContext>,
    fjord_art: Res<FjordTextures>,
    vfx: Res<VfxTextures>,
    sound_bank: Res<SoundBank>,
) {
    spawn_title_tree(&mut commands, &ctx, &fjord_art, &vfx, sound_bank.muted);
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
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_title_tree(
    commands: &mut Commands,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    muted: bool,
) {
    let i18n = &ctx.data.i18n;
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
                        base_left_pct: left,
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
                    row_gap: Val::Px(20.0),
                    padding: UiRect::all(Val::Px(24.0)),
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
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            margin: UiRect::bottom(Val::Px(2.0)),
                            ..default()
                        },
                    ))
                    .with_children(|hero| {
                        hero.spawn((
                            ImageNode::new(fjord_art.logo.clone()),
                            Node {
                                width: Val::Px(560.0),
                                max_width: Val::Percent(100.0),
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
                            max_width: Val::Percent(90.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(6.0),
                            padding: UiRect::axes(Val::Px(32.0), Val::Px(16.0)),
                            ..default()
                        },
                        panel_shadow(),
                    ))
                    .with_children(|plaque| {
                        plaque.spawn((
                            Text::new(i18n.t("app.title.subtitle")),
                            display_font(fjord_art, FONT_HEADING + 2.0),
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                        plaque.spawn((
                            Text::new(i18n.t("app.title.footer")),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                        ));
                    });

                // Primary sculpted forged-gold Play button (9-slice).
                parent
                    .spawn((
                        Button,
                        TitleAction::EnterFjord,
                        sliced_gold_button_image(vfx),
                        Node {
                            width: Val::Px(320.0),
                            height: Val::Px(64.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        gold_glow_shadow(),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(play_label),
                            display_font(fjord_art, FONT_HEADING + 1.0),
                            TextColor(COLOR_TEXT_DARK),
                        ));
                    });

                // Utility buttons row (Language + Audio + Optional New Saga)
                parent
                    .spawn(Node {
                        column_gap: Val::Px(12.0),
                        row_gap: Val::Px(8.0),
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
                        for (label, act, bg_col) in util_btns {
                            row.spawn((
                                Button,
                                act,
                                Node {
                                    width: Val::Px(178.0),
                                    height: Val::Px(MIN_TOUCH_TARGET_PX),
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
                                        font_size: FONT_BODY,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                            });
                        }
                    });
            });

            // Save and recovery notices (restored from backup, read-only, unreadable save).
            if let Some(notice_key) = &ctx.status_notice_key {
                let life = (!ctx.save_read_only).then_some(NOTICE_TOAST_DURATION_SECS);
                spawn_notice_toast(bg, i18n.t(notice_key), life);
            }
        });
}

/// Animates the floating WYRDSTONE emblem, aurora ribbons, and snowfall on the Title screen.
#[allow(clippy::type_complexity)]
pub fn animate_title_ambience(
    time: Res<Time>,
    ctx: Res<GameContext>,
    mut logos: Query<
        &mut Node,
        (
            With<TitleLogoFloat>,
            Without<TitleAurora>,
            Without<TitleSnowflake>,
        ),
    >,
    mut auroras: Query<
        (&TitleAurora, &mut Node, &mut ImageNode),
        (Without<TitleLogoFloat>, Without<TitleSnowflake>),
    >,
    mut flakes: Query<
        (&mut TitleSnowflake, &mut Node),
        (Without<TitleLogoFloat>, Without<TitleAurora>),
    >,
) {
    if ctx.reduced_motion {
        return;
    }
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    let float_px = (elapsed * 1.6).sin() * 5.0;
    for mut node in &mut logos {
        node.margin.top = Val::Px(float_px);
        node.margin.bottom = Val::Px(2.0 - float_px);
    }

    for (aurora, mut node, mut img) in &mut auroras {
        let drift = (elapsed * 0.32 + aurora.phase).sin() * 4.5;
        node.left = Val::Percent(aurora.base_left_pct + drift);
        let shimmer = 0.72 + 0.28 * (elapsed * 0.78 + aurora.phase * 1.4).cos();
        img.color = img.color.with_alpha(aurora.base_alpha * shimmer);
    }

    for (mut flake, mut node) in &mut flakes {
        flake.y_pct += flake.speed_y * dt;
        if flake.y_pct > 102.0 {
            flake.y_pct = -3.0;
        }
        let sway = (elapsed * 1.1 + flake.phase).sin() * 1.4;
        node.left = Val::Percent((flake.x_pct + sway).clamp(0.0, 99.0));
        node.top = Val::Percent(flake.y_pct);
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
    ),
    Changed<Interaction>,
>;

/// Handles clicks and keyboard shortcuts (`Enter`/`Space`) on the Title screen.
#[allow(clippy::too_many_arguments)]
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
    touches: Res<Touches>,
    scroll_mem: Res<UiScrollMemory>,
    mut press_arm: Local<PressArm>,
    mut buttons: TitleButtonQuery,
) {
    for (entity, mut life) in &mut toasts {
        life.0 -= time.delta_secs();
        if life.0 <= 0.0 {
            commands.entity(entity).despawn_recursive();
            if !ctx.save_read_only {
                ctx.status_notice_key = None;
            }
        }
    }

    if keys.just_pressed(KeyCode::Escape) && ctx.confirm_reset {
        ctx.confirm_reset = false;
        for entity in &roots {
            commands.entity(entity).despawn_recursive();
        }
        spawn_title_tree(&mut commands, &ctx, &fjord_art, &vfx, sound_bank.muted);
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        ctx.confirm_reset = false;
        ctx.has_existing_save = true;
        next_state.set(AppState::FjordCity);
        return;
    }

    if sound_bank.is_changed() && !sound_bank.is_added() {
        for entity in &roots {
            commands.entity(entity).despawn_recursive();
        }
        spawn_title_tree(&mut commands, &ctx, &fjord_art, &vfx, sound_bank.muted);
        return;
    }

    // Actions fire on release so a touch that starts a scroll never triggers a button.
    let tapped = press_arm.completed_taps(
        buttons.iter().map(|(entity, i, ..)| (entity, *i)),
        &touches,
        &scroll_mem,
    );
    let tapped_actions: Vec<TitleAction> = buttons
        .iter()
        .filter(|(entity, ..)| tapped.contains(entity))
        .map(|(_, _, action, ..)| *action)
        .collect();
    for action in tapped_actions {
        match action {
            TitleAction::EnterFjord => {
                ctx.confirm_reset = false;
                ctx.has_existing_save = true;
                next_state.set(AppState::FjordCity);
            }
            TitleAction::NewSaga => {
                if ctx.confirm_reset {
                    if ctx.reset_saga() {
                        next_state.set(AppState::FjordCity);
                    } else {
                        // The newer save could not be preserved: stay here, notice is set.
                        ctx.confirm_reset = false;
                        for entity in &roots {
                            commands.entity(entity).despawn_recursive();
                        }
                        spawn_title_tree(&mut commands, &ctx, &fjord_art, &vfx, sound_bank.muted);
                    }
                } else {
                    ctx.confirm_reset = true;
                    for entity in &roots {
                        commands.entity(entity).despawn_recursive();
                    }
                    spawn_title_tree(&mut commands, &ctx, &fjord_art, &vfx, sound_bank.muted);
                }
                return;
            }
            TitleAction::ToggleLanguage => {
                ctx.confirm_reset = false;
                ctx.data.i18n.toggle_language();
                ctx.persist();
                for entity in &roots {
                    commands.entity(entity).despawn_recursive();
                }
                spawn_title_tree(&mut commands, &ctx, &fjord_art, &vfx, sound_bank.muted);
                return;
            }
            TitleAction::ToggleAudio => {
                ctx.confirm_reset = false;
                // The title rebuilds on the next frame through `sound_bank` change detection.
                toggle_mute(&mut ctx, &mut sound_bank, &mut bgm_sinks);
                return;
            }
        }
    }

    for (_, interaction, action, mut bg, img_opt) in &mut buttons {
        match *interaction {
            Interaction::Pressed => {}
            Interaction::Hovered => match action {
                TitleAction::EnterFjord => {
                    if let Some(mut img) = img_opt {
                        img.color = Color::srgb(1.08, 1.04, 0.94);
                    }
                }
                TitleAction::NewSaga => {
                    *bg = BackgroundColor(if ctx.confirm_reset {
                        Color::srgb(0.88, 0.28, 0.30)
                    } else {
                        Color::srgba(0.42, 0.16, 0.20, 0.96)
                    })
                }
                TitleAction::ToggleLanguage | TitleAction::ToggleAudio => {
                    *bg = BackgroundColor(COLOR_ACCENT_TEAL)
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
                    })
                }
                TitleAction::ToggleLanguage | TitleAction::ToggleAudio => {
                    *bg = BackgroundColor(Color::srgba(0.10, 0.14, 0.22, 0.90))
                }
            },
        }
    }
}
