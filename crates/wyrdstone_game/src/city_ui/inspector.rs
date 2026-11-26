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
    format_countdown, spawn_cost_pills, spawn_full_button, spawn_small_button, CityAction,
    InspectorHarvestTimerText, InspectorTab,
};
use crate::fjord_art::FjordTextures;
use crate::scroll::{spawn_scrollbar_track, ScrollMoreHint, ScrollPanelKind, ScrollablePanel};
use crate::state::{now_unix_secs, GameContext};
use crate::theme::*;
use crate::ui_style::display_font;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use wyrdstone_core::{
    format_building_bonus, BuildingDef, I18nCatalog, Objective, ValhallaFlameTier, HUB_BUILDING_ID,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_building_inspector(
    inspector: &mut ChildBuilder,
    b: &BuildingDef,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    scroll_offset_y: f32,
    is_compact: bool,
    active_tab: InspectorTab,
) {
    let i18n = &ctx.data.i18n;
    let tier = ctx.city.building_tier(&b.id);
    let stars = ctx.city.building_mastery_stars(&b.id);
    let unlocked = ctx.city.is_building_unlocked(b);
    let next_cost = ctx.city.next_upgrade_cost(b);
    let can_afford = next_cost
        .map(|c| ctx.city.resources.can_afford(&c))
        .unwrap_or(false);
    let can_upgrade_now = unlocked && next_cost.is_some() && can_afford;
    let harvest_rem = ctx
        .city
        .harvest_remaining_secs(&b.id, now_unix_secs(), &ctx.data.balance);
    let harvest_ready = harvest_rem == Some(0);

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

    // Sticky drawer header with optional portrait thumbnail, building title, and Close button
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
            hdr.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                flex_shrink: 1.0,
                min_width: Val::Px(0.0),
                ..default()
            })
            .with_children(|left| {
                if is_compact {
                    if let Some(img) = fjord_art.building_image(&b.id) {
                        let tint = if !unlocked {
                            Color::srgb(0.42, 0.46, 0.54)
                        } else if tier == 0 {
                            Color::srgb(0.68, 0.76, 0.86)
                        } else {
                            Color::WHITE
                        };
                        left.spawn((
                            ImageNode {
                                image: img,
                                color: tint,
                                ..default()
                            },
                            Node {
                                width: Val::Px(44.0),
                                height: Val::Px(44.0),
                                flex_shrink: 0.0,
                                border: UiRect::all(Val::Px(1.5)),
                                ..default()
                            },
                            BorderColor(if tier > 0 {
                                COLOR_ACCENT_GOLD
                            } else {
                                COLOR_PANEL_BORDER
                            }),
                            BorderRadius::all(Val::Px(RADIUS_SM)),
                        ));
                    }
                }
                left.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(1.0),
                    flex_shrink: 1.0,
                    min_width: Val::Px(0.0),
                    ..default()
                })
                .with_children(|tcol| {
                    tcol.spawn((
                        Text::new(i18n.t(&b.name_key)),
                        display_font(
                            fjord_art,
                            if is_compact {
                                FONT_TITLE_MD - 7.0
                            } else {
                                FONT_TITLE_MD - 4.0
                            },
                        ),
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                    if is_compact {
                        tcol.spawn((
                            Text::new(status_line.clone()),
                            TextFont {
                                font_size: FONT_SMALL - 1.0,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                        ));
                    }
                });
            });
            spawn_small_button(
                hdr,
                i18n.t("city.panel.amulets.close"),
                CityAction::CloseInspector,
                COLOR_ACCENT_CRIMSON,
                true,
            );
        });

    // Segmented 2-tab switch (Raid vs Upgrade & Mastery)
    let upgrade_attention = can_upgrade_now || harvest_ready;
    inspector
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|tabs| {
            for (tab, label, attention) in [
                (
                    InspectorTab::Raid,
                    i18n.t("city.inspector.tab_raid").to_string(),
                    false,
                ),
                (
                    InspectorTab::Upgrade,
                    if upgrade_attention {
                        i18n.t("city.inspector.tab_upgrade_ready").to_string()
                    } else {
                        i18n.t("city.inspector.tab_upgrade").to_string()
                    },
                    upgrade_attention,
                ),
            ] {
                let selected = active_tab == tab;
                let (bg, fg, border_w, border_col) = if selected {
                    (
                        COLOR_ACCENT_TEAL,
                        COLOR_TEXT_ON_ACCENT,
                        2.0,
                        COLOR_ACCENT_GOLD_HOVER,
                    )
                } else if attention {
                    (COLOR_PANEL_CARD, COLOR_ACCENT_GOLD, 1.5, COLOR_ACCENT_GOLD)
                } else {
                    (
                        COLOR_PANEL_CARD,
                        COLOR_TEXT_PRIMARY,
                        1.0,
                        COLOR_PANEL_BORDER,
                    )
                };
                tabs.spawn((
                    Button,
                    CityAction::SelectInspectorTab(tab),
                    Node {
                        flex_grow: 1.0,
                        flex_basis: Val::Px(0.0),
                        min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(border_w)),
                        ..default()
                    },
                    BackgroundColor(bg),
                    BorderColor(border_col),
                    BorderRadius::all(Val::Px(RADIUS_SM)),
                ))
                .with_children(|btn| {
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
                    if active_tab == InspectorTab::Upgrade {
                        if !is_compact {
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
                                ));
                            }
                            top.spawn((
                                Text::new(status_line),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_MUTED),
                            ));
                        }

                        // Live passive tribute timer when the building is rebuilt (tier > 0)
                        if let Some(rem) = harvest_rem {
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

                        // 5-Star Building Mastery Progress Card
                        if unlocked {
                            let spoils_pct = ctx.city.building_mastery_spoils_pct(&b.id);
                            let harvest_pct = ctx.city.building_mastery_harvest_pct(&b.id);
                            let rank_name = i18n
                                .t(wyrdstone_core::CityState::mastery_rank_name_key(stars))
                                .to_string();
                            top.spawn((
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::all(Val::Px(8.0)),
                                    row_gap: Val::Px(4.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(COLOR_PANEL_CARD),
                                BorderColor(if stars >= 5 {
                                    COLOR_ACCENT_GOLD
                                } else {
                                    COLOR_PANEL_BORDER
                                }),
                                BorderRadius::all(Val::Px(8.0)),
                            ))
                            .with_children(|mbox| {
                                mbox.spawn((
                                    Text::new(i18n.t_fmt(
                                        "city.inspector.mastery_title",
                                        &[("rank", rank_name), ("stars", stars.to_string())],
                                    )),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                                // 5-pip mastery bar (NO BoxShadow inside ScrollablePanel)
                                mbox.spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(4.0),
                                    ..default()
                                })
                                .with_children(|pips| {
                                    for idx in 1u8..=wyrdstone_core::MAX_MASTERY_STARS {
                                        let lit = idx <= stars;
                                        pips.spawn((
                                            Node {
                                                flex_grow: 1.0,
                                                height: Val::Px(6.0),
                                                ..default()
                                            },
                                            BackgroundColor(if lit {
                                                COLOR_ACCENT_GOLD
                                            } else {
                                                COLOR_PANEL_DARK
                                            }),
                                            BorderRadius::all(Val::Px(3.0)),
                                        ));
                                    }
                                });
                                if stars > 0 {
                                    let active_line = if harvest_pct > 0 {
                                        i18n.t_fmt(
                                            "city.inspector.mastery_bonus_harvest",
                                            &[
                                                ("spoils", spoils_pct.to_string()),
                                                ("harvest", harvest_pct.to_string()),
                                            ],
                                        )
                                    } else {
                                        i18n.t_fmt(
                                            "city.inspector.mastery_bonus",
                                            &[("spoils", spoils_pct.to_string())],
                                        )
                                    };
                                    mbox.spawn((
                                        Text::new(active_line),
                                        TextFont {
                                            font_size: FONT_SMALL - 1.0,
                                            ..default()
                                        },
                                        TextColor(COLOR_ACCENT_TEAL_TEXT),
                                    ));
                                }
                                let next_line = match stars {
                                    0 => i18n.t("city.inspector.mastery_next_mastery").to_string(),
                                    1 => i18n.t_fmt(
                                        "city.inspector.mastery_next_spoils",
                                        &[
                                            ("stars", "2".to_string()),
                                            (
                                                "spoils",
                                                (2 * wyrdstone_core::MASTERY_SPOILS_PCT_PER_STAR)
                                                    .to_string(),
                                            ),
                                        ],
                                    ),
                                    2 => {
                                        i18n.t("city.inspector.mastery_next_legendary").to_string()
                                    }
                                    3 => i18n.t_fmt(
                                        "city.inspector.mastery_next_harvest",
                                        &[
                                            ("stars", "4".to_string()),
                                            (
                                                "spoils",
                                                (4 * wyrdstone_core::MASTERY_SPOILS_PCT_PER_STAR)
                                                    .to_string(),
                                            ),
                                            (
                                                "harvest",
                                                wyrdstone_core::MASTERY_HARVEST_PCT_AT_4_STARS
                                                    .to_string(),
                                            ),
                                        ],
                                    ),
                                    4 => i18n.t_fmt(
                                        "city.inspector.mastery_next_harvest",
                                        &[
                                            (
                                                "stars",
                                                wyrdstone_core::MAX_MASTERY_STARS.to_string(),
                                            ),
                                            (
                                                "spoils",
                                                (u32::from(wyrdstone_core::MAX_MASTERY_STARS)
                                                    * wyrdstone_core::MASTERY_SPOILS_PCT_PER_STAR)
                                                    .to_string(),
                                            ),
                                            (
                                                "harvest",
                                                wyrdstone_core::MASTERY_HARVEST_PCT_AT_5_STARS
                                                    .to_string(),
                                            ),
                                        ],
                                    ),
                                    _ => i18n.t("city.inspector.mastery_maxed").to_string(),
                                };
                                mbox.spawn((
                                    Text::new(next_line),
                                    TextFont {
                                        font_size: FONT_SMALL - 1.0,
                                        ..default()
                                    },
                                    TextColor(if stars >= 5 {
                                        COLOR_ACCENT_GOLD
                                    } else {
                                        COLOR_TEXT_MUTED
                                    }),
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
                                spawn_cost_pills(top, i18n, ctx.city.resources, cost);
                            }
                        }
                        return;
                    }

                    // Raid Tab: Shortage guidance, difficulty tabs, board preview, relics & combat loadout.
                    if !is_compact {
                        top.spawn((
                            Text::new(status_line),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                        ));
                    }

                    // Shortage Guidance callout when jumped from a missing resource pill
                    if let Some((kind, g_bldg_id, g_diff, g_amt)) = &ctx.shortage_guidance {
                        if g_bldg_id == &b.id && *g_diff == active_diff {
                            let diff_key = match g_diff {
                                2 => "city.difficulty.legendary",
                                1 => "city.difficulty.mastery",
                                _ => "city.difficulty.standard",
                            };
                            let banner = i18n.t_fmt(
                                "city.shortage.banner",
                                &[
                                    ("resource", i18n.t(kind.label_key()).to_string()),
                                    ("building", i18n.t(&b.name_key).to_string()),
                                    ("diff", i18n.t(diff_key).to_string()),
                                    ("amount", g_amt.to_string()),
                                ],
                            );
                            top.spawn((
                                Node {
                                    padding: UiRect::axes(Val::Px(9.0), Val::Px(6.0)),
                                    border: UiRect::all(Val::Px(1.5)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.10, 0.25, 0.30, 0.94)),
                                BorderColor(COLOR_ACCENT_GOLD),
                                BorderRadius::all(Val::Px(6.0)),
                            ))
                            .with_children(|call| {
                                call.spawn((
                                    Text::new(banner),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                            });
                        }
                    }

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
                            // Visual Board Mode Badges (Threat, Blitz, Relics, Ice, Frost, Fog, Cursed)
                            let mut mode_badges: Vec<(&str, Color)> = Vec::new();
                            let threat = level.threat_rating();
                            let threat_col = match threat {
                                wyrdstone_core::ThreatRating::Skirmish => COLOR_ACCENT_TEAL_TEXT,
                                wyrdstone_core::ThreatRating::Battle => COLOR_ACCENT_GOLD,
                                wyrdstone_core::ThreatRating::Fierce => {
                                    Color::srgb(0.98, 0.58, 0.24)
                                }
                                wyrdstone_core::ThreatRating::Ragnarok => COLOR_ACCENT_CRIMSON_TEXT,
                            };
                            mode_badges.push((i18n.t(threat.label_key()), threat_col));
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
                            if let Some(aid_key) =
                                ctx.city.valkyrie_aid_for_level(preview_level_id).desc_key()
                            {
                                prev.spawn((
                                    Text::new(i18n.t(aid_key)),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                            }
                            prev.spawn((
                                Text::new(i18n.t(level.tactical_tip_key())),
                                TextFont {
                                    font_size: FONT_SMALL - 1.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_MUTED),
                            ));
                        });
                    }

                    // Linked Relic Drop progress pill for this building (when a collection drops here)
                    if unlocked {
                        if let Some(col) = ctx
                            .city
                            .collection_for_building(&b.id, &ctx.data.collections)
                        {
                            let found = ctx.city.collection_found_count(col);
                            let total = col.relic_ids.len();
                            let ready = ctx.city.is_collection_ready(col);
                            let col_name = i18n.t(&col.name_key).to_string();
                            if ready {
                                super::spawn_small_button_colored(
                                    top,
                                    &i18n.t_fmt(
                                        "city.inspector.relic_drop_ready",
                                        &[("collection", col_name)],
                                    ),
                                    CityAction::ToggleModal(crate::state::ActiveModal::Collections),
                                    COLOR_ACCENT_GOLD,
                                    COLOR_TEXT_DARK,
                                    true,
                                );
                            } else {
                                top.spawn((
                                    Text::new(i18n.t_fmt(
                                        "city.inspector.relic_drop",
                                        &[
                                            ("collection", col_name),
                                            ("found", found.to_string()),
                                            ("total", total.to_string()),
                                        ],
                                    )),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_TEAL_TEXT),
                                ));
                            }
                        }
                    }

                    // Valhalla Flame 4-pip staging + Full Combat Loadout breakdown (scrollable on portrait)
                    if unlocked {
                        let streak = ctx.city.stats.win_streak;
                        let flame = ctx.city.valhalla_flame_tier();
                        let active_pips = flame.tier_index();
                        top.spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(8.0)),
                                row_gap: Val::Px(4.0),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.16, 0.11, 0.08, 0.90)),
                            BorderColor(if active_pips > 0 {
                                COLOR_ACCENT_GOLD
                            } else {
                                COLOR_PANEL_BORDER
                            }),
                            BorderRadius::all(Val::Px(8.0)),
                        ))
                        .with_children(|fbox| {
                            let title = if streak >= 1 {
                                i18n.t_fmt(
                                    "city.loadout.flame_title",
                                    &[
                                        ("streak", streak.to_string()),
                                        ("tier", i18n.t(flame.name_key()).to_string()),
                                    ],
                                )
                            } else {
                                i18n.t("city.loadout.flame_idle").to_string()
                            };
                            fbox.spawn((
                                Text::new(title),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(if streak >= 1 {
                                    COLOR_ACCENT_GOLD
                                } else {
                                    COLOR_TEXT_MUTED
                                }),
                            ));
                            // 4-pip tier gauge (NO BoxShadow inside ScrollablePanel)
                            fbox.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(5.0),
                                ..default()
                            })
                            .with_children(|pips| {
                                for idx in 1u8..=4u8 {
                                    let lit = idx <= active_pips;
                                    pips.spawn((
                                        Node {
                                            flex_grow: 1.0,
                                            height: Val::Px(7.0),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(if lit {
                                            COLOR_ACCENT_GOLD
                                        } else {
                                            COLOR_PANEL_DARK
                                        }),
                                        BorderColor(if lit {
                                            COLOR_ACCENT_GOLD_HOVER
                                        } else {
                                            COLOR_PANEL_BORDER
                                        }),
                                        BorderRadius::all(Val::Px(3.0)),
                                    ));
                                }
                            });
                            if streak >= 1 {
                                fbox.spawn((
                                    Text::new(i18n.t(flame.desc_key())),
                                    TextFont {
                                        font_size: FONT_SMALL - 1.0,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                                let next_line =
                                    match ValhallaFlameTier::streak_needed_for_next(streak) {
                                        Some((next_tier, wins_left)) => i18n.t_fmt(
                                            "city.loadout.flame_next",
                                            &[
                                                ("tier", i18n.t(next_tier.name_key()).to_string()),
                                                ("wins", wins_left.to_string()),
                                            ],
                                        ),
                                        None => i18n.t("city.loadout.flame_max").to_string(),
                                    };
                                fbox.spawn((
                                    Text::new(next_line),
                                    TextFont {
                                        font_size: FONT_SMALL - 1.0,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_TEAL_TEXT),
                                ));
                            }
                        });

                        // Active Combat Loadout effects (Patron, Primary + Secondary Amulet, Synergies, War Draught)
                        let patron_summary = wyrdstone_core::format_patron_desc(
                            i18n,
                            ctx.city.selected_patron,
                            &ctx.data.balance.session,
                        );
                        let primary_amulet = ctx
                            .city
                            .equipped_amulet
                            .as_deref()
                            .and_then(|id| ctx.data.amulet_by_id(id));
                        let secondary_amulet = ctx
                            .city
                            .equipped_amulet_secondary
                            .as_deref()
                            .and_then(|id| ctx.data.amulet_by_id(id));
                        let synergies = ctx.city.active_synergies();
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
                        .with_children(|lbox| {
                            lbox.spawn((
                                Text::new(patron_summary),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_MUTED),
                            ));
                            if let Some(am) = primary_amulet {
                                let am_eff = super::modals::format_amulet_effect(
                                    i18n,
                                    ctx.city.scaled_amulet_effect(am),
                                );
                                lbox.spawn((
                                    Text::new(am_eff),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_TEAL_TEXT),
                                ));
                            }
                            if let Some(sec) = secondary_amulet {
                                let sec_eff = super::modals::format_amulet_effect(
                                    i18n,
                                    ctx.city.scaled_amulet_effect(sec),
                                );
                                lbox.spawn((
                                    Text::new(i18n.t_fmt(
                                        "city.loadout.secondary_amulet",
                                        &[
                                            ("name", i18n.t(&sec.name_key).to_string()),
                                            ("effect", sec_eff),
                                        ],
                                    )),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_TEAL_TEXT),
                                ));
                            }
                            if !synergies.is_empty() {
                                let names: Vec<String> = synergies
                                    .iter()
                                    .map(|s| i18n.t(s.name_key()).to_string())
                                    .collect();
                                lbox.spawn((
                                    Text::new(i18n.t_fmt(
                                        "city.loadout.synergies",
                                        &[("names", names.join(i18n.t("ui.separator.comma")))],
                                    )),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                            }
                            if let Some(draught) = ctx.city.active_war_draught {
                                lbox.spawn((
                                    Text::new(i18n.t_fmt(
                                        "city.header.war_draught",
                                        &[("name", i18n.t(draught.name_key()).to_string())],
                                    )),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                            }
                        });
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

            // Tab-contextual bottom action strip so portrait viewports preserve vertical scroll room
            if active_tab == InspectorTab::Raid {
                // 1-click Pre-Combat Loadout strip (Patron God + Amulet)
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
            } else {
                if harvest_ready {
                    super::spawn_small_button_colored(
                        actions,
                        i18n.t("city.harvest.ready"),
                        CityAction::CollectHarvest(b.id.clone()),
                        COLOR_ACCENT_GOLD,
                        COLOR_TEXT_DARK,
                        true,
                    );
                }
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
            }
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
