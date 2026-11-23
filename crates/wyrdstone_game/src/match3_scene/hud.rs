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

use super::{
    FuryReadyPulse, LowMovesPulse, Match3Cleanup, Match3ComboText, Match3FuryText, Match3HudRoot,
    Match3MovesText, Match3ObjectiveText, Match3ScoreText, Match3TimerText, Match3TooltipText,
    Match3UiAction, LOW_MOVES_THRESHOLD,
};
use crate::audio::SoundBank;
use crate::fjord_art::FjordTextures;
use crate::runes_art::RuneTextures;
use crate::scroll::{spawn_scrollbar_track, ScrollMoreHint, ScrollPanelKind, ScrollablePanel};
use crate::state::{ActiveModal, GameContext};
use crate::theme::*;
use crate::ui_style::{
    display_font, gold_glow_shadow, modal_shadow, panel_shadow, sliced_panel_image, NoButtonScale,
};
use crate::vfx_art::VfxTextures;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui::{FocusPolicy, RelativeCursorPosition};
use wyrdstone_core::{ActiveTool, I18nCatalog, LevelSession, Objective, ObjectiveProgress};

fn moves_label(i18n: &I18nCatalog, session: &LevelSession) -> String {
    i18n.t_fmt(
        "match3.hud.moves",
        &[("moves", session.moves_left.to_string())],
    )
}

fn score_label(i18n: &I18nCatalog, session: &LevelSession) -> String {
    i18n.t_fmt("match3.hud.score", &[("score", session.score.to_string())])
}

fn combo_label(i18n: &I18nCatalog, session: &LevelSession) -> String {
    i18n.t_fmt(
        "match3.hud.combo",
        &[("combo", session.last_combo.to_string())],
    )
}

/// Objective pill text; the compact layout uses the short shared "progress" template.
fn objective_label(i18n: &I18nCatalog, prog: &ObjectiveProgress, is_compact: bool) -> String {
    let counts = [
        ("done", prog.done.to_string()),
        ("target", prog.target.to_string()),
    ];
    if is_compact {
        return i18n.t_fmt("match3.objective.progress", &counts);
    }
    match &prog.objective {
        Objective::ClearIce { .. } => i18n.t_fmt("match3.objective.clear_ice", &counts),
        Objective::CollectRune { rune, .. } => i18n.t_fmt(
            "match3.objective.collect_rune",
            &[
                ("rune", i18n.t(rune.i18n_key()).to_string()),
                ("done", prog.done.to_string()),
                ("target", prog.target.to_string()),
            ],
        ),
        Objective::ReachScore { .. } => i18n.t_fmt("match3.objective.reach_score", &counts),
        Objective::DropRelics { .. } => i18n.t_fmt("match3.objective.drop_relics", &counts),
    }
}

fn fury_label(i18n: &I18nCatalog, session: &LevelSession, is_compact: bool) -> String {
    let fury_ready = session.can_trigger_fury();
    let fury_pct = ((session.fury as f32 / session.fury_max.max(1) as f32) * 100.0).round() as u32;
    let patron_name = i18n.t(session.patron.name_key()).to_string();
    if is_compact {
        if fury_ready {
            i18n.t("match3.fury.ready_short").to_string()
        } else {
            i18n.t_fmt("match3.fury.pct_short", &[("pct", fury_pct.to_string())])
        }
    } else if fury_ready {
        i18n.t_fmt("match3.fury.ready", &[("patron", patron_name)])
    } else {
        i18n.t_fmt(
            "match3.fury.charge",
            &[("patron", patron_name), ("pct", fury_pct.to_string())],
        )
    }
}

/// Which optional top-bar banner is showing next to the Undo button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudBanner {
    None,
    FrostSpread,
    Shuffled,
    Combo,
}

/// Everything in the HUD that changes its structure, colours or widget set (not just a text).
/// When the shape is unchanged between two states, the dynamic texts can be updated in place;
/// otherwise the HUD must be rebuilt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudShape {
    low_moves: bool,
    has_boss: bool,
    banner: HudBanner,
    fury_ready: bool,
    objectives_complete: Vec<bool>,
}

pub(super) fn hud_shape(session: &LevelSession) -> HudShape {
    let banner = if session.last_frost_spread.is_some() {
        HudBanner::FrostSpread
    } else if session.last_shuffled {
        HudBanner::Shuffled
    } else if session.last_combo >= 2 {
        HudBanner::Combo
    } else {
        HudBanner::None
    };
    HudShape {
        low_moves: session.moves_left <= LOW_MOVES_THRESHOLD,
        has_boss: session.boss.is_some(),
        banner,
        fury_ready: session.can_trigger_fury(),
        objectives_complete: session.objectives.iter().map(|o| o.is_complete()).collect(),
    }
}

