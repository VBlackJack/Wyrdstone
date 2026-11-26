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

use super::{
    spawn_cost_pills, spawn_resource_pill, spawn_resource_pill_with_icon, spawn_small_button,
    spawn_small_button_colored, CityAction,
};
use crate::audio::SoundBank;
use crate::fjord_art::FjordTextures;
use crate::platform::SafeAreaInsets;
use crate::scroll::{spawn_scrollbar_track, ScrollMoreHint, ScrollPanelKind, ScrollablePanel};
use crate::state::{today_day_index, ActiveModal, GameContext};
use crate::theme::*;
use crate::ui_style::{display_font, modal_shadow, sliced_panel_image, NoButtonScale};
use crate::vfx_art::VfxTextures;
use bevy::prelude::*;
use bevy::ui::{FocusPolicy, RelativeCursorPosition};
use wyrdstone_core::{
    achievement_progress, format_achievement_desc, format_boon_desc, format_boss_desc,
    format_patron_desc, patron_devotion_upgrade_cost, realm_ascension_glory_reward,
    skald_pass_tier_reward_for_cycle, wonder_boss_damage_bonus_pct, wonder_extra_max_energy,
    wonder_glory_reward, wonder_harvest_bonus_pct, wonder_milestone_tool, wonder_spoils_bonus_pct,
    wonder_starting_fury, AmuletEffect, CityState, ClanRank, I18nCatalog, PatronGod, RealmTheme,
    ResourceBundle, MAX_DAILY_STREAK_CYCLE, MAX_PATRON_DEVOTION_TIER,
    PATRON_DEVOTION_FURY_PCT_PER_TIER, PATRON_DEVOTION_SCORE_PCT_PER_TIER,
    REALM_ASCENSION_KEEP_RESOURCE_PCT, REALM_BOSS_DMG_PCT_PER_ERA, REALM_BUILDING_COST_PCT_PER_ERA,
    REALM_MAX_SCALING_ERA, REALM_SCORE_PCT_PER_ERA, REALM_SPOILS_PCT_PER_ERA,
    SKALD_PASS_CYCLE_BONUS_PCT, SKALD_PASS_CYCLE_MAX_BONUS_PCT, SKALD_PASS_TIERS,
    SKALD_PASS_XP_PER_TIER, WONDER_MAX_STAGE,
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

/// Spawns a compact Timber/Stone/Gold/Mead balance strip inside purchase modals.
fn spawn_modal_resource_strip(parent: &mut ChildBuilder, ctx: &GameContext, vfx: &VfxTextures) {
    let i18n = &ctx.data.i18n;
    let res = ctx.city.resources;
    parent
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(4.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|strip| {
            let items = [
                (
                    vfx.icon_wood.clone(),
                    "resources.label.wood",
                    res.wood,
                    COLOR_RES_WOOD,
                ),
                (
                    vfx.icon_stone.clone(),
                    "resources.label.stone",
                    res.stone,
                    COLOR_RES_STONE,
                ),
                (
                    vfx.icon_gold.clone(),
                    "resources.label.gold",
                    res.gold,
                    COLOR_ACCENT_GOLD,
                ),
                (
                    vfx.icon_mead.clone(),
                    "resources.label.mead",
                    res.mead,
                    COLOR_RES_MEAD,
                ),
            ];
            for (icon, key, amount, col) in items {
                let lbl = i18n.t_fmt(
                    "city.header.resource_pill",
                    &[
                        ("label", i18n.t(key).to_string()),
                        ("amount", amount.to_string()),
                    ],
                );
                spawn_resource_pill_with_icon(strip, icon, &lbl, col);
            }
        });
}

pub(super) fn format_amulet_effect(i18n: &I18nCatalog, effect: AmuletEffect) -> String {
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
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
    body_fn: F,
) where
    F: FnOnce(&mut ChildBuilder),
{
    spawn_modal_shell_ex(
        root,
        ctx,
        fjord_art,
        vfx,
        width_px,
        title,
        close_label,
        close_action,
        safe,
        scroll_offset_y,
        is_mobile,
        true,
        true,
        body_fn,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_modal_shell_ex<F>(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    width_px: f32,
    title: &str,
    close_label: &str,
    close_action: CityAction,
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
    dismiss_on_backdrop: bool,
    show_header_close: bool,
    body_fn: F,
) where
    F: FnOnce(&mut ChildBuilder),
{
    let i18n = &ctx.data.i18n;
    let mut backdrop_cmd = root.spawn((
        FocusPolicy::Block,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: if is_mobile {
                UiRect {
                    left: Val::Px(6.0 + safe.left),
                    right: Val::Px(6.0 + safe.right),
                    top: Val::Px(6.0 + safe.top),
                    bottom: Val::Px(6.0 + safe.bottom),
                }
            } else {
                UiRect {
                    left: Val::Px(14.0 + safe.left),
                    right: Val::Px(14.0 + safe.right),
                    top: Val::Px(14.0 + safe.top),
                    bottom: Val::Px(14.0 + safe.bottom),
                }
            },
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.80)),
        ZIndex(100),
    ));
    if dismiss_on_backdrop {
        backdrop_cmd.insert((Button, NoButtonScale, CityAction::CloseModal));
    } else {
        backdrop_cmd.insert(Interaction::default());
    }
    backdrop_cmd.with_children(|backdrop| {
        backdrop
            .spawn((
                FocusPolicy::Block,
                Interaction::default(),
                sliced_panel_image(vfx),
                Node {
                    width: Val::Px(width_px),
                    max_width: if is_mobile {
                        Val::Percent(97.0)
                    } else {
                        Val::Percent(94.0)
                    },
                    max_height: if is_mobile {
                        Val::Percent(95.0)
                    } else {
                        Val::Percent(90.0)
                    },
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(if is_mobile { 12.0 } else { 22.0 })),
                    row_gap: Val::Px(if is_mobile { 8.0 } else { 10.0 }),
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
                            column_gap: Val::Px(8.0),
                            flex_shrink: 0.0,
                            padding: UiRect::bottom(Val::Px(if is_mobile { 6.0 } else { 8.0 })),
                            border: UiRect::bottom(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor(Color::srgba(0.93, 0.74, 0.29, 0.35)),
                    ))
                    .with_children(|hdr| {
                        hdr.spawn(Node {
                            flex_shrink: 1.0,
                            min_width: Val::Px(0.0),
                            ..default()
                        })
                        .with_children(|tbox| {
                            tbox.spawn((
                                Text::new(title),
                                display_font(
                                    fjord_art,
                                    if is_mobile {
                                        FONT_HEADING - 3.0
                                    } else {
                                        FONT_HEADING
                                    },
                                ),
                                TextColor(COLOR_ACCENT_GOLD),
                            ));
                        });
                        if show_header_close {
                            spawn_small_button(
                                hdr,
                                close_label,
                                close_action,
                                COLOR_ACCENT_CRIMSON,
                                true,
                            );
                        }
                    });

                // Inline status/error banner and celebration banner render independently so
                // persistent notices (such as read-only save mode) never hide celebrations.
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
                }
                if let Some(msg) = &ctx.celebration_message {
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
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
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
        safe,
        scroll_offset_y,
        is_mobile,
        |modal| {
            spawn_modal_resource_strip(modal, ctx, vfx);
            let synergies = ctx.city.active_synergies();
            if !synergies.is_empty() {
                modal
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(8.0)),
                            row_gap: Val::Px(4.0),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(COLOR_ACCENT_GOLD),
                        BorderRadius::all(Val::Px(RADIUS_MD)),
                    ))
                    .with_children(|sbox| {
                        for syn in synergies {
                            sbox.spawn((
                                Text::new(format!(
                                    "{} - {}",
                                    i18n.t(syn.name_key()),
                                    i18n.t(syn.desc_key())
                                )),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GOLD),
                            ));
                        }
                    });
            } else {
                modal.spawn((
                    Text::new(i18n.t("city.panel.amulets.synergy_hint")),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_MUTED),
                ));
            }
            let slot2_unlocked = ctx.city.is_secondary_amulet_unlocked();
            if !slot2_unlocked {
                modal.spawn((
                    Text::new(i18n.t("city.panel.amulets.slot2_locked")),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_TEAL_TEXT),
                ));
            }
            for amulet in &ctx.data.amulets {
                let tier = ctx.city.amulet_tier(&amulet.id);
                let crafted = tier > 0;
                let equipped = ctx.city.equipped_amulet.as_deref() == Some(amulet.id.as_str());
                let in_slot2 = slot2_unlocked
                    && ctx.city.equipped_amulet_secondary.as_deref() == Some(amulet.id.as_str());
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
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(10.0)),
                            column_gap: Val::Px(10.0),
                            row_gap: Val::Px(8.0),
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
                                        TextColor(COLOR_ACCENT_TEAL_TEXT),
                                    ));
                                }
                                if let Some(c) = next_cost {
                                    spawn_cost_pills(info, i18n, ctx.city.resources, c);
                                } else if crafted {
                                    info.spawn((
                                        Text::new(i18n.t("city.panel.amulets.max_tier")),
                                        TextFont {
                                            font_size: FONT_SMALL,
                                            ..default()
                                        },
                                        TextColor(COLOR_ACCENT_GOLD),
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
                            if crafted && slot2_unlocked && !equipped {
                                spawn_small_button_colored(
                                    btns,
                                    if in_slot2 {
                                        i18n.t("city.panel.amulets.slot2_active")
                                    } else {
                                        i18n.t("city.panel.amulets.slot2")
                                    },
                                    CityAction::EquipSecondaryAmulet(amulet.id.clone()),
                                    if in_slot2 {
                                        COLOR_ACCENT_GOLD
                                    } else {
                                        COLOR_ACCENT_TEAL
                                    },
                                    if in_slot2 {
                                        COLOR_TEXT_DARK
                                    } else {
                                        COLOR_TEXT_ON_ACCENT
                                    },
                                    true,
                                );
                            }
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
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
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
        safe,
        scroll_offset_y,
        is_mobile,
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
                } else if ctx.city.collection_found_count(col) > 0 {
                    1u8
                } else {
                    2u8
                }
            });
            for col in sorted_cols {
                let found = ctx.city.collection_found_count(col);
                let total = col.relic_ids.len();
                let ready = ctx.city.is_collection_ready(col);
                let charges = ctx.city.collection_charge_count(&col.id);
                let claimed = charges > 0 || ctx.city.claimed_collections.contains(&col.id);

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
                                row.spawn(Node {
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(6.0),
                                    ..default()
                                })
                                .with_children(|rbox| {
                                    if charges >= 1 {
                                        rbox.spawn((
                                            Text::new(i18n.t_fmt(
                                                "city.panel.collections.charged",
                                                &[("count", charges.to_string())],
                                            )),
                                            TextFont {
                                                font_size: FONT_SMALL,
                                                ..default()
                                            },
                                            TextColor(COLOR_ACCENT_GOLD),
                                        ));
                                    }
                                    spawn_small_button(
                                        rbox,
                                        i18n.t("city.panel.collections.claim"),
                                        CityAction::ClaimCollection(col.id.clone()),
                                        COLOR_ACCENT_GREEN,
                                        true,
                                    );
                                });
                            } else if claimed {
                                let badge_txt = if charges >= 2 {
                                    i18n.t_fmt(
                                        "city.panel.collections.charged",
                                        &[("count", charges.to_string())],
                                    )
                                } else {
                                    i18n.t("city.panel.collections.claimed").to_string()
                                };
                                row.spawn((
                                    Text::new(badge_txt),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GREEN_TEXT),
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
                                pills
                                    .spawn((
                                        Node {
                                            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                                            column_gap: Val::Px(5.0),
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(if owned {
                                                1.5
                                            } else {
                                                1.0
                                            })),
                                            ..default()
                                        },
                                        BackgroundColor(COLOR_PANEL_CARD),
                                        BorderColor(if owned {
                                            COLOR_ACCENT_GOLD
                                        } else {
                                            COLOR_PANEL_BORDER
                                        }),
                                        BorderRadius::all(Val::Px(6.0)),
                                    ))
                                    .with_children(|pill| {
                                        if owned {
                                            pill.spawn((
                                                ImageNode::new(vfx.icon_star.clone()),
                                                Node {
                                                    width: Val::Px(14.0),
                                                    height: Val::Px(14.0),
                                                    ..default()
                                                },
                                            ));
                                        }
                                        pill.spawn((
                                            Text::new(i18n.t(rkey)),
                                            TextFont {
                                                font_size: FONT_SMALL,
                                                ..default()
                                            },
                                            TextColor(if owned {
                                                COLOR_ACCENT_GOLD
                                            } else {
                                                COLOR_TEXT_MUTED
                                            }),
                                        ));
                                    });
                            }
                        });

                        if !ready && !col.source_building_ids.is_empty() {
                            card.spawn(Node {
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(6.0),
                                row_gap: Val::Px(4.0),
                                ..default()
                            })
                            .with_children(|hrow| {
                                for bid in &col.source_building_ids {
                                    let src_name = if bid == wyrdstone_core::DAILY_TRIAL_BUILDING_ID
                                    {
                                        Some(i18n.t("city.panel.daily.button").to_string())
                                    } else {
                                        ctx.data
                                            .building_by_id(bid)
                                            .map(|b| i18n.t(&b.name_key).to_string())
                                    };
                                    let Some(name) = src_name else {
                                        continue;
                                    };
                                    hrow.spawn((
                                        Button,
                                        CityAction::HuntCollectionSource(bid.clone()),
                                        Node {
                                            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(1.5)),
                                            ..default()
                                        },
                                        BackgroundColor(COLOR_PANEL_DARK),
                                        BorderColor(COLOR_ACCENT_TEAL),
                                        BorderRadius::all(Val::Px(6.0)),
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(i18n.t_fmt(
                                                "city.panel.collections.hunt",
                                                &[("source", name)],
                                            )),
                                            TextFont {
                                                font_size: FONT_SMALL,
                                                ..default()
                                            },
                                            TextColor(COLOR_ACCENT_TEAL_TEXT),
                                        ));
                                    });
                                }
                            });
                        }

                        card.spawn((
                            Text::new(format_reward_summary(i18n, col.reward_resources, 0)),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GREEN_TEXT),
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
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
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
        safe,
        scroll_offset_y,
        is_mobile,
        |modal| {
            spawn_modal_resource_strip(modal, ctx, vfx);
            for recipe in &ctx.data.tool_shop {
                let count = ctx.city.tools.count(recipe.tool);
                let can_afford = ctx.city.resources.can_afford(&recipe.cost);
                let c = recipe.cost;

                modal
                    .spawn((
                        Node {
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(8.0)),
                            column_gap: Val::Px(10.0),
                            row_gap: Val::Px(8.0),
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
                                spawn_cost_pills(info, i18n, ctx.city.resources, c);
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
            modal.spawn((
                Text::new(i18n.t("city.panel.shop.draughts_title")),
                TextFont {
                    font_size: FONT_HEADING,
                    ..default()
                },
                TextColor(COLOR_ACCENT_GOLD),
            ));
            for draught in wyrdstone_core::WarDraught::ALL {
                let cost = draught.cost();
                let is_active = ctx.city.active_war_draught == Some(draught);
                let can_afford = !is_active && ctx.city.resources.can_afford(&cost);
                modal
                    .spawn((
                        Node {
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(10.0)),
                            column_gap: Val::Px(10.0),
                            row_gap: Val::Px(8.0),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(if is_active {
                            COLOR_ACCENT_GOLD
                        } else {
                            COLOR_PANEL_BORDER
                        }),
                        BorderRadius::all(Val::Px(10.0)),
                    ))
                    .with_children(|row| {
                        row.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 1.0,
                            row_gap: Val::Px(3.0),
                            ..default()
                        })
                        .with_children(|info| {
                            info.spawn((
                                Text::new(i18n.t(draught.name_key())),
                                TextFont {
                                    font_size: FONT_BODY,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GOLD),
                            ));
                            info.spawn((
                                Text::new(i18n.t(draught.desc_key())),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                            spawn_cost_pills(info, i18n, ctx.city.resources, cost);
                        });
                        spawn_small_button_colored(
                            row,
                            if is_active {
                                i18n.t("city.panel.shop.draught_active")
                            } else {
                                i18n.t("city.panel.shop.brew")
                            },
                            CityAction::BrewWarDraught(draught),
                            if is_active {
                                COLOR_ACCENT_GOLD
                            } else if can_afford {
                                COLOR_ACCENT_GREEN
                            } else {
                                COLOR_BUTTON_DISABLED
                            },
                            if is_active {
                                COLOR_TEXT_DARK
                            } else {
                                COLOR_TEXT_ON_ACCENT
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
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
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
        safe,
        scroll_offset_y,
        is_mobile,
        |card| {
            let bal = &ctx.data.balance;
            let rank = ctx.city.clan_rank_with_balance(bal);
            let next_rank_info = match rank {
                ClanRank::Scout => Some((ClanRank::Huscarl, bal.clan_rank_huscarl_glory)),
                ClanRank::Huscarl => Some((ClanRank::Jarl, bal.clan_rank_jarl_glory)),
                ClanRank::Jarl => Some((ClanRank::HighKing, bal.clan_rank_high_king_glory)),
                ClanRank::HighKing => None,
            };
            let rank_line = match next_rank_info {
                Some((next_r, next_glory)) => i18n.t_fmt(
                    "city.panel.stats.rank_progress",
                    &[
                        ("rank", i18n.t(rank.i18n_key()).to_string()),
                        ("glory", ctx.city.glory.to_string()),
                        ("next", next_glory.to_string()),
                        ("next_rank", i18n.t(next_r.i18n_key()).to_string()),
                    ],
                ),
                None => i18n.t_fmt(
                    "city.panel.stats.rank_max",
                    &[
                        ("rank", i18n.t(rank.i18n_key()).to_string()),
                        ("glory", ctx.city.glory.to_string()),
                    ],
                ),
            };
            card.spawn((
                Node {
                    padding: UiRect::all(Val::Px(9.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_CARD),
                BorderColor(COLOR_ACCENT_GOLD),
                BorderRadius::all(Val::Px(RADIUS_MD)),
            ))
            .with_children(|rbox| {
                rbox.spawn((
                    Text::new(rank_line),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_GOLD),
                ));
                if rank == ClanRank::HighKing {
                    let legend = ctx.city.legend_level(bal);
                    let pct = ctx.city.legend_spoils_bonus_pct(bal);
                    let next = ctx.city.glory_for_next_legend(bal);
                    rbox.spawn((
                        Text::new(i18n.t_fmt(
                            "city.panel.stats.legend_progress",
                            &[
                                ("level", legend.to_string()),
                                ("pct", pct.to_string()),
                                ("glory", ctx.city.glory.to_string()),
                                ("next", next.to_string()),
                            ],
                        )),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_TEAL_TEXT),
                    ));
                }
            });

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
                    &[(
                        "val",
                        ctx.city
                            .total_relics_collected(&ctx.data.collections)
                            .to_string(),
                    )],
                ),
                i18n.t_fmt(
                    "city.panel.stats.streak",
                    &[
                        ("val", st.daily_streak.to_string()),
                        ("max", MAX_DAILY_STREAK_CYCLE.to_string()),
                    ],
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
                    "city.panel.stats.best_blitz",
                    &[
                        ("val", st.best_blitz_score.to_string()),
                        ("runs", st.blitz_runs_completed.to_string()),
                    ],
                ),
                i18n.t_fmt(
                    "city.panel.stats.win_streak",
                    &[
                        ("val", st.win_streak.to_string()),
                        ("best", st.best_win_streak.to_string()),
                    ],
                ),
                i18n.t_fmt(
                    "city.panel.stats.trials",
                    &[
                        ("done", ctx.city.completed_trials.len().to_string()),
                        ("total", ctx.data.trials.len().to_string()),
                        ("par", ctx.city.par_trials.len().to_string()),
                    ],
                ),
                i18n.t_fmt(
                    "city.panel.stats.gauntlet",
                    &[
                        ("best", st.best_gauntlet_stage.to_string()),
                        ("total", ctx.data.bosses.len().to_string()),
                        ("clears", st.gauntlets_cleared.to_string()),
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

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_settings_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    sound_bank: &SoundBank,
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
    #[cfg(not(target_os = "android"))] is_fullscreen: bool,
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
        safe,
        scroll_offset_y,
        is_mobile,
        |card| {
            let sound_label = if sound_bank.muted {
                i18n.t("app.audio.off")
            } else {
                i18n.t("app.audio.on")
            };
            let bgm_pct = crate::audio::bgm_volume_percent(sound_bank.bgm_volume);
            let sfx_pct = (sound_bank.sfx_volume * 100.0).round() as u32;
            let vol_bg = if sound_bank.muted {
                COLOR_BUTTON_DISABLED
            } else {
                COLOR_PANEL_CARD
            };

            // Section 1: Audio & Language
            card.spawn((
                Text::new(i18n.t("city.panel.settings.section_audio")),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_ACCENT_GOLD),
            ));
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
                    if sound_bank.muted {
                        COLOR_BUTTON_DISABLED
                    } else {
                        COLOR_ACCENT_TEAL
                    },
                    true,
                );
                spawn_small_button(
                    row,
                    &i18n.t_fmt("city.panel.audio.bgm", &[("pct", bgm_pct.to_string())]),
                    CityAction::CycleBgmVolume,
                    vol_bg,
                    true,
                );
                spawn_small_button(
                    row,
                    &i18n.t_fmt("city.panel.audio.sfx", &[("pct", sfx_pct.to_string())]),
                    CityAction::CycleSfxVolume,
                    vol_bg,
                    true,
                );
                spawn_small_button(
                    row,
                    i18n.t("app.title.button.language"),
                    CityAction::ToggleLanguage,
                    COLOR_PANEL_CARD,
                    true,
                );
                #[cfg(target_arch = "wasm32")]
                if crate::platform::is_fullscreen_supported() {
                    if crate::platform::is_orientation_lock_supported() {
                        spawn_small_button(
                            row,
                            i18n.t("app.display.fullscreen_portrait"),
                            CityAction::EnterFullscreen(
                                crate::platform::FullscreenOrientation::Portrait,
                            ),
                            COLOR_PANEL_CARD,
                            true,
                        );
                        spawn_small_button(
                            row,
                            i18n.t("app.display.fullscreen_landscape"),
                            CityAction::EnterFullscreen(
                                crate::platform::FullscreenOrientation::Landscape,
                            ),
                            COLOR_PANEL_CARD,
                            true,
                        );
                        if is_fullscreen {
                            spawn_small_button(
                                row,
                                i18n.t("app.display.windowed"),
                                CityAction::ToggleFullscreen,
                                COLOR_ACCENT_TEAL,
                                true,
                            );
                        }
                    } else {
                        let fs_label = if is_fullscreen {
                            i18n.t("app.display.windowed")
                        } else {
                            i18n.t("app.display.fullscreen")
                        };
                        spawn_small_button(
                            row,
                            fs_label,
                            CityAction::ToggleFullscreen,
                            if is_fullscreen {
                                COLOR_ACCENT_TEAL
                            } else {
                                COLOR_PANEL_CARD
                            },
                            true,
                        );
                    }
                }
                #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
                if crate::platform::is_fullscreen_supported() {
                    let fs_label = if is_fullscreen {
                        i18n.t("city.settings.display_windowed")
                    } else {
                        i18n.t("city.settings.display_fullscreen")
                    };
                    spawn_small_button(
                        row,
                        fs_label,
                        CityAction::ToggleFullscreen,
                        if is_fullscreen {
                            COLOR_ACCENT_TEAL
                        } else {
                            COLOR_PANEL_CARD
                        },
                        true,
                    );
                }
            });

            // Section 2: Accessibility & Speed
            card.spawn((
                Text::new(i18n.t("city.panel.settings.section_access")),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_ACCENT_GOLD),
                Node {
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                },
            ));
            card.spawn(Node {
                column_gap: Val::Px(10.0),
                row_gap: Val::Px(8.0),
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

            // Section 3: Save Backup & Navigation (Return to Title separated onto its own footer row)
            card.spawn((
                Text::new(i18n.t("city.panel.settings.section_save")),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_ACCENT_GOLD),
                Node {
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                },
            ));
            card.spawn(Node {
                column_gap: Val::Px(10.0),
                row_gap: Val::Px(8.0),
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
            });
            card.spawn(Node {
                margin: UiRect::top(Val::Px(6.0)),
                justify_content: JustifyContent::FlexEnd,
                ..default()
            })
            .with_children(|nav_row| {
                spawn_small_button(
                    nav_row,
                    i18n.t("city.button.title"),
                    CityAction::BackToTitle,
                    COLOR_PANEL_CARD,
                    true,
                );
            });
        },
    );
}

pub(crate) fn spawn_help_topic_cards(
    body: &mut ChildBuilder,
    i18n: &I18nCatalog,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
) {
    let topics: [(&str, &str, Option<Handle<Image>>); 5] = [
        (
            "city.panel.help.controls_title",
            "city.panel.help.controls_body",
            Some(vfx.icon_energy.clone()),
        ),
        (
            "city.panel.help.boosters_title",
            "city.panel.help.boosters_body",
            fjord_art.tool_icon(wyrdstone_core::ActiveTool::Mjolnir),
        ),
        (
            "city.panel.help.obstacles_title",
            "city.panel.help.obstacles_body",
            Some(vfx.icon_stone.clone()),
        ),
        (
            "city.panel.help.fog_curse_title",
            "city.panel.help.fog_curse_body",
            fjord_art.tool_icon(wyrdstone_core::ActiveTool::SurtrEmber),
        ),
        (
            "city.panel.help.village_title",
            "city.panel.help.village_body",
            Some(vfx.icon_gold.clone()),
        ),
    ];
    for (t_key, b_key, icon_opt) in topics {
        body.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(COLOR_PANEL_CARD),
            BorderColor(COLOR_PANEL_BORDER),
            BorderRadius::all(Val::Px(RADIUS_MD)),
        ))
        .with_children(|card| {
            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|hdr| {
                if let Some(icon) = icon_opt {
                    hdr.spawn((
                        ImageNode::new(icon),
                        Node {
                            width: Val::Px(20.0),
                            height: Val::Px(20.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BorderRadius::all(Val::Px(4.0)),
                    ));
                }
                hdr.spawn((
                    Text::new(i18n.t(t_key)),
                    TextFont {
                        font_size: FONT_BODY,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_GOLD),
                ));
            });
            card.spawn((
                Text::new(i18n.t(b_key)),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_TEXT_PRIMARY),
            ));
        });
    }
}

pub(super) fn spawn_help_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
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
        safe,
        scroll_offset_y,
        is_mobile,
        |card| {
            spawn_help_topic_cards(card, i18n, fjord_art, vfx);
        },
    );
}

pub(super) fn spawn_patrons_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
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
        safe,
        scroll_offset_y,
        is_mobile,
        |modal| {
            for god in PatronGod::ALL {
                let active = ctx.city.selected_patron == god;
                let dev_tier = ctx.city.patron_devotion_tier(god);
                let dev_fury = u32::from(dev_tier) * PATRON_DEVOTION_FURY_PCT_PER_TIER;
                let dev_score = u32::from(dev_tier) * PATRON_DEVOTION_SCORE_PCT_PER_TIER;
                let next_dev = (dev_tier < MAX_PATRON_DEVOTION_TIER).then(|| {
                    let next_t = dev_tier + 1;
                    let (cost, req_glory) = patron_devotion_upgrade_cost(next_t);
                    (
                        next_t,
                        cost,
                        req_glory,
                        ctx.city.glory >= req_glory && ctx.city.resources.can_afford(&cost),
                    )
                });
                modal
                    .spawn((
                        Node {
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(10.0)),
                            column_gap: Val::Px(10.0),
                            row_gap: Val::Px(8.0),
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
                                    Text::new(format_patron_desc(
                                        i18n,
                                        god,
                                        &ctx.data.balance.session,
                                    )),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                                info.spawn((
                                    Text::new(
                                        i18n.t_fmt(
                                            "patron.favored_rune_hint",
                                            &[
                                                (
                                                    "rune",
                                                    i18n.t(god.favored_rune().i18n_key())
                                                        .to_string(),
                                                ),
                                                (
                                                    "pct",
                                                    ctx.data
                                                        .balance
                                                        .session
                                                        .patron_favored_rune_fury_pct
                                                        .to_string(),
                                                ),
                                            ],
                                        ),
                                    ),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_TEAL_TEXT),
                                ));
                                info.spawn((
                                    Text::new(i18n.t_fmt(
                                        "patron.devotion.status",
                                        &[
                                            ("tier", dev_tier.to_string()),
                                            ("max", MAX_PATRON_DEVOTION_TIER.to_string()),
                                            ("fury", dev_fury.to_string()),
                                            ("score", dev_score.to_string()),
                                        ],
                                    )),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                                info.spawn((
                                    Text::new(i18n.t("patron.devotion.milestones")),
                                    TextFont {
                                        font_size: FONT_SMALL - 1.0,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_MUTED),
                                ));
                                if let Some((_, cost, req_glory, _)) = next_dev {
                                    spawn_cost_pills(info, i18n, ctx.city.resources, cost);
                                    info.spawn((
                                        Text::new(i18n.t_fmt(
                                            "patron.devotion.req_glory",
                                            &[("glory", req_glory.to_string())],
                                        )),
                                        TextFont {
                                            font_size: FONT_SMALL - 1.0,
                                            ..default()
                                        },
                                        TextColor(if ctx.city.glory >= req_glory {
                                            COLOR_ACCENT_GREEN_TEXT
                                        } else {
                                            COLOR_ACCENT_CRIMSON_TEXT
                                        }),
                                    ));
                                }
                            });
                        });
                        row.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexEnd,
                            row_gap: Val::Px(6.0),
                            ..default()
                        })
                        .with_children(|acts| {
                            spawn_small_button_colored(
                                acts,
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
                            if let Some((next_t, _, _, can_upgrade)) = next_dev {
                                spawn_small_button_colored(
                                    acts,
                                    &i18n.t_fmt(
                                        "patron.devotion.upgrade",
                                        &[("tier", next_t.to_string())],
                                    ),
                                    CityAction::UpgradePatronDevotion(god),
                                    if can_upgrade {
                                        COLOR_ACCENT_GREEN
                                    } else {
                                        COLOR_BUTTON_DISABLED
                                    },
                                    if can_upgrade {
                                        COLOR_TEXT_ON_ACCENT
                                    } else {
                                        COLOR_TEXT_MUTED
                                    },
                                    can_upgrade,
                                );
                            } else {
                                acts.spawn((
                                    Text::new(i18n.t("patron.devotion.maxed")),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                            }
                        });
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
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
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
        safe,
        scroll_offset_y,
        is_mobile,
        |modal| {
            let glory_lbl = i18n.t_fmt(
                "city.header.mastery",
                &[("glory", ctx.city.glory.to_string())],
            );
            let energy_lbl = i18n.t_fmt(
                "city.header.energy_full",
                &[
                    ("cur", ctx.city.energy.to_string()),
                    ("max", ctx.city.max_energy(&ctx.data.balance).to_string()),
                ],
            );
            modal
                .spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|hdr| {
                    hdr.spawn(Node {
                        column_gap: Val::Px(8.0),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    })
                    .with_children(|pills| {
                        spawn_resource_pill(pills, &glory_lbl, COLOR_ACCENT_GOLD);
                        spawn_resource_pill(pills, &energy_lbl, COLOR_RES_ENERGY);
                    });
                    hdr.spawn(Node {
                        column_gap: Val::Px(6.0),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    })
                    .with_children(|loadout| {
                        spawn_small_button(
                            loadout,
                            i18n.t(ctx.city.selected_patron.name_key()),
                            CityAction::CyclePatron,
                            COLOR_PANEL_CARD,
                            true,
                        );
                        let amulet_lbl = ctx
                            .city
                            .equipped_amulet
                            .as_deref()
                            .and_then(|id| ctx.data.amulet_by_id(id))
                            .map(|a| i18n.t(&a.name_key))
                            .unwrap_or_else(|| i18n.t("city.loadout.amulet_none"));
                        spawn_small_button(
                            loadout,
                            amulet_lbl,
                            CityAction::CycleAmulet,
                            if ctx.city.equipped_amulet.is_some() {
                                COLOR_ACCENT_TEAL
                            } else {
                                COLOR_PANEL_CARD
                            },
                            true,
                        );
                    });
                });

            let gauntlet_unlocked = ctx.city.glory >= 50 || !ctx.city.defeated_bosses.is_empty();
            modal
                .spawn((
                    Node {
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(12.0)),
                        column_gap: Val::Px(12.0),
                        row_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.20, 0.10, 0.14, 0.96)),
                    BorderColor(COLOR_ACCENT_GOLD),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .with_children(|gbox| {
                    gbox.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 1.0,
                        row_gap: Val::Px(3.0),
                        ..default()
                    })
                    .with_children(|info| {
                        info.spawn((
                            Text::new(i18n.t("city.panel.raids.gauntlet_title")),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                        info.spawn((
                            Text::new(i18n.t_fmt(
                                "city.panel.raids.gauntlet_desc",
                                &[
                                    ("moves", wyrdstone_core::GAUNTLET_STARTING_MOVES.to_string()),
                                    (
                                        "bonus",
                                        wyrdstone_core::GAUNTLET_STAGE_BONUS_MOVES.to_string(),
                                    ),
                                    ("glory", wyrdstone_core::GAUNTLET_CLEAR_GLORY.to_string()),
                                ],
                            )),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_PRIMARY),
                        ));
                        info.spawn((
                            Text::new(i18n.t_fmt(
                                "city.panel.raids.gauntlet_stats",
                                &[
                                    ("best", ctx.city.stats.best_gauntlet_stage.to_string()),
                                    ("total", ctx.data.bosses.len().to_string()),
                                    ("clears", ctx.city.stats.gauntlets_cleared.to_string()),
                                ],
                            )),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_TEAL_TEXT),
                        ));
                    });
                    if gauntlet_unlocked {
                        let glbl = i18n.t_fmt(
                            "city.panel.raids.gauntlet_enter",
                            &[("cost", energy_cost.to_string())],
                        );
                        spawn_small_button_colored(
                            gbox,
                            &glbl,
                            CityAction::StartGauntlet,
                            if has_energy {
                                COLOR_ACCENT_GOLD
                            } else {
                                COLOR_BUTTON_DISABLED
                            },
                            if has_energy {
                                COLOR_TEXT_DARK
                            } else {
                                COLOR_TEXT_MUTED
                            },
                            has_energy,
                        );
                    } else {
                        spawn_resource_pill(
                            gbox,
                            i18n.t("city.panel.raids.gauntlet_locked"),
                            COLOR_TEXT_MUTED,
                        );
                    }
                });

            for boss in &ctx.data.bosses {
                let unlocked = ctx.city.glory >= boss.required_glory;
                let slain = ctx.city.defeated_bosses.contains(&boss.id);
                modal
                    .spawn((
                        Node {
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(12.0)),
                            column_gap: Val::Px(12.0),
                            row_gap: Val::Px(8.0),
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
                                    Text::new(format_boss_desc(i18n, boss)),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                                info.spawn((
                                    Text::new(
                                        i18n.t_fmt(
                                            "city.panel.raids.weakness",
                                            &[
                                                (
                                                    "rune",
                                                    i18n.t(boss.weakness.i18n_key()).to_string(),
                                                ),
                                                (
                                                    "mult",
                                                    ctx.data
                                                        .balance
                                                        .boss
                                                        .weakness_multiplier
                                                        .to_string(),
                                                ),
                                                ("hp", boss.max_hp.to_string()),
                                            ],
                                        ),
                                    ),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_TEAL_TEXT),
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
                                    TextColor(COLOR_ACCENT_GREEN_TEXT),
                                ));
                                if let Some(aid_key) =
                                    ctx.city.valkyrie_aid_for_level(&boss.id).desc_key()
                                {
                                    info.spawn((
                                        Text::new(i18n.t(aid_key)),
                                        TextFont {
                                            font_size: FONT_SMALL,
                                            ..default()
                                        },
                                        TextColor(COLOR_ACCENT_GOLD),
                                    ));
                                }
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

pub(super) fn spawn_trials_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        640.0,
        i18n.t("city.panel.trials.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::ToggleModal(ActiveModal::Trials),
        safe,
        scroll_offset_y,
        is_mobile,
        |modal| {
            modal.spawn((
                Text::new(i18n.t_fmt(
                    "city.panel.trials.subtitle",
                    &[
                        ("done", ctx.city.completed_trials.len().to_string()),
                        ("total", ctx.data.trials.len().to_string()),
                        ("par", ctx.city.par_trials.len().to_string()),
                    ],
                )),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_ACCENT_TEAL_TEXT),
            ));

            for trial in &ctx.data.trials {
                let cleared = ctx.city.completed_trials.contains(&trial.id);
                let par_done = ctx.city.par_trials.contains(&trial.id);
                modal
                    .spawn((
                        Node {
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(12.0)),
                            column_gap: Val::Px(12.0),
                            row_gap: Val::Px(8.0),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(if par_done {
                            COLOR_ACCENT_GOLD
                        } else if cleared {
                            COLOR_ACCENT_GREEN
                        } else {
                            COLOR_PANEL_BORDER
                        }),
                        BorderRadius::all(Val::Px(10.0)),
                    ))
                    .with_children(|row| {
                        row.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 1.0,
                            row_gap: Val::Px(3.0),
                            ..default()
                        })
                        .with_children(|info| {
                            info.spawn((
                                Text::new(i18n.t(&trial.name_key)),
                                TextFont {
                                    font_size: FONT_BODY,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GOLD),
                            ));
                            info.spawn((
                                Text::new(i18n.t(&trial.hint_key)),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                            let meta = i18n.t_fmt(
                                "city.panel.trials.moves",
                                &[
                                    ("par", trial.par_moves.to_string()),
                                    ("max", trial.max_moves.to_string()),
                                ],
                            );
                            info.spawn((
                                Text::new(meta),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_TEAL_TEXT),
                            ));
                            if par_done {
                                info.spawn((
                                    Text::new(i18n.t("city.panel.trials.par_seal")),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                            } else if cleared {
                                info.spawn((
                                    Text::new(i18n.t("city.panel.trials.solved")),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GREEN_TEXT),
                                ));
                            } else {
                                info.spawn((
                                    Text::new(format_reward_summary(
                                        i18n,
                                        trial.first_clear_rewards,
                                        trial.first_clear_glory,
                                    )),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GREEN_TEXT),
                                ));
                            }
                        });

                        spawn_small_button_colored(
                            row,
                            if cleared {
                                i18n.t("city.panel.trials.replay")
                            } else {
                                i18n.t("city.panel.trials.solve")
                            },
                            CityAction::StartTrial(trial.id.clone()),
                            if par_done {
                                COLOR_PANEL_CARD
                            } else {
                                COLOR_ACCENT_TEAL
                            },
                            if par_done {
                                COLOR_TEXT_PRIMARY
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

pub(super) fn spawn_achievements_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
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
        safe,
        scroll_offset_y,
        is_mobile,
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
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(9.0)),
                            column_gap: Val::Px(10.0),
                            row_gap: Val::Px(8.0),
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
                                Text::new(format_achievement_desc(i18n, ach)),
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
                                TextColor(COLOR_ACCENT_GREEN_TEXT),
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
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
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
        safe,
        scroll_offset_y,
        is_mobile,
        |modal| {
            if (run.wave + 1).is_multiple_of(5) {
                modal.spawn((
                    Text::new(i18n.t_fmt(
                        "city.panel.expedition.next_boss_wave",
                        &[("wave", (run.wave + 1).to_string())],
                    )),
                    TextFont {
                        font_size: FONT_BODY,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_CRIMSON_TEXT),
                ));
            }
            let mut active_parts = Vec::new();
            for &relic in &run.relics {
                active_parts.push(i18n.t(relic.name_key()).to_string());
            }
            if run.extra_moves > 0 {
                active_parts.push(i18n.t_fmt(
                    "expedition.boon.moves.desc",
                    &[("val", run.extra_moves.to_string())],
                ));
            }
            if run.score_bonus_pct > 0 {
                active_parts.push(i18n.t_fmt(
                    "expedition.boon.score.desc",
                    &[("val", run.score_bonus_pct.to_string())],
                ));
            }
            if run.starting_boosters > 0 {
                active_parts.push(i18n.t_fmt(
                    "city.panel.expedition.active_boosters",
                    &[("val", run.starting_boosters.to_string())],
                ));
            }
            if run.fury_headstart > 0 {
                active_parts.push(i18n.t_fmt(
                    "expedition.boon.fury.desc",
                    &[("val", run.fury_headstart.to_string())],
                ));
            }
            if !active_parts.is_empty() {
                modal
                    .spawn((
                        Node {
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(6.0),
                            row_gap: Val::Px(4.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(COLOR_ACCENT_TEAL),
                        BorderRadius::all(Val::Px(RADIUS_MD)),
                    ))
                    .with_children(|strip| {
                        for part in active_parts {
                            spawn_resource_pill(strip, &part, COLOR_ACCENT_TEAL_TEXT);
                        }
                    });
            }

            for (idx, boon) in run.pending_boons.iter().enumerate() {
                let is_relic = matches!(boon, wyrdstone_core::ExpeditionBoon::Relic(_));
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
                        BorderColor(if is_relic {
                            COLOR_ACCENT_TEAL
                        } else {
                            COLOR_ACCENT_GOLD
                        }),
                        BorderRadius::all(Val::Px(10.0)),
                    ))
                    .with_children(|card| {
                        let title_lbl = if is_relic {
                            format!(
                                "{} {}",
                                i18n.t("city.panel.expedition.relic_tag"),
                                i18n.t(boon.name_key())
                            )
                        } else {
                            i18n.t(boon.name_key()).to_string()
                        };
                        card.spawn((
                            Text::new(title_lbl),
                            TextFont {
                                font_size: FONT_HEADING,
                                ..default()
                            },
                            TextColor(if is_relic {
                                COLOR_ACCENT_TEAL_TEXT
                            } else {
                                COLOR_ACCENT_GOLD
                            }),
                        ));
                        card.spawn((
                            Text::new(format_boon_desc(i18n, *boon)),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_PRIMARY),
                        ));
                    });
            }

            if let Some(oath) = run.pending_oath {
                let accepted = run.oath_accepted;
                modal
                    .spawn((
                        Node {
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(10.0)),
                            column_gap: Val::Px(10.0),
                            row_gap: Val::Px(6.0),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(if accepted {
                            Color::srgba(0.32, 0.14, 0.14, 0.94)
                        } else {
                            COLOR_PANEL_CARD
                        }),
                        BorderColor(if accepted {
                            COLOR_ACCENT_CRIMSON
                        } else {
                            COLOR_PANEL_BORDER
                        }),
                        BorderRadius::all(Val::Px(10.0)),
                    ))
                    .with_children(|orow| {
                        orow.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 1.0,
                            row_gap: Val::Px(2.0),
                            ..default()
                        })
                        .with_children(|oinfo| {
                            oinfo.spawn((
                                Text::new(i18n.t(oath.name_key())),
                                TextFont {
                                    font_size: FONT_BODY,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_CRIMSON_TEXT),
                            ));
                            oinfo.spawn((
                                Text::new(i18n.t(oath.desc_key())),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                        });
                        spawn_small_button_colored(
                            orow,
                            if accepted {
                                i18n.t("city.panel.expedition.oath_active")
                            } else {
                                i18n.t("city.panel.expedition.oath_accept")
                            },
                            CityAction::ToggleExpeditionOath,
                            if accepted {
                                COLOR_ACCENT_CRIMSON
                            } else {
                                COLOR_ACCENT_GOLD
                            },
                            if accepted {
                                COLOR_TEXT_PRIMARY
                            } else {
                                COLOR_TEXT_DARK
                            },
                            true,
                        );
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
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
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
        safe,
        scroll_offset_y,
        is_mobile,
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
                    COLOR_ACCENT_GREEN_TEXT
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
                                    COLOR_ACCENT_GREEN_TEXT
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
                                        BorderRadius::all(Val::Px(RADIUS_SM)),
                                    ));
                                }
                                if let Some(recipe) =
                                    ctx.data.tool_shop.iter().find(|r| r.tool == t)
                                {
                                    card.spawn((
                                        Text::new(i18n.t_fmt(
                                            &recipe.name_key,
                                            &[("count", "+1".to_string())],
                                        )),
                                        TextFont {
                                            font_size: FONT_SMALL - 1.5,
                                            ..default()
                                        },
                                        TextColor(COLOR_ACCENT_TEAL_TEXT),
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
                                    TextColor(COLOR_ACCENT_GREEN_TEXT),
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

            modal
                .spawn(Node {
                    justify_content: JustifyContent::Center,
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                })
                .with_children(|row| {
                    if available {
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
                    } else {
                        spawn_small_button_colored(
                            row,
                            i18n.t("city.chapter.continue"),
                            CityAction::CloseModal,
                            COLOR_ACCENT_GOLD,
                            COLOR_TEXT_DARK,
                            true,
                        );
                    }
                });
        },
    );
}

pub(super) fn spawn_chapter_dialogue_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
) {
    let i18n = &ctx.data.i18n;
    let Some(dlg) = &ctx.pending_dialogue else {
        return;
    };
    spawn_modal_shell_ex(
        root,
        ctx,
        fjord_art,
        vfx,
        580.0,
        i18n.t(&dlg.title_key),
        i18n.t("city.chapter.continue"),
        CityAction::CloseModal,
        safe,
        scroll_offset_y,
        is_mobile,
        false,
        false,
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

pub(super) fn spawn_welcome_back_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
) {
    let i18n = &ctx.data.i18n;
    let Some(summary) = &ctx.pending_offline_vault else {
        return;
    };
    let mins = (summary.capped_secs / 60).max(1);
    let hours = mins / 60;
    let rem_mins = mins % 60;
    let away_text = if hours > 0 {
        i18n.t_fmt(
            "city.vault.away_hours",
            &[
                ("hours", hours.to_string()),
                ("mins", format!("{rem_mins:02}")),
                ("cap", summary.max_hours.to_string()),
            ],
        )
    } else {
        i18n.t_fmt(
            "city.vault.away_mins",
            &[
                ("mins", mins.to_string()),
                ("cap", summary.max_hours.to_string()),
            ],
        )
    };

    spawn_modal_shell_ex(
        root,
        ctx,
        fjord_art,
        vfx,
        540.0,
        i18n.t("city.vault.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::CloseModal,
        safe,
        scroll_offset_y,
        is_mobile,
        false,
        false,
        |modal| {
            modal.spawn((
                Text::new(away_text),
                TextFont {
                    font_size: FONT_BODY,
                    ..default()
                },
                TextColor(COLOR_ACCENT_TEAL_TEXT),
            ));
            modal.spawn((
                Text::new(i18n.t("city.vault.subtitle")),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_TEXT_PRIMARY),
            ));
            modal
                .spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(6.0),
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                })
                .with_children(|row| {
                    for (icon, amount, col) in [
                        (vfx.icon_wood.clone(), summary.rewards.wood, COLOR_RES_WOOD),
                        (
                            vfx.icon_stone.clone(),
                            summary.rewards.stone,
                            COLOR_RES_STONE,
                        ),
                        (
                            vfx.icon_gold.clone(),
                            summary.rewards.gold,
                            COLOR_ACCENT_GOLD,
                        ),
                        (vfx.icon_mead.clone(), summary.rewards.mead, COLOR_RES_MEAD),
                    ] {
                        if amount > 0 {
                            spawn_resource_pill_with_icon(row, icon, &format!("+{amount}"), col);
                        }
                    }
                });
            modal
                .spawn(Node {
                    justify_content: JustifyContent::Center,
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                })
                .with_children(|row| {
                    spawn_small_button_colored(
                        row,
                        i18n.t("city.vault.claim"),
                        CityAction::ClaimOfflineVault,
                        COLOR_ACCENT_GOLD,
                        COLOR_TEXT_DARK,
                        true,
                    );
                });
        },
    );
}

pub(super) fn spawn_bounties_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        720.0,
        i18n.t("city.panel.bounties.title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::CloseModal,
        safe,
        scroll_offset_y,
        is_mobile,
        |modal| {
            let claimable_bounties = ctx.city.daily_bounties.claimable_count();
            modal
                .spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|hdr| {
                    hdr.spawn((
                        Text::new(i18n.t("city.bounties.section_daily")),
                        display_font(fjord_art, FONT_HEADING),
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                    if claimable_bounties > 1 {
                        spawn_small_button_colored(
                            hdr,
                            i18n.t("city.bounties.claim_all"),
                            CityAction::ClaimAllBounties,
                            COLOR_ACCENT_GREEN,
                            COLOR_TEXT_ON_ACCENT,
                            true,
                        );
                    }
                });

            for (idx, bounty) in ctx.city.daily_bounties.bounties.iter().enumerate() {
                let claimable = bounty.is_claimable();
                let summary = format_reward_summary(i18n, bounty.reward, 0);
                modal
                    .spawn((
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(10.0),
                            row_gap: Val::Px(6.0),
                            padding: UiRect::all(Val::Px(10.0)),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(if claimable {
                            COLOR_ACCENT_GREEN
                        } else {
                            COLOR_PANEL_BORDER
                        }),
                        BorderRadius::all(Val::Px(RADIUS_MD)),
                    ))
                    .with_children(|row| {
                        row.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(3.0),
                            flex_shrink: 1.0,
                            ..default()
                        })
                        .with_children(|col| {
                            col.spawn((
                                Text::new(i18n.t_fmt(
                                    bounty.kind.desc_key(),
                                    &[("target", bounty.target.to_string())],
                                )),
                                TextFont {
                                    font_size: FONT_BODY,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                            col.spawn((
                                Text::new(i18n.t_fmt(
                                    "city.bounties.progress_reward",
                                    &[
                                        ("done", bounty.progress.to_string()),
                                        ("target", bounty.target.to_string()),
                                        ("reward", summary),
                                        ("xp", bounty.skald_xp.to_string()),
                                    ],
                                )),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(if bounty.is_complete() {
                                    COLOR_ACCENT_GREEN_TEXT
                                } else {
                                    COLOR_ACCENT_TEAL_TEXT
                                }),
                            ));
                        });
                        let (lbl, bg, fg, enabled) = if bounty.claimed {
                            (
                                i18n.t("city.bounties.claimed"),
                                COLOR_BUTTON_DISABLED,
                                COLOR_TEXT_MUTED,
                                false,
                            )
                        } else if claimable {
                            (
                                i18n.t("city.bounties.claim"),
                                COLOR_ACCENT_GREEN,
                                COLOR_TEXT_ON_ACCENT,
                                true,
                            )
                        } else {
                            (
                                i18n.t("city.bounties.in_progress"),
                                COLOR_BUTTON_DISABLED,
                                COLOR_TEXT_MUTED,
                                false,
                            )
                        };
                        spawn_small_button_colored(
                            row,
                            lbl,
                            CityAction::ClaimBounty(idx),
                            bg,
                            fg,
                            enabled,
                        );
                    });
            }

            let cur_tier = ctx.city.skald_pass.current_tier();
            let xp_cap = u32::from(SKALD_PASS_TIERS) * SKALD_PASS_XP_PER_TIER;
            let xp_shown = ctx.city.skald_pass.xp.min(xp_cap);
            let pass_cycle = ctx.city.skald_pass.cycle;
            let can_advance = ctx.city.skald_pass.can_advance_cycle();
            let claimable_tiers = ctx.city.skald_pass.claimable_count();
            modal
                .spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|hdr| {
                    hdr.spawn((
                        Text::new(i18n.t_fmt(
                            "city.skald_pass.header",
                            &[
                                ("tier", cur_tier.to_string()),
                                ("max", SKALD_PASS_TIERS.to_string()),
                                ("xp", xp_shown.to_string()),
                                ("xp_cap", xp_cap.to_string()),
                            ],
                        )),
                        display_font(fjord_art, FONT_HEADING),
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                    if pass_cycle > 0 {
                        let bonus_pct = pass_cycle
                            .saturating_mul(SKALD_PASS_CYCLE_BONUS_PCT)
                            .min(SKALD_PASS_CYCLE_MAX_BONUS_PCT);
                        spawn_resource_pill(
                            hdr,
                            &i18n.t_fmt(
                                "city.skald_pass.cycle_badge",
                                &[
                                    ("cycle", (pass_cycle + 1).to_string()),
                                    ("bonus", bonus_pct.to_string()),
                                ],
                            ),
                            COLOR_ACCENT_TEAL_TEXT,
                        );
                    }
                    if can_advance {
                        spawn_small_button_colored(
                            hdr,
                            &i18n.t_fmt(
                                "city.skald_pass.advance_cycle",
                                &[("next", (pass_cycle + 2).to_string())],
                            ),
                            CityAction::AdvanceSkaldCycle,
                            COLOR_ACCENT_GOLD,
                            COLOR_TEXT_DARK,
                            true,
                        );
                    } else if claimable_tiers > 1 {
                        spawn_small_button_colored(
                            hdr,
                            &i18n.t_fmt(
                                "city.panel.claim_all",
                                &[("count", claimable_tiers.to_string())],
                            ),
                            CityAction::ClaimAllSkaldTiers,
                            COLOR_ACCENT_GOLD,
                            COLOR_TEXT_DARK,
                            true,
                        );
                    }
                });

            let mut ordered_tiers: Vec<u8> = (1..=SKALD_PASS_TIERS).collect();
            ordered_tiers.sort_by_key(|&tier| {
                let unlocked = cur_tier >= tier;
                let claimed = ctx.city.skald_pass.is_tier_claimed(tier);
                let bucket = if unlocked && !claimed {
                    0u8
                } else if !unlocked {
                    1u8
                } else {
                    2u8
                };
                (bucket, tier)
            });

            for tier in ordered_tiers {
                let unlocked = cur_tier >= tier;
                let claimed = ctx.city.skald_pass.is_tier_claimed(tier);
                let claimable = unlocked && !claimed;
                let (res, tool_opt, glory) = skald_pass_tier_reward_for_cycle(tier, pass_cycle);
                let mut reward_text = format_reward_summary(i18n, res, glory);
                if let Some(tool) = tool_opt {
                    if let Some(recipe) = ctx.data.tool_shop.iter().find(|r| r.tool == tool) {
                        let tool_label =
                            i18n.t_fmt(&recipe.name_key, &[("count", "+1".to_string())]);
                        reward_text.push_str(&format!(" + {tool_label}"));
                    }
                }
                modal
                    .spawn((
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(10.0),
                            row_gap: Val::Px(6.0),
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(if claimable {
                            COLOR_ACCENT_GOLD
                        } else {
                            COLOR_PANEL_BORDER
                        }),
                        BorderRadius::all(Val::Px(RADIUS_SM)),
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(i18n.t_fmt(
                                "city.skald_pass.tier_line",
                                &[("tier", tier.to_string()), ("reward", reward_text)],
                            )),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(if unlocked {
                                COLOR_TEXT_PRIMARY
                            } else {
                                COLOR_TEXT_MUTED
                            }),
                        ));
                        let (lbl, bg, fg, enabled) = if claimed {
                            (
                                i18n.t("city.bounties.claimed"),
                                COLOR_BUTTON_DISABLED,
                                COLOR_TEXT_MUTED,
                                false,
                            )
                        } else if claimable {
                            (
                                i18n.t("city.bounties.claim"),
                                COLOR_ACCENT_GOLD,
                                COLOR_TEXT_DARK,
                                true,
                            )
                        } else {
                            (
                                i18n.t("city.panel.building.badge.locked"),
                                COLOR_BUTTON_DISABLED,
                                COLOR_TEXT_MUTED,
                                false,
                            )
                        };
                        spawn_small_button_colored(
                            row,
                            lbl,
                            CityAction::ClaimSkaldTier(tier),
                            bg,
                            fg,
                            enabled,
                        );
                    });
            }
        },
    );
}

pub(super) fn spawn_wonder_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
) {
    let i18n = &ctx.data.i18n;
    let stage = ctx.city.wonder_stage;
    let next_cost = ctx.city.next_wonder_cost();
    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        560.0,
        i18n.t("city.wonder.modal_title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::CloseModal,
        safe,
        scroll_offset_y,
        is_mobile,
        |modal| {
            modal
                .spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|hdr| {
                    hdr.spawn((
                        ImageNode::new(vfx.yggdrasil_wonder.clone()),
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                    hdr.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        flex_shrink: 1.0,
                        ..default()
                    })
                    .with_children(|info| {
                        info.spawn((
                            Text::new(i18n.t("city.wonder.subtitle")),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_PRIMARY),
                        ));
                        info.spawn((
                            Text::new(i18n.t_fmt(
                                "city.wonder.stage_badge",
                                &[
                                    ("stage", stage.to_string()),
                                    ("max", WONDER_MAX_STAGE.to_string()),
                                ],
                            )),
                            display_font(fjord_art, FONT_BODY),
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                    });
                });

            // 10-pip radiant Yggdrasil infusion bar (never attach BoxShadow inside ScrollablePanel).
            modal
                .spawn(Node {
                    width: Val::Percent(100.0),
                    column_gap: Val::Px(5.0),
                    ..default()
                })
                .with_children(|pips| {
                    for s in 1..=WONDER_MAX_STAGE {
                        let filled = stage >= s;
                        pips.spawn((
                            Node {
                                flex_grow: 1.0,
                                height: Val::Px(10.0),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(if filled {
                                COLOR_ACCENT_GREEN
                            } else {
                                COLOR_PANEL_DARK
                            }),
                            BorderColor(if filled {
                                COLOR_ACCENT_GOLD
                            } else {
                                COLOR_PANEL_BORDER
                            }),
                            BorderRadius::all(Val::Px(3.0)),
                        ));
                    }
                });

            // Active blessings summary card.
            modal
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    BackgroundColor(COLOR_PANEL_CARD),
                    BorderColor(COLOR_PANEL_BORDER),
                    BorderRadius::all(Val::Px(RADIUS_SM)),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new(i18n.t_fmt(
                            "city.wonder.active_bonuses",
                            &[
                                ("dmg", wonder_boss_damage_bonus_pct(stage).to_string()),
                                ("spoils", wonder_spoils_bonus_pct(stage).to_string()),
                                ("harvest", wonder_harvest_bonus_pct(stage).to_string()),
                                ("energy", wonder_extra_max_energy(stage).to_string()),
                                ("fury", wonder_starting_fury(stage).to_string()),
                            ],
                        )),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GREEN_TEXT),
                    ));
                });

            if let Some(cost) = next_cost {
                let next_stage = stage.saturating_add(1);
                let next_glory = wonder_glory_reward(next_stage);
                let next_tool = wonder_milestone_tool(next_stage);
                let affordable = ctx.city.resources.can_afford(&cost);

                modal
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            padding: UiRect::all(Val::Px(12.0)),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(if affordable {
                            COLOR_ACCENT_GOLD
                        } else {
                            COLOR_PANEL_BORDER
                        }),
                        BorderRadius::all(Val::Px(RADIUS_MD)),
                    ))
                    .with_children(|card| {
                        card.spawn((
                            Text::new(i18n.t_fmt(
                                "city.wonder.next_stage",
                                &[
                                    ("next", next_stage.to_string()),
                                    ("max", WONDER_MAX_STAGE.to_string()),
                                ],
                            )),
                            display_font(fjord_art, FONT_BODY),
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                        card.spawn((
                            Text::new(i18n.t_fmt(
                                "city.wonder.next_rewards",
                                &[("glory", next_glory.to_string())],
                            )),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_PRIMARY),
                        ));
                        if let Some(tool) = next_tool {
                            if let Some(recipe) = ctx.data.tool_shop.iter().find(|r| r.tool == tool)
                            {
                                let tool_lbl =
                                    i18n.t_fmt(&recipe.name_key, &[("count", "+1".to_string())]);
                                card.spawn((
                                    Text::new(i18n.t_fmt(
                                        "city.wonder.milestone_tool",
                                        &[("tool", tool_lbl)],
                                    )),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_TEAL_TEXT),
                                ));
                            }
                        }
                        spawn_cost_pills(card, i18n, ctx.city.resources, cost);
                        spawn_small_button_colored(
                            card,
                            i18n.t("city.wonder.infuse_button"),
                            CityAction::UpgradeWonder,
                            if affordable {
                                COLOR_ACCENT_GOLD
                            } else {
                                COLOR_BUTTON_DISABLED
                            },
                            if affordable {
                                COLOR_TEXT_DARK
                            } else {
                                COLOR_TEXT_MUTED
                            },
                            affordable,
                        );
                    });
            } else {
                modal
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            padding: UiRect::all(Val::Px(12.0)),
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_CARD),
                        BorderColor(COLOR_ACCENT_GOLD),
                        BorderRadius::all(Val::Px(RADIUS_MD)),
                    ))
                    .with_children(|card| {
                        card.spawn((
                            Text::new(i18n.t("city.wonder.maxed_banner")),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                    });
            }
        },
    );
}

