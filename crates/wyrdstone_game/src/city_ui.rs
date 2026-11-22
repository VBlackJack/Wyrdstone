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

//! Fjord village hub UI: resource bar, Saga quests, and building cards.

use crate::state::{AppState, GameContext};
use crate::theme::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct CityRoot;

#[derive(Component)]
pub enum CityAction {
    PlayBuildingLevel(String),
    UpgradeBuilding(String),
    ClaimQuest(String),
    CraftOrEquipAmulet(String),
    ToggleLanguage,
    BackToTitle,
}

/// Spawns the Fjord City hub UI.
pub fn setup_city_ui(mut commands: Commands, ctx: Res<GameContext>) {
    spawn_city_tree(&mut commands, &ctx);
}

/// Cleans up the Fjord City UI on state exit.
pub fn cleanup_city_ui(mut commands: Commands, roots: Query<Entity, With<CityRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_city_tree(commands: &mut Commands, ctx: &GameContext) {
    let i18n = &ctx.data.i18n;
    let res = ctx.city.resources;

    commands
        .spawn((
            CityRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(COLOR_BG_DEEP),
        ))
        .with_children(|root| {
            // Top bar: title, resources, language, back
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    column_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_DARK),
                BorderColor(COLOR_PANEL_BORDER),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|bar| {
                // Title + Clan Glory
                bar.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        Text::new(i18n.t("city.header.title")),
                        TextFont {
                            font_size: FONT_HEADING,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                    col.spawn((
                        Text::new(i18n.t_fmt(
                            "city.header.mastery",
                            &[("glory", ctx.city.glory.to_string())],
                        )),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_MUTED),
                    ));
                });

                // Resources strip
                let resource_line = format!(
                    "{}: {}   {}: {}   {}: {}   {}: {}   {}: {}/{}",
                    i18n.t("resources.label.wood"),
                    res.wood,
                    i18n.t("resources.label.stone"),
                    res.stone,
                    i18n.t("resources.label.gold"),
                    res.gold,
                    i18n.t("resources.label.mead"),
                    res.mead,
                    i18n.t("resources.label.energy"),
                    ctx.city.energy,
                    ctx.data.balance.max_energy,
                );
                bar.spawn((
                    Text::new(resource_line),
                    TextFont {
                        font_size: FONT_BODY,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_PRIMARY),
                ));

                // Utility buttons
                bar.spawn(Node {
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|btns| {
                    spawn_small_button(
                        btns,
                        i18n.t("app.title.button.language"),
                        CityAction::ToggleLanguage,
                        COLOR_PANEL_CARD,
                    );
                    spawn_small_button(
                        btns,
                        i18n.t("city.button.title"),
                        CityAction::BackToTitle,
                        COLOR_PANEL_CARD,
                    );
                });
            });

            // Active Saga Quest banner
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                    column_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_CARD),
                BorderColor(COLOR_ACCENT_GOLD),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|qbar| {
                if let Some(quest) = ctx.city.active_quest(&ctx.data.quests) {
                    let ready = ctx.city.is_quest_ready(quest);
                    qbar.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        ..default()
                    })
                    .with_children(|info| {
                        info.spawn((
                            Text::new(format!(
                                "{} - {}",
                                i18n.t("city.panel.quests.title"),
                                i18n.t(&quest.title_key)
                            )),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                        info.spawn((
                            Text::new(i18n.t(&quest.desc_key)),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_PRIMARY),
                        ));
                    });

                    if ready {
                        spawn_small_button(
                            qbar,
                            i18n.t("city.panel.quests.complete"),
                            CityAction::ClaimQuest(quest.id.clone()),
                            COLOR_ACCENT_GREEN,
                        );
                    }
                } else {
                    qbar.spawn((
                        Text::new(i18n.t("city.panel.quests.done")),
                        TextFont {
                            font_size: FONT_BODY,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                }
            });

            // Nordic Amulets strip
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                    column_gap: Val::Px(12.0),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_DARK),
                BorderColor(COLOR_PANEL_BORDER),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|abar| {
                abar.spawn((
                    Text::new(i18n.t("city.panel.amulets.title")),
                    TextFont {
                        font_size: FONT_BODY,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_GOLD),
                ));

                for amulet in &ctx.data.amulets {
                    let crafted = ctx.city.crafted_amulets.contains(&amulet.id);
                    let equipped = ctx.city.equipped_amulet.as_deref() == Some(amulet.id.as_str());
                    let can_afford = ctx.city.resources.can_afford(&amulet.cost);
                    let state_label = if equipped {
                        i18n.t("city.panel.amulets.equipped")
                    } else if crafted {
                        i18n.t("city.panel.amulets.equip")
                    } else {
                        i18n.t("city.panel.amulets.craft")
                    };
                    let btn_bg = if equipped {
                        COLOR_ACCENT_GOLD
                    } else if crafted {
                        COLOR_ACCENT_TEAL
                    } else if can_afford {
                        COLOR_ACCENT_GREEN
                    } else {
                        COLOR_BUTTON_DISABLED
                    };
                    let label = format!("{} [{}]", i18n.t(&amulet.name_key), state_label);
                    spawn_small_button(
                        abar,
                        &label,
                        CityAction::CraftOrEquipAmulet(amulet.id.clone()),
                        btn_bg,
                    );
                }
            });

            // Optional status notice (e.g. not enough resources / endurance)
            if let Some(notice_key) = &ctx.status_notice_key {
                root.spawn((
                    Text::new(i18n.t(notice_key)),
                    TextFont {
                        font_size: FONT_BODY,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_CRIMSON),
                ));
            }

            // Buildings grid (responsive wrap)
            root.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_wrap: FlexWrap::Wrap,
                align_content: AlignContent::FlexStart,
                row_gap: Val::Px(14.0),
                column_gap: Val::Px(14.0),
                ..default()
            })
            .with_children(|grid| {
                for b in &ctx.data.buildings {
                    let tier = ctx.city.building_tier(&b.id);
                    let stars = ctx.city.building_mastery_stars(&b.id);
                    let unlocked = ctx.city.is_building_unlocked(b);
                    let next_cost = ctx.city.next_upgrade_cost(b);
                    let can_afford = next_cost
                        .map(|c| ctx.city.resources.can_afford(c))
                        .unwrap_or(false);

                    grid.spawn((
                        Node {
                            width: Val::Px(330.0),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::SpaceBetween,
                            padding: UiRect::all(Val::Px(14.0)),
                            row_gap: Val::Px(8.0),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(if unlocked {
                            COLOR_PANEL_BORDER
                        } else {
                            COLOR_BUTTON_DISABLED
                        }),
                        BorderRadius::all(Val::Px(10.0)),
                    ))
                    .with_children(|card| {
                        // Name + Tier
                        card.spawn((
                            Text::new(i18n.t(&b.name_key)),
                            TextFont {
                                font_size: FONT_HEADING,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));

                        let meta = format!(
                            "{}   |   {}",
                            i18n.t_fmt(
                                "city.panel.building.level",
                                &[
                                    ("level", tier.to_string()),
                                    ("max", b.max_tier.to_string()),
                                ],
                            ),
                            i18n.t_fmt(
                                "city.panel.building.mastery",
                                &[("stars", stars.to_string())],
                            ),
                        );
                        card.spawn((
                            Text::new(meta),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                        ));

                        card.spawn((
                            Text::new(i18n.t(&b.desc_key)),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_PRIMARY),
                        ));

                        // Cost / lock line
                        let cost_text = if !unlocked {
                            i18n.t("city.panel.building.locked").to_string()
                        } else if let Some(c) = next_cost {
                            i18n.t_fmt(
                                "city.panel.building.cost",
                                &[
                                    ("wood", c.wood.to_string()),
                                    ("stone", c.stone.to_string()),
                                    ("gold", c.gold.to_string()),
                                ],
                            )
                        } else {
                            i18n.t("city.button.max_level").to_string()
                        };
                        card.spawn((
                            Text::new(cost_text),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(if unlocked && can_afford {
                                COLOR_ACCENT_GOLD
                            } else {
                                COLOR_TEXT_MUTED
                            }),
                        ));

                        // Action buttons row
                        card.spawn(Node {
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(8.0),
                            row_gap: Val::Px(6.0),
                            margin: UiRect::top(Val::Px(4.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            spawn_small_button(
                                row,
                                i18n.t("city.button.play_level"),
                                CityAction::PlayBuildingLevel(b.level_id.clone()),
                                if unlocked {
                                    COLOR_ACCENT_TEAL
                                } else {
                                    COLOR_BUTTON_DISABLED
                                },
                            );

                            if unlocked && stars >= 1 {
                                if let Some(mastery_id) = &b.mastery_level_id {
                                    spawn_small_button(
                                        row,
                                        i18n.t("city.button.play_mastery"),
                                        CityAction::PlayBuildingLevel(mastery_id.clone()),
                                        COLOR_ACCENT_GOLD,
                                    );
                                }
                            }

                            let upgrade_color = if unlocked && next_cost.is_some() && can_afford {
                                COLOR_ACCENT_GREEN
                            } else {
                                COLOR_BUTTON_DISABLED
                            };
                            spawn_small_button(
                                row,
                                if next_cost.is_some() {
                                    i18n.t("city.button.upgrade")
                                } else {
                                    i18n.t("city.button.max_level")
                                },
                                CityAction::UpgradeBuilding(b.id.clone()),
                                upgrade_color,
                            );
                        });
                    });
                }
            });
        });
}