/// True when the HUD can follow `new` from `old` by rewriting texts only. Boss fights always
/// rebuild: the boss panel carries an HP bar and portrait state that texts alone cannot express.
pub(super) fn can_update_in_place(old: Option<&HudShape>, new: &HudShape) -> bool {
    !new.has_boss && old == Some(new)
}

type HudTextItems = (
    &'static mut Text,
    Option<&'static Match3MovesText>,
    Option<&'static Match3ScoreText>,
    Option<&'static Match3ObjectiveText>,
    Option<&'static Match3ComboText>,
    Option<&'static Match3FuryText>,
);

type HudTextFilter = Or<(
    With<Match3MovesText>,
    With<Match3ScoreText>,
    With<Match3ObjectiveText>,
    With<Match3ComboText>,
    With<Match3FuryText>,
)>;

/// Dynamic HUD texts that cascade steps rewrite in place instead of rebuilding the whole HUD.
#[derive(SystemParam)]
pub struct HudTextQueries<'w, 's> {
    texts: Query<'w, 's, HudTextItems, HudTextFilter>,
}

impl HudTextQueries<'_, '_> {
    /// Rewrites moves, score, objective, combo and fury texts from the live session.
    pub(super) fn refresh(&mut self, ctx: &GameContext, is_compact: bool) {
        let Some(session) = &ctx.active_session else {
            return;
        };
        let i18n = &ctx.data.i18n;
        for (mut text, moves, score, objective, combo, fury) in &mut self.texts {
            let label = if moves.is_some() {
                moves_label(i18n, session)
            } else if score.is_some() {
                score_label(i18n, session)
            } else if let Some(Match3ObjectiveText(idx)) = objective {
                match session.objectives.get(*idx) {
                    Some(prog) => objective_label(i18n, prog, is_compact),
                    None => continue,
                }
            } else if combo.is_some() {
                combo_label(i18n, session)
            } else if fury.is_some() {
                fury_label(i18n, session, is_compact)
            } else {
                continue;
            };
            if text.0 != label {
                text.0 = label;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_match3_hud(
    commands: &mut Commands,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    textures: &RuneTextures,
    vfx: &VfxTextures,
    sound_bank: &SoundBank,
    modal_scroll_y: f32,
    is_compact: bool,
    can_undo: bool,
    confirm_retreat: bool,
    confirm_loki_shuffle: bool,
    in_valhalla: bool,
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
            // Top HUD bar (9-slice sculpted stone & bronze frame)
            root.spawn((
                sliced_panel_image(vfx),
                Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                    column_gap: Val::Px(14.0),
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                panel_shadow(),
            ))
            .with_children(|top| {
                top.spawn(Node {
                    column_gap: Val::Px(18.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|stats| {
                    let low_moves = session.moves_left <= LOW_MOVES_THRESHOLD;
                    let mut moves_box = stats.spawn((
                        Node {
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(3.0)),
                            border: UiRect::all(Val::Px(if low_moves { 2.0 } else { 1.0 })),
                            ..default()
                        },
                        BackgroundColor(if low_moves {
                            Color::srgba(0.36, 0.08, 0.10, 0.92)
                        } else {
                            Color::NONE
                        }),
                        BorderColor(if low_moves {
                            COLOR_ACCENT_CRIMSON
                        } else {
                            Color::NONE
                        }),
                        BorderRadius::all(Val::Px(8.0)),
                    ));
                    if low_moves {
                        moves_box.insert(LowMovesPulse);
                    }
                    moves_box.with_children(|mbox| {
                        mbox.spawn((
                            Match3MovesText,
                            Text::new(moves_label(i18n, session)),
                            display_font(fjord_art, FONT_HEADING),
                            TextColor(if low_moves {
                                COLOR_ACCENT_CRIMSON_TEXT
                            } else {
                                COLOR_ACCENT_GOLD
                            }),
                        ));
                    });
                    if let Some(secs) = session.time_left_secs {
                        let whole = secs.ceil() as u32;
                        stats.spawn((
                            Match3TimerText,
                            Text::new(
                                i18n.t_fmt("match3.hud.timer", &[("secs", whole.to_string())]),
                            ),
                            display_font(fjord_art, FONT_HEADING),
                            TextColor(if whole <= 15 {
                                COLOR_ACCENT_CRIMSON_TEXT
                            } else {
                                COLOR_ACCENT_TEAL
                            }),
                        ));
                    }
                    stats.spawn((
                        Match3ScoreText,
                        Text::new(score_label(i18n, session)),
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
                                    BorderColor(if hit_flash || boss.is_phase2() {
                                        COLOR_ACCENT_GOLD
                                    } else {
                                        COLOR_ACCENT_CRIMSON
                                    }),
                                    BorderRadius::all(Val::Px(8.0)),
                                ));
                            }
                            if let Some(weak_icon) =
                                textures.runes.get(&boss.active_weakness()).cloned()
                            {
                                brow.spawn((
                                    Node {
                                        padding: UiRect::all(Val::Px(3.0)),
                                        border: UiRect::all(Val::Px(1.5)),
                                        ..default()
                                    },
                                    BackgroundColor(COLOR_PANEL_CARD),
                                    BorderColor(COLOR_ACCENT_TEAL),
                                    BorderRadius::all(Val::Px(8.0)),
                                ))
                                .with_children(|wbox| {
                                    wbox.spawn((
                                        ImageNode::new(weak_icon),
                                        Node {
                                            width: Val::Px(26.0),
                                            height: Val::Px(26.0),
                                            ..default()
                                        },
                                    ));
                                });
                            }
                            brow.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(4.0),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|bcol| {
                                let boss_hp_key = if boss.is_phase2() {
                                    "match3.boss.phase2_badge"
                                } else {
                                    "match3.boss.hp"
                                };
                                bcol.spawn((
                                    Text::new(
                                        i18n.t_fmt(
                                            boss_hp_key,
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
                                        COLOR_ACCENT_CRIMSON_TEXT
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
                        for (obj_idx, prog) in session.objectives.iter().enumerate() {
                            let icon_opt = match &prog.objective {
                                Objective::ClearIce { .. } => Some(textures.ice_top_layer1.clone()),
                                Objective::CollectRune { rune, .. } => {
                                    textures.runes.get(rune).cloned()
                                }
                                Objective::ReachScore { .. } => Some(vfx.icon_star.clone()),
                                Objective::DropRelics { .. } => Some(textures.relic_icon.clone()),
                            };
                            let label = objective_label(i18n, prog, is_compact);
                            let complete = prog.is_complete();
                            objs.spawn((
                                Node {
                                    padding: UiRect::axes(Val::Px(9.0), Val::Px(4.0)),
                                    column_gap: Val::Px(6.0),
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.5)),
                                    ..default()
                                },
                                BackgroundColor(if complete {
                                    Color::srgba(0.12, 0.30, 0.20, 0.92)
                                } else {
                                    COLOR_PANEL_CARD
                                }),
                                BorderColor(if complete {
                                    COLOR_ACCENT_GREEN
                                } else {
                                    COLOR_PANEL_BORDER
                                }),
                                BorderRadius::all(Val::Px(8.0)),
                            ))
                            .with_children(|pill| {
                                if let Some(icon) = icon_opt {
                                    pill.spawn((
                                        ImageNode::new(icon),
                                        Node {
                                            width: Val::Px(22.0),
                                            height: Val::Px(22.0),
                                            ..default()
                                        },
                                    ));
                                }
                                pill.spawn((
                                    Match3ObjectiveText(obj_idx),
                                    Text::new(label),
                                    TextFont {
                                        font_size: if is_compact { FONT_SMALL } else { FONT_BODY },
                                        ..default()
                                    },
                                    TextColor(if complete {
                                        COLOR_ACCENT_GREEN
                                    } else {
                                        COLOR_TEXT_PRIMARY
                                    }),
                                ));
                            });
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
                            Match3ComboText,
                            Text::new(combo_label(i18n, session)),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                        ));
                    }

                    let mut undo_btn = right.spawn((
                        Button,
                        Match3UiAction::Undo,
                        Node {
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                            min_width: Val::Px(MIN_TOUCH_TARGET_PX),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(if can_undo {
                            COLOR_ACCENT_TEAL
                        } else {
                            COLOR_BUTTON_DISABLED
                        }),
                        BorderRadius::all(Val::Px(6.0)),
                    ));
                    if !can_undo {
                        undo_btn.insert(NoButtonScale);
                    }
                    undo_btn.with_children(|btn| {
                        btn.spawn((
                            Text::new(i18n.t("match3.hud.undo")),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(if can_undo {
                                COLOR_TEXT_ON_ACCENT
                            } else {
                                COLOR_TEXT_MUTED
                            }),
                        ));
                    });

                    right
                        .spawn((
                            Button,
                            Match3UiAction::ToggleHelp,
                            Node {
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                min_width: Val::Px(MIN_TOUCH_TARGET_PX),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
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
                                min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                min_width: Val::Px(MIN_TOUCH_TARGET_PX),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(if confirm_retreat {
                                    1.5
                                } else {
                                    0.0
                                })),
                                ..default()
                            },
                            BackgroundColor(if confirm_retreat {
                                Color::srgb(0.92, 0.20, 0.22)
                            } else {
                                COLOR_ACCENT_CRIMSON
                            }),
                            BorderColor(COLOR_ACCENT_GOLD),
                            BorderRadius::all(Val::Px(6.0)),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(if confirm_retreat {
                                    i18n.t("match3.hud.retreat_confirm")
                                } else {
                                    i18n.t("match3.hud.back")
                                }),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_PRIMARY),
                            ));
                        });
                });
            });

            // Bottom active tools bar (9-slice sculpted stone & bronze frame)
            root.spawn((
                sliced_panel_image(vfx),
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(6.0),
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                    ..default()
                },
                panel_shadow(),
            ))
            .with_children(|bottom| {
                let active_hint_key = if in_valhalla {
                    Some("match3.valhalla.skip_hint")
                } else if confirm_loki_shuffle {
                    Some("match3.tool.hint.loki_shuffle")
                } else {
                    ctx.armed_tool.map(super::tool_hint_key)
                };
                let initial_tip = active_hint_key
                    .map(|k| i18n.t(k).to_string())
                    .unwrap_or_default();
                let show_cancel_tool =
                    !in_valhalla && (ctx.armed_tool.is_some() || confirm_loki_shuffle);
                bottom
                    .spawn(Node {
                        column_gap: Val::Px(10.0),
                        row_gap: Val::Px(4.0),
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|tip_row| {
                        tip_row.spawn((
                            Match3TooltipText,
                            Text::new(initial_tip),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_GOLD),
                            Node {
                                display: if active_hint_key.is_some() {
                                    Display::Flex
                                } else {
                                    Display::None
                                },
                                ..default()
                            },
                        ));
                        if show_cancel_tool {
                            tip_row
                                .spawn((
                                    Button,
                                    Match3UiAction::CancelTool,
                                    Node {
                                        padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                                        min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                        min_width: Val::Px(MIN_TOUCH_TARGET_PX),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(COLOR_ACCENT_CRIMSON),
                                    BorderRadius::all(Val::Px(6.0)),
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        Text::new(i18n.t("match3.tool.cancel")),
                                        TextFont {
                                            font_size: FONT_SMALL,
                                            ..default()
                                        },
                                        TextColor(COLOR_TEXT_PRIMARY),
                                    ));
                                });
                        }
                    });

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
                        let fury_lbl = fury_label(i18n, session, is_compact);
                        let mut fury_btn = row.spawn((
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
                                min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                min_width: Val::Px(MIN_TOUCH_TARGET_PX),
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
                        ));
                        if fury_ready {
                            fury_btn.insert((FuryReadyPulse, gold_glow_shadow()));
                        }
                        fury_btn.with_children(|btn| {
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
                                Match3FuryText,
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
                            let armed = ctx.armed_tool == Some(tool)
                                || (tool == ActiveTool::LokiShuffle && confirm_loki_shuffle);
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
                            let tool_lbl =
                                if tool == ActiveTool::LokiShuffle && confirm_loki_shuffle {
                                    i18n.t("match3.tool.confirm_loki").to_string()
                                } else if is_compact {
                                    i18n.t_fmt(
                                        "match3.tool.count_short",
                                        &[("count", count.to_string())],
                                    )
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
                                    min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                    min_width: Val::Px(MIN_TOUCH_TARGET_PX),
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
                    Button,
                    NoButtonScale,
                    Match3UiAction::ToggleHelp,
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
                .with_children(|bg| {
                    bg.spawn((
                        FocusPolicy::Block,
                        Interaction::default(),
                        sliced_panel_image(vfx),
                        Node {
                            width: Val::Px(540.0),
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
                    .with_children(|card| {
                        // Sticky modal header
                        card.spawn((
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
                                Text::new(i18n.t("city.panel.help.title")),
                                display_font(fjord_art, FONT_HEADING),
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

                        // Scrollable Guide & Quick Settings body with visual scrollbar
                        card.spawn(Node {
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
                                        offset_y: modal_scroll_y,
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
                                .with_children(|body| {
                                    // Quick Pause Settings (Audio + Colorblind) right inside Match-3
                                    body.spawn((
                                        Text::new(i18n.t("match3.pause.settings_title")),
                                        TextFont {
                                            font_size: FONT_BODY,
                                            ..default()
                                        },
                                        TextColor(COLOR_ACCENT_GOLD),
                                    ));
                                    body.spawn(Node {
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
                                        let bgm_pct =
                                            (sound_bank.bgm_volume * 200.0).round() as u32;
                                        let sfx_pct =
                                            (sound_bank.sfx_volume * 100.0).round() as u32;
                                        let cb_lbl = if ctx.colorblind_mode {
                                            i18n.t("city.panel.colorblind.on")
                                        } else {
                                            i18n.t("city.panel.colorblind.off")
                                        };
                                        let fast_lbl = if ctx.fast_animations {
                                            i18n.t("city.panel.fast_anim.on")
                                        } else {
                                            i18n.t("city.panel.fast_anim.off")
                                        };
                                        let rm_lbl = if ctx.reduced_motion {
                                            i18n.t("city.panel.reduced_motion.on")
                                        } else {
                                            i18n.t("city.panel.reduced_motion.off")
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
                                            (
                                                fast_lbl.to_string(),
                                                Match3UiAction::ToggleFastAnim,
                                                if ctx.fast_animations {
                                                    COLOR_ACCENT_TEAL
                                                } else {
                                                    COLOR_PANEL_CARD
                                                },
                                            ),
                                            (
                                                rm_lbl.to_string(),
                                                Match3UiAction::ToggleReducedMotion,
                                                if ctx.reduced_motion {
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
                                                    padding: UiRect::axes(
                                                        Val::Px(10.0),
                                                        Val::Px(6.0),
                                                    ),
                                                    min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                                    min_width: Val::Px(MIN_TOUCH_TARGET_PX),
                                                    justify_content: JustifyContent::Center,
                                                    align_items: AlignItems::Center,
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
                                                    TextColor(if bg_col == COLOR_ACCENT_TEAL {
                                                        COLOR_TEXT_ON_ACCENT
                                                    } else {
                                                        COLOR_TEXT_PRIMARY
                                                    }),
                                                ));
                                            });
                                        }
                                    });

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
                                        body.spawn((
                                            Text::new(i18n.t(t_key)),
                                            TextFont {
                                                font_size: FONT_BODY,
                                                ..default()
                                            },
                                            TextColor(COLOR_ACCENT_GOLD),
                                        ));
                                        body.spawn((
                                            Text::new(i18n.t(b_key)),
                                            TextFont {
                                                font_size: FONT_SMALL,
                                                ..default()
                                            },
                                            TextColor(COLOR_TEXT_PRIMARY),
                                        ));
                                    }
                                });
                            spawn_scrollbar_track(scroll_row, ScrollPanelKind::Modal);
                        });

                        // Auto-hiding scroll indicator
                        card.spawn((
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
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape() -> HudShape {
        HudShape {
            low_moves: false,
            has_boss: false,
            banner: HudBanner::None,
            fury_ready: false,
            objectives_complete: vec![false, false],
        }
    }

    #[test]
    fn identical_shape_updates_in_place() {
        assert!(can_update_in_place(Some(&shape()), &shape()));
    }

    #[test]
    fn missing_previous_shape_forces_rebuild() {
        assert!(!can_update_in_place(None, &shape()));
    }

    #[test]
    fn objective_completion_forces_rebuild() {
        let mut done = shape();
        done.objectives_complete[1] = true;
        assert!(!can_update_in_place(Some(&shape()), &done));
    }

    #[test]
    fn low_moves_banner_and_fury_flips_force_rebuild() {
        let mut low = shape();
        low.low_moves = true;
        assert!(!can_update_in_place(Some(&shape()), &low));
        let mut combo = shape();
        combo.banner = HudBanner::Combo;
        assert!(!can_update_in_place(Some(&shape()), &combo));
        let mut fury = shape();
        fury.fury_ready = true;
        assert!(!can_update_in_place(Some(&shape()), &fury));
    }

    #[test]
    fn boss_fights_always_rebuild() {
        let mut boss = shape();
        boss.has_boss = true;
        assert!(!can_update_in_place(Some(&boss), &boss));
    }
}