pub(super) fn spawn_ascension_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    safe: SafeAreaInsets,
    scroll_offset_y: f32,
    is_mobile: bool,
) {
    let i18n = &ctx.data.i18n;
    let cur_era = ctx.city.realm_era;
    let next_era = cur_era.saturating_add(1);
    let cur_theme = RealmTheme::for_era(cur_era);
    let next_theme = RealmTheme::for_era(next_era);
    let can_ascend = ctx.city.can_ascend_realm(&ctx.data.buildings);
    let glory_reward = realm_ascension_glory_reward(next_era);

    let effective_era = cur_era.min(REALM_MAX_SCALING_ERA);
    let cur_spoils = effective_era.saturating_mul(REALM_SPOILS_PCT_PER_ERA);
    let cur_score = effective_era.saturating_mul(REALM_SCORE_PCT_PER_ERA);
    let cur_boss = effective_era.saturating_mul(REALM_BOSS_DMG_PCT_PER_ERA);
    let next_effective = next_era.min(REALM_MAX_SCALING_ERA);
    let next_spoils = next_effective.saturating_mul(REALM_SPOILS_PCT_PER_ERA);
    let next_score = next_effective.saturating_mul(REALM_SCORE_PCT_PER_ERA);
    let next_boss = next_effective.saturating_mul(REALM_BOSS_DMG_PCT_PER_ERA);

    spawn_modal_shell(
        root,
        ctx,
        fjord_art,
        vfx,
        580.0,
        i18n.t("city.ascension.modal_title"),
        i18n.t("city.panel.amulets.close"),
        CityAction::CloseModal,
        safe,
        scroll_offset_y,
        is_mobile,
        |modal| {
            // Current & Next Realm header card (no BoxShadow inside ScrollablePanel).
            modal
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    BackgroundColor(COLOR_PANEL_CARD),
                    BorderColor(COLOR_ACCENT_GOLD),
                    BorderRadius::all(Val::Px(RADIUS_MD)),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new(i18n.t_fmt(
                            "city.ascension.current_realm",
                            &[
                                ("realm", i18n.t(cur_theme.name_key()).to_string()),
                                ("era", (cur_era + 1).to_string()),
                            ],
                        )),
                        display_font(fjord_art, FONT_BODY),
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                    card.spawn((
                        Text::new(i18n.t(cur_theme.desc_key())),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_MUTED),
                    ));
                    card.spawn((
                        Text::new(i18n.t_fmt(
                            "city.ascension.current_bonuses",
                            &[
                                ("spoils", cur_spoils.to_string()),
                                ("score", cur_score.to_string()),
                                ("dmg", cur_boss.to_string()),
                            ],
                        )),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_TEAL_TEXT),
                    ));
                });

            // Destination preview card.
            modal
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    BackgroundColor(COLOR_PANEL_CARD),
                    BorderColor(if can_ascend {
                        COLOR_ACCENT_GREEN
                    } else {
                        COLOR_PANEL_BORDER
                    }),
                    BorderRadius::all(Val::Px(RADIUS_MD)),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new(i18n.t_fmt(
                            "city.ascension.next_realm",
                            &[
                                ("realm", i18n.t(next_theme.name_key()).to_string()),
                                ("era", (next_era + 1).to_string()),
                                ("glory", glory_reward.to_string()),
                            ],
                        )),
                        display_font(fjord_art, FONT_BODY),
                        TextColor(COLOR_ACCENT_GREEN_TEXT),
                    ));
                    card.spawn((
                        Text::new(i18n.t_fmt(
                            "city.ascension.next_bonuses",
                            &[
                                ("spoils", next_spoils.to_string()),
                                ("score", next_score.to_string()),
                                ("dmg", next_boss.to_string()),
                            ],
                        )),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_PRIMARY),
                    ));
                });

            // Kept Forever vs Rebuilt comparison cards.
            modal
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(COLOR_PANEL_CARD),
                    BorderColor(COLOR_ACCENT_GREEN),
                    BorderRadius::all(Val::Px(RADIUS_SM)),
                ))
                .with_children(|kept| {
                    kept.spawn((
                        Text::new(i18n.t("city.ascension.kept_title")),
                        display_font(fjord_art, FONT_SMALL),
                        TextColor(COLOR_ACCENT_GREEN_TEXT),
                    ));
                    kept.spawn((
                        Text::new(i18n.t("city.ascension.kept_desc")),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_PRIMARY),
                    ));
                });

            modal
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(COLOR_PANEL_CARD),
                    BorderColor(COLOR_PANEL_BORDER),
                    BorderRadius::all(Val::Px(RADIUS_SM)),
                ))
                .with_children(|reset_card| {
                    reset_card.spawn((
                        Text::new(i18n.t("city.ascension.reset_title")),
                        display_font(fjord_art, FONT_SMALL),
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                    reset_card.spawn((
                        Text::new(i18n.t_fmt(
                            "city.ascension.reset_desc",
                            &[
                                ("keep", REALM_ASCENSION_KEEP_RESOURCE_PCT.to_string()),
                                ("cost", REALM_BUILDING_COST_PCT_PER_ERA.to_string()),
                            ],
                        )),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_MUTED),
                    ));
                });

            if !can_ascend {
                modal.spawn((
                    Text::new(i18n.t("city.ascension.locked")),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_CRIMSON_TEXT),
                ));
            }
            let cta_label = if !can_ascend {
                i18n.t("mode.locked.button").to_string()
            } else if ctx.confirm_ascension {
                i18n.t("city.ascension.confirm_button").to_string()
            } else {
                i18n.t("city.ascension.sail_button").to_string()
            };
            spawn_small_button_colored(
                modal,
                &cta_label,
                CityAction::AscendRealm,
                if !can_ascend {
                    COLOR_BUTTON_DISABLED
                } else if ctx.confirm_ascension {
                    COLOR_ACCENT_GREEN
                } else {
                    COLOR_ACCENT_GOLD
                },
                if !can_ascend {
                    COLOR_TEXT_MUTED
                } else if ctx.confirm_ascension {
                    COLOR_TEXT_ON_ACCENT
                } else {
                    COLOR_TEXT_DARK
                },
                can_ascend,
            );
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use wyrdstone_core::{GameDataBundle, Language};

    #[test]
    fn reward_summary_formats_glory_and_non_zero_resources() {
        let data = GameDataBundle::load_bundled(Language::En).expect("bundled data loads");
        let res = ResourceBundle {
            wood: 15,
            stone: 0,
            gold: 25,
            mead: 5,
        };
        let en = format_reward_summary(&data.i18n, res, 40);
        assert!(en.contains("+40 Glory"));
        assert!(en.contains("+15 Timber"));
        assert!(!en.contains("Stone"));
        assert!(en.contains("+25 Gold"));
        assert!(en.contains("+5 Mead"));

        let mut fr_i18n = data.i18n.clone();
        fr_i18n.set_language(Language::Fr);
        let fr = format_reward_summary(&fr_i18n, res, 40);
        assert!(fr.contains("+40 Gloire"));
        assert!(fr.contains("+15 Bois"));
        assert!(!fr.contains("Pierre"));
    }

    #[test]
    fn amulet_effect_formats_all_variants_without_raw_placeholders() {
        let data = GameDataBundle::load_bundled(Language::En).expect("bundled data loads");
        let effects = [
            AmuletEffect::ResourceBonusPct(25),
            AmuletEffect::ExtraMoves(3),
            AmuletEffect::ScoreBonusPct(20),
            AmuletEffect::StartingBoosters(2),
            AmuletEffect::ExtraTimePct(30),
        ];
        for eff in effects {
            let rendered = format_amulet_effect(&data.i18n, eff);
            assert!(!rendered.is_empty());
            assert!(
                !rendered.contains("{val}"),
                "unreplaced placeholder in {rendered}"
            );
        }
    }
}
