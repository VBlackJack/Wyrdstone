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

//! Slide-out right drawers for Play Modes, Clan & Gear progression, and the Burger Menu.

use super::{
    badged_label, spawn_resource_pill, spawn_small_button, spawn_small_button_colored, CityAction,
    CityDrawer,
};
use crate::audio::SoundBank;
use crate::fjord_art::FjordTextures;
use crate::scroll::{spawn_scrollbar_track, ScrollMoreHint, ScrollPanelKind, ScrollablePanel};
use crate::state::{today_day_index, ActiveModal, GameContext};
use crate::theme::*;
use crate::ui_style::display_font;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use wyrdstone_core::{format_blessing_desc, PlayMode};

fn spawn_drawer_shell<F>(
    drawer: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    title: &str,
    scroll_offset_y: f32,
    body_builder: F,
) where
    F: FnOnce(&mut ChildBuilder),
{
    let i18n = &ctx.data.i18n;
    drawer
        .spawn((
            Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
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
                display_font(fjord_art, FONT_TITLE_MD - 4.0),
                TextColor(COLOR_ACCENT_GOLD),
            ));
            spawn_small_button(
                hdr,
                i18n.t("city.panel.amulets.close"),
                CityAction::ToggleDrawer(CityDrawer::None),
                COLOR_ACCENT_CRIMSON,
                true,
            );
        });

    drawer
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
                        row_gap: Val::Px(8.0),
                        padding: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                ))
                .with_children(body_builder);
            spawn_scrollbar_track(scroll_row, ScrollPanelKind::Inspector);
        });

    drawer
        .spawn((
            ScrollMoreHint {
                kind: ScrollPanelKind::Inspector,
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
}

#[allow(clippy::too_many_arguments)]
fn spawn_drawer_action_card(
    parent: &mut ChildBuilder,
    title: &str,
    desc: &str,
    btn_label: &str,
    action: CityAction,
    btn_bg: Color,
    btn_fg: Color,
    border_col: Color,
    enabled: bool,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.5)),
                ..default()
            },
            BackgroundColor(COLOR_PANEL_CARD),
            BorderColor(border_col),
            BorderRadius::all(Val::Px(RADIUS_MD)),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(title),
                TextFont {
                    font_size: FONT_BODY,
                    ..default()
                },
                TextColor(COLOR_ACCENT_GOLD),
            ));
            card.spawn((
                Text::new(desc),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_TEXT_PRIMARY),
            ));
            card.spawn(Node {
                justify_content: JustifyContent::FlexEnd,
                ..default()
            })
            .with_children(|row| {
                spawn_small_button_colored(row, btn_label, action, btn_bg, btn_fg, enabled);
            });
        });
}

