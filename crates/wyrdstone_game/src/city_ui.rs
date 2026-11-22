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

//! Interactive painted Fjord village map with building inspector drawer and Amulets modal.

use crate::audio::{play_sfx, toggle_mute, BgmTrack, SfxKind, SoundBank};
use crate::fjord_art::FjordTextures;
use crate::state::{AppState, GameContext};
use crate::theme::*;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use wyrdstone_core::{BuildingDef, Objective};

const CELEBRATION_DURATION_SECS: f32 = 3.2;

#[derive(Component)]
pub struct CityRoot;

/// Tracks whether the City layout was last built in portrait mode.
#[derive(Resource, Default)]
pub struct CityLayoutMode {
    pub is_portrait: bool,
}

#[derive(Component)]
pub enum CityAction {
    SelectBuilding(String),
    PlayBuildingLevel(String),
    UpgradeBuilding(String),
    ClaimQuest(String),
    ToggleAmuletsModal,
    CraftOrEquipAmulet(String),
    ToggleLanguage,
    ToggleAudio,
    BackToTitle,
}

/// Spawns the Fjord City hub UI.
pub fn setup_city_ui(
    mut commands: Commands,
    ctx: Res<GameContext>,
    fjord_art: Res<FjordTextures>,
    sound_bank: Res<SoundBank>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let is_portrait = windows
        .get_single()
        .map(|w| w.width() < w.height())
        .unwrap_or(false);
    commands.insert_resource(CityLayoutMode { is_portrait });
    spawn_city_tree(&mut commands, &ctx, &fjord_art, sound_bank.muted, is_portrait);
}

/// Cleans up the Fjord City UI on state exit.
pub fn cleanup_city_ui(mut commands: Commands, roots: Query<Entity, With<CityRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<CityLayoutMode>();
}

