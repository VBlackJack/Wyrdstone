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

//! Building inspector drawer with difficulty selector tabs, live board preview, and upgrade actions.

use super::{spawn_full_button, spawn_resource_pill, spawn_small_button_colored, CityAction};
use crate::fjord_art::FjordTextures;
use crate::scroll::{ScrollPanelKind, ScrollablePanel};
use crate::state::GameContext;
use crate::theme::*;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use wyrdstone_core::{BuildingDef, Objective};

pub(super) fn spawn_building_inspector(
    inspector: &mut ChildBuilder,
    b: &BuildingDef,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    let tier = ctx.city.building_tier(&b.id);
    let stars = ctx.city.building_mastery_stars(&b.id);
    let unlocked = ctx.city.is_building_unlocked(b);
    let next_cost = ctx.city.next_upgrade_cost(b);
    let can_afford = next_cost
        .map(|c| ctx.city.resources.can_afford(c))
        .unwrap_or(false);

    let has_mastery = unlocked && stars >= 1 && b.mastery_level_id.is_some();
    let has_legendary = unlocked && stars >= 3 && b.legendary_level_id.is_some();

    // Resolve which difficulty tier is currently previewed.
    let preview_level_id = match ctx.selected_difficulty {
        2 if has_legendary => b.legendary_level_id.as_deref().unwrap_or(&b.level_id),
        1 if has_mastery => b.mastery_level_id.as_deref().unwrap_or(&b.level_id),
        _ => b.level_id.as_str(),
    };
    let active_diff = if preview_level_id == b.legendary_level_id.as_deref().unwrap_or("") {
        2u8
    } else if preview_level_id == b.mastery_level_id.as_deref().unwrap_or("") {
        1u8
    } else {
        0u8
    };

    inspector
        .spawn((
            ScrollablePanel {
                kind: ScrollPanelKind::Inspector,
            },
            ScrollPosition {
                offset_x: 0.0,
                offset_y: scroll_offset_y,
            },
            RelativeCursorPosition::default(),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_height: Val::Px(0.0),
                overflow: Overflow::scroll_y(),
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|top| {
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
                        width: Val::Px(210.0),
                        height: Val::Px(124.0),
                        align_self: AlignSelf::Center,
                        flex_shrink: 0.0,
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
                        &[("level", tier.to_string()), ("max", b.max_tier.to_string())],
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

            // Village Effect box
            top.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(9.0)),
                    row_gap: Val::Px(3.0),
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

            // Difficulty selector tabs (when Mastery or Legendary is unlocked)
            if has_mastery || has_legendary {
                top.spawn(Node {
                    column_gap: Val::Px(6.0),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                })
                .with_children(|tabs| {
                    let std_stars = ctx.city.level_stars(&b.level_id);
                    let std_lbl = if std_stars > 0 {
                        format!("{} ({}/3)", i18n.t("city.difficulty.standard"), std_stars)
                    } else {
                        i18n.t("city.difficulty.standard").to_string()
                    };
                    spawn_small_button_colored(
                        tabs,
                        &std_lbl,
                        CityAction::SelectDifficulty(0),
                        if active_diff == 0 {
                            COLOR_ACCENT_TEAL
                        } else {
                            COLOR_PANEL_CARD
                        },
                        COLOR_TEXT_PRIMARY,
                    );
                    if has_mastery {
                        let m_id = b.mastery_level_id.as_deref().unwrap_or("");
                        let m_stars = ctx.city.level_stars(m_id);
                        let m_lbl = if m_stars > 0 {
                            format!("{} ({}/3)", i18n.t("city.difficulty.mastery"), m_stars)
                        } else {
                            i18n.t("city.difficulty.mastery").to_string()
                        };
                        spawn_small_button_colored(
                            tabs,
                            &m_lbl,
                            CityAction::SelectDifficulty(1),
                            if active_diff == 1 {
                                COLOR_ACCENT_GOLD
                            } else {
                                COLOR_PANEL_CARD
                            },
                            if active_diff == 1 {
                                COLOR_TEXT_DARK
                            } else {
                                COLOR_TEXT_PRIMARY
                            },
                        );
                    }
                    if has_legendary {
                        let l_id = b.legendary_level_id.as_deref().unwrap_or("");
                        let l_stars = ctx.city.level_stars(l_id);
                        let l_lbl = if l_stars > 0 {
                            format!("{} ({}/3)", i18n.t("city.difficulty.legendary"), l_stars)
                        } else {
                            i18n.t("city.difficulty.legendary").to_string()
                        };
                        spawn_small_button_colored(
                            tabs,
                            &l_lbl,
                            CityAction::SelectDifficulty(2),
                            if active_diff == 2 {
                                Color::srgb(0.66, 0.30, 0.82)
                            } else {
                                COLOR_PANEL_CARD
                            },
                            COLOR_TEXT_PRIMARY,
                        );
                    }
                });
            }

            // Board Preview for the selected difficulty
            if let Some(level) = ctx.data.level_by_id(preview_level_id) {
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
                    let mut header = format!(
                        "{} ({}",
                        i18n.t("city.panel.building.preview_title"),
                        i18n.t_fmt(
                            "match3.hud.moves",
                            &[("moves", level.max_moves.to_string())]
                        ),
                    );
                    if let Some(secs) = level.time_limit_seconds {
                        header.push_str(", ");
                        header.push_str(
                            &i18n.t_fmt("match3.hud.timer", &[("secs", secs.to_string())]),
                        );
                    }
                    header.push(')');
                    prev.spawn((
                        Text::new(header),
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
                    if level.frost_spread {
                        prev.spawn((
                            Text::new(i18n.t("city.panel.building.frost_warning")),
                            TextFont {
                                font_size: 11.5,
                                ..default()
                            },
                            TextColor(Color::srgb(0.45, 0.86, 1.0)),
                        ));
                    }
                    if !level.fog_cells.is_empty() {
                        prev.spawn((
                            Text::new(i18n.t("city.panel.building.fog_warning")),
                            TextFont {
                                font_size: 11.5,
                                ..default()
                            },
                            TextColor(Color::srgb(0.72, 0.82, 0.96)),
                        ));
                    }
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

            // Upgrade cost breakdown
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
            flex_shrink: 0.0,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|actions| {
            let (play_label, play_bg, play_fg) = match active_diff {
                2 => (
                    i18n.t("city.button.play_legendary"),
                    Color::srgb(0.66, 0.30, 0.82),
                    COLOR_TEXT_PRIMARY,
                ),
                1 => (
                    i18n.t("city.button.play_mastery"),
                    COLOR_ACCENT_GOLD,
                    COLOR_TEXT_DARK,
                ),
                _ => (
                    i18n.t("city.button.play_level"),
                    if unlocked {
                        COLOR_ACCENT_TEAL
                    } else {
                        COLOR_BUTTON_DISABLED
                    },
                    COLOR_TEXT_PRIMARY,
                ),
            };
            spawn_full_button(
                actions,
                play_label,
                CityAction::PlayBuildingLevel(preview_level_id.to_string()),
                play_bg,
                play_fg,
            );

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