fn spawn_small_button(
    parent: &mut ChildBuilder,
    label: &str,
    action: CityAction,
    bg_color: Color,
) {
    parent
        .spawn((
            Button,
            action,
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            BorderColor(COLOR_PANEL_BORDER),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_TEXT_PRIMARY),
            ));
        });
}

/// Processes button presses in the Fjord City hub.
pub fn handle_city_buttons(
    mut commands: Commands,
    mut ctx: ResMut<GameContext>,
    mut next_state: ResMut<NextState<AppState>>,
    roots: Query<Entity, With<CityRoot>>,
    buttons: Query<(&Interaction, &CityAction), Changed<Interaction>>,
) {
    let mut rebuild = false;

    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            CityAction::PlayBuildingLevel(level_id) => {
                // Verify building is unlocked before launching.
                let is_unlocked = ctx
                    .data
                    .level_by_id(level_id)
                    .and_then(|l| ctx.data.building_by_id(&l.building_id))
                    .map(|b| ctx.city.is_building_unlocked(b))
                    .unwrap_or(false);
                if !is_unlocked {
                    ctx.status_notice_key = Some("city.panel.building.locked".to_string());
                    rebuild = true;
                } else {
                    match ctx.start_level(level_id) {
                        Ok(()) => {
                            next_state.set(AppState::Match3);
                            return;
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                            rebuild = true;
                        }
                    }
                }
            }
            CityAction::UpgradeBuilding(building_id) => {
                if let Some(def) = ctx.data.building_by_id(building_id).cloned() {
                    match ctx.city.try_upgrade_building(&def) {
                        Ok(_) => {
                            ctx.status_notice_key = None;
                            ctx.persist();
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                        }
                    }
                    rebuild = true;
                }
            }
            CityAction::ClaimQuest(quest_id) => {
                if let Some(quest) = ctx.data.quests.iter().find(|q| q.id == *quest_id).cloned() {
                    if ctx.city.try_claim_quest(&quest) {
                        ctx.status_notice_key = None;
                        ctx.persist();
                        rebuild = true;
                    }
                }
            }
            CityAction::CraftOrEquipAmulet(amulet_id) => {
                if let Some(amulet) = ctx.data.amulet_by_id(amulet_id).cloned() {
                    match ctx.city.try_craft_amulet(&amulet) {
                        Ok(()) => {
                            ctx.status_notice_key = None;
                            ctx.persist();
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                        }
                    }
                    rebuild = true;
                }
            }
            CityAction::ToggleLanguage => {
                ctx.data.i18n.toggle_language();
                ctx.persist();
                rebuild = true;
            }
            CityAction::BackToTitle => {
                next_state.set(AppState::Title);
                return;
            }
        }
    }

    if rebuild {
        for entity in &roots {
            commands.entity(entity).despawn_recursive();
        }
        spawn_city_tree(&mut commands, &ctx);
    }
}
