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

//! Victory and defeat modal screen after completing a Match-3 level.

use crate::state::{AppState, GameContext};
use crate::theme::*;
use bevy::prelude::*;
use wyrdstone_core::LevelOutcome;

#[derive(Component)]
pub struct ResultRoot;

#[derive(Component)]
pub enum ResultAction {
    ReturnToFjord,
    Replay,
}

/// Spawns the Victory / Defeat summary card.
pub fn setup_result_ui(mut commands: Commands, ctx: Res<GameContext>) {
    let i18n = &ctx.data.i18n;
    let is_victory = ctx.last_outcome == Some(LevelOutcome::Victory);

    let title = if is_victory {
        i18n.t("result.victory.title")
    } else {
        i18n.t("result.defeat.title")
    };
    let subtitle = if is_victory {
        i18n.t("result.victory.subtitle")
    } else {
        i18n.t("result.defeat.subtitle")
    };
    let accent = if is_victory {
        COLOR_ACCENT_GOLD
    } else {
        COLOR_ACCENT_CRIMSON
    };

    commands
        .spawn((
            ResultRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(COLOR_BG_DEEP),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(460.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(28.0)),
                    row_gap: Val::Px(14.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_DARK),
                BorderColor(accent),
                BorderRadius::all(Val::Px(14.0)),
            ))
            .with_children(|card| {
                card.spawn((
                    Text::new(title),
                    TextFont {
                        font_size: FONT_TITLE_MD + 10.0,
                        ..default()
                    },
                    TextColor(accent),
                ));

                card.spawn((
                    Text::new(subtitle),
                    TextFont {
                        font_size: FONT_BODY,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_PRIMARY),
                ));

                card.spawn((
                    Text::new(i18n.t_fmt(
                        "match3.hud.score",
                        &[("score", ctx.last_score.to_string())],
                    )),
                    TextFont {
                        font_size: FONT_HEADING,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_PRIMARY),
                ));

                if is_victory {
                    let r = ctx.last_rewards;
                    card.spawn((
                        Text::new(i18n.t("result.rewards.title")),
                        TextFont {
                            font_size: FONT_BODY,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                    card.spawn((
                        Text::new(i18n.t_fmt(
                            "result.rewards.line",
                            &[
                                ("wood", r.wood.to_string()),
                                ("stone", r.stone.to_string()),
                                ("gold", r.gold.to_string()),
                                ("mead", r.mead.to_string()),
                            ],
                        )),
                        TextFont {
                            font_size: FONT_BODY,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_PRIMARY),
                    ));
                }

                card.spawn(Node {
                    column_gap: Val::Px(14.0),
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                })
                .with_children(|row| {
                    spawn_result_btn(
                        row,
                        i18n.t("result.button.fjord"),
                        ResultAction::ReturnToFjord,
                        COLOR_ACCENT_GOLD,
                        COLOR_TEXT_DARK,
                    );
                    spawn_result_btn(
                        row,
                        i18n.t("result.button.replay"),
                        ResultAction::Replay,
                        COLOR_ACCENT_TEAL,
                        COLOR_TEXT_PRIMARY,
                    );
                });
            });
        });
}

fn spawn_result_btn(
    parent: &mut ChildBuilder,
    label: &str,
    action: ResultAction,
    bg: Color,
    text_color: Color,
) {
    parent
        .spawn((
            Button,
            action,
            Node {
                padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(bg),
            BorderRadius::all(Val::Px(8.0)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: FONT_BODY,
                    ..default()
                },
                TextColor(text_color),
            ));
        });
}

/// Cleans up result UI entities.
pub fn cleanup_result_ui(mut commands: Commands, roots: Query<Entity, With<ResultRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

/// Handles Return to Fjord and Play Again buttons.
pub fn handle_result_buttons(
    mut ctx: ResMut<GameContext>,
    mut next_state: ResMut<NextState<AppState>>,
    buttons: Query<(&Interaction, &ResultAction), Changed<Interaction>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            ResultAction::ReturnToFjord => {
                next_state.set(AppState::FjordCity);
                return;
            }
            ResultAction::Replay => {
                if let Some(level_id) = ctx.active_level_id.clone() {
                    if ctx.start_level(&level_id).is_ok() {
                        next_state.set(AppState::Match3);
                        return;
                    }
                }
                next_state.set(AppState::FjordCity);
                return;
            }
        }
    }
}