pub(super) fn spawn_modes_drawer(
    drawer: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    let energy_cost = ctx.data.balance.energy_cost_per_level;
    spawn_drawer_shell(
        drawer,
        ctx,
        fjord_art,
        i18n.t("city.drawer.modes.title"),
        scroll_offset_y,
        |body| {
            let blessing = ctx.today_blessing();
            let blessing_lbl = i18n.t_fmt(
                "city.header.blessing_pill",
                &[
                    ("name", i18n.t(blessing.name_key()).to_string()),
                    (
                        "desc",
                        format_blessing_desc(i18n, blessing, &ctx.data.balance.session),
                    ),
                ],
            );
            spawn_resource_pill(body, &blessing_lbl, COLOR_ACCENT_TEAL_TEXT);

            if let Some(draught) = ctx.city.active_war_draught {
                spawn_resource_pill(
                    body,
                    &i18n.t_fmt(
                        "city.header.war_draught",
                        &[("name", i18n.t(draught.name_key()).to_string())],
                    ),
                    COLOR_ACCENT_GOLD,
                );
            }

            let daily_unlocked = ctx.city.is_mode_unlocked(PlayMode::DailyTrial);
            let daily_ready = daily_unlocked && ctx.city.is_daily_available(today_day_index());
            let daily_btn = if !daily_unlocked {
                i18n.t("mode.locked.button").to_string()
            } else if daily_ready {
                if ctx.city.stats.daily_streak > 0 {
                    i18n.t_fmt(
                        "city.button.label_with_count",
                        &[
                            ("label", i18n.t("city.panel.daily.button").to_string()),
                            (
                                "count",
                                i18n.t_fmt(
                                    "city.panel.daily.streak",
                                    &[("days", ctx.city.stats.daily_streak.to_string())],
                                ),
                            ),
                        ],
                    )
                } else {
                    i18n.t("city.panel.daily.button").to_string()
                }
            } else {
                i18n.t("city.panel.daily.done").to_string()
            };
            let daily_desc = if daily_unlocked {
                i18n.t_fmt(
                    "city.drawer.modes.daily_desc",
                    &[("streak", ctx.city.stats.daily_streak.to_string())],
                )
            } else {
                i18n.t(PlayMode::DailyTrial.unlock_hint_key()).to_string()
            };
            spawn_drawer_action_card(
                body,
                i18n.t("city.panel.daily.button"),
                &daily_desc,
                &daily_btn,
                CityAction::PlayDailyTrial,
                if daily_ready {
                    COLOR_ACCENT_GOLD
                } else {
                    COLOR_BUTTON_DISABLED
                },
                if daily_ready {
                    COLOR_TEXT_DARK
                } else {
                    COLOR_TEXT_MUTED
                },
                if daily_ready {
                    COLOR_ACCENT_GOLD
                } else {
                    COLOR_PANEL_BORDER
                },
                daily_ready,
            );

            let raids_unlocked = ctx.city.is_mode_unlocked(PlayMode::BossRaids);
            let raids_desc = if raids_unlocked {
                i18n.t_fmt(
                    "city.drawer.modes.raids_desc",
                    &[
                        ("slain", ctx.city.defeated_bosses.len().to_string()),
                        ("total", ctx.data.bosses.len().to_string()),
                    ],
                )
            } else {
                i18n.t(PlayMode::BossRaids.unlock_hint_key()).to_string()
            };
            spawn_drawer_action_card(
                body,
                i18n.t("city.panel.raids.button"),
                &raids_desc,
                if raids_unlocked {
                    i18n.t("city.panel.raids.button")
                } else {
                    i18n.t("mode.locked.button")
                },
                CityAction::ToggleModal(ActiveModal::Raids),
                if raids_unlocked {
                    COLOR_ACCENT_CRIMSON
                } else {
                    COLOR_BUTTON_DISABLED
                },
                if raids_unlocked {
                    COLOR_TEXT_PRIMARY
                } else {
                    COLOR_TEXT_MUTED
                },
                if raids_unlocked {
                    COLOR_ACCENT_CRIMSON
                } else {
                    COLOR_PANEL_BORDER
                },
                raids_unlocked,
            );

            let exp_unlocked = ctx.city.is_mode_unlocked(PlayMode::Expedition);
            let exp_btn = if !exp_unlocked {
                i18n.t("mode.locked.button").to_string()
            } else if let Some(run) = &ctx.active_expedition {
                i18n.t_fmt(
                    "city.panel.expedition.resume",
                    &[("wave", run.wave.to_string())],
                )
            } else {
                i18n.t_fmt(
                    "city.button.play_cost",
                    &[
                        ("label", i18n.t("city.panel.expedition.button").to_string()),
                        ("cost", energy_cost.to_string()),
                    ],
                )
            };
            let exp_desc = if exp_unlocked {
                i18n.t_fmt(
                    "city.drawer.modes.expedition_desc",
                    &[("best", ctx.city.stats.best_expedition_wave.to_string())],
                )
            } else {
                i18n.t(PlayMode::Expedition.unlock_hint_key()).to_string()
            };
            spawn_drawer_action_card(
                body,
                i18n.t("city.panel.expedition.button"),
                &exp_desc,
                &exp_btn,
                CityAction::StartExpedition,
                if !exp_unlocked {
                    COLOR_BUTTON_DISABLED
                } else if ctx.active_expedition.is_some() {
                    COLOR_ACCENT_GREEN
                } else {
                    COLOR_ACCENT_TEAL
                },
                if exp_unlocked {
                    COLOR_TEXT_ON_ACCENT
                } else {
                    COLOR_TEXT_MUTED
                },
                if exp_unlocked {
                    COLOR_ACCENT_TEAL
                } else {
                    COLOR_PANEL_BORDER
                },
                exp_unlocked,
            );

            let blitz_unlocked = ctx.city.is_mode_unlocked(PlayMode::ValhallaBlitz);
            let blitz_btn = if blitz_unlocked {
                i18n.t_fmt(
                    "city.button.play_cost",
                    &[
                        ("label", i18n.t("city.panel.blitz.button").to_string()),
                        ("cost", energy_cost.to_string()),
                    ],
                )
            } else {
                i18n.t("mode.locked.button").to_string()
            };
            let blitz_desc = if blitz_unlocked {
                i18n.t_fmt(
                    "city.drawer.modes.blitz_desc",
                    &[("best", ctx.city.stats.best_blitz_score.to_string())],
                )
            } else {
                i18n.t(PlayMode::ValhallaBlitz.unlock_hint_key())
                    .to_string()
            };
            spawn_drawer_action_card(
                body,
                i18n.t("city.panel.blitz.button"),
                &blitz_desc,
                &blitz_btn,
                CityAction::StartValhallaBlitz,
                if blitz_unlocked {
                    COLOR_ACCENT_GOLD
                } else {
                    COLOR_BUTTON_DISABLED
                },
                if blitz_unlocked {
                    COLOR_TEXT_DARK
                } else {
                    COLOR_TEXT_MUTED
                },
                if blitz_unlocked {
                    COLOR_ACCENT_GOLD
                } else {
                    COLOR_PANEL_BORDER
                },
                blitz_unlocked,
            );

            let trials_unlocked = ctx.city.is_mode_unlocked(PlayMode::YggdrasilTrials);
            let trials_btn = if trials_unlocked {
                i18n.t_fmt(
                    "city.panel.trials.button",
                    &[
                        ("done", ctx.city.completed_trials.len().to_string()),
                        ("total", ctx.data.trials.len().to_string()),
                    ],
                )
            } else {
                i18n.t("mode.locked.button").to_string()
            };
            let trials_desc = if trials_unlocked {
                i18n.t_fmt(
                    "city.drawer.modes.trials_desc",
                    &[
                        ("par", ctx.city.par_trials.len().to_string()),
                        ("total", ctx.data.trials.len().to_string()),
                    ],
                )
            } else {
                i18n.t(PlayMode::YggdrasilTrials.unlock_hint_key())
                    .to_string()
            };
            spawn_drawer_action_card(
                body,
                i18n.t("city.panel.trials.title"),
                &trials_desc,
                &trials_btn,
                CityAction::ToggleModal(ActiveModal::Trials),
                if trials_unlocked {
                    COLOR_ACCENT_TEAL
                } else {
                    COLOR_BUTTON_DISABLED
                },
                if trials_unlocked {
                    COLOR_TEXT_ON_ACCENT
                } else {
                    COLOR_TEXT_MUTED
                },
                COLOR_PANEL_BORDER,
                trials_unlocked,
            );
        },
    );
}