fn spawn_city_tree(
    commands: &mut Commands,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    muted: bool,
    is_portrait: bool,
) {
    let i18n = &ctx.data.i18n;
    let res = ctx.city.resources;
    let max_energy = ctx.city.max_energy(&ctx.data.balance);
    let sound_label = if muted {
        i18n.t("app.audio.off")
    } else {
        i18n.t("app.audio.on")
    };

    commands
        .spawn((
            CityRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(COLOR_BG_DEEP),
        ))
        .with_children(|root| {
            // 1. Sleek Top Resource & Navigation Bar
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(9.0)),
                    row_gap: Val::Px(6.0),
                    column_gap: Val::Px(10.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_DARK),
                BorderColor(COLOR_PANEL_BORDER),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|bar| {
                // Village title + Clan Glory
                bar.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(1.0),
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

                // Resource pills
                bar.spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|pills| {
                    spawn_resource_pill(
                        pills,
                        &format!("{}: {}", i18n.t("resources.label.wood"), res.wood),
                        Color::srgb(0.44, 0.80, 0.52),
                    );
                    spawn_resource_pill(
                        pills,
                        &format!("{}: {}", i18n.t("resources.label.stone"), res.stone),
                        Color::srgb(0.76, 0.74, 0.88),
                    );
                    spawn_resource_pill(
                        pills,
                        &format!("{}: {}", i18n.t("resources.label.gold"), res.gold),
                        COLOR_ACCENT_GOLD,
                    );
                    spawn_resource_pill(
                        pills,
                        &format!("{}: {}", i18n.t("resources.label.mead"), res.mead),
                        Color::srgb(0.95, 0.66, 0.30),
                    );
                    spawn_resource_pill(
                        pills,
                        &format!(
                            "{}: {}/{}",
                            i18n.t("resources.label.energy"),
                            ctx.city.energy,
                            max_energy
                        ),
                        Color::srgb(0.45, 0.84, 0.96),
                    );
                });

                // Utility buttons
                bar.spawn(Node {
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|btns| {
                    spawn_small_button(
                        btns,
                        i18n.t("city.panel.amulets.button"),
                        CityAction::ToggleAmuletsModal,
                        if ctx.city.equipped_amulet.is_some() {
                            COLOR_ACCENT_TEAL
                        } else {
                            COLOR_PANEL_CARD
                        },
                    );
                    spawn_small_button(
                        btns,
                        sound_label,
                        CityAction::ToggleAudio,
                        COLOR_PANEL_CARD,
                    );
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

            // 2. Compact Active Saga Quest Ribbon with Speaker Portrait
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    column_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_CARD),
                BorderColor(COLOR_ACCENT_GOLD),
                BorderRadius::all(Val::Px(8.0)),
            ))
            .with_children(|qbar| {
                if let Some(quest) = ctx.city.active_quest(&ctx.data.quests) {
                    let ready = ctx.city.is_quest_ready(quest);
                    qbar.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.0),
                        ..default()
                    })
                    .with_children(|left| {
                        if let Some(portrait) = fjord_art.portrait(&quest.speaker) {
                            left.spawn((
                                ImageNode::new(portrait),
                                Node {
                                    width: Val::Px(46.0),
                                    height: Val::Px(46.0),
                                    border: UiRect::all(Val::Px(1.5)),
                                    ..default()
                                },
                                BorderColor(COLOR_ACCENT_GOLD),
                                BorderRadius::all(Val::Px(8.0)),
                            ));
                        }
                        left.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(1.0),
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

            // Optional Upgrade Celebration Banner
            if let Some(msg) = &ctx.celebration_message {
                root.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.14, 0.42, 0.26, 0.95)),
                    BorderColor(COLOR_ACCENT_GOLD),
                    BorderRadius::all(Val::Px(8.0)),
                ))
                .with_children(|cel| {
                    cel.spawn((
                        Text::new(msg.clone()),
                        TextFont {
                            font_size: FONT_BODY,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                });
            }

            // 3. Main Split View: Responsive (Row on Desktop/Landscape, Column on Mobile Portrait)
            let (map_w, map_h, insp_w, insp_h) = if is_portrait {
                (
                    Val::Percent(100.0),
                    Val::Percent(52.0),
                    Val::Percent(100.0),
                    Val::Percent(48.0),
                )
            } else {
                (
                    Val::Percent(66.0),
                    Val::Percent(100.0),
                    Val::Percent(34.0),
                    Val::Percent(100.0),
                )
            };

            root.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: if is_portrait {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                column_gap: Val::Px(12.0),
                row_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|main| {
                // LEFT / TOP: Full Interactive Painted Fjord Map
                main.spawn((
                    ImageNode::new(fjord_art.fjord_map.clone()),
                    Node {
                        width: map_w,
                        height: map_h,
                        border: UiRect::all(Val::Px(2.0)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BorderColor(COLOR_PANEL_BORDER),
                    BorderRadius::all(Val::Px(12.0)),
                ))
                .with_children(|map| {
                    for b in &ctx.data.buildings {
                        spawn_map_building_pin(map, b, ctx, fjord_art);
                    }
                });

                // RIGHT / BOTTOM: Spacious Building Inspector Drawer
                main.spawn((
                    Node {
                        width: insp_w,
                        height: insp_h,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::all(Val::Px(14.0)),
                        row_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(COLOR_PANEL_DARK),
                    BorderColor(COLOR_PANEL_BORDER),
                    BorderRadius::all(Val::Px(12.0)),
                ))
                .with_children(|inspector| {
                    if let Some(selected) = ctx.data.building_by_id(&ctx.selected_building_id) {
                        spawn_building_inspector(inspector, selected, ctx, fjord_art);
                    } else {
                        inspector.spawn((
                            Text::new(i18n.t("city.panel.building.select_hint")),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                        ));
                    }
                });
            });

            // 4. Optional Nordic Amulets Modal Overlay
            if ctx.amulets_modal_open {
                spawn_amulets_modal(root, ctx);
            }
        });
}

fn spawn_map_building_pin(
    map: &mut ChildBuilder,
    b: &BuildingDef,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
) {
    let i18n = &ctx.data.i18n;
    let tier = ctx.city.building_tier(&b.id);
    let unlocked = ctx.city.is_building_unlocked(b);
    let selected = ctx.selected_building_id == b.id;
    let can_upgrade = unlocked
        && ctx
            .city
            .next_upgrade_cost(b)
            .map(|c| ctx.city.resources.can_afford(c))
            .unwrap_or(false);

    let border_col = if selected {
        COLOR_ACCENT_GOLD
    } else if can_upgrade {
        COLOR_ACCENT_GREEN
    } else if !unlocked {
        COLOR_BUTTON_DISABLED
    } else {
        COLOR_PANEL_BORDER
    };

    let img_tint = if !unlocked {
        Color::srgba(0.38, 0.42, 0.50, 0.85)
    } else if tier == 0 {
        Color::srgba(0.62, 0.70, 0.80, 0.95)
    } else {
        Color::WHITE
    };

    // Positioned directly on its scenic clearing in the Fjord Map
    map.spawn((
        Button,
        CityAction::SelectBuilding(b.id.clone()),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent((b.map_x_pct - 9.0).max(2.0)),
            top: Val::Percent((b.map_y_pct - 12.0).max(4.0)),
            width: Val::Px(134.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(4.0)),
            row_gap: Val::Px(3.0),
            border: UiRect::all(Val::Px(if selected { 3.0 } else { 2.0 })),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.09, 0.14, 0.88)),
        BorderColor(border_col),
        BorderRadius::all(Val::Px(10.0)),
    ))
    .with_children(|pin| {
        if let Some(img) = fjord_art.building_image(&b.id) {
            pin.spawn((
                ImageNode {
                    image: img,
                    color: img_tint,
                    ..default()
                },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(64.0),
                    ..default()
                },
                BorderRadius::all(Val::Px(6.0)),
            ));
        }

        pin.spawn((
            Text::new(i18n.t(&b.name_key)),
            TextFont {
                font_size: FONT_SMALL,
                ..default()
            },
            TextColor(if selected {
                COLOR_ACCENT_GOLD
            } else {
                COLOR_TEXT_PRIMARY
            }),
        ));

        let badge = if !unlocked {
            "---".to_string()
        } else if can_upgrade {
            format!("{} ({}/{})", i18n.t("city.panel.building.status.ready"), tier, b.max_tier)
        } else if tier == 0 {
            i18n.t("city.panel.building.status.ruined").to_string()
        } else {
            format!("T{} / {}", tier, b.max_tier)
        };
        pin.spawn((
            Text::new(badge),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(if can_upgrade {
                COLOR_ACCENT_GREEN
            } else if tier > 0 {
                COLOR_ACCENT_GOLD
            } else {
                COLOR_TEXT_MUTED
            }),
        ));
    });
}

