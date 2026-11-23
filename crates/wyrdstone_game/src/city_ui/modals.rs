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

//! Responsive modal overlays for Amulets, Relic Collections, Runic Tool Forge, Chronicles, and Guide.

use super::{spawn_resource_pill, spawn_small_button, spawn_small_button_colored, CityAction};
use crate::audio::SoundBank;
use crate::fjord_art::FjordTextures;
use crate::scroll::{spawn_scrollbar_track, ScrollMoreHint, ScrollPanelKind, ScrollablePanel};
use crate::state::{today_day_index, ActiveModal, GameContext};
use crate::theme::*;
use crate::ui_style::{display_font, modal_shadow, sliced_panel_image, NoButtonScale};
use crate::vfx_art::VfxTextures;
use bevy::prelude::*;
use bevy::ui::{FocusPolicy, RelativeCursorPosition};
use wyrdstone_core::{
    achievement_progress, AmuletEffect, CityState, I18nCatalog, PatronGod, ResourceBundle,
};

pub(super) fn format_reward_summary(i18n: &I18nCatalog, res: ResourceBundle, glory: u32) -> String {
    let mut parts = Vec::new();
    if glory > 0 {
        parts.push(i18n.t_fmt("city.panel.reward.glory", &[("glory", glory.to_string())]));
    }
    parts.extend(reward_item_lines(i18n, res));
    i18n.t_fmt(
        "city.panel.reward.prefix",
        &[("items", parts.join(i18n.t("ui.separator.comma")))],
    )
}