pub(super) fn spawn_clan_drawer(
    drawer: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    let claimable_feats = ctx.city.claimable_achievement_count(
        &ctx.data.achievements,
        &ctx.data.buildings,
        ctx.data.bosses.len(),
    );
    let claimable_cols = ctx.city.claimable_collection_count(&ctx.data.collections);
    let claimable_bounties = ctx.city.claimable_bounties_and_pass_count();
    spawn_drawer_shell(
        drawer,
        ctx,
        fjord_art,
        i18n.t("city.drawer.clan.title"),
        scroll_offset_y,
        |body| {
            let bounties_lbl = badged_label(i18n, "city.panel.bounties.button", claimable_bounties);
            spawn_drawer_action_card(
                body,
                &bounties_lbl,
                &i18n.t_fmt(
                    "city.drawer.clan.bounties_desc",
                    &[
                        ("tier", ctx.city.skald_pass.current_tier().to_string()),
                        ("max", wyrdstone_core::SKALD_PASS_TIERS.to_string()),
                    ],
                ),
                &bounties_lbl,
                CityAction::ToggleModal(ActiveModal::Bounties),
                if claimable_bounties > 0 {
                    COLOR_ACCENT_GREEN
                } else {
                    COLOR_ACCENT_GOLD
                },
                COLOR_TEXT_DARK,
                if claimable_bounties > 0 {
                    COLOR_ACCENT_GREEN
                } else {
                    COLOR_ACCENT_GOLD
                },
                true,
            );
            let am1 = ctx
                .city
                .equipped_amulet
                .as_deref()
                .and_then(|id| ctx.data.amulet_by_id(id))
                .map(|a| i18n.t(&a.name_key))
                .unwrap_or_else(|| i18n.t("city.loadout.amulet_none"));
            let loadout_line = i18n.t_fmt(
                "city.drawer.clan.loadout",
                &[
                    (
                        "patron",
                        i18n.t(ctx.city.selected_patron.name_key()).to_string(),
                    ),
                    ("amulet", am1.to_string()),
                    ("synergies", ctx.city.active_synergies().len().to_string()),
                ],
            );
            spawn_resource_pill(body, &loadout_line, COLOR_ACCENT_GOLD);

            spawn_drawer_action_card(
                body,
                i18n.t("city.panel.patrons.button"),
                i18n.t(ctx.city.selected_patron.name_key()),
                i18n.t("city.panel.patrons.button"),
                CityAction::ToggleModal(ActiveModal::Patrons),
                COLOR_ACCENT_TEAL,
                COLOR_TEXT_ON_ACCENT,
                COLOR_PANEL_BORDER,
                true,
            );

            spawn_drawer_action_card(
                body,
                i18n.t("city.panel.amulets.button"),
                &i18n.t_fmt(
                    "city.drawer.clan.amulets_desc",
                    &[
                        ("crafted", ctx.city.crafted_amulets.len().to_string()),
                        ("total", ctx.data.amulets.len().to_string()),
                    ],
                ),
                i18n.t("city.panel.amulets.button"),
                CityAction::ToggleModal(ActiveModal::Amulets),
                if ctx.city.equipped_amulet.is_some() {
                    COLOR_ACCENT_TEAL
                } else {
                    COLOR_PANEL_DARK
                },
                if ctx.city.equipped_amulet.is_some() {
                    COLOR_TEXT_ON_ACCENT
                } else {
                    COLOR_TEXT_PRIMARY
                },
                COLOR_PANEL_BORDER,
                true,
            );

            spawn_drawer_action_card(
                body,
                i18n.t("city.panel.shop.button"),
                i18n.t("city.drawer.clan.shop_desc"),
                i18n.t("city.panel.shop.button"),
                CityAction::ToggleModal(ActiveModal::Shop),
                COLOR_PANEL_DARK,
                COLOR_TEXT_PRIMARY,
                COLOR_PANEL_BORDER,
                true,
            );

            let feats_lbl = badged_label(i18n, "city.panel.feats.button", claimable_feats);
            spawn_drawer_action_card(
                body,
                &feats_lbl,
                &i18n.t_fmt(
                    "city.drawer.clan.feats_desc",
                    &[
                        ("claimed", ctx.city.claimed_achievements.len().to_string()),
                        ("total", ctx.data.achievements.len().to_string()),
                    ],
                ),
                &feats_lbl,
                CityAction::ToggleModal(ActiveModal::Achievements),
                if claimable_feats > 0 {
                    COLOR_ACCENT_GREEN
                } else {
                    COLOR_PANEL_DARK
                },
                if claimable_feats > 0 {
                    COLOR_TEXT_ON_ACCENT
                } else {
                    COLOR_TEXT_PRIMARY
                },
                if claimable_feats > 0 {
                    COLOR_ACCENT_GREEN
                } else {
                    COLOR_PANEL_BORDER
                },
                true,
            );

            let cols_lbl = badged_label(i18n, "city.panel.collections.button", claimable_cols);
            spawn_drawer_action_card(
                body,
                &cols_lbl,
                &i18n.t_fmt(
                    "city.drawer.clan.collections_desc",
                    &[
                        ("claimed", ctx.city.claimed_collections.len().to_string()),
                        ("total", ctx.data.collections.len().to_string()),
                    ],
                ),
                &cols_lbl,
                CityAction::ToggleModal(ActiveModal::Collections),
                if claimable_cols > 0 {
                    COLOR_ACCENT_GREEN
                } else {
                    COLOR_PANEL_DARK
                },
                if claimable_cols > 0 {
                    COLOR_TEXT_ON_ACCENT
                } else {
                    COLOR_TEXT_PRIMARY
                },
                if claimable_cols > 0 {
                    COLOR_ACCENT_GREEN
                } else {
                    COLOR_PANEL_BORDER
                },
                true,
            );
        },
    );
}

