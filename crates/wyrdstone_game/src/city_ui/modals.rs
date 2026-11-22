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
use crate::state::{ActiveModal, GameContext};
use crate::theme::*;
use bevy::prelude::*;
use wyrdstone_core::{is_achievement_unlocked, PatronGod, ACHIEVEMENTS};

fn spawn_modal_shell<F>(
    root: &mut ChildBuilder,
    width_px: f32,
    title: &str,
    close_label: &str,
    close_modal: ActiveModal,
    body_fn: F,
) where
    F: FnOnce(&mut ChildBuilder),
{
    root.spawn((
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
    ))
    .with_children(|backdrop| {
        backdrop
            .spawn((
                Node {
                    width: Val::Px(width_px),
                    max_width: Val::Percent(94.0),
                    max_height: Val::Percent(90.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(20.0)),
                    row_gap: Val::Px(10.0),
                    border: UiRect::all(Val::Px(2.0)),
                    overflow: Overflow::scroll_y(),
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
                            Text::new(title),
                            TextFont {
                                font_size: FONT_HEADING,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                        spawn_small_button(
                            hdr,
                            close_label,
                            CityAction::ToggleModal(close_modal),
                            COLOR_ACCENT_CRIMSON,
                        );
                    });

                body_fn(modal);
            });
    });
}

pub(super) fn spawn_amulets_modal(root: &mut ChildBuilder, ctx: &GameContext) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        560.0,
        i18n.t("city.panel.amulets.title"),
        i18n.t("city.panel.amulets.close"),
        ActiveModal::Amulets,
        |modal| {
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
                let (btn_bg, btn_fg) = if equipped {
                    (COLOR_ACCENT_GOLD, COLOR_TEXT_DARK)
                } else if crafted {
                    (COLOR_ACCENT_TEAL, COLOR_TEXT_PRIMARY)
                } else if can_afford {
                    (COLOR_ACCENT_GREEN, COLOR_TEXT_PRIMARY)
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
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 1.0,
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
                                        "city.panel.amulet.cost",
                                        &[
                                            ("wood", c.wood.to_string()),
                                            ("stone", c.stone.to_string()),
                                            ("gold", c.gold.to_string()),
                                            ("mead", c.mead.to_string()),
                                        ],
                                    )),
                                    TextFont {
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_MUTED),
                                ));
                            }
                        });

                        spawn_small_button_colored(
                            row,
                            action_lbl,
                            CityAction::CraftOrEquipAmulet(amulet.id.clone()),
                            btn_bg,
                            btn_fg,
                        );
                    });
            }
        },
    );
}

pub(super) fn spawn_collections_modal(root: &mut ChildBuilder, ctx: &GameContext) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        580.0,
        i18n.t("city.panel.collections.title"),
        i18n.t("city.panel.amulets.close"),
        ActiveModal::Collections,
        |modal| {
            for col in &ctx.data.collections {
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
                            row.spawn((
                                Text::new(format!(
                                    "{}  ({})",
                                    i18n.t(&col.name_key),
                                    i18n.t_fmt(
                                        "city.panel.collections.progress",
                                        &[
                                            ("found", found.to_string()),
                                            ("total", total.to_string()),
                                        ],
                                    )
                                )),
                                TextFont {
                                    font_size: FONT_BODY,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GOLD),
                            ));

                            if ready {
                                spawn_small_button(
                                    row,
                                    i18n.t("city.panel.collections.claim"),
                                    CityAction::ClaimCollection(col.id.clone()),
                                    COLOR_ACCENT_GREEN,
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
                    });
            }
        },
    );
}

pub(super) fn spawn_tool_shop_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        620.0,
        i18n.t("city.panel.shop.title"),
        i18n.t("city.panel.amulets.close"),
        ActiveModal::Shop,
        |modal| {
            for recipe in &ctx.data.tool_shop {
                let count = ctx.city.tools.count(recipe.tool);
                let can_afford = ctx.city.resources.can_afford(&recipe.cost);
                let c = recipe.cost;
                let mut cost_parts = Vec::new();
                if c.wood > 0 {
                    cost_parts.push(format!("{} {}", c.wood, i18n.t("resources.label.wood")));
                }
                if c.stone > 0 {
                    cost_parts.push(format!("{} {}", c.stone, i18n.t("resources.label.stone")));
                }
                if c.gold > 0 {
                    cost_parts.push(format!("{} {}", c.gold, i18n.t("resources.label.gold")));
                }
                if c.mead > 0 {
                    cost_parts.push(format!("{} {}", c.mead, i18n.t("resources.label.mead")));
                }

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
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                                info.spawn((
                                    Text::new(cost_parts.join("  |  ")),
                                    TextFont {
                                        font_size: 12.0,
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
                        );
                    });
            }
        },
    );
}