fn spawn_building_inspector(
    inspector: &mut ChildBuilder,
    b: &BuildingDef,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
) {
    let i18n = &ctx.data.i18n;
    let tier = ctx.city.building_tier(&b.id);
    let stars = ctx.city.building_mastery_stars(&b.id);
    let unlocked = ctx.city.is_building_unlocked(b);
    let next_cost = ctx.city.next_upgrade_cost(b);
    let can_afford = next_cost
        .map(|c| ctx.city.resources.can_afford(c))
        .unwrap_or(false);

    // Top content group
    inspector
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|top| {
            // Large painted illustration
            if let Some(img) = fjord_art.building_image(&b.id) {
                let tint = if !unlocked {
                    Color::srgb(0.42, 0.46, 0.54)
                } else if tier == 0 {
                    Color::srgb(0.68, 0.76, 0.86)
                } else {
                    Color::WHITE
                };
                top.spawn((
                    ImageNode {
                        image: img,
                        color: tint,
                        ..default()
                    },
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(156.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor(if tier > 0 {
                        COLOR_ACCENT_GOLD
                    } else {
                        COLOR_PANEL_BORDER
                    }),
                    BorderRadius::all(Val::Px(10.0)),
                ));
            }

            // Title + Tier / Mastery
            top.spawn((
                Text::new(i18n.t(&b.name_key)),
                TextFont {
                    font_size: FONT_TITLE_MD - 4.0,
                    ..default()
                },
                TextColor(COLOR_ACCENT_GOLD),
            ));

            let status_line = format!(
                "{}   |   {}",
                if tier == 0 {
                    i18n.t("city.panel.building.status.ruined").to_string()
                } else {
                    i18n.t_fmt(
                        "city.panel.building.level",
                        &[
                            ("level", tier.to_string()),
                            ("max", b.max_tier.to_string()),
                        ],
                    )
                },
                i18n.t_fmt(
                    "city.panel.building.mastery",
                    &[("stars", stars.to_string())],
                ),
            );
            top.spawn((
                Text::new(status_line),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_TEXT_MUTED),
            ));

            top.spawn((
                Text::new(i18n.t(&b.desc_key)),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_TEXT_PRIMARY),
            ));

            // Clear, highlighted Village Effect box ("what this building actually does!")
            top.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.10, 0.22, 0.26, 0.90)),
                BorderColor(COLOR_ACCENT_TEAL),
                BorderRadius::all(Val::Px(8.0)),
            ))
            .with_children(|box_node| {
                box_node.spawn((
                    Text::new(i18n.t("city.panel.building.benefit_title")),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_GOLD),
                ));
                box_node.spawn((
                    Text::new(i18n.t(&b.bonus_key)),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_PRIMARY),
                ));
            });

            // Board Preview: show objectives & base spoils before spending Endurance
            if let Some(level) = ctx.data.level_by_id(&b.level_id) {
                let obj_summary: Vec<String> = level
                    .objectives
                    .iter()
                    .map(|o| match o {
                        Objective::ClearIce { target } => i18n.t_fmt(
                            "match3.objective.clear_ice",
                            &[("done", "0".into()), ("target", target.to_string())],
                        ),
                        Objective::CollectRune { rune, target } => i18n.t_fmt(
                            "match3.objective.collect_rune",
                            &[
                                ("rune", i18n.t(rune.i18n_key()).to_string()),
                                ("done", "0".into()),
                                ("target", target.to_string()),
                            ],
                        ),
                        Objective::ReachScore { target } => i18n.t_fmt(
                            "match3.objective.reach_score",
                            &[("done", "0".into()), ("target", target.to_string())],
                        ),
                        Objective::DropRelics { target } => i18n.t_fmt(
                            "match3.objective.drop_relics",
                            &[("done", "0".into()), ("target", target.to_string())],
                        ),
                    })
                    .collect();

                top.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(8.0)),
                        row_gap: Val::Px(3.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(COLOR_PANEL_CARD),
                    BorderColor(COLOR_PANEL_BORDER),
                    BorderRadius::all(Val::Px(8.0)),
                ))
                .with_children(|prev| {
                    prev.spawn((
                        Text::new(format!(
                            "{} ({})",
                            i18n.t("city.panel.building.preview_title"),
                            i18n.t_fmt("match3.hud.moves", &[("moves", level.max_moves.to_string())]),
                        )),
                        TextFont {
                            font_size: 12.5,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                    prev.spawn((
                        Text::new(obj_summary.join("  |  ")),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_PRIMARY),
                    ));
                    let r = level.base_reward;
                    prev.spawn((
                        Text::new(i18n.t_fmt(
                            "city.panel.building.reward_line",
                            &[
                                ("wood", r.wood.to_string()),
                                ("stone", r.stone.to_string()),
                                ("gold", r.gold.to_string()),
                                ("mead", r.mead.to_string()),
                            ],
                        )),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GREEN),
                    ));
                });
            }

            // Color-coded upgrade cost breakdown
            if !unlocked {
                top.spawn((
                    Text::new(i18n.t("city.panel.building.locked")),
                    TextFont {
                        font_size: FONT_BODY,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_CRIMSON),
                ));
            } else if let Some(cost) = next_cost {
                top.spawn((
                    Text::new(i18n.t("city.panel.building.cost_title")),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_MUTED),
                ));
                top.spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|costs| {
                    let res = ctx.city.resources;
                    let entries = [
                        (i18n.t("resources.label.wood"), res.wood, cost.wood),
                        (i18n.t("resources.label.stone"), res.stone, cost.stone),
                        (i18n.t("resources.label.gold"), res.gold, cost.gold),
                        (i18n.t("resources.label.mead"), res.mead, cost.mead),
                    ];
                    for (label, have, need) in entries {
                        if need == 0 {
                            continue;
                        }
                        let ok = have >= need;
                        spawn_resource_pill(
                            costs,
                            &format!("{}: {}/{}", label, have, need),
                            if ok {
                                COLOR_ACCENT_GREEN
                            } else {
                                COLOR_ACCENT_CRIMSON
                            },
                        );
                    }
                });
            }

            // Optional error notice
            if let Some(notice_key) = &ctx.status_notice_key {
                top.spawn((
                    Text::new(i18n.t(notice_key)),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_CRIMSON),
                ));
            }
        });

    // Bottom Action Buttons
    inspector
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|actions| {
            // Play buttons row
            actions
                .spawn(Node {
                    width: Val::Percent(100.0),
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|play_row| {
                    spawn_full_button(
                        play_row,
                        i18n.t("city.button.play_level"),
                        CityAction::PlayBuildingLevel(b.level_id.clone()),
                        if unlocked {
                            COLOR_ACCENT_TEAL
                        } else {
                            COLOR_BUTTON_DISABLED
                        },
                        COLOR_TEXT_PRIMARY,
                    );
                    if unlocked && stars >= 1 {
                        if let Some(mastery_id) = &b.mastery_level_id {
                            spawn_full_button(
                                play_row,
                                i18n.t("city.button.play_mastery"),
                                CityAction::PlayBuildingLevel(mastery_id.clone()),
                                COLOR_ACCENT_GOLD,
                                COLOR_TEXT_DARK,
                            );
                        }
                    }
                });

            // Upgrade button
            let upg_bg = if unlocked && next_cost.is_some() && can_afford {
                COLOR_ACCENT_GREEN
            } else {
                COLOR_BUTTON_DISABLED
            };
            spawn_full_button(
                actions,
                if next_cost.is_some() {
                    i18n.t("city.button.upgrade")
                } else {
                    i18n.t("city.button.max_level")
                },
                CityAction::UpgradeBuilding(b.id.clone()),
                upg_bg,
                COLOR_TEXT_PRIMARY,
            );
        });
}