pub(super) fn spawn_menu_drawer(
    drawer: &mut ChildBuilder,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    sound_bank: &SoundBank,
    scroll_offset_y: f32,
) {
    let i18n = &ctx.data.i18n;
    let tribute_ready = ctx.city.is_login_tribute_available(today_day_index());
    let sound_label = if sound_bank.muted {
        i18n.t("app.audio.off")
    } else {
        i18n.t("app.audio.on")
    };
    spawn_drawer_shell(
        drawer,
        ctx,
        fjord_art,
        i18n.t("city.drawer.menu.title"),
        scroll_offset_y,
        |body| {
            spawn_small_button_colored(
                body,
                i18n.t("city.panel.tribute.button"),
                CityAction::ToggleModal(ActiveModal::LoginCalendar),
                if tribute_ready {
                    COLOR_ACCENT_GOLD
                } else {
                    COLOR_PANEL_CARD
                },
                if tribute_ready {
                    COLOR_TEXT_DARK
                } else {
                    COLOR_TEXT_PRIMARY
                },
                true,
            );
            spawn_small_button(
                body,
                i18n.t("city.panel.stats.button"),
                CityAction::ToggleModal(ActiveModal::Stats),
                COLOR_PANEL_CARD,
                true,
            );
            spawn_small_button(
                body,
                i18n.t("city.panel.help.title"),
                CityAction::ToggleModal(ActiveModal::Help),
                COLOR_PANEL_CARD,
                true,
            );
            spawn_small_button(
                body,
                i18n.t("city.panel.settings.button"),
                CityAction::ToggleModal(ActiveModal::Settings),
                COLOR_PANEL_CARD,
                true,
            );

            body.spawn(Node {
                column_gap: Val::Px(8.0),
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
                    i18n.t("app.title.button.language"),
                    CityAction::ToggleLanguage,
                    COLOR_PANEL_CARD,
                    true,
                );
                spawn_small_button(
                    row,
                    i18n.t("city.panel.help.button"),
                    CityAction::ToggleModal(ActiveModal::Help),
                    COLOR_PANEL_CARD,
                    true,
                );
            });

            spawn_small_button_colored(
                body,
                i18n.t("city.drawer.menu.title_btn"),
                CityAction::BackToTitle,
                COLOR_ACCENT_CRIMSON,
                COLOR_TEXT_PRIMARY,
                true,
            );
        },
    );
}