pub(super) fn spawn_stats_modal(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    sound_bank: &SoundBank,
) {
    let i18n = &ctx.data.i18n;
    let st = ctx.city.stats;
    spawn_modal_shell(
        root,
        460.0,
        i18n.t("city.panel.stats.title"),
        i18n.t("city.panel.amulets.close"),
        ActiveModal::Stats,
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

            // Persisted Audio Volume Controls
            card.spawn(Node {
                column_gap: Val::Px(10.0),
                margin: UiRect::top(Val::Px(8.0)),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .with_children(|audio_row| {
                let bgm_pct = (sound_bank.bgm_volume * 200.0).round() as u32;
                let sfx_pct = (sound_bank.sfx_volume * 100.0).round() as u32;
                spawn_small_button(
                    audio_row,
                    &i18n.t_fmt("city.panel.audio.bgm", &[("pct", bgm_pct.to_string())]),
                    CityAction::CycleBgmVolume,
                    COLOR_PANEL_CARD,
                );
                spawn_small_button(
                    audio_row,
                    &i18n.t_fmt("city.panel.audio.sfx", &[("pct", sfx_pct.to_string())]),
                    CityAction::CycleSfxVolume,
                    COLOR_PANEL_CARD,
                );
            });
        },
    );
}

pub(super) fn spawn_help_modal(root: &mut ChildBuilder, ctx: &GameContext) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        560.0,
        i18n.t("city.panel.help.title"),
        i18n.t("city.panel.amulets.close"),
        ActiveModal::Help,
        |card| {
            for (t_key, b_key) in [
                (
                    "city.panel.help.boosters_title",
                    "city.panel.help.boosters_body",
                ),
                (
                    "city.panel.help.obstacles_title",
                    "city.panel.help.obstacles_body",
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

pub(super) fn spawn_patrons_modal(root: &mut ChildBuilder, ctx: &GameContext) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        580.0,
        i18n.t("city.panel.patrons.title"),
        i18n.t("city.panel.amulets.close"),
        ActiveModal::Patrons,
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
                                COLOR_TEXT_PRIMARY
                            },
                        );
                    });
            }
        },
    );
}

pub(super) fn spawn_raids_modal(root: &mut ChildBuilder, ctx: &GameContext) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        620.0,
        i18n.t("city.panel.raids.title"),
        i18n.t("city.panel.amulets.close"),
        ActiveModal::Raids,
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
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 1.0,
                            row_gap: Val::Px(3.0),
                            ..default()
                        })
                        .with_children(|info| {
                            let header = if slain {
                                format!(
                                    "{} - {} [{}]",
                                    i18n.t(&boss.name_key),
                                    i18n.t(&boss.title_key),
                                    i18n.t("city.panel.raids.defeated")
                                )
                            } else {
                                format!("{} - {}", i18n.t(&boss.name_key), i18n.t(&boss.title_key))
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
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_TEAL),
                            ));
                        });

                        if unlocked {
                            spawn_small_button(
                                row,
                                i18n.t("city.panel.raids.fight"),
                                CityAction::FightBoss(boss.id.clone()),
                                COLOR_ACCENT_CRIMSON,
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

pub(super) fn spawn_achievements_modal(root: &mut ChildBuilder, ctx: &GameContext) {
    let i18n = &ctx.data.i18n;
    spawn_modal_shell(
        root,
        620.0,
        i18n.t("city.panel.feats.title"),
        i18n.t("city.panel.amulets.close"),
        ActiveModal::Achievements,
        |modal| {
            for ach in ACHIEVEMENTS {
                let claimed = ctx.city.claimed_achievements.contains(ach.id);
                let unlocked = is_achievement_unlocked(
                    ach.id,
                    &ctx.city,
                    &ctx.data.buildings,
                    ctx.data.bosses.len(),
                );
                let ready = unlocked && !claimed;

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
                                Text::new(i18n.t(ach.title_key)),
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
                                Text::new(i18n.t(ach.desc_key)),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                        });

                        if ready {
                            spawn_small_button(
                                row,
                                i18n.t("city.panel.feats.claim"),
                                CityAction::ClaimAchievement(ach.id.to_string()),
                                COLOR_ACCENT_GREEN,
                            );
                        } else if claimed {
                            spawn_resource_pill(
                                row,
                                i18n.t("city.panel.feats.claimed"),
                                COLOR_ACCENT_GOLD,
                            );
                        } else {
                            spawn_resource_pill(
                                row,
                                i18n.t("city.panel.feats.locked"),
                                COLOR_TEXT_MUTED,
                            );
                        }
                    });
            }
        },
    );
}

pub(super) fn spawn_expedition_boons_modal(root: &mut ChildBuilder, ctx: &GameContext) {
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
        580.0,
        &title,
        i18n.t("city.panel.expedition.cashout"),
        ActiveModal::ExpeditionBoons,
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
                        i18n.t("city.panel.expedition.cashout"),
                        CityAction::EndExpedition,
                        COLOR_ACCENT_CRIMSON,
                    );
                });
        },
    );
}
