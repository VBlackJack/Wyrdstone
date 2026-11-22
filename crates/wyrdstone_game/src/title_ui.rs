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

//! Title screen UI with language switch and play/continue entry.

use crate::state::{AppState, GameContext};
use crate::theme::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct TitleRoot;

#[derive(Component)]
pub enum TitleAction {
    EnterFjord,
    ToggleLanguage,
}

/// Spawns the Title screen hierarchy.
pub fn setup_title_ui(mut commands: Commands, ctx: Res<GameContext>) {
    spawn_title_tree(&mut commands, &ctx);
}

/// Cleans up Title screen entities on state exit.
pub fn cleanup_title_ui(mut commands: Commands, roots: Query<Entity, With<TitleRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_title_tree(commands: &mut Commands, ctx: &GameContext) {
    let i18n = &ctx.data.i18n;
    let play_label = if ctx.has_existing_save {
        i18n.t("app.title.button.continue")
    } else {
        i18n.t("app.title.button.play")
    };

    commands
        .spawn((
            TitleRoot,
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
            BackgroundColor(COLOR_BG_DEEP),
        ))
        .with_children(|parent| {
            // Emblem card
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(12.0),
                        padding: UiRect::axes(Val::Px(48.0), Val::Px(36.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(COLOR_PANEL_DARK),
                    BorderColor(COLOR_ACCENT_GOLD),
                    BorderRadius::all(Val::Px(14.0)),
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
                        width: Val::Px(280.0),
                        height: Val::Px(58.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(COLOR_ACCENT_GOLD),
                    BorderColor(COLOR_ACCENT_GOLD_HOVER),
                    BorderRadius::all(Val::Px(10.0)),
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

            // Language toggle button
            parent
                .spawn((
                    Button,
                    TitleAction::ToggleLanguage,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(44.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    BackgroundColor(COLOR_PANEL_CARD),
                    BorderColor(COLOR_PANEL_BORDER),
                    BorderRadius::all(Val::Px(8.0)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(i18n.t("app.title.button.language")),
                        TextFont {
                            font_size: FONT_BODY,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_PRIMARY),
                    ));
                });
        });
}

/// Handles clicks on the Title screen buttons.
pub fn handle_title_buttons(
    mut commands: Commands,
    mut ctx: ResMut<GameContext>,
    mut next_state: ResMut<NextState<AppState>>,
    roots: Query<Entity, With<TitleRoot>>,
    mut buttons: Query<(&Interaction, &TitleAction, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, action, mut bg) in &mut buttons {
        match *interaction {
            Interaction::Pressed => match action {
                TitleAction::EnterFjord => {
                    next_state.set(AppState::FjordCity);
                }
                TitleAction::ToggleLanguage => {
                    ctx.data.i18n.toggle_language();
                    ctx.persist();
                    for entity in &roots {
                        commands.entity(entity).despawn_recursive();
                    }
                    spawn_title_tree(&mut commands, &ctx);
                    return;
                }
            },
            Interaction::Hovered => match action {
                TitleAction::EnterFjord => *bg = BackgroundColor(COLOR_ACCENT_GOLD_HOVER),
                TitleAction::ToggleLanguage => *bg = BackgroundColor(COLOR_ACCENT_TEAL),
            },
            Interaction::None => match action {
                TitleAction::EnterFjord => *bg = BackgroundColor(COLOR_ACCENT_GOLD),
                TitleAction::ToggleLanguage => *bg = BackgroundColor(COLOR_PANEL_CARD),
            },
        }
    }
}
