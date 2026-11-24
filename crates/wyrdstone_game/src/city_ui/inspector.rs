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

use super::{
    format_countdown, spawn_full_button, spawn_resource_pill, spawn_small_button, CityAction,
    InspectorHarvestTimerText,
};
use crate::fjord_art::FjordTextures;
use crate::scroll::{spawn_scrollbar_track, ScrollMoreHint, ScrollPanelKind, ScrollablePanel};
use crate::state::{now_unix_secs, GameContext};
use crate::theme::*;
use crate::ui_style::{button_shadow, display_font};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use wyrdstone_core::{format_building_bonus, BuildingDef, I18nCatalog, Objective, HUB_BUILDING_ID};

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

    // Sticky drawer header with building title and Close button
    inspector
        .spawn((
            Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                flex_shrink: 0.0,
                padding: UiRect::bottom(Val::Px(6.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BorderColor(Color::srgba(0.93, 0.74, 0.29, 0.35)),
        ))
        .with_children(|hdr| {
            hdr.spawn((
                Text::new(i18n.t(&b.name_key)),
                display_font(fjord_art, FONT_TITLE_MD - 4.0),
                TextColor(COLOR_ACCENT_GOLD),
            ));
            spawn_small_button(
                hdr,
                i18n.t("city.panel.amulets.close"),
                CityAction::CloseInspector,
                COLOR_ACCENT_CRIMSON,
                true,
            );
        });

    inspector
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_height: Val::Px(0.0),
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|scroll_row| {
            scroll_row
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
                            BorderRadius::all(Val::Px(RADIUS_LG)),
                            button_shadow(),
                        ));
                    }

                    let level_text = if tier == 0 {
                        i18n.t("city.panel.building.status.ruined").to_string()
                    } else {
                        i18n.t_fmt(
                            "city.panel.building.level",
                            &[("level", tier.to_string()), ("max", b.max_tier.to_string())],
                        )
                    };
                    let status_line = i18n.t_fmt(
                        "city.panel.building.status_line",
                        &[
                            ("level", level_text),
                            (
                                "mastery",
                                i18n.t_fmt(
                                    "city.panel.building.mastery",
                                    &[("stars", stars.to_string())],
                                ),
                            ),
                        ],
                    );
                    top.spawn((
                        Text::new(status_line),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_MUTED),
                    ));

                    // Live passive tribute timer when the building is rebuilt (tier > 0)
                    if let Some(rem) =
                        ctx.city
                            .harvest_remaining_secs(&b.id, now_unix_secs(), &ctx.data.balance)
                    {
                        let (h_lbl, h_col) = if rem == 0 {
                            (
                                i18n.t("city.panel.building.harvest_ready").to_string(),
                                COLOR_ACCENT_GOLD,
                            )
                        } else {
                            (
                                i18n.t_fmt(
                                    "city.panel.building.harvest_in",
                                    &[("time", format_countdown(i18n, rem))],
                                ),
                                COLOR_ACCENT_TEAL_TEXT,
                            )
                        };
                        top.spawn((
                            InspectorHarvestTimerText,
                            Text::new(h_lbl),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(h_col),
                        ));
                        if rem == 0 {
                            super::spawn_small_button_colored(
                                top,
                                i18n.t("city.harvest.ready"),
                                CityAction::CollectHarvest(b.id.clone()),
                                COLOR_ACCENT_GOLD,
                                COLOR_TEXT_DARK,
                                true,
                            );
                        }
                    }

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
                            Text::new(format_building_bonus(i18n, b, &ctx.data.balance)),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_PRIMARY),
                        ));
                        if next_cost.is_some() && b.glory_per_tier > 0 {
                            box_node.spawn((
                                Text::new(i18n.t_fmt(
                                    "city.panel.building.glory_gain",
                                    &[("glory", b.glory_per_tier.to_string())],
                                )),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_TEAL_TEXT),
                            ));
                        }
                    });

                    // Difficulty selector tabs (always visible on unlocked buildings so Mastery/Legendary criteria are clear)
                    if unlocked && (b.mastery_level_id.is_some() || b.legendary_level_id.is_some())
                    {
                        top.spawn(Node {
                            column_gap: Val::Px(6.0),
                            row_gap: Val::Px(4.0),
                            flex_wrap: FlexWrap::Wrap,
                            ..default()
                        })
                        .with_children(|tabs| {
                            let std_stars = ctx.city.level_stars(&b.level_id);
                            let std_lbl = if std_stars > 0 {
                                difficulty_label(i18n, "city.difficulty.standard", std_stars)
                            } else {
                                i18n.t("city.difficulty.standard").to_string()
                            };
                            spawn_difficulty_tab(
                                tabs,
                                &std_lbl,
                                0,
                                active_diff == 0,
                                true,
                                COLOR_ACCENT_TEAL,
                                COLOR_TEXT_ON_ACCENT,
                            );
                            if b.mastery_level_id.is_some() {
                                let m_id = b.mastery_level_id.as_deref().unwrap_or("");
                                let m_stars = ctx.city.level_stars(m_id);
                                let m_lbl = if !has_mastery {
                                    i18n.t("city.difficulty.mastery_locked").to_string()
                                } else if m_stars > 0 {
                                    difficulty_label(i18n, "city.difficulty.mastery", m_stars)
                                } else {
                                    i18n.t("city.difficulty.mastery").to_string()
                                };
                                spawn_difficulty_tab(
                                    tabs,
                                    &m_lbl,
                                    1,
                                    active_diff == 1,
                                    has_mastery,
                                    COLOR_ACCENT_GOLD,
                                    COLOR_TEXT_DARK,
                                );
                            }
                            if b.legendary_level_id.is_some() {
                                let l_id = b.legendary_level_id.as_deref().unwrap_or("");
                                let l_stars = ctx.city.level_stars(l_id);
                                let l_lbl = if !has_legendary {
                                    i18n.t("city.difficulty.legendary_locked").to_string()
                                } else if l_stars > 0 {
                                    difficulty_label(i18n, "city.difficulty.legendary", l_stars)
                                } else {
                                    i18n.t("city.difficulty.legendary").to_string()
                                };
                                spawn_difficulty_tab(
                                    tabs,
                                    &l_lbl,
                                    2,
                                    active_diff == 2,
                                    has_legendary,
                                    COLOR_ACCENT_LEGENDARY,
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
                                    "city.preview.clear_ice",
                                    &[("target", target.to_string())],
                                ),
                                Objective::CollectRune { rune, target } => i18n.t_fmt(
                                    "city.preview.collect_rune",
                                    &[
                                        ("rune", i18n.t(rune.i18n_key()).to_string()),
                                        ("target", target.to_string()),
                                    ],
                                ),
                                Objective::ReachScore { target } => i18n.t_fmt(
                                    "city.preview.reach_score",
                                    &[("target", target.to_string())],
                                ),
                                Objective::DropRelics { target } => i18n.t_fmt(
                                    "city.preview.drop_relics",
                                    &[("target", target.to_string())],
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
                            let preview_title = i18n.t("city.panel.building.preview_title");
                            let moves_text = i18n.t_fmt(
                                "match3.hud.moves",
                                &[("moves", level.max_moves.to_string())],
                            );
                            let header = if let Some(secs) = level.time_limit_seconds {
                                i18n.t_fmt(
                                    "city.panel.building.preview_header_timed",
                                    &[
                                        ("title", preview_title.to_string()),
                                        ("moves", moves_text),
                                        (
                                            "time",
                                            i18n.t_fmt(
                                                "match3.hud.timer",
                                                &[("secs", secs.to_string())],
                                            ),
                                        ),
                                    ],
                                )
                            } else {
                                i18n.t_fmt(
                                    "city.panel.building.preview_header",
                                    &[("title", preview_title.to_string()), ("moves", moves_text)],
                                )
                            };
                            prev.spawn((
                                Text::new(header),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GOLD),
                            ));
                            // Visual Board Mode Badges (Blitz, Relics, Ice, Frost, Fog, Cursed)
                            let mut mode_badges: Vec<(&str, Color)> = Vec::new();
                            if level.time_limit_seconds.is_some() {
                                mode_badges.push((i18n.t("city.mode.blitz"), COLOR_ACCENT_GOLD));
                            }
                            if level
                                .objectives
                                .iter()
                                .any(|o| matches!(o, Objective::DropRelics { .. }))
                            {
                                mode_badges
                                    .push((i18n.t("city.mode.relics"), COLOR_ACCENT_TEAL_TEXT));
                            }
                            if level
                                .objectives
                                .iter()
                                .any(|o| matches!(o, Objective::ClearIce { .. }))
                            {
                                mode_badges
                                    .push((i18n.t("city.mode.ice"), Color::srgb(0.45, 0.86, 1.0)));
                            }
                            if level.frost_spread {
                                mode_badges.push((
                                    i18n.t("city.mode.frost"),
                                    Color::srgb(0.45, 0.86, 1.0),
                                ));
                            }
                            if !level.fog_cells.is_empty() {
                                mode_badges
                                    .push((i18n.t("city.mode.fog"), Color::srgb(0.72, 0.82, 0.96)));
                            }
                            if !level.cursed_cells.is_empty() {
                                mode_badges
                                    .push((i18n.t("city.mode.cursed"), COLOR_ACCENT_CRIMSON_TEXT));
                            }
                            if !mode_badges.is_empty() {
                                prev.spawn(Node {
                                    flex_wrap: FlexWrap::Wrap,
                                    column_gap: Val::Px(6.0),
                                    row_gap: Val::Px(4.0),
                                    margin: UiRect::vertical(Val::Px(2.0)),
                                    ..default()
                                })
                                .with_children(|mrow| {
                                    for (lbl, col) in mode_badges {
                                        mrow.spawn((
                                            Node {
                                                padding: UiRect::axes(Val::Px(7.0), Val::Px(2.0)),
                                                border: UiRect::all(Val::Px(1.0)),
                                                ..default()
                                            },
                                            BackgroundColor(COLOR_PANEL_DARK),
                                            BorderColor(col),
                                            BorderRadius::all(Val::Px(6.0)),
                                        ))
                                        .with_children(
                                            |pill| {
                                                pill.spawn((
                                                    Text::new(lbl),
                                                    TextFont {
                                                        font_size: FONT_SMALL,
                                                        ..default()
                                                    },
                                                    TextColor(col),
                                                ));
                                            },
                                        );
                                    }
                                });
                            }
                            prev.spawn((
                                Text::new(obj_summary.join(i18n.t("ui.separator.pipe"))),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                            if level.frost_spread {
                                prev.spawn((
                                    Text::new(i18n.t("city.panel.building.frost_warning")),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.45, 0.86, 1.0)),
                                ));
                            }
                            if !level.fog_cells.is_empty() {
                                prev.spawn((
                                    Text::new(i18n.t("city.panel.building.fog_warning")),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.72, 0.82, 0.96)),
                                ));
                            }
                            if !level.cursed_cells.is_empty() {
                                prev.spawn((
                                    Text::new(i18n.t("city.panel.building.cursed_warning")),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.96, 0.45, 0.56)),
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
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GREEN_TEXT),
                            ));
                        });
                    }

                    // Upgrade cost breakdown
                    if unlocked {
                        if let Some(cost) = next_cost {
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
                                        &i18n.t_fmt(
                                            "city.panel.building.cost_pill",
                                            &[
                                                ("label", label.to_string()),
                                                ("have", have.to_string()),
                                                ("need", need.to_string()),
                                            ],
                                        ),
                                        if ok {
                                            COLOR_ACCENT_GREEN_TEXT
                                        } else {
                                            COLOR_ACCENT_CRIMSON_TEXT
                                        },
                                    );
                                }
                            });
                        }
                    }
                });
            spawn_scrollbar_track(scroll_row, ScrollPanelKind::Inspector);
        });

    // Auto-hiding scroll indicator when inspector content overflows vertically
    inspector
        .spawn((
            ScrollMoreHint {
                kind: ScrollPanelKind::Inspector,
            },
            Node {
                display: Display::None,
                justify_content: JustifyContent::Center,
                padding: UiRect::vertical(Val::Px(2.0)),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|hint| {
            hint.spawn((
                Text::new(i18n.t("city.panel.scroll_hint")),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_TEXT_MUTED),
            ));
        });

    // Bottom Pre-Combat Loadout + Action Buttons
    let energy_cost = ctx.data.balance.energy_cost_per_level;
    let can_play = unlocked && ctx.city.energy >= energy_cost;
    inspector
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|actions| {
            // Notice pinned above the action buttons so it never hides at the end of scroll content.
            if !unlocked {
                let hub_name = ctx
                    .data
                    .building_by_id(HUB_BUILDING_ID)
                    .map_or_else(String::new, |hub| i18n.t(&hub.name_key).to_string());
                actions.spawn((
                    Text::new(i18n.t_fmt(
                        "city.panel.building.locked_tier",
                        &[
                            ("building", hub_name),
                            ("tier", b.requires_great_hall_tier.to_string()),
                        ],
                    )),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_CRIMSON_TEXT),
                ));
            } else if let Some(notice_key) = &ctx.status_notice_key {
                actions.spawn((
                    Text::new(i18n.t(notice_key)),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_CRIMSON_TEXT),
                ));
            }

            // 1-click Pre-Combat Loadout strip (Patron God + Amulet) with live effect summary
            if unlocked {
                actions
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    })
                    .with_children(|loadout| {
                        loadout.spawn((
                            Text::new(i18n.t("city.loadout.title")),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                        ));
                        let active_amulet = ctx
                            .city
                            .equipped_amulet
                            .as_deref()
                            .and_then(|id| ctx.data.amulet_by_id(id));
                        loadout
                            .spawn(Node {
                                column_gap: Val::Px(6.0),
                                row_gap: Val::Px(4.0),
                                flex_wrap: FlexWrap::Wrap,
                                ..default()
                            })
                            .with_children(|lrow| {
                                // 1-click Patron God switcher
                                lrow.spawn((
                                    Button,
                                    CityAction::CyclePatron,
                                    Node {
                                        flex_grow: 1.0,
                                        min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                        padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                                        column_gap: Val::Px(6.0),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(COLOR_PANEL_CARD),
                                    BorderColor(COLOR_ACCENT_GOLD),
                                    BorderRadius::all(Val::Px(RADIUS_SM)),
                                ))
                                .with_children(|btn| {
                                    if let Some(img) =
                                        fjord_art.patron_portrait(ctx.city.selected_patron)
                                    {
                                        btn.spawn((
                                            ImageNode::new(img),
                                            Node {
                                                width: Val::Px(22.0),
                                                height: Val::Px(22.0),
                                                ..default()
                                            },
                                            BorderRadius::all(Val::Px(4.0)),
                                        ));
                                    }
                                    btn.spawn((
                                        Text::new(i18n.t(ctx.city.selected_patron.name_key())),
                                        TextFont {
                                            font_size: FONT_SMALL,
                                            ..default()
                                        },
                                        TextColor(COLOR_ACCENT_GOLD),
                                    ));
                                });

                                // 1-click Amulet switcher (cycles crafted amulets or opens Amulets modal)
                                let amulet_lbl = active_amulet
                                    .map(|a| i18n.t(&a.name_key))
                                    .unwrap_or_else(|| i18n.t("city.loadout.amulet_none"));
                                lrow.spawn((
                                    Button,
                                    CityAction::CycleAmulet,
                                    Node {
                                        flex_grow: 1.0,
                                        min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                        padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                                        column_gap: Val::Px(6.0),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(COLOR_PANEL_CARD),
                                    BorderColor(if active_amulet.is_some() {
                                        COLOR_ACCENT_TEAL
                                    } else {
                                        COLOR_PANEL_BORDER
                                    }),
                                    BorderRadius::all(Val::Px(RADIUS_SM)),
                                ))
                                .with_children(|btn| {
                                    if let Some(icon) =
                                        active_amulet.and_then(|a| fjord_art.amulet_icon(&a.id))
                                    {
                                        btn.spawn((
                                            ImageNode::new(icon),
                                            Node {
                                                width: Val::Px(22.0),
                                                height: Val::Px(22.0),
                                                ..default()
                                            },
                                            BorderRadius::all(Val::Px(4.0)),
                                        ));
                                    }
                                    btn.spawn((
                                        Text::new(amulet_lbl),
                                        TextFont {
                                            font_size: FONT_SMALL,
                                            ..default()
                                        },
                                        TextColor(if active_amulet.is_some() {
                                            COLOR_TEXT_PRIMARY
                                        } else {
                                            COLOR_TEXT_MUTED
                                        }),
                                    ));
                                });
                            });
                        let patron_summary = wyrdstone_core::format_patron_desc(
                            i18n,
                            ctx.city.selected_patron,
                            &ctx.data.balance.session,
                        );
                        loadout.spawn((
                            Text::new(patron_summary),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                        ));
                        if let Some(am) = active_amulet {
                            let am_eff = super::modals::format_amulet_effect(
                                i18n,
                                ctx.city.scaled_amulet_effect(am),
                            );
                            loadout.spawn((
                                Text::new(am_eff),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_TEAL_TEXT),
                            ));
                        }
                    });
            }
            let (base_play_label, raw_play_bg, raw_play_fg) = match active_diff {
                2 => (
                    i18n.t("city.button.play_legendary"),
                    COLOR_ACCENT_LEGENDARY,
                    COLOR_TEXT_PRIMARY,
                ),
                1 => (
                    i18n.t("city.button.play_mastery"),
                    COLOR_ACCENT_GOLD,
                    COLOR_TEXT_DARK,
                ),
                _ => (
                    i18n.t("city.button.play_level"),
                    COLOR_ACCENT_TEAL,
                    COLOR_TEXT_ON_ACCENT,
                ),
            };
            let (play_bg, play_fg) = if can_play {
                (raw_play_bg, raw_play_fg)
            } else {
                (COLOR_BUTTON_DISABLED, COLOR_TEXT_PRIMARY)
            };
            let play_label = i18n.t_fmt(
                "city.button.play_cost",
                &[
                    ("label", base_play_label.to_string()),
                    ("cost", energy_cost.to_string()),
                ],
            );
            spawn_full_button(
                actions,
                &play_label,
                CityAction::PlayBuildingLevel(preview_level_id.to_string()),
                play_bg,
                play_fg,
                can_play,
            );

            let can_upgrade_now = unlocked && next_cost.is_some() && can_afford;
            let upg_bg = if can_upgrade_now {
                COLOR_ACCENT_GREEN
            } else {
                COLOR_BUTTON_DISABLED
            };
            let upg_label = if next_cost.is_some() {
                if tier == 0 {
                    i18n.t("city.button.rebuild").to_string()
                } else {
                    i18n.t_fmt(
                        "city.button.upgrade_tier",
                        &[("tier", (tier + 1).to_string())],
                    )
                }
            } else {
                i18n.t("city.button.max_level").to_string()
            };
            spawn_full_button(
                actions,
                &upg_label,
                CityAction::UpgradeBuilding(b.id.clone()),
                upg_bg,
                if can_upgrade_now {
                    COLOR_TEXT_ON_ACCENT
                } else {
                    COLOR_TEXT_PRIMARY
                },
                can_upgrade_now,
            );
        });
}