fn spawn_amulets_modal(root: &mut ChildBuilder, ctx: &GameContext) {
    let i18n = &ctx.data.i18n;
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.78)),
    ))
    .with_children(|backdrop| {
        backdrop
            .spawn((
                Node {
                    width: Val::Px(540.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(22.0)),
                    row_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_DARK),
                BorderColor(COLOR_ACCENT_GOLD),
                BorderRadius::all(Val::Px(14.0)),
            ))
            .with_children(|modal| {
                modal
                    .spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|hdr| {
                        hdr.spawn((
                            Text::new(i18n.t("city.panel.amulets.title")),
                            TextFont {
                                font_size: FONT_HEADING,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                        spawn_small_button(
                            hdr,
                            i18n.t("city.panel.amulets.close"),
                            CityAction::ToggleAmuletsModal,
                            COLOR_ACCENT_CRIMSON,
                        );
                    });

                for amulet in &ctx.data.amulets {
                    let crafted = ctx.city.crafted_amulets.contains(&amulet.id);
                    let equipped = ctx.city.equipped_amulet.as_deref() == Some(amulet.id.as_str());
                    let can_afford = ctx.city.resources.can_afford(&amulet.cost);
                    let action_lbl = if equipped {
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

                    modal
                        .spawn((
                            Node {
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                padding: UiRect::all(Val::Px(12.0)),
                                column_gap: Val::Px(12.0),
                                border: UiRect::all(Val::Px(1.5)),
                                ..default()
                            },
                            BackgroundColor(COLOR_PANEL_CARD),
                            BorderColor(if equipped {
                                COLOR_ACCENT_GOLD
                            } else {
                                COLOR_PANEL_BORDER
                            }),
                            BorderRadius::all(Val::Px(10.0)),
                        ))
                        .with_children(|row| {
                            row.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(3.0),
                                ..default()
                            })
                            .with_children(|info| {
                                info.spawn((
                                    Text::new(i18n.t(&amulet.name_key)),
                                    TextFont {
                                        font_size: FONT_BODY,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                                info.spawn((
                                    Text::new(i18n.t(&amulet.desc_key)),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                                if !crafted {
                                    let c = amulet.cost;
                                    info.spawn((
                                        Text::new(i18n.t_fmt(
                                            "city.panel.building.cost",
                                            &[
                                                ("wood", c.wood.to_string()),
                                                ("stone", c.stone.to_string()),
                                                ("gold", c.gold.to_string()),
                                            ],
                                        )),
                                        TextFont {
                                            font_size: 12.5,
                                            ..default()
                                        },
                                        TextColor(COLOR_TEXT_MUTED),
                                    ));
                                }
                            });

                            spawn_small_button(
                                row,
                                action_lbl,
                                CityAction::CraftOrEquipAmulet(amulet.id.clone()),
                                btn_bg,
                            );
                        });
                }
            });
    });
}

fn spawn_resource_pill(parent: &mut ChildBuilder, text: &str, text_color: Color) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(COLOR_PANEL_CARD),
            BorderColor(COLOR_PANEL_BORDER),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|pill| {
            pill.spawn((
                Text::new(text),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(text_color),
            ));
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
                padding: UiRect::axes(Val::Px(11.0), Val::Px(7.0)),
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

fn spawn_full_button(
    parent: &mut ChildBuilder,
    label: &str,
    action: CityAction,
    bg_color: Color,
    text_color: Color,
) {
    parent
        .spawn((
            Button,
            action,
            Node {
                flex_grow: 1.0,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(11.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.5)),
                ..default()
            },
            BackgroundColor(bg_color),
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
                TextColor(text_color),
            ));
        });
}

/// Processes button presses in the Fjord City hub.
#[allow(clippy::too_many_arguments)]
pub fn handle_city_buttons(
    mut commands: Commands,
    time: Res<Time>,
    mut ctx: ResMut<GameContext>,
    fjord_art: Res<FjordTextures>,
    mut sound_bank: ResMut<SoundBank>,
    mut bgm_sinks: Query<&mut AudioSink, With<BgmTrack>>,
    mut layout_mode: ResMut<CityLayoutMode>,
    mut resize_events: EventReader<WindowResized>,
    mut next_state: ResMut<NextState<AppState>>,
    roots: Query<Entity, With<CityRoot>>,
    buttons: Query<(&Interaction, &CityAction), Changed<Interaction>>,
) {
    let mut rebuild = false;

    // Auto-clear celebration banner after timeout.
    if ctx.celebration_timer_secs > 0.0 {
        ctx.celebration_timer_secs -= time.delta_secs();
        if ctx.celebration_timer_secs <= 0.0 {
            ctx.celebration_message = None;
            rebuild = true;
        }
    }

    // Respond to window orientation changes (Portrait <-> Landscape).
    for ev in resize_events.read() {
        let now_portrait = ev.width < ev.height;
        if now_portrait != layout_mode.is_portrait {
            layout_mode.is_portrait = now_portrait;
            rebuild = true;
        }
    }

    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            CityAction::SelectBuilding(id) => {
                ctx.selected_building_id = id.clone();
                ctx.status_notice_key = None;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::PlayBuildingLevel(level_id) => {
                let is_unlocked = ctx
                    .data
                    .level_by_id(level_id)
                    .and_then(|l| ctx.data.building_by_id(&l.building_id))
                    .map(|b| ctx.city.is_building_unlocked(b))
                    .unwrap_or(false);
                if !is_unlocked {
                    ctx.status_notice_key = Some("city.panel.building.locked".to_string());
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    rebuild = true;
                } else {
                    match ctx.start_level(level_id) {
                        Ok(()) => {
                            ctx.amulets_modal_open = false;
                            next_state.set(AppState::Match3);
                            return;
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                            rebuild = true;
                        }
                    }
                }
            }
            CityAction::UpgradeBuilding(building_id) => {
                if let Some(def) = ctx.data.building_by_id(building_id).cloned() {
                    match ctx.city.try_upgrade_building(&def) {
                        Ok(new_tier) => {
                            ctx.status_notice_key = None;
                            let bname = ctx.data.i18n.t(&def.name_key).to_string();
                            ctx.celebration_message = Some(ctx.data.i18n.t_fmt(
                                "city.celebration.upgraded",
                                &[("building", bname), ("tier", new_tier.to_string())],
                            ));
                            ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                            ctx.persist();
                            play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
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
                        play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                        rebuild = true;
                    }
                }
            }
            CityAction::ToggleAmuletsModal => {
                ctx.amulets_modal_open = !ctx.amulets_modal_open;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::CraftOrEquipAmulet(amulet_id) => {
                if let Some(amulet) = ctx.data.amulet_by_id(amulet_id).cloned() {
                    match ctx.city.try_craft_amulet(&amulet) {
                        Ok(()) => {
                            ctx.status_notice_key = None;
                            ctx.persist();
                            play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
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
            CityAction::ToggleAudio => {
                toggle_mute(&mut sound_bank, &mut bgm_sinks);
                rebuild = true;
            }
            CityAction::BackToTitle => {
                ctx.amulets_modal_open = false;
                next_state.set(AppState::Title);
                return;
            }
        }
    }

    if rebuild {
        for entity in &roots {
            commands.entity(entity).despawn_recursive();
        }
        spawn_city_tree(
            &mut commands,
            &ctx,
            &fjord_art,
            sound_bank.muted,
            layout_mode.is_portrait,
        );
    }
}
