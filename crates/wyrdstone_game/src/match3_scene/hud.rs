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

//! Match-3 HUD overlay: moves, live countdown timer, objectives, active tools bar, and Guide modal.

use super::{Match3Cleanup, Match3HudRoot, Match3TimerText, Match3UiAction};
use crate::audio::SoundBank;
use crate::fjord_art::FjordTextures;
use crate::scroll::{ScrollPanelKind, ScrollablePanel};
use crate::state::{ActiveModal, GameContext};
use crate::theme::*;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use wyrdstone_core::{ActiveTool, Objective};

pub(super) fn spawn_match3_hud(
    commands: &mut Commands,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    sound_bank: &SoundBank,
    modal_scroll_y: f32,
    is_compact: bool,
) {
    let i18n = &ctx.data.i18n;
    let Some(session) = &ctx.active_session else {
        return;
    };

    commands
        .spawn((
            Match3Cleanup,
            Match3HudRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
        ))
        .with_children(|root| {
            // Top HUD bar
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                    column_gap: Val::Px(14.0),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_DARK),
                BorderColor(COLOR_PANEL_BORDER),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|top| {
                top.spawn(Node {
                    column_gap: Val::Px(18.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|stats| {
                    stats.spawn((
                        Text::new(i18n.t_fmt(
                            "match3.hud.moves",
                            &[("moves", session.moves_left.to_string())],
                        )),
                        TextFont {
                            font_size: FONT_HEADING,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                    if let Some(secs) = session.time_left_secs {
                        let whole = secs.ceil() as u32;
                        stats.spawn((
                            Match3TimerText,
                            Text::new(
                                i18n.t_fmt("match3.hud.timer", &[("secs", whole.to_string())]),
                            ),
                            TextFont {
                                font_size: FONT_HEADING,
                                ..default()
                            },
                            TextColor(if whole <= 15 {
                                COLOR_ACCENT_CRIMSON
                            } else {
                                COLOR_ACCENT_TEAL
                            }),
                        ));
                    }
                    stats.spawn((
                        Text::new(
                            i18n.t_fmt("match3.hud.score", &[("score", session.score.to_string())]),
                        ),
                        TextFont {
                            font_size: FONT_BODY,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_PRIMARY),
                    ));
                });

                top.spawn(Node {
                    column_gap: Val::Px(14.0),
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|objs| {
                    if let Some(boss) = &session.boss {
                        let hp_pct = (boss.hp as f32 / boss.def.max_hp.max(1) as f32 * 100.0)
                            .clamp(0.0, 100.0);
                        objs.spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            ..default()
                        })
                        .with_children(|brow| {
                            if let Some(img) = fjord_art.boss_portrait(&boss.def.id) {
                                let hit_flash = boss.last_damage_dealt > 0;
                                brow.spawn((
                                    ImageNode {
                                        image: img,
                                        color: if hit_flash {
                                            Color::srgb(1.35, 0.72, 0.72)
                                        } else {
                                            Color::WHITE
                                        },
                                        ..default()
                                    },
                                    Node {
                                        width: Val::Px(46.0),
                                        height: Val::Px(46.0),
                                        border: UiRect::all(Val::Px(2.0)),
                                        ..default()
                                    },
                                    BorderColor(if hit_flash {
                                        COLOR_ACCENT_GOLD
                                    } else {
                                        COLOR_ACCENT_CRIMSON
                                    }),
                                    BorderRadius::all(Val::Px(8.0)),
                                ));
                            }
                            brow.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(4.0),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|bcol| {
                                bcol.spawn((
                                    Text::new(
                                        i18n.t_fmt(
                                            "match3.boss.hp",
                                            &[
                                                ("boss", i18n.t(&boss.def.name_key).to_string()),
                                                ("hp", boss.hp.to_string()),
                                                ("max", boss.def.max_hp.to_string()),
                                                (
                                                    "rune",
                                                    i18n.t(boss.active_weakness().i18n_key())
                                                        .to_string(),
                                                ),
                                                ("turns", boss.turns_until_riposte.to_string()),
                                            ],
                                        ),
                                    ),
                                    TextFont {
                                        font_size: if is_compact { FONT_SMALL } else { FONT_BODY },
                                        ..default()
                                    },
                                    TextColor(if boss.is_phase2() {
                                        COLOR_ACCENT_GOLD
                                    } else {
                                        COLOR_ACCENT_CRIMSON
                                    }),
                                ));
                                bcol.spawn((
                                    Node {
                                        width: Val::Px(if is_compact { 190.0 } else { 280.0 }),
                                        height: Val::Px(10.0),
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.16, 0.08, 0.10, 0.95)),
                                    BorderColor(COLOR_ACCENT_GOLD),
                                    BorderRadius::all(Val::Px(5.0)),
                                ))
                                .with_children(|track| {
                                    track.spawn((
                                        Node {
                                            width: Val::Percent(hp_pct),
                                            height: Val::Percent(100.0),
                                            ..default()
                                        },
                                        BackgroundColor(COLOR_ACCENT_CRIMSON),
                                        BorderRadius::all(Val::Px(4.0)),
                                    ));
                                });
                            });
                        });
                    } else {
                        for prog in &session.objectives {
                            let label = match &prog.objective {
                                Objective::ClearIce { .. } => i18n.t_fmt(
                                    "match3.objective.clear_ice",
                                    &[
                                        ("done", prog.done.to_string()),
                                        ("target", prog.target.to_string()),
                                    ],
                                ),
                                Objective::CollectRune { rune, .. } => i18n.t_fmt(
                                    "match3.objective.collect_rune",
                                    &[
                                        ("rune", i18n.t(rune.i18n_key()).to_string()),
                                        ("done", prog.done.to_string()),
                                        ("target", prog.target.to_string()),
                                    ],
                                ),
                                Objective::ReachScore { .. } => i18n.t_fmt(
                                    "match3.objective.reach_score",
                                    &[
                                        ("done", prog.done.to_string()),
                                        ("target", prog.target.to_string()),
                                    ],
                                ),
                                Objective::DropRelics { .. } => i18n.t_fmt(
                                    "match3.objective.drop_relics",
                                    &[
                                        ("done", prog.done.to_string()),
                                        ("target", prog.target.to_string()),
                                    ],
                                ),
                            };
                            objs.spawn((
                                Text::new(label),
                                TextFont {
                                    font_size: FONT_BODY,
                                    ..default()
                                },
                                TextColor(if prog.is_complete() {
                                    COLOR_ACCENT_GREEN
                                } else {
                                    COLOR_TEXT_PRIMARY
                                }),
                            ));
                        }
                    }
                });

                top.spawn(Node {
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|right| {
                    if session.last_frost_spread.is_some() {
                        right.spawn((
                            Text::new(i18n.t("match3.hud.frost_spread")),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(Color::srgb(0.45, 0.86, 1.0)),
                        ));
                    } else if session.last_shuffled {
                        right.spawn((
                            Text::new(i18n.t("match3.hud.shuffled")),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                    } else if session.last_combo >= 2 {
                        right.spawn((
                            Text::new(i18n.t_fmt(
                                "match3.hud.combo",
                                &[("combo", session.last_combo.to_string())],
                            )),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                    }

                    right
                        .spawn((
                            Button,
                            Match3UiAction::ToggleHelp,
                            Node {
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(COLOR_PANEL_CARD),
                            BorderRadius::all(Val::Px(6.0)),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(i18n.t("city.panel.help.button")),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GOLD),
                            ));
                        });

                    right
                        .spawn((
                            Button,
                            Match3UiAction::Retreat,
                            Node {
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(COLOR_ACCENT_CRIMSON),
                            BorderRadius::all(Val::Px(6.0)),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(i18n.t("match3.hud.back")),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                        });
                });
            });

            // Bottom active tools bar
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(6.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_DARK),
                BorderColor(COLOR_PANEL_BORDER),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|bottom| {
                if let Some(armed) = ctx.armed_tool {
                    let hint_key = match armed {
                        ActiveTool::Mjolnir => "match3.tool.hint.mjolnir",
                        ActiveTool::SurtrEmber => "match3.tool.hint.surtr_ember",
                        ActiveTool::LokiShuffle => "match3.tool.hint.loki_shuffle",
                        ActiveTool::ThorBolt => "match3.tool.hint.thor_bolt",
                        ActiveTool::BifrostPrism => "match3.tool.hint.bifrost_prism",
                        ActiveTool::DrakkarCharge => "match3.tool.hint.drakkar_charge",
                    };
                    bottom.spawn((
                        Text::new(i18n.t(hint_key)),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                }

                bottom
                    .spawn(Node {
                        column_gap: Val::Px(8.0),
                        row_gap: Val::Px(6.0),
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        // Divine Fury Patron button
                        let fury_ready = session.can_trigger_fury();
                        let fury_pct = ((session.fury as f32 / session.fury_max.max(1) as f32)
                            * 100.0)
                            .round() as u32;
                        let patron_name = i18n.t(session.patron.name_key()).to_string();
                        let fury_lbl = if is_compact {
                            if fury_ready {
                                "!".to_string()
                            } else {
                                format!("{fury_pct}%")
                            }
                        } else if fury_ready {
                            i18n.t_fmt("match3.fury.ready", &[("patron", patron_name)])
                        } else {
                            i18n.t_fmt(
                                "match3.fury.charge",
                                &[("patron", patron_name), ("pct", fury_pct.to_string())],
                            )
                        };
                        row.spawn((
                            Button,
                            Match3UiAction::TriggerFury,
                            Node {
                                padding: if is_compact {
                                    UiRect::axes(Val::Px(6.0), Val::Px(4.0))
                                } else {
                                    UiRect::axes(Val::Px(10.0), Val::Px(6.0))
                                },
                                column_gap: Val::Px(if is_compact { 4.0 } else { 8.0 }),
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(if fury_ready {
                                COLOR_ACCENT_GOLD
                            } else {
                                Color::srgba(0.16, 0.14, 0.24, 0.95)
                            }),
                            BorderColor(if fury_ready {
                                COLOR_ACCENT_GOLD_HOVER
                            } else {
                                COLOR_PANEL_BORDER
                            }),
                            BorderRadius::all(Val::Px(8.0)),
                        ))
                        .with_children(|btn| {
                            if let Some(p_img) = fjord_art.patron_portrait(session.patron) {
                                let icon_px = if is_compact { 26.0 } else { 32.0 };
                                btn.spawn((
                                    ImageNode::new(p_img),
                                    Node {
                                        width: Val::Px(icon_px),
                                        height: Val::Px(icon_px),
                                        ..default()
                                    },
                                    BorderRadius::all(Val::Px(6.0)),
                                ));
                            }
                            btn.spawn((
                                Text::new(fury_lbl),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(if fury_ready {
                                    COLOR_TEXT_DARK
                                } else {
                                    COLOR_ACCENT_GOLD
                                }),
                            ));
                        });

                        let tools = [
                            (ActiveTool::Mjolnir, "match3.tool.mjolnir"),
                            (ActiveTool::SurtrEmber, "match3.tool.surtr_ember"),
                            (ActiveTool::ThorBolt, "match3.tool.thor_bolt"),
                            (ActiveTool::BifrostPrism, "match3.tool.bifrost_prism"),
                            (ActiveTool::DrakkarCharge, "match3.tool.drakkar_charge"),
                            (ActiveTool::LokiShuffle, "match3.tool.loki_shuffle"),
                        ];
                        for (tool, key) in tools {
                            let count = ctx.city.tools.count(tool);
                            let armed = ctx.armed_tool == Some(tool);
                            let bg = if armed {
                                COLOR_ACCENT_GOLD
                            } else if count > 0 {
                                COLOR_PANEL_CARD
                            } else {
                                COLOR_BUTTON_DISABLED
                            };
                            let text_col = if armed {
                                COLOR_TEXT_DARK
                            } else {
                                COLOR_TEXT_PRIMARY
                            };
                            let tool_lbl = if is_compact {
                                format!("x{count}")
                            } else {
                                i18n.t_fmt(key, &[("count", count.to_string())])
                            };
                            row.spawn((
                                Button,
                                Match3UiAction::SelectTool(tool),
                                Node {
                                    padding: if is_compact {
                                        UiRect::axes(Val::Px(6.0), Val::Px(4.0))
                                    } else {
                                        UiRect::axes(Val::Px(10.0), Val::Px(6.0))
                                    },
                                    column_gap: Val::Px(if is_compact { 4.0 } else { 8.0 }),
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.5)),
                                    ..default()
                                },
                                BackgroundColor(bg),
                                BorderColor(if armed {
                                    COLOR_ACCENT_GOLD_HOVER
                                } else {
                                    COLOR_PANEL_BORDER
                                }),
                                BorderRadius::all(Val::Px(8.0)),
                            ))
                            .with_children(|btn| {
                                if let Some(icon) = fjord_art.tool_icon(tool) {
                                    let icon_px = if is_compact { 26.0 } else { 34.0 };
                                    btn.spawn((
                                        ImageNode {
                                            image: icon,
                                            color: if count > 0 {
                                                Color::WHITE
                                            } else {
                                                Color::srgba(0.55, 0.55, 0.60, 0.7)
                                            },
                                            ..default()
                                        },
                                        Node {
                                            width: Val::Px(icon_px),
                                            height: Val::Px(icon_px),
                                            ..default()
                                        },
                                        BorderRadius::all(Val::Px(6.0)),
                                    ));
                                }
                                btn.spawn((
                                    Text::new(tool_lbl),
                                    TextFont {
                                        font_size: if is_compact { FONT_SMALL } else { FONT_BODY },
                                        ..default()
                                    },
                                    TextColor(text_col),
                                ));
                            });
                        }
                    });
            });

            // Optional Wyrdstone Guide overlay inside Match-3 (pauses countdown timer)
            if ctx.active_modal == ActiveModal::Help {
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
                .with_children(|bg| {
                    bg.spawn((
                        ScrollablePanel {
                            kind: ScrollPanelKind::Modal,
                        },
                        ScrollPosition {
                            offset_x: 0.0,
                            offset_y: modal_scroll_y,
                        },
                        RelativeCursorPosition::default(),
                        Node {
                            width: Val::Px(540.0),
                            max_width: Val::Percent(94.0),
                            max_height: Val::Percent(90.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(22.0)),
                            row_gap: Val::Px(10.0),
                            border: UiRect::all(Val::Px(2.0)),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        BackgroundColor(COLOR_PANEL_DARK),
                        BorderColor(COLOR_ACCENT_GOLD),
                        BorderRadius::all(Val::Px(14.0)),
                    ))
                    .with_children(|card| {
                        card.spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|hdr| {
                            hdr.spawn((
                                Text::new(i18n.t("city.panel.help.title")),
                                TextFont {
                                    font_size: FONT_HEADING,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GOLD),
                            ));
                            hdr.spawn((
                                Button,
                                Match3UiAction::ToggleHelp,
                                Node {
                                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                                    ..default()
                                },
                                BackgroundColor(COLOR_ACCENT_CRIMSON),
                                BorderRadius::all(Val::Px(6.0)),
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(i18n.t("city.panel.amulets.close")),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                            });
                        });

                        // Quick Pause Settings (Audio + Colorblind) right inside Match-3
                        card.spawn((
                            Text::new(i18n.t("match3.pause.settings_title")),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                        card.spawn(Node {
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(8.0),
                            row_gap: Val::Px(6.0),
                            ..default()
                        })
                        .with_children(|srow| {
                            let mute_lbl = if sound_bank.muted {
                                i18n.t("app.audio.off")
                            } else {
                                i18n.t("app.audio.on")
                            };
                            let bgm_pct = (sound_bank.bgm_volume * 200.0).round() as u32;
                            let sfx_pct = (sound_bank.sfx_volume * 100.0).round() as u32;
                            let cb_lbl = if ctx.colorblind_mode {
                                i18n.t("city.panel.colorblind.on")
                            } else {
                                i18n.t("city.panel.colorblind.off")
                            };
                            for (lbl, act, bg_col) in [
                                (
                                    mute_lbl.to_string(),
                                    Match3UiAction::ToggleAudio,
                                    COLOR_PANEL_CARD,
                                ),
                                (
                                    i18n.t_fmt(
                                        "city.panel.audio.bgm",
                                        &[("pct", bgm_pct.to_string())],
                                    ),
                                    Match3UiAction::CycleBgmVolume,
                                    COLOR_PANEL_CARD,
                                ),
                                (
                                    i18n.t_fmt(
                                        "city.panel.audio.sfx",
                                        &[("pct", sfx_pct.to_string())],
                                    ),
                                    Match3UiAction::CycleSfxVolume,
                                    COLOR_PANEL_CARD,
                                ),
                                (
                                    cb_lbl.to_string(),
                                    Match3UiAction::ToggleColorblind,
                                    if ctx.colorblind_mode {
                                        COLOR_ACCENT_TEAL
                                    } else {
                                        COLOR_PANEL_CARD
                                    },
                                ),
                            ] {
                                srow.spawn((
                                    Button,
                                    act,
                                    Node {
                                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(bg_col),
                                    BorderColor(COLOR_PANEL_BORDER),
                                    BorderRadius::all(Val::Px(6.0)),
                                ))
                                .with_children(|b| {
                                    b.spawn((
                                        Text::new(lbl),
                                        TextFont {
                                            font_size: FONT_SMALL,
                                            ..default()
                                        },
                                        TextColor(COLOR_TEXT_PRIMARY),
                                    ));
                                });
                            }
                        });

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
                    });
                });
            }
        });
}