/// Spawns a difficulty selector tab with both color and a thick gold border on the active tab.
fn spawn_difficulty_tab(
    parent: &mut ChildBuilder,
    label: &str,
    diff: u8,
    selected: bool,
    enabled: bool,
    active_bg: Color,
    active_fg: Color,
) {
    let (bg, fg, border_w, border_col) = if !enabled {
        (
            COLOR_BUTTON_DISABLED,
            COLOR_TEXT_MUTED,
            1.0,
            Color::srgba(0.28, 0.34, 0.42, 0.65),
        )
    } else if selected {
        (active_bg, active_fg, 2.5, COLOR_ACCENT_GOLD_HOVER)
    } else {
        (
            COLOR_PANEL_CARD,
            COLOR_TEXT_PRIMARY,
            1.0,
            COLOR_PANEL_BORDER,
        )
    };
    let mut cmd = parent.spawn((
        Button,
        CityAction::SelectDifficulty(diff),
        Node {
            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
            min_width: Val::Px(MIN_TOUCH_TARGET_PX),
            padding: UiRect::axes(Val::Px(11.0), Val::Px(7.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(border_w)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor(border_col),
        BorderRadius::all(Val::Px(RADIUS_SM)),
    ));
    if !enabled {
        cmd.insert(super::NoButtonScale);
    }
    cmd.with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont {
                font_size: FONT_SMALL,
                ..default()
            },
            TextColor(fg),
        ));
    });
}

/// Formats a difficulty tab label with its earned stars, e.g. "Standard (2/3)".
fn difficulty_label(i18n: &I18nCatalog, key: &str, stars: u8) -> String {
    if stars > 0 {
        i18n.t_fmt(
            "city.difficulty.with_stars",
            &[
                ("label", i18n.t(key).to_string()),
                ("stars", stars.to_string()),
            ],
        )
    } else {
        i18n.t(key).to_string()
    }
}