/// Resource amounts paired with their locale label keys, in display order.
fn resource_entries(res: ResourceBundle) -> [(u32, &'static str); 4] {
    [
        (res.wood, "resources.label.wood"),
        (res.stone, "resources.label.stone"),
        (res.gold, "resources.label.gold"),
        (res.mead, "resources.label.mead"),
    ]
}

/// One "+amount resource" text per non-zero resource of the bundle.
fn reward_item_lines(i18n: &I18nCatalog, res: ResourceBundle) -> Vec<String> {
    resource_entries(res)
        .into_iter()
        .filter(|(amount, _)| *amount > 0)
        .map(|(amount, label_key)| {
            i18n.t_fmt(
                "city.panel.reward.item",
                &[
                    ("amount", amount.to_string()),
                    ("resource", i18n.t(label_key).to_string()),
                ],
            )
        })
        .collect()
}

/// One "amount resource" text per non-zero resource of a cost bundle.
fn cost_item_lines(i18n: &I18nCatalog, cost: ResourceBundle) -> Vec<String> {
    resource_entries(cost)
        .into_iter()
        .filter(|(amount, _)| *amount > 0)
        .map(|(amount, label_key)| {
            i18n.t_fmt(
                "city.panel.shop.cost_item",
                &[
                    ("amount", amount.to_string()),
                    ("resource", i18n.t(label_key).to_string()),
                ],
            )
        })
        .collect()
}

fn format_amulet_effect(i18n: &I18nCatalog, effect: AmuletEffect) -> String {
    match effect {
        AmuletEffect::ResourceBonusPct(v) => i18n.t_fmt(
            "city.panel.amulets.effect.resources",
            &[("val", v.to_string())],
        ),
        AmuletEffect::ExtraMoves(v) => {
            i18n.t_fmt("city.panel.amulets.effect.moves", &[("val", v.to_string())])
        }
        AmuletEffect::ScoreBonusPct(v) => {
            i18n.t_fmt("city.panel.amulets.effect.score", &[("val", v.to_string())])
        }
        AmuletEffect::StartingBoosters(v) => i18n.t_fmt(
            "city.panel.amulets.effect.boosters",
            &[("val", v.to_string())],
        ),
        AmuletEffect::ExtraTimePct(v) => {
            i18n.t_fmt("city.panel.amulets.effect.time", &[("val", v.to_string())])
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_modal_shell<F>(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    width_px: f32,
    title: &str,
    close_label: &str,
    close_action: CityAction,
    scroll_offset_y: f32,
    body_fn: F,
) where
    F: FnOnce(&mut ChildBuilder),
{
    let i18n = &ctx.data.i18n;
    root.spawn((
        Button,
        NoButtonScale,
        CityAction::CloseModal,
        FocusPolicy::Block,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.80)),
        ZIndex(100),
    ))
    .with_children(|backdrop| {
        backdrop
            .spawn((
                FocusPolicy::Block,
                Interaction::default(),
                sliced_panel_image(vfx),
                Node {
                    width: Val::Px(width_px),
                    max_width: Val::Percent(94.0),
                    max_height: Val::Percent(90.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(22.0)),
                    row_gap: Val::Px(10.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                modal_shadow(),
            ))
            .with_children(|modal| {
                // Sticky modal header (never scrolls out of view)
                modal
                    .spawn((
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            flex_shrink: 0.0,
                            padding: UiRect::bottom(Val::Px(8.0)),
                            border: UiRect::bottom(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor(Color::srgba(0.93, 0.74, 0.29, 0.35)),
                    ))
                    .with_children(|hdr| {
                        hdr.spawn((
                            Text::new(title),
                            display_font(fjord_art, FONT_HEADING),
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                        spawn_small_button(
                            hdr,
                            close_label,
                            close_action,
                            COLOR_ACCENT_CRIMSON,
                            true,
                        );
                    });

                // Inline status/error banner so feedback triggered inside modals is immediately visible
                if let Some(notice_key) = &ctx.status_notice_key {
                    modal
                        .spawn((
                            Node {
                                flex_shrink: 0.0,
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.32, 0.10, 0.12, 0.92)),
                            BorderColor(COLOR_ACCENT_CRIMSON),
                            BorderRadius::all(Val::Px(6.0)),
                        ))
                        .with_children(|banner| {
                            banner.spawn((
                                Text::new(i18n.t(notice_key)),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                        });
                } else if let Some(msg) = &ctx.celebration_message {
                    modal
                        .spawn((
                            Node {
                                flex_shrink: 0.0,
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.12, 0.34, 0.22, 0.94)),
                            BorderColor(COLOR_ACCENT_GREEN),
                            BorderRadius::all(Val::Px(6.0)),
                        ))
                        .with_children(|banner| {
                            banner.spawn((
                                Text::new(msg.clone()),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                        });
                }

                // Scrollable modal body with visual proportional scrollbar
                modal
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_grow: 1.0,
                        flex_shrink: 1.0,
                        min_height: Val::Px(0.0),
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|scroll_row| {
                        scroll_row
                            .spawn((
                                ScrollablePanel {
                                    kind: ScrollPanelKind::Modal,
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
                                    row_gap: Val::Px(10.0),
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                            ))
                            .with_children(body_fn);
                        spawn_scrollbar_track(scroll_row, ScrollPanelKind::Modal);
                    });

                // Auto-hiding scroll indicator when body overflows vertically
                modal
                    .spawn((
                        ScrollMoreHint {
                            kind: ScrollPanelKind::Modal,
                        },
                        Node {
                            display: Display::None,
                            flex_shrink: 0.0,
                            justify_content: JustifyContent::Center,
                            padding: UiRect::top(Val::Px(2.0)),
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
            });
    });
}

pub(super) fn spawn_amulets_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        600.0,
        i18n.t("city.panel.amulets.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::ToggleModal(ActiveModal::Amulets),
        scroll_offset_y,
        |modal| {
            for amulet in &ctx.data.amulets {
                let tier = ctx.city.amulet_tier(&amulet.id);
                let crafted = tier > 0;
                let equipped = ctx.city.equipped_amulet.as_deref() == Some(amulet.id.as_str());
                let next_cost = ctx.city.amulet_next_cost(amulet);
                let can_afford_next = next_cost
                    .map(|c| ctx.city.resources.can_afford(&c))
                    .unwrap_or(false);
                let action_lbl = if equipped {
                    i18n.t("city.panel.amulets.equipped")
                } else if crafted {
                    i18n.t("city.panel.amulets.equip")
                } else {
                    i18n.t("city.panel.amulets.craft")
                };
                let (btn_bg, btn_fg) = if equipped {
                    (COLOR_ACCENT_GOLD, COLOR_TEXT_DARK)
                } else if crafted {
                    (COLOR_ACCENT_TEAL, COLOR_TEXT_ON_ACCENT)
                } else if can_afford_next {
                    (COLOR_ACCENT_GREEN, COLOR_TEXT_ON_ACCENT)
                } else {
                    (COLOR_BUTTON_DISABLED, COLOR_TEXT_PRIMARY)
                };

                modal
                    .spawn((
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(10.0)),
                            column_gap: Val::Px(10.0),
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
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            flex_shrink: 1.0,
                            ..default()
                        })
                        .with_children(|left| {
                            if let Some(icon) = fjord_art.amulet_icon(&amulet.id) {
                                left.spawn((
                                    ImageNode::new(icon),
                                    Node {
                                        width: Val::Px(48.0),
                                        height: Val::Px(48.0),
                                        flex_shrink: 0.0,
                                        border: UiRect::all(Val::Px(1.5)),
                                        ..default()
                                    },
                                    BorderColor(if equipped {
                                        COLOR_ACCENT_GOLD
                                    } else {
                                        COLOR_PANEL_BORDER
                                    }),
                                    BorderRadius::all(Val::Px(8.0)),
                                ));
                            }
                            left.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                flex_shrink: 1.0,
                                row_gap: Val::Px(3.0),
                                ..default()
                            })
                            .with_children(|info| {
                                let title = if crafted {
                                    i18n.t_fmt(
                                        "city.panel.amulets.name_with_tier",
                                        &[
                                            ("name", i18n.t(&amulet.name_key).to_string()),
                                            ("tier", tier.to_string()),
                                        ],
                                    )
                                } else {
                                    i18n.t(&amulet.name_key).to_string()
                                };
                                info.spawn((
                                    Text::new(title),
                                    TextFont {
                                        font_size: FONT_BODY,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                                let effect_desc = format_amulet_effect(
                                    i18n,
                                    ctx.city.scaled_amulet_effect(amulet),
                                );
                                info.spawn((
                                    Text::new(effect_desc),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                                if crafted && next_cost.is_some() {
                                    let next_eff = format_amulet_effect(
                                        i18n,
                                        CityState::scaled_amulet_effect_for_tier(amulet, tier + 1),
                                    );
                                    info.spawn((
                                        Text::new(i18n.t_fmt(
                                            "city.panel.amulets.next_tier",
                                            &[("effect", next_eff)],
                                        )),
                                        TextFont {
                                            font_size: FONT_SMALL,
                                            ..default()
                                        },
                                        TextColor(COLOR_ACCENT_TEAL),
                                    ));
                                }
                                if let Some(c) = next_cost {
                                    info.spawn((
                                        Text::new(i18n.t_fmt(
                                            "city.panel.amulet.cost",
                                            &[
                                                ("wood", c.wood.to_string()),
                                                ("stone", c.stone.to_string()),
                                                ("gold", c.gold.to_string()),
                                                ("mead", c.mead.to_string()),
                                            ],
                                        )),
                                        TextFont {
                                            font_size: FONT_SMALL,
                                            ..default()
                                        },
                                        TextColor(COLOR_TEXT_MUTED),
                                    ));
                                }
                            });
                        });

                        row.spawn(Node {
                            column_gap: Val::Px(6.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|btns| {
                            spawn_small_button_colored(
                                btns,
                                action_lbl,
                                CityAction::CraftOrEquipAmulet(amulet.id.clone()),
                                btn_bg,
                                btn_fg,
                                equipped || crafted || can_afford_next,
                            );
                            if crafted && next_cost.is_some() {
                                spawn_small_button(
                                    btns,
                                    i18n.t("city.panel.amulets.upgrade"),
                                    CityAction::UpgradeAmulet(amulet.id.clone()),
                                    if can_afford_next {
                                        COLOR_ACCENT_GREEN
                                    } else {
                                        COLOR_BUTTON_DISABLED
                                    },
                                    can_afford_next,
                                );
                            }
                        });
                    });
            }
        },
    );
}

pub(super) fn spawn_collections_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        610.0,
        i18n.t("city.panel.collections.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::ToggleModal(ActiveModal::Collections),
        scroll_offset_y,
        |modal| {
            let claimable_cols = ctx.city.claimable_collection_count(&ctx.data.collections);
            if claimable_cols >= 2 {
                modal
                    .spawn(Node {
                        justify_content: JustifyContent::FlexEnd,
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_small_button(
                            row,
                            &i18n.t_fmt(
                                "city.panel.claim_all",
                                &[("count", claimable_cols.to_string())],
                            ),
                            CityAction::ClaimAllCollections,
                            COLOR_ACCENT_GREEN,
                            true,
                        );
                    });
            }
            let mut sorted_cols: Vec<_> = ctx.data.collections.iter().collect();
            sorted_cols.sort_by_key(|col| {
                if ctx.city.is_collection_ready(col) {
                    0u8
                } else if ctx.city.claimed_collections.contains(&col.id) {
                    2u8
                } else {
                    1u8
                }
            });
            for col in sorted_cols {
                let found = ctx.city.collection_found_count(col);
                let total = col.relic_ids.len();
                let ready = ctx.city.is_collection_ready(col);
                let claimed = ctx.city.claimed_collections.contains(&col.id);

                modal
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(10.0)),
                            row_gap: Val::Px(6.0),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(if ready {
                            COLOR_ACCENT_GREEN
                        } else if claimed {
                            COLOR_ACCENT_GOLD
                        } else {
                            COLOR_PANEL_BORDER
                        }),
                        BorderRadius::all(Val::Px(10.0)),
                    ))
                    .with_children(|card| {
                        card.spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn(Node {
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(10.0),
                                flex_shrink: 1.0,
                                ..default()
                            })
                            .with_children(|hdr_left| {
                                if let Some(icon) = fjord_art.collection_icon(&col.id) {
                                    hdr_left.spawn((
                                        ImageNode::new(icon),
                                        Node {
                                            width: Val::Px(42.0),
                                            height: Val::Px(42.0),
                                            flex_shrink: 0.0,
                                            border: UiRect::all(Val::Px(1.5)),
                                            ..default()
                                        },
                                        BorderColor(if claimed {
                                            COLOR_ACCENT_GOLD
                                        } else {
                                            COLOR_PANEL_BORDER
                                        }),
                                        BorderRadius::all(Val::Px(8.0)),
                                    ));
                                }
                                hdr_left.spawn((
                                    Text::new(i18n.t_fmt(
                                        "city.panel.collections.header",
                                        &[
                                            ("name", i18n.t(&col.name_key).to_string()),
                                            ("found", found.to_string()),
                                            ("total", total.to_string()),
                                        ],
                                    )),
                                    TextFont {
                                        font_size: FONT_BODY,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                            });

                            if ready {
                                spawn_small_button(
                                    row,
                                    i18n.t("city.panel.collections.claim"),
                                    CityAction::ClaimCollection(col.id.clone()),
                                    COLOR_ACCENT_GREEN,
                                    true,
                                );
                            } else if claimed {
                                row.spawn((
                                    Text::new(i18n.t("city.panel.collections.claimed")),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GREEN),
                                ));
                            }
                        });

                        card.spawn(Node {
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(8.0),
                            row_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|pills| {
                            for (rid, rkey) in col.relic_ids.iter().zip(col.relic_name_keys.iter())
                            {
                                let owned = ctx.city.found_relics.contains(rid);
                                spawn_resource_pill(
                                    pills,
                                    i18n.t(rkey),
                                    if owned {
                                        COLOR_ACCENT_GOLD
                                    } else {
                                        COLOR_TEXT_MUTED
                                    },
                                );
                            }
                        });

                        card.spawn((
                            Text::new(format_reward_summary(i18n, col.reward_resources, 0)),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GREEN),
                        ));
                    });
            }
        },
    );
}

pub(super) fn spawn_tool_shop_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        620.0,
        i18n.t("city.panel.shop.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::ToggleModal(ActiveModal::Shop),
        scroll_offset_y,
        |modal| {
            for recipe in &ctx.data.tool_shop {
                let count = ctx.city.tools.count(recipe.tool);
                let can_afford = ctx.city.resources.can_afford(&recipe.cost);
                let c = recipe.cost;
                let cost_parts = cost_item_lines(i18n, c);

                modal
                    .spawn((
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(8.0)),
                            column_gap: Val::Px(10.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(COLOR_PANEL_BORDER),
                        BorderRadius::all(Val::Px(8.0)),
                    ))
                    .with_children(|row| {
                        row.spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            flex_shrink: 1.0,
                            ..default()
                        })
                        .with_children(|left| {
                            if let Some(icon) = fjord_art.tool_icon(recipe.tool) {
                                left.spawn((
                                    ImageNode::new(icon),
                                    Node {
                                        width: Val::Px(38.0),
                                        height: Val::Px(38.0),
                                        flex_shrink: 0.0,
                                        ..default()
                                    },
                                    BorderRadius::all(Val::Px(6.0)),
                                ));
                            }
                            left.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|info| {
                                info.spawn((
                                    Text::new(
                                        i18n.t_fmt(
                                            &recipe.name_key,
                                            &[("count", count.to_string())],
                                        ),
                                    ),
                                    TextFont {
                                        font_size: FONT_BODY,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                                info.spawn((
                                    Text::new(i18n.t(&recipe.desc_key)),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                                info.spawn((
                                    Text::new(cost_parts.join(i18n.t("ui.separator.pipe"))),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_MUTED),
                                ));
                            });
                        });

                        spawn_small_button(
                            row,
                            i18n.t("city.panel.shop.craft"),
                            CityAction::CraftTool(recipe.tool),
                            if can_afford {
                                COLOR_ACCENT_GREEN
                            } else {
                                COLOR_BUTTON_DISABLED
                            },
                            can_afford,
                        );
                    });
            }
        },
    );
}

pub(super) fn spawn_stats_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    let st = ctx.city.stats;
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        460.0,
        i18n.t("city.panel.stats.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::ToggleModal(ActiveModal::Stats),
        scroll_offset_y,
        |card| {
            let rows = [
                i18n.t_fmt(
                    "city.panel.stats.victories",
                    &[("val", st.total_victories.to_string())],
                ),
                i18n.t_fmt(
                    "city.panel.stats.runes",
                    &[("val", st.total_runes_cleared.to_string())],
                ),
                i18n.t_fmt(
                    "city.panel.stats.best_combo",
                    &[("val", st.best_combo.to_string())],
                ),
                i18n.t_fmt(
                    "city.panel.stats.best_score",
                    &[("val", st.best_score.to_string())],
                ),
                i18n.t_fmt(
                    "city.panel.stats.relics",
                    &[("val", ctx.city.found_relics.len().to_string())],
                ),
                i18n.t_fmt(
                    "city.panel.stats.streak",
                    &[("val", st.daily_streak.to_string())],
                ),
                i18n.t_fmt(
                    "city.panel.stats.bosses",
                    &[("val", st.bosses_defeated.to_string())],
                ),
                i18n.t_fmt(
                    "city.panel.stats.expedition",
                    &[("val", st.best_expedition_wave.to_string())],
                ),
                i18n.t_fmt(
                    "city.panel.stats.win_streak",
                    &[
                        ("val", st.win_streak.to_string()),
                        ("best", st.best_win_streak.to_string()),
                    ],
                ),
            ];
            for line in rows {
                card.spawn((
                    Text::new(line),
                    TextFont {
                        font_size: FONT_BODY,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_PRIMARY),
                ));
            }
        },
    );
}

pub(super) fn spawn_settings_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    sound_bank: &SoundBank,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        520.0,
        i18n.t("city.panel.settings.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::ToggleModal(ActiveModal::Settings),
        scroll_offset_y,
        |card| {
            let sound_label = if sound_bank.muted {
                i18n.t("app.audio.off")
            } else {
                i18n.t("app.audio.on")
            };
            let bgm_pct = crate::audio::bgm_volume_percent(sound_bank.bgm_volume);
            let sfx_pct = (sound_bank.sfx_volume * 100.0).round() as u32;

            // Audio & Language Controls
            card.spawn(Node {
                column_gap: Val::Px(10.0),
                row_gap: Val::Px(8.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|row| {
                spawn_small_button(
                    row,
                    sound_label,
                    CityAction::ToggleAudio,
                    COLOR_PANEL_CARD,
                    true,
                );
                spawn_small_button(
                    row,
                    &i18n.t_fmt("city.panel.audio.bgm", &[("pct", bgm_pct.to_string())]),
                    CityAction::CycleBgmVolume,
                    COLOR_PANEL_CARD,
                    true,
                );
                spawn_small_button(
                    row,
                    &i18n.t_fmt("city.panel.audio.sfx", &[("pct", sfx_pct.to_string())]),
                    CityAction::CycleSfxVolume,
                    COLOR_PANEL_CARD,
                    true,
                );
                spawn_small_button(
                    row,
                    i18n.t("app.title.button.language"),
                    CityAction::ToggleLanguage,
                    COLOR_PANEL_CARD,
                    true,
                );
            });

            // Accessibility & Gameplay Speed Controls
            card.spawn(Node {
                column_gap: Val::Px(10.0),
                row_gap: Val::Px(8.0),
                margin: UiRect::top(Val::Px(6.0)),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|acc_row| {
                spawn_small_button(
                    acc_row,
                    if ctx.colorblind_mode {
                        i18n.t("city.panel.colorblind.on")
                    } else {
                        i18n.t("city.panel.colorblind.off")
                    },
                    CityAction::ToggleColorblind,
                    if ctx.colorblind_mode {
                        COLOR_ACCENT_TEAL
                    } else {
                        COLOR_PANEL_CARD
                    },
                    true,
                );
                spawn_small_button(
                    acc_row,
                    if ctx.fast_animations {
                        i18n.t("city.panel.fast_anim.on")
                    } else {
                        i18n.t("city.panel.fast_anim.off")
                    },
                    CityAction::ToggleFastAnim,
                    if ctx.fast_animations {
                        COLOR_ACCENT_TEAL
                    } else {
                        COLOR_PANEL_CARD
                    },
                    true,
                );
                spawn_small_button(
                    acc_row,
                    if ctx.reduced_motion {
                        i18n.t("city.panel.reduced_motion.on")
                    } else {
                        i18n.t("city.panel.reduced_motion.off")
                    },
                    CityAction::ToggleReducedMotion,
                    if ctx.reduced_motion {
                        COLOR_ACCENT_TEAL
                    } else {
                        COLOR_PANEL_CARD
                    },
                    true,
                );
            });

            // Save JSON Backup Export / Import Controls (2-click confirmation on Import)
            card.spawn(Node {
                column_gap: Val::Px(10.0),
                row_gap: Val::Px(8.0),
                margin: UiRect::top(Val::Px(6.0)),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|backup_row| {
                spawn_small_button(
                    backup_row,
                    i18n.t("city.panel.backup.export"),
                    CityAction::ExportBackup,
                    COLOR_ACCENT_TEAL,
                    true,
                );
                spawn_small_button(
                    backup_row,
                    if ctx.confirm_import_backup {
                        i18n.t("city.panel.backup.confirm_import")
                    } else {
                        i18n.t("city.panel.backup.import")
                    },
                    CityAction::ImportBackup,
                    if ctx.confirm_import_backup {
                        COLOR_ACCENT_CRIMSON
                    } else {
                        COLOR_PANEL_CARD
                    },
                    true,
                );
                spawn_small_button(
                    backup_row,
                    i18n.t("city.button.title"),
                    CityAction::BackToTitle,
                    COLOR_PANEL_CARD,
                    true,
                );
            });
        },
    );
}

pub(super) fn spawn_help_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        560.0,
        i18n.t("city.panel.help.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::ToggleModal(ActiveModal::Help),
        scroll_offset_y,
        |card| {
            for (t_key, b_key) in [
                (
                    "city.panel.help.controls_title",
                    "city.panel.help.controls_body",
                ),
                (
                    "city.panel.help.boosters_title",
                    "city.panel.help.boosters_body",
                ),
                (
                    "city.panel.help.obstacles_title",
                    "city.panel.help.obstacles_body",
                ),
                (
                    "city.panel.help.fog_curse_title",
                    "city.panel.help.fog_curse_body",
                ),
                (
                    "city.panel.help.village_title",
                    "city.panel.help.village_body",
                ),
            ] {
                card.spawn((
                    Text::new(i18n.t(t_key)),
                    TextFont {
                        font_size: FONT_BODY,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_GOLD),
                ));
                card.spawn((
                    Text::new(i18n.t(b_key)),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_PRIMARY),
                ));
            }
        },
    );
}

pub(super) fn spawn_patrons_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        620.0,
        i18n.t("city.panel.patrons.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::ToggleModal(ActiveModal::Patrons),
        scroll_offset_y,
        |modal| {
            for god in PatronGod::ALL {
                let active = ctx.city.selected_patron == god;
                modal
                    .spawn((
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(10.0)),
                            column_gap: Val::Px(10.0),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(if active {
                            COLOR_ACCENT_GOLD
                        } else {
                            COLOR_PANEL_BORDER
                        }),
                        BorderRadius::all(Val::Px(10.0)),
                    ))
                    .with_children(|row| {
                        row.spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            flex_shrink: 1.0,
                            ..default()
                        })
                        .with_children(|left| {
                            if let Some(img) = fjord_art.patron_portrait(god) {
                                left.spawn((
                                    ImageNode::new(img),
                                    Node {
                                        width: Val::Px(56.0),
                                        height: Val::Px(56.0),
                                        flex_shrink: 0.0,
                                        border: UiRect::all(Val::Px(1.5)),
                                        ..default()
                                    },
                                    BorderColor(if active {
                                        COLOR_ACCENT_GOLD
                                    } else {
                                        COLOR_PANEL_BORDER
                                    }),
                                    BorderRadius::all(Val::Px(8.0)),
                                ));
                            }
                            left.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                flex_shrink: 1.0,
                                row_gap: Val::Px(3.0),
                                ..default()
                            })
                            .with_children(|info| {
                                info.spawn((
                                    Text::new(i18n.t(god.name_key())),
                                    TextFont {
                                        font_size: FONT_BODY,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                                info.spawn((
                                    Text::new(i18n.t(god.desc_key())),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                            });
                        });
                        spawn_small_button_colored(
                            row,
                            if active {
                                i18n.t("city.panel.patrons.active")
                            } else {
                                i18n.t("city.panel.patrons.select")
                            },
                            CityAction::SelectPatron(god),
                            if active {
                                COLOR_ACCENT_GOLD
                            } else {
                                COLOR_ACCENT_TEAL
                            },
                            if active {
                                COLOR_TEXT_DARK
                            } else {
                                COLOR_TEXT_ON_ACCENT
                            },
                            true,
                        );
                    });
            }
        },
    );
}

pub(super) fn spawn_raids_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    let energy_cost = ctx.data.balance.energy_cost_per_level;
    let has_energy = ctx.city.energy >= energy_cost;
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        650.0,
        i18n.t("city.panel.raids.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::ToggleModal(ActiveModal::Raids),
        scroll_offset_y,
        |modal| {
            for boss in &ctx.data.bosses {
                let unlocked = ctx.city.glory >= boss.required_glory;
                let slain = ctx.city.defeated_bosses.contains(&boss.id);
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
                        BorderColor(if slain {
                            COLOR_ACCENT_GOLD
                        } else if unlocked {
                            COLOR_ACCENT_CRIMSON
                        } else {
                            COLOR_PANEL_BORDER
                        }),
                        BorderRadius::all(Val::Px(10.0)),
                    ))
                    .with_children(|row| {
                        row.spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(12.0),
                            flex_shrink: 1.0,
                            ..default()
                        })
                        .with_children(|left| {
                            if let Some(img) = fjord_art.boss_portrait(&boss.id) {
                                left.spawn((
                                    ImageNode::new(img),
                                    Node {
                                        width: Val::Px(64.0),
                                        height: Val::Px(64.0),
                                        flex_shrink: 0.0,
                                        border: UiRect::all(Val::Px(2.0)),
                                        ..default()
                                    },
                                    BorderColor(if slain {
                                        COLOR_ACCENT_GOLD
                                    } else {
                                        COLOR_ACCENT_CRIMSON
                                    }),
                                    BorderRadius::all(Val::Px(8.0)),
                                ));
                            }
                            left.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                flex_shrink: 1.0,
                                row_gap: Val::Px(3.0),
                                ..default()
                            })
                            .with_children(|info| {
                                let header = if slain {
                                    i18n.t_fmt(
                                        "city.panel.raids.header_slain",
                                        &[
                                            ("name", i18n.t(&boss.name_key).to_string()),
                                            ("title", i18n.t(&boss.title_key).to_string()),
                                            (
                                                "state",
                                                i18n.t("city.panel.raids.defeated").to_string(),
                                            ),
                                        ],
                                    )
                                } else {
                                    i18n.t_fmt(
                                        "city.panel.raids.header",
                                        &[
                                            ("name", i18n.t(&boss.name_key).to_string()),
                                            ("title", i18n.t(&boss.title_key).to_string()),
                                        ],
                                    )
                                };
                                info.spawn((
                                    Text::new(header),
                                    TextFont {
                                        font_size: FONT_BODY,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                                info.spawn((
                                    Text::new(i18n.t(&boss.desc_key)),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                                info.spawn((
                                    Text::new(i18n.t_fmt(
                                        "city.panel.raids.weakness",
                                        &[
                                            ("rune", i18n.t(boss.weakness.i18n_key()).to_string()),
                                            ("hp", boss.max_hp.to_string()),
                                        ],
                                    )),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_TEAL),
                                ));
                                info.spawn((
                                    Text::new(format_reward_summary(
                                        i18n,
                                        boss.reward_resources,
                                        boss.glory_reward,
                                    )),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GREEN),
                                ));
                            });
                        });

                        if unlocked {
                            let fight_lbl = i18n.t_fmt(
                                "city.panel.raids.fight_cost",
                                &[("cost", energy_cost.to_string())],
                            );
                            spawn_small_button(
                                row,
                                &fight_lbl,
                                CityAction::FightBoss(boss.id.clone()),
                                if has_energy {
                                    COLOR_ACCENT_CRIMSON
                                } else {
                                    COLOR_BUTTON_DISABLED
                                },
                                has_energy,
                            );
                        } else {
                            spawn_resource_pill(
                                row,
                                &i18n.t_fmt(
                                    "city.panel.raids.locked",
                                    &[("glory", boss.required_glory.to_string())],
                                ),
                                COLOR_TEXT_MUTED,
                            );
                        }
                    });
            }
        },
    );
}

pub(super) fn spawn_achievements_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        620.0,
        i18n.t("city.panel.feats.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::ToggleModal(ActiveModal::Achievements),
        scroll_offset_y,
        |modal| {
            let claimable_feats = ctx.city.claimable_achievement_count(
                &ctx.data.achievements,
                &ctx.data.buildings,
                ctx.data.bosses.len(),
            );
            if claimable_feats >= 2 {
                modal
                    .spawn(Node {
                        justify_content: JustifyContent::FlexEnd,
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_small_button(
                            row,
                            &i18n.t_fmt(
                                "city.panel.claim_all",
                                &[("count", claimable_feats.to_string())],
                            ),
                            CityAction::ClaimAllAchievements,
                            COLOR_ACCENT_GREEN,
                            true,
                        );
                    });
            }
            let mut sorted_achs: Vec<_> = ctx.data.achievements.iter().collect();
            sorted_achs.sort_by_key(|ach| {
                let claimed = ctx.city.claimed_achievements.contains(&ach.id);
                let (done, target) = achievement_progress(
                    ach,
                    &ctx.city,
                    &ctx.data.buildings,
                    ctx.data.bosses.len(),
                );
                let unlocked = target > 0 && done >= target;
                if unlocked && !claimed {
                    0u8
                } else if claimed {
                    2u8
                } else {
                    1u8
                }
            });
            for ach in sorted_achs {
                let claimed = ctx.city.claimed_achievements.contains(&ach.id);
                let (done, target) = achievement_progress(
                    ach,
                    &ctx.city,
                    &ctx.data.buildings,
                    ctx.data.bosses.len(),
                );
                let unlocked = target > 0 && done >= target;
                let ready = unlocked && !claimed;
                let pct = if target > 0 {
                    (done.min(target) as f32 / target as f32 * 100.0).clamp(0.0, 100.0)
                } else {
                    0.0
                };

                modal
                    .spawn((
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(9.0)),
                            column_gap: Val::Px(10.0),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(if ready {
                            COLOR_ACCENT_GREEN
                        } else if claimed {
                            COLOR_ACCENT_GOLD
                        } else {
                            COLOR_PANEL_BORDER
                        }),
                        BorderRadius::all(Val::Px(8.0)),
                    ))
                    .with_children(|row| {
                        row.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 1.0,
                            row_gap: Val::Px(2.0),
                            ..default()
                        })
                        .with_children(|info| {
                            info.spawn((
                                Text::new(i18n.t(&ach.title_key)),
                                TextFont {
                                    font_size: FONT_BODY,
                                    ..default()
                                },
                                TextColor(if unlocked {
                                    COLOR_ACCENT_GOLD
                                } else {
                                    COLOR_TEXT_MUTED
                                }),
                            ));
                            info.spawn((
                                Text::new(i18n.t(&ach.desc_key)),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                            info.spawn((
                                Text::new(format_reward_summary(i18n, ach.reward, ach.glory)),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GREEN),
                            ));
                            if !unlocked && target > 0 {
                                info.spawn((
                                    Node {
                                        width: Val::Px(220.0),
                                        max_width: Val::Percent(100.0),
                                        height: Val::Px(6.0),
                                        margin: UiRect::top(Val::Px(3.0)),
                                        ..default()
                                    },
                                    BackgroundColor(COLOR_PANEL_DARK),
                                    BorderRadius::all(Val::Px(3.0)),
                                ))
                                .with_children(|track| {
                                    track.spawn((
                                        Node {
                                            width: Val::Percent(pct),
                                            height: Val::Percent(100.0),
                                            ..default()
                                        },
                                        BackgroundColor(COLOR_ACCENT_TEAL),
                                        BorderRadius::all(Val::Px(3.0)),
                                    ));
                                });
                            }
                        });

                        if ready {
                            spawn_small_button(
                                row,
                                i18n.t("city.panel.feats.claim"),
                                CityAction::ClaimAchievement(ach.id.clone()),
                                COLOR_ACCENT_GREEN,
                                true,
                            );
                        } else if claimed {
                            spawn_resource_pill(
                                row,
                                i18n.t("city.panel.feats.claimed"),
                                COLOR_ACCENT_GOLD,
                            );
                        } else {
                            let prog_lbl = i18n.t_fmt(
                                "city.panel.feats.progress",
                                &[("done", done.to_string()), ("target", target.to_string())],
                            );
                            spawn_resource_pill(row, &prog_lbl, COLOR_TEXT_MUTED);
                        }
                    });
            }
        },
    );
}

pub(super) fn spawn_expedition_boons_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    let Some(run) = &ctx.active_expedition else {
        return;
    };
    let title = i18n.t_fmt(
        "city.panel.expedition.boons_title",
        &[("wave", run.wave.to_string())],
    );
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        580.0,
        &title,
        i18n.t("city.panel.expedition.later"),
        CityAction::CloseModal,
        scroll_offset_y,
        |modal| {
            for (idx, boon) in run.pending_boons.iter().enumerate() {
                modal
                    .spawn((
                        Button,
                        CityAction::PickExpeditionBoon(idx),
                        Node {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(12.0)),
                            row_gap: Val::Px(4.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(COLOR_ACCENT_GOLD),
                        BorderRadius::all(Val::Px(10.0)),
                    ))
                    .with_children(|card| {
                        card.spawn((
                            Text::new(i18n.t(boon.name_key())),
                            TextFont {
                                font_size: FONT_HEADING,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                        card.spawn((
                            Text::new(i18n.t(boon.desc_key())),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_PRIMARY),
                        ));
                    });
            }

            modal
                .spawn(Node {
                    justify_content: JustifyContent::Center,
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                })
                .with_children(|row| {
                    spawn_small_button(
                        row,
                        if ctx.confirm_end_expedition {
                            i18n.t("city.panel.expedition.confirm_cashout")
                        } else {
                            i18n.t("city.panel.expedition.cashout")
                        },
                        CityAction::EndExpedition,
                        if ctx.confirm_end_expedition {
                            COLOR_ACCENT_CRIMSON
                        } else {
                            Color::srgba(0.32, 0.14, 0.16, 0.95)
                        },
                        true,
                    );
                });
        },
    );
}

pub(super) fn spawn_login_calendar_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    let today = today_day_index();
    let available = ctx.city.is_login_tribute_available(today);
    let current_day = ctx.city.next_login_tribute_day(today);

    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        640.0,
        i18n.t("city.panel.tribute.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::CloseModal,
        scroll_offset_y,
        |modal| {
            modal.spawn((
                Text::new(if available {
                    i18n.t("city.panel.tribute.subtitle")
                } else {
                    i18n.t("city.panel.tribute.done_today")
                }),
                TextFont {
                    font_size: FONT_BODY,
                    ..default()
                },
                TextColor(if available {
                    COLOR_ACCENT_GOLD
                } else {
                    COLOR_ACCENT_GREEN
                }),
            ));

            modal
                .spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(8.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|grid| {
                    for day in 1..=7u8 {
                        let (res, tool) =
                            CityState::login_tribute_reward_with(day, &ctx.data.balance);
                        let is_claimed = if available {
                            day < current_day
                        } else {
                            day <= current_day
                        };
                        let is_today = available && day == current_day;

                        let border_col = if is_today {
                            COLOR_ACCENT_GOLD
                        } else if is_claimed {
                            COLOR_ACCENT_GREEN
                        } else {
                            COLOR_PANEL_BORDER
                        };

                        grid.spawn((
                            Node {
                                width: Val::Px(134.0),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                padding: UiRect::all(Val::Px(9.0)),
                                row_gap: Val::Px(4.0),
                                border: UiRect::all(Val::Px(if is_today { 2.0 } else { 1.5 })),
                                ..default()
                            },
                            BackgroundColor(if is_today {
                                Color::srgba(0.18, 0.24, 0.34, 0.96)
                            } else {
                                COLOR_PANEL_CARD
                            }),
                            BorderColor(border_col),
                            BorderRadius::all(Val::Px(10.0)),
                        ))
                        .with_children(|card| {
                            card.spawn((
                                Text::new(
                                    i18n.t_fmt(
                                        "city.panel.tribute.day",
                                        &[("day", day.to_string())],
                                    ),
                                ),
                                TextFont {
                                    font_size: FONT_BODY,
                                    ..default()
                                },
                                TextColor(if is_today {
                                    COLOR_ACCENT_GOLD
                                } else if is_claimed {
                                    COLOR_ACCENT_GREEN
                                } else {
                                    COLOR_TEXT_PRIMARY
                                }),
                            ));
                            if let Some(t) = tool {
                                if let Some(icon) = fjord_art.tool_icon(t) {
                                    card.spawn((
                                        ImageNode::new(icon),
                                        Node {
                                            width: Val::Px(32.0),
                                            height: Val::Px(32.0),
                                            ..default()
                                        },
                                        BorderRadius::all(Val::Px(6.0)),
                                    ));
                                }
                            }
                            let lines = reward_item_lines(i18n, res);
                            card.spawn((
                                Text::new(lines.join("\n")),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                            if is_claimed {
                                card.spawn((
                                    Text::new(i18n.t("city.panel.tribute.claimed")),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GREEN),
                                ));
                            } else if is_today {
                                card.spawn((
                                    Text::new(i18n.t("city.panel.tribute.today")),
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

            if available {
                modal
                    .spawn(Node {
                        justify_content: JustifyContent::Center,
                        margin: UiRect::top(Val::Px(6.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_small_button_colored(
                            row,
                            &i18n.t_fmt(
                                "city.panel.tribute.claim",
                                &[("day", current_day.to_string())],
                            ),
                            CityAction::ClaimLoginTribute,
                            COLOR_ACCENT_GOLD,
                            COLOR_TEXT_DARK,
                            true,
                        );
                    });
            }
        },
    );
}

pub(super) fn spawn_chapter_dialogue_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    let Some(dlg) = &ctx.pending_dialogue else {
        return;
    };
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        580.0,
        i18n.t(&dlg.title_key),
        i18n.t("city.chapter.continue"),
        CityAction::CloseModal,
        scroll_offset_y,
        |modal| {
            modal
                .spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(16.0),
                    ..default()
                })
                .with_children(|row| {
                    if let Some(portrait) = fjord_art.portrait(&dlg.speaker) {
                        row.spawn((
                            ImageNode::new(portrait),
                            Node {
                                width: Val::Px(88.0),
                                height: Val::Px(88.0),
                                flex_shrink: 0.0,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BorderColor(COLOR_ACCENT_GOLD),
                            BorderRadius::all(Val::Px(12.0)),
                        ));
                    }
                    row.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 1.0,
                        row_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|col| {
                        col.spawn((
                            Text::new(i18n.t(&dlg.line1_key)),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_PRIMARY),
                        ));
                        col.spawn((
                            Text::new(i18n.t(&dlg.line2_key)),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                        ));
                    });
                });

            modal
                .spawn(Node {
                    justify_content: JustifyContent::Center,
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|row| {
                    spawn_small_button_colored(
                        row,
                        i18n.t("city.chapter.continue"),
                        CityAction::CloseModal,
                        COLOR_ACCENT_GOLD,
                        COLOR_TEXT_DARK,
                        true,
                    );
                });
        },
    );
}
