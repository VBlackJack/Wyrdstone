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

//! Full-bleed painted Title screen with language switch, audio toggle, and play entry.

use crate::audio::{toggle_mute, BgmTrack, SoundBank};
use crate::fjord_art::FjordTextures;
use crate::state::{AppState, GameContext};
use crate::theme::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct TitleRoot;

#[derive(Component)]
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
    sound_bank: Res<SoundBank>,
) {
    spawn_title_tree(&mut commands, &ctx, &fjord_art, sound_bank.muted);
}

/// Cleans up Title screen entities on state exit.
pub fn cleanup_title_ui(mut commands: Commands, roots: Query<Entity, With<TitleRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_title_tree(
    commands: &mut Commands,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
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
                ..default()
            },
        ))
        .with_children(|bg| {
            // Subtle dark vignette overlay for crisp typography contrast
            bg.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(22.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.07, 0.12, 0.56)),
            ))
            .with_children(|parent| {
                // Translucent Nordic emblem card
                parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(10.0),
                            padding: UiRect::axes(Val::Px(54.0), Val::Px(32.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.07, 0.10, 0.16, 0.86)),
                        BorderColor(COLOR_ACCENT_GOLD),
                        BorderRadius::all(Val::Px(16.0)),
                    ))
                    .with_children(|card| {
                        card.spawn((
                            Text::new(i18n.t("app.title.name")),
                            TextFont {
                                font_size: FONT_TITLE_LG,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));

                        card.spawn((
                            Text::new(i18n.t("app.title.subtitle")),
                            TextFont {
                                font_size: FONT_TITLE_MD,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_PRIMARY),
                        ));

                        card.spawn((
                            Text::new(i18n.t("app.title.footer")),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                        ));
                    });

                // Primary Play button
                parent
                    .spawn((
                        Button,
                        TitleAction::EnterFjord,
                        Node {
                            width: Val::Px(300.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(COLOR_ACCENT_GOLD),
                        BorderColor(COLOR_ACCENT_GOLD_HOVER),
                        BorderRadius::all(Val::Px(12.0)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(play_label),
                            TextFont {
                                font_size: FONT_HEADING,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_DARK),
                        ));
                    });

                // Utility buttons row (Language + Audio + Optional New Saga)
                parent
                    .spawn(Node {
                        column_gap: Val::Px(12.0),
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        let mut util_btns: Vec<(&str, TitleAction, Color)> = vec![
                            (
                                i18n.t("app.title.button.language"),
                                TitleAction::ToggleLanguage,
                                Color::srgba(0.10, 0.14, 0.21, 0.90),
                            ),
                            (
                                sound_label,
                                TitleAction::ToggleAudio,
                                Color::srgba(0.10, 0.14, 0.21, 0.90),
                            ),
                        ];
                        if ctx.has_existing_save {
                            let (lbl, col) = if ctx.confirm_reset {
                                (i18n.t("app.title.button.confirm_reset"), COLOR_ACCENT_CRIMSON)
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
                                    width: Val::Px(176.0),
                                    height: Val::Px(42.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.5)),
                                    ..default()
                                },
                                BackgroundColor(bg_col),
                                BorderColor(COLOR_PANEL_BORDER),
                                BorderRadius::all(Val::Px(8.0)),
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
        });
}

/// Handles clicks on the Title screen buttons.
#[allow(clippy::too_many_arguments)]
pub fn handle_title_buttons(
    mut commands: Commands,
    mut ctx: ResMut<GameContext>,
    fjord_art: Res<FjordTextures>,
    mut sound_bank: ResMut<SoundBank>,
    mut bgm_sinks: Query<&mut AudioSink, With<BgmTrack>>,
    mut next_state: ResMut<NextState<AppState>>,
    roots: Query<Entity, With<TitleRoot>>,
    mut buttons: Query<(&Interaction, &TitleAction, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, action, mut bg) in &mut buttons {
        match *interaction {
            Interaction::Pressed => match action {
                TitleAction::EnterFjord => {
                    ctx.confirm_reset = false;
                    next_state.set(AppState::FjordCity);
                }
                TitleAction::NewSaga => {
                    if ctx.confirm_reset {
                        ctx.reset_saga();
                        next_state.set(AppState::FjordCity);
                    } else {
                        ctx.confirm_reset = true;
                        for entity in &roots {
                            commands.entity(entity).despawn_recursive();
                        }
                        spawn_title_tree(&mut commands, &ctx, &fjord_art, sound_bank.muted);
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
                    spawn_title_tree(&mut commands, &ctx, &fjord_art, sound_bank.muted);
                    return;
                }
                TitleAction::ToggleAudio => {
                    ctx.confirm_reset = false;
                    toggle_mute(&mut sound_bank, &mut bgm_sinks);
                    for entity in &roots {
                        commands.entity(entity).despawn_recursive();
                    }
                    spawn_title_tree(&mut commands, &ctx, &fjord_art, sound_bank.muted);
                    return;
                }
            },
            Interaction::Hovered => match action {
                TitleAction::EnterFjord => *bg = BackgroundColor(COLOR_ACCENT_GOLD_HOVER),
                TitleAction::NewSaga => *bg = BackgroundColor(COLOR_ACCENT_CRIMSON),
                TitleAction::ToggleLanguage | TitleAction::ToggleAudio => {
                    *bg = BackgroundColor(COLOR_ACCENT_TEAL)
                }
            },
            Interaction::None => match action {
                TitleAction::EnterFjord => *bg = BackgroundColor(COLOR_ACCENT_GOLD),
                TitleAction::NewSaga => {
                    *bg = BackgroundColor(if ctx.confirm_reset {
                        COLOR_ACCENT_CRIMSON
                    } else {
                        Color::srgba(0.24, 0.12, 0.14, 0.92)
                    })
                }
                TitleAction::ToggleLanguage | TitleAction::ToggleAudio => {
                    *bg = BackgroundColor(Color::srgba(0.10, 0.14, 0.21, 0.90))
                }
            },
        }
    }
}
