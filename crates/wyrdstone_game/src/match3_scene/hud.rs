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
    FuryReadyPulse, LowMovesPulse, Match3BossHpFill, Match3BossHpText, Match3BottomHudBar,
    Match3Cleanup, Match3ComboText, Match3FuryFill, Match3FuryText, Match3HudBar, Match3HudRoot,
    Match3MovesText, Match3ObjectiveText, Match3OverdriveText, Match3ScoreText, Match3TimerText,
    Match3TooltipText, Match3TopHudBar, Match3UiAction, LOW_MOVES_THRESHOLD,
};
use crate::audio::SoundBank;
use crate::fjord_art::FjordTextures;
use crate::platform::{HudLayoutMode, SafeAreaInsets};
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
use wyrdstone_core::{
    ActiveTool, BlitzMedal, BossState, I18nCatalog, LevelSession, Objective, ObjectiveProgress,
    RuneType,
};

fn moves_label(i18n: &I18nCatalog, session: &LevelSession) -> String {
    if session.is_blitz {
        i18n.t_fmt(
            "match3.blitz.multiplier",
            &[("mult", session.blitz_multiplier.to_string())],
        )
    } else {
        i18n.t_fmt(
            "match3.hud.moves",
            &[("moves", session.moves_left.to_string())],
        )
    }
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
fn objective_label_for_session(
    i18n: &I18nCatalog,
    session: &LevelSession,
    prog: &ObjectiveProgress,
    is_compact: bool,
) -> String {
    if session.is_blitz {
        let medal = BlitzMedal::from_score(session.score, &session.blitz_tuning);
        return match medal.next_target(&session.blitz_tuning) {
            Some((next_medal, target)) => i18n.t_fmt(
                "match3.blitz.next_medal",
                &[
                    ("medal", i18n.t(next_medal.name_key()).to_string()),
                    ("target", target.to_string()),
                ],
            ),
            None => i18n.t("match3.blitz.valkyrie_reached").to_string(),
        };
    }
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

fn boss_hp_label(i18n: &I18nCatalog, boss: &BossState, weakness_mult: u32) -> String {
    let boss_hp_key = if boss.is_phase2() {
        "match3.boss.phase2_badge"
    } else {
        "match3.boss.hp"
    };
    let mut base = i18n.t_fmt(
        boss_hp_key,
        &[
            ("boss", i18n.t(&boss.def.name_key).to_string()),
            ("hp", boss.hp.to_string()),
            ("max", boss.def.max_hp.to_string()),
            ("mult", weakness_mult.to_string()),
            (
                "rune",
                i18n.t(boss.active_weakness().i18n_key()).to_string(),
            ),
            ("turns", boss.turns_until_riposte.to_string()),
        ],
    );
    if boss.exposed_turns > 0 {
        base.push(' ');
        base.push_str(&i18n.t_fmt(
            "match3.boss.exposed_tag",
            &[("turns", boss.exposed_turns.to_string())],
        ));
    }
    base
}

fn overdrive_label(i18n: &I18nCatalog, session: &LevelSession) -> String {
    if session.overdrive_turns_left > 0 {
        i18n.t_fmt(
            "match3.hud.overdrive_active",
            &[("turns", session.overdrive_turns_left.to_string())],
        )
    } else {
        let pct = ((session.overdrive_charge as f32 / session.overdrive_max.max(1) as f32) * 100.0)
            .round() as u32;
        i18n.t_fmt("match3.hud.overdrive_gauge", &[("pct", pct.to_string())])
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
/// When the shape is unchanged between two states, the dynamic texts and boss HP bar can be updated in place;
/// otherwise the HUD must be rebuilt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudShape {
    low_moves: bool,
    projected_stars: u8,
    has_boss: bool,
    boss_phase2: bool,
    boss_hit_flash: bool,
    boss_riposte_imminent: bool,
    boss_exposed_turns: u32,
    boss_weakness: Option<RuneType>,
    banner: HudBanner,
    fury_ready: bool,
    overdrive_active: bool,
    overdrive_turns_left: u32,
    is_blitz: bool,
    is_trial: bool,
    blitz_multiplier: u32,
    blitz_frozen: bool,
    blitz_medal: BlitzMedal,
    objectives_complete: Vec<bool>,
    ftue_coach_key: Option<&'static str>,
}

fn ftue_coach_key(total_victories: u32, session: &LevelSession) -> Option<&'static str> {
    if total_victories > 0 || session.is_trial || session.is_blitz {
        return None;
    }
    if session.can_trigger_fury() {
        Some("match3.ftue.fury_ready")
    } else if !session.board.find_booster_combo_pairs().is_empty() {
        Some("match3.ftue.booster_combo")
    } else if !session.board.find_remaining_boosters().is_empty() {
        Some("match3.ftue.booster_trigger")
    } else {
        Some("match3.ftue.first_swap")
    }
}

pub(super) fn projected_stars(
    session: &LevelSession,
    tuning: &wyrdstone_core::SessionTuning,
) -> u8 {
    let ratio = if session.initial_moves > 0 {
        session.moves_left as f32 / session.initial_moves as f32
    } else {
        0.0
    };
    if ratio >= tuning.three_star_moves_ratio {
        3
    } else if ratio >= tuning.two_star_moves_ratio {
        2
    } else {
        1
    }
}

pub(super) fn hud_shape(
    session: &LevelSession,
    tuning: &wyrdstone_core::SessionTuning,
    total_victories: u32,
) -> HudShape {
    let banner = if session.last_frost_spread.is_some() {
        HudBanner::FrostSpread
    } else if session.last_shuffled {
        HudBanner::Shuffled
    } else if session.last_combo >= 2 {
        HudBanner::Combo
    } else {
        HudBanner::None
    };
    let boss = session.boss.as_ref();
    HudShape {
        low_moves: !session.is_blitz && session.moves_left <= LOW_MOVES_THRESHOLD,
        projected_stars: projected_stars(session, tuning),
        has_boss: boss.is_some(),
        boss_phase2: boss.is_some_and(|b| b.is_phase2()),
        boss_hit_flash: boss.is_some_and(|b| b.last_damage_dealt > 0),
        boss_riposte_imminent: boss.is_some_and(|b| b.turns_until_riposte <= 1),
        boss_exposed_turns: boss.map_or(0, |b| b.exposed_turns),
        boss_weakness: boss.map(|b| b.active_weakness()),
        banner,
        fury_ready: session.can_trigger_fury(),
        overdrive_active: session.overdrive_turns_left > 0,
        overdrive_turns_left: session.overdrive_turns_left,
        is_blitz: session.is_blitz,
        is_trial: session.is_trial,
        blitz_multiplier: session.blitz_multiplier,
        blitz_frozen: session.blitz_freeze_secs > 0.0,
        blitz_medal: if session.is_blitz {
            BlitzMedal::from_score(session.score, &session.blitz_tuning)
        } else {
            BlitzMedal::None
        },
        objectives_complete: session.objectives.iter().map(|o| o.is_complete()).collect(),
        ftue_coach_key: ftue_coach_key(total_victories, session),
    }
}

/// True when the HUD can follow `new` from `old` by rewriting texts and boss HP fill in place.
pub(super) fn can_update_in_place(old: Option<&HudShape>, new: &HudShape) -> bool {
    old == Some(new)
}

type HudTextItems = (
    &'static mut Text,
    Option<&'static Match3MovesText>,
    Option<&'static Match3ScoreText>,
    Option<&'static Match3ObjectiveText>,
    Option<&'static Match3ComboText>,
    Option<&'static Match3FuryText>,
    Option<&'static Match3BossHpText>,
    Option<&'static Match3OverdriveText>,
);

type HudTextFilter = Or<(
    With<Match3MovesText>,
    With<Match3ScoreText>,
    With<Match3ObjectiveText>,
    With<Match3ComboText>,
    With<Match3FuryText>,
    With<Match3BossHpText>,
    With<Match3OverdriveText>,
)>;

const FURY_GAUGE_HEIGHT_PX: f32 = 5.0;

fn fury_charge_pct(session: &LevelSession) -> f32 {
    fury_charge_pct_values(session.fury, session.fury_max)
}

fn fury_charge_pct_values(fury: u32, fury_max: u32) -> f32 {
    ((fury as f32 / fury_max.max(1) as f32) * 100.0).clamp(0.0, 100.0)
}

/// Dynamic HUD texts and boss HP / Patron Fury bars that cascade steps rewrite in place instead of rebuilding the whole HUD.
#[derive(SystemParam)]
pub struct HudTextQueries<'w, 's> {
    texts: Query<'w, 's, HudTextItems, HudTextFilter>,
    boss_hp_fills:
        Query<'w, 's, &'static mut Node, (With<Match3BossHpFill>, Without<Match3FuryFill>)>,
    fury_fills: Query<'w, 's, &'static mut Node, (With<Match3FuryFill>, Without<Match3BossHpFill>)>,
}

impl HudTextQueries<'_, '_> {
    /// Rewrites moves, score, objective, combo, fury texts and boss HP / Patron Fury bars from the live session.
    pub(super) fn refresh(&mut self, ctx: &GameContext, is_compact: bool) {
        let Some(session) = &ctx.active_session else {
            return;
        };
        let i18n = &ctx.data.i18n;
        let weakness_mult = ctx.data.balance.boss.weakness_multiplier;
        for (mut text, moves, score, objective, combo, fury, boss_hp, overdrive) in &mut self.texts
        {
            let label = if moves.is_some() {
                moves_label(i18n, session)
            } else if score.is_some() {
                score_label(i18n, session)
            } else if let Some(Match3ObjectiveText(idx)) = objective {
                match session.objectives.get(*idx) {
                    Some(prog) => objective_label_for_session(i18n, session, prog, is_compact),
                    None => continue,
                }
            } else if combo.is_some() {
                combo_label(i18n, session)
            } else if fury.is_some() {
                fury_label(i18n, session, is_compact)
            } else if boss_hp.is_some() {
                match &session.boss {
                    Some(boss) => boss_hp_label(i18n, boss, weakness_mult),
                    None => continue,
                }
            } else if overdrive.is_some() {
                overdrive_label(i18n, session)
            } else {
                continue;
            };
            if text.0 != label {
                text.0 = label;
            }
        }
        let fury_target = Val::Percent(fury_charge_pct(session));
        for mut node in &mut self.fury_fills {
            if node.width != fury_target {
                node.width = fury_target;
            }
        }
        if let Some(boss) = &session.boss {
            let hp_pct = (boss.hp as f32 / boss.def.max_hp.max(1) as f32 * 100.0).clamp(0.0, 100.0);
            let target = Val::Percent(hp_pct);
            for mut node in &mut self.boss_hp_fills {
                if node.width != target {
                    node.width = target;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_hud_divider(parent: &mut ChildBuilder) {
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::srgba(0.93, 0.74, 0.29, 0.25)),
    ));
}

fn spawn_section_header(parent: &mut ChildBuilder, fjord_art: &FjordTextures, label: &str) {
    parent.spawn((
        Text::new(label),
        display_font(fjord_art, FONT_BODY),
        TextColor(COLOR_ACCENT_GOLD),
    ));
}

fn spawn_moves_score_overdrive(
    stats: &mut ChildBuilder,
    ctx: &GameContext,
    session: &LevelSession,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    vertical: bool,
) {
    let i18n = &ctx.data.i18n;
    let low_moves = !session.is_blitz && session.moves_left <= LOW_MOVES_THRESHOLD;
    let hot_blitz = session.is_blitz && session.blitz_multiplier >= 4;
    let mut moves_box = stats.spawn((
        Node {
            padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
            justify_content: if vertical {
                JustifyContent::SpaceBetween
            } else {
                JustifyContent::FlexStart
            },
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(if low_moves || hot_blitz { 2.0 } else { 1.0 })),
            ..default()
        },
        BackgroundColor(if low_moves {
            Color::srgba(0.36, 0.08, 0.10, 0.92)
        } else if hot_blitz {
            Color::srgba(0.42, 0.22, 0.04, 0.95)
        } else if vertical {
            COLOR_PANEL_CARD
        } else {
            Color::NONE
        }),
        BorderColor(if low_moves {
            COLOR_ACCENT_CRIMSON
        } else if hot_blitz {
            COLOR_ACCENT_GOLD
        } else if vertical {
            COLOR_PANEL_BORDER
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
        if session.is_trial {
            mbox.spawn((
                Text::new(i18n.t_fmt(
                    "match3.hud.trial_par",
                    &[("par", session.trial_par_moves.to_string())],
                )),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_ACCENT_TEAL_TEXT),
                Node {
                    margin: UiRect::left(Val::Px(6.0)),
                    ..default()
                },
            ));
        } else if !session.is_blitz
            && session.boss.is_none()
            && session.level.building_id != wyrdstone_core::expedition::EXPEDITION_BUILDING_ID
        {
            let stars_now = projected_stars(session, &ctx.data.balance.session);
            mbox.spawn(Node {
                column_gap: Val::Px(2.0),
                margin: UiRect::left(Val::Px(6.0)),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|srow| {
                for s_idx in 0..3u8 {
                    let active = s_idx < stars_now;
                    srow.spawn((
                        ImageNode::new(vfx.icon_star.clone()).with_color(if active {
                            Color::WHITE
                        } else {
                            Color::srgba(0.28, 0.34, 0.44, 0.40)
                        }),
                        Node {
                            width: Val::Px(13.0),
                            height: Val::Px(13.0),
                            ..default()
                        },
                    ));
                }
            });
        }
    });

    if let Some(secs) = session.time_left_secs {
        let whole = secs.ceil() as u32;
        let frozen = session.is_blitz && session.blitz_freeze_secs > 0.0;
        let timer_text = if frozen {
            i18n.t_fmt("match3.hud.timer_frozen", &[("secs", whole.to_string())])
        } else {
            i18n.t_fmt("match3.hud.timer", &[("secs", whole.to_string())])
        };
        stats.spawn((
            Match3TimerText,
            Text::new(timer_text),
            display_font(fjord_art, FONT_HEADING),
            TextColor(if frozen {
                Color::srgb(0.45, 0.88, 1.0)
            } else if whole <= 15 {
                COLOR_ACCENT_CRIMSON_TEXT
            } else {
                COLOR_ACCENT_TEAL_TEXT
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

    if session.session_tuning.overdrive_turns > 0 {
        let od_active = session.overdrive_turns_left > 0;
        stats
            .spawn((
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    justify_content: if vertical {
                        JustifyContent::Center
                    } else {
                        JustifyContent::FlexStart
                    },
                    ..default()
                },
                BackgroundColor(if od_active {
                    Color::srgba(0.42, 0.22, 0.04, 0.95)
                } else {
                    COLOR_PANEL_CARD
                }),
                BorderColor(if od_active {
                    COLOR_ACCENT_GOLD
                } else {
                    COLOR_PANEL_BORDER
                }),
                BorderRadius::all(Val::Px(6.0)),
            ))
            .with_children(|od| {
                od.spawn((
                    Match3OverdriveText,
                    Text::new(overdrive_label(i18n, session)),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(if od_active {
                        COLOR_ACCENT_GOLD
                    } else {
                        COLOR_ACCENT_TEAL_TEXT
                    }),
                ));
            });
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_objectives_or_boss(
    objs: &mut ChildBuilder,
    ctx: &GameContext,
    session: &LevelSession,
    fjord_art: &FjordTextures,
    textures: &RuneTextures,
    vfx: &VfxTextures,
    hud_mode: HudLayoutMode,
) {
    let i18n = &ctx.data.i18n;
    let is_compact = hud_mode.is_compact();
    let is_portrait = hud_mode == HudLayoutMode::Portrait;
    let is_wide = hud_mode.is_desktop_wide();
    if let Some(boss) = &session.boss {
        let hp_pct = (boss.hp as f32 / boss.def.max_hp.max(1) as f32 * 100.0).clamp(0.0, 100.0);
        objs.spawn(Node {
            flex_direction: if is_wide {
                FlexDirection::Column
            } else {
                FlexDirection::Row
            },
            flex_wrap: FlexWrap::Wrap,
            align_items: if is_wide {
                AlignItems::Stretch
            } else {
                AlignItems::Center
            },
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(6.0),
            width: if is_wide {
                Val::Percent(100.0)
            } else {
                Val::Auto
            },
            ..default()
        })
        .with_children(|brow| {
            brow.spawn(Node {
                column_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                justify_content: if is_wide {
                    JustifyContent::Center
                } else {
                    JustifyContent::FlexStart
                },
                ..default()
            })
            .with_children(|icons| {
                if let Some(img) = fjord_art.boss_portrait(&boss.def.id) {
                    let hit_flash = boss.last_damage_dealt > 0;
                    icons.spawn((
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
                            width: Val::Px(if is_wide {
                                54.0
                            } else if is_portrait {
                                36.0
                            } else {
                                46.0
                            }),
                            height: Val::Px(if is_wide {
                                54.0
                            } else if is_portrait {
                                36.0
                            } else {
                                46.0
                            }),
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
                if let Some(weak_icon) = textures.runes.get(&boss.active_weakness()).cloned() {
                    icons
                        .spawn((
                            Node {
                                padding: UiRect::all(Val::Px(3.0)),
                                border: UiRect::all(Val::Px(1.5)),
                                ..default()
                            },
                            BackgroundColor(COLOR_PANEL_CARD),
                            BorderColor(COLOR_ACCENT_TEAL),
                            BorderRadius::all(Val::Px(RADIUS_MD)),
                        ))
                        .with_children(|wbox| {
                            wbox.spawn(Node {
                                width: Val::Px(26.0),
                                height: Val::Px(26.0),
                                ..default()
                            })
                            .with_children(|icon_wrap| {
                                icon_wrap.spawn((
                                    ImageNode::new(weak_icon),
                                    Node {
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                ));
                                if ctx.colorblind_mode {
                                    if let Some(badge) = textures
                                        .colorblind_badges
                                        .get(&boss.active_weakness())
                                        .cloned()
                                    {
                                        icon_wrap.spawn((
                                            ImageNode::new(badge),
                                            Node {
                                                position_type: PositionType::Absolute,
                                                left: Val::Px(0.0),
                                                top: Val::Px(0.0),
                                                width: Val::Percent(100.0),
                                                height: Val::Percent(100.0),
                                                ..default()
                                            },
                                        ));
                                    }
                                }
                            });
                        });
                }
            });

            brow.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                align_items: if is_wide {
                    AlignItems::Stretch
                } else {
                    AlignItems::Center
                },
                ..default()
            })
            .with_children(|bcol| {
                if let Some(run) = &ctx.active_gauntlet {
                    bcol.spawn((
                        Text::new(i18n.t_fmt(
                            "match3.hud.gauntlet_stage",
                            &[
                                ("cur", (run.stage_index + 1).to_string()),
                                ("total", run.total_stages.to_string()),
                            ],
                        )),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                }
                bcol.spawn((
                    Match3BossHpText,
                    Text::new(boss_hp_label(
                        i18n,
                        boss,
                        ctx.data.balance.boss.weakness_multiplier,
                    )),
                    TextFont {
                        font_size: if is_compact || is_wide {
                            FONT_SMALL
                        } else {
                            FONT_BODY
                        },
                        ..default()
                    },
                    TextColor(if boss.turns_until_riposte <= 1 {
                        COLOR_ACCENT_CRIMSON_TEXT
                    } else if boss.is_phase2() {
                        COLOR_ACCENT_GOLD
                    } else {
                        COLOR_ACCENT_CRIMSON_TEXT
                    }),
                ));
                bcol.spawn((
                    Node {
                        width: if is_wide {
                            Val::Percent(100.0)
                        } else if is_portrait {
                            Val::Px(132.0)
                        } else if is_compact {
                            Val::Px(150.0)
                        } else {
                            Val::Px(280.0)
                        },
                        height: Val::Px(10.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.16, 0.08, 0.10, 0.95)),
                    BorderColor(if boss.turns_until_riposte <= 1 {
                        COLOR_ACCENT_CRIMSON
                    } else {
                        COLOR_ACCENT_GOLD
                    }),
                    BorderRadius::all(Val::Px(5.0)),
                ))
                .with_children(|track| {
                    track.spawn((
                        Match3BossHpFill,
                        Node {
                            width: Val::Percent(hp_pct),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(COLOR_ACCENT_CRIMSON),
                        BorderRadius::all(Val::Px(4.0)),
                    ));
                    // Phase-2 enrage threshold marker at 50% HP.
                    track.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(50.0),
                            top: Val::Px(0.0),
                            width: Val::Px(2.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(COLOR_ACCENT_GOLD),
                    ));
                });
                if boss.turns_until_riposte <= 1 && !boss.is_defeated() {
                    bcol.spawn((
                        Text::new(i18n.t_fmt(
                            "match3.boss.stagger_hint",
                            &[
                                (
                                    "tiles",
                                    ctx.data.balance.boss.stagger_weakness_tiles.to_string(),
                                ),
                                (
                                    "rune",
                                    i18n.t(boss.active_weakness().i18n_key()).to_string(),
                                ),
                                ("combo", ctx.data.balance.boss.stagger_min_combo.to_string()),
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
        });
    } else {
        for (obj_idx, prog) in session.objectives.iter().enumerate() {
            let icon_opt = match &prog.objective {
                Objective::ClearIce { .. } => Some(textures.ice_top_layer1.clone()),
                Objective::CollectRune { rune, .. } => textures.runes.get(rune).cloned(),
                Objective::ReachScore { .. } => Some(vfx.icon_star.clone()),
                Objective::DropRelics { .. } => Some(textures.relic_icon.clone()),
            };
            let label = objective_label_for_session(i18n, session, prog, is_compact);
            let complete = prog.is_complete();
            objs.spawn((
                Node {
                    width: if is_wide {
                        Val::Percent(100.0)
                    } else {
                        Val::Auto
                    },
                    padding: if is_portrait {
                        UiRect::axes(Val::Px(7.0), Val::Px(3.0))
                    } else {
                        UiRect::axes(Val::Px(9.0), Val::Px(6.0))
                    },
                    column_gap: Val::Px(if is_portrait { 5.0 } else { 8.0 }),
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
                    let badge_opt = if ctx.colorblind_mode {
                        match &prog.objective {
                            Objective::CollectRune { rune, .. } => {
                                textures.colorblind_badges.get(rune).cloned()
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let icon_px = if is_portrait { 18.0 } else { 22.0 };
                    pill.spawn(Node {
                        width: Val::Px(icon_px),
                        height: Val::Px(icon_px),
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_children(|icon_box| {
                        icon_box.spawn((
                            ImageNode::new(icon),
                            Node {
                                width: Val::Px(icon_px),
                                height: Val::Px(icon_px),
                                ..default()
                            },
                        ));
                        if let Some(badge) = badge_opt {
                            icon_box.spawn((
                                ImageNode::new(badge),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(0.0),
                                    top: Val::Px(0.0),
                                    width: Val::Px(icon_px),
                                    height: Val::Px(icon_px),
                                    ..default()
                                },
                            ));
                        }
                    });
                }
                pill.spawn((
                    Match3ObjectiveText(obj_idx),
                    Text::new(label),
                    TextFont {
                        font_size: if is_compact { FONT_SMALL } else { FONT_BODY },
                        ..default()
                    },
                    TextColor(if complete {
                        COLOR_ACCENT_GREEN_TEXT
                    } else {
                        COLOR_TEXT_PRIMARY
                    }),
                ));
            });
        }
    }
}

fn spawn_ftue_coach_pill(
    parent: &mut ChildBuilder,
    i18n: &I18nCatalog,
    total_victories: u32,
    session: &LevelSession,
    full_width_row: bool,
) {
    let Some(key) = ftue_coach_key(total_victories, session) else {
        return;
    };
    parent
        .spawn((
            Node {
                width: if full_width_row {
                    Val::Percent(100.0)
                } else {
                    Val::Auto
                },
                padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                border: UiRect::all(Val::Px(1.5)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(COLOR_PANEL_CARD),
            BorderColor(COLOR_ACCENT_GOLD),
            BorderRadius::all(Val::Px(8.0)),
        ))
        .with_children(|pill| {
            pill.spawn((
                Text::new(i18n.t(key)),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_ACCENT_GOLD),
            ));
        });
}

fn spawn_status_banner(parent: &mut ChildBuilder, i18n: &I18nCatalog, session: &LevelSession) {
    if session.last_frost_spread.is_some() {
        parent.spawn((
            Text::new(i18n.t("match3.hud.frost_spread")),
            TextFont {
                font_size: FONT_SMALL,
                ..default()
            },
            TextColor(Color::srgb(0.45, 0.86, 1.0)),
        ));
    } else if session.last_shuffled {
        parent.spawn((
            Text::new(i18n.t("match3.hud.shuffled")),
            TextFont {
                font_size: FONT_SMALL,
                ..default()
            },
            TextColor(COLOR_ACCENT_GOLD),
        ));
    } else if session.last_combo >= 2 {
        parent.spawn((
            Match3ComboText,
            Text::new(combo_label(i18n, session)),
            TextFont {
                font_size: FONT_BODY,
                ..default()
            },
            TextColor(COLOR_ACCENT_GOLD),
        ));
    }
}

fn spawn_active_buffs_card(parent: &mut ChildBuilder, i18n: &I18nCatalog, session: &LevelSession) {
    if session.is_trial {
        return;
    }
    let mut entries: Vec<(String, Color)> = Vec::new();
    if let Some(blessing) = session.daily_blessing {
        entries.push((
            i18n.t(blessing.name_key()).to_string(),
            COLOR_ACCENT_TEAL_TEXT,
        ));
    }
    if session.active_flame_tier != wyrdstone_core::ValhallaFlameTier::None {
        entries.push((
            i18n.t(session.active_flame_tier.name_key()).to_string(),
            COLOR_ACCENT_GOLD,
        ));
    }
    if let Some(aid_key) = session.active_valkyrie_aid.desc_key() {
        entries.push((i18n.t(aid_key).to_string(), COLOR_ACCENT_GREEN_TEXT));
    }
    if let Some(draught) = session.active_war_draught {
        entries.push((i18n.t(draught.name_key()).to_string(), COLOR_TEXT_PRIMARY));
    }
    if entries.is_empty() {
        return;
    }
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(COLOR_PANEL_CARD),
            BorderColor(COLOR_ACCENT_GOLD),
            BorderRadius::all(Val::Px(RADIUS_SM)),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(i18n.t("match3.hud.active_buffs")),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_ACCENT_GOLD),
            ));
            for (line, col) in entries {
                card.spawn((
                    Text::new(line),
                    TextFont {
                        font_size: FONT_SMALL - 1.0,
                        ..default()
                    },
                    TextColor(col),
                ));
            }
        });
}

fn spawn_utility_buttons(
    row: &mut ChildBuilder,
    i18n: &I18nCatalog,
    can_undo: bool,
    undo_used: bool,
    confirm_retreat: bool,
    show_hint_btn: bool,
) {
    if show_hint_btn {
        row.spawn((
            Button,
            Match3UiAction::RequestHint,
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
                Text::new(i18n.t("match3.hud.hint")),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_ACCENT_TEAL_TEXT),
            ));
        });
    }
    let mut undo_btn = row.spawn((
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
        let undo_lbl = if undo_used {
            i18n.t("match3.hud.undo_used")
        } else {
            i18n.t("match3.hud.undo_ready")
        };
        btn.spawn((
            Text::new(undo_lbl),
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

    row.spawn((
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
            Text::new(i18n.t("match3.hud.pause")),
            TextFont {
                font_size: FONT_SMALL,
                ..default()
            },
            TextColor(COLOR_ACCENT_GOLD),
        ));
    });

    row.spawn((
        Button,
        Match3UiAction::Retreat,
        Node {
            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
            min_width: Val::Px(MIN_TOUCH_TARGET_PX),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(if confirm_retreat { 1.5 } else { 0.0 })),
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
}

fn spawn_fury_card(
    parent: &mut ChildBuilder,
    i18n: &I18nCatalog,
    session: &LevelSession,
    fjord_art: &FjordTextures,
    textures: &RuneTextures,
    hud_mode: HudLayoutMode,
    colorblind_mode: bool,
) {
    let is_compact = hud_mode.is_compact();
    let is_wide = hud_mode.is_desktop_wide();
    let fury_ready = session.can_trigger_fury();
    let fury_pct = fury_charge_pct(session);
    let fury_lbl = fury_label(i18n, session, is_compact);
    let mut fury_btn = parent.spawn((
        Button,
        Match3UiAction::TriggerFury,
        Node {
            width: if is_wide {
                Val::Percent(100.0)
            } else {
                Val::Auto
            },
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            align_items: AlignItems::Stretch,
            padding: if is_compact {
                UiRect::axes(Val::Px(6.0), Val::Px(4.0))
            } else {
                UiRect::axes(Val::Px(10.0), Val::Px(6.0))
            },
            justify_content: JustifyContent::Center,
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
        btn.spawn(Node {
            width: Val::Percent(100.0),
            column_gap: Val::Px(if is_compact { 4.0 } else { 8.0 }),
            justify_content: if is_wide {
                JustifyContent::SpaceBetween
            } else {
                JustifyContent::Center
            },
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn(Node {
                column_gap: Val::Px(if is_compact { 4.0 } else { 8.0 }),
                align_items: AlignItems::Center,
                flex_shrink: 1.0,
                ..default()
            })
            .with_children(|left| {
                if let Some(p_img) = fjord_art.patron_portrait(session.patron) {
                    let icon_px = if is_compact { 26.0 } else { 32.0 };
                    left.spawn((
                        ImageNode::new(p_img),
                        Node {
                            width: Val::Px(icon_px),
                            height: Val::Px(icon_px),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BorderRadius::all(Val::Px(6.0)),
                    ));
                }
                if let Some(fav_img) = textures.runes.get(&session.patron.favored_rune()).cloned() {
                    let fav_px = if is_compact { 14.0 } else { 18.0 };
                    left.spawn(Node {
                        width: Val::Px(fav_px),
                        height: Val::Px(fav_px),
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_children(|fbox| {
                        fbox.spawn((
                            ImageNode::new(fav_img),
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                        ));
                        if colorblind_mode {
                            if let Some(badge) = textures
                                .colorblind_badges
                                .get(&session.patron.favored_rune())
                                .cloned()
                            {
                                fbox.spawn((
                                    ImageNode::new(badge),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        top: Val::Px(0.0),
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                ));
                            }
                        }
                    });
                }
                left.spawn((
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
            if is_wide {
                row.spawn((
                    Text::new(i18n.t("match3.hud.keycap_fury")),
                    TextFont {
                        font_size: FONT_SMALL - 2.0,
                        ..default()
                    },
                    TextColor(if fury_ready {
                        COLOR_TEXT_DARK
                    } else {
                        COLOR_TEXT_MUTED
                    }),
                ));
            }
        });

        // Visual Patron Fury charge gauge updated in-place on every cascade.
        btn.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(FURY_GAUGE_HEIGHT_PX),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.10, 0.78)),
            BorderRadius::all(Val::Px(3.0)),
        ))
        .with_children(|track| {
            track.spawn((
                Match3FuryFill,
                Node {
                    width: Val::Percent(fury_pct),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(if fury_ready {
                    Color::srgb(0.24, 0.14, 0.03)
                } else {
                    COLOR_ACCENT_GOLD
                }),
                BorderRadius::all(Val::Px(3.0)),
            ));
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn spawn_tools_section(
    row: &mut ChildBuilder,
    ctx: &GameContext,
    session: &LevelSession,
    fjord_art: &FjordTextures,
    textures: &RuneTextures,
    hud_mode: HudLayoutMode,
    confirm_loki_shuffle: bool,
) {
    let i18n = &ctx.data.i18n;
    let is_compact = hud_mode.is_compact();
    let is_wide = hud_mode.is_desktop_wide();
    if session.is_trial {
        let trial_hint = ctx
            .data
            .trial_by_id(&session.level.id)
            .map(|t| i18n.t(&t.hint_key).to_string())
            .unwrap_or_default();
        if !trial_hint.is_empty() {
            row.spawn((
                Text::new(trial_hint),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_TEXT_PRIMARY),
            ));
        }
        row.spawn((
            Button,
            Match3UiAction::RestartBoard,
            Node {
                width: if is_wide {
                    Val::Percent(100.0)
                } else {
                    Val::Auto
                },
                padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(COLOR_ACCENT_TEAL),
            BorderRadius::all(Val::Px(8.0)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(i18n.t("match3.hud.restart")),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(COLOR_TEXT_ON_ACCENT),
            ));
        });
        return;
    }

    if !is_wide {
        spawn_fury_card(
            row,
            i18n,
            session,
            fjord_art,
            textures,
            hud_mode,
            ctx.colorblind_mode,
        );
    }

    let tools = [
        (1, ActiveTool::Mjolnir, "match3.tool.mjolnir"),
        (2, ActiveTool::SurtrEmber, "match3.tool.surtr_ember"),
        (3, ActiveTool::ThorBolt, "match3.tool.thor_bolt"),
        (4, ActiveTool::BifrostPrism, "match3.tool.bifrost_prism"),
        (5, ActiveTool::DrakkarCharge, "match3.tool.drakkar_charge"),
        (6, ActiveTool::LokiShuffle, "match3.tool.loki_shuffle"),
    ];
    for (hotkey, tool, key) in tools {
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
        } else if count == 0 {
            COLOR_TEXT_MUTED
        } else {
            COLOR_TEXT_PRIMARY
        };
        let base_lbl = if tool == ActiveTool::LokiShuffle && confirm_loki_shuffle {
            i18n.t("match3.tool.confirm_loki").to_string()
        } else if is_compact {
            i18n.t_fmt("match3.tool.count_short", &[("count", count.to_string())])
        } else {
            i18n.t_fmt(key, &[("count", count.to_string())])
        };
        let tool_lbl = if is_wide {
            i18n.t_fmt(
                "match3.tool.keycap_label",
                &[("key", hotkey.to_string()), ("label", base_lbl)],
            )
        } else {
            base_lbl
        };
        let mut tool_btn = row.spawn((
            Button,
            Match3UiAction::SelectTool(tool),
            Node {
                width: if is_wide {
                    Val::Percent(100.0)
                } else {
                    Val::Auto
                },
                padding: if is_compact {
                    UiRect::axes(Val::Px(6.0), Val::Px(4.0))
                } else {
                    UiRect::axes(Val::Px(8.0), Val::Px(5.0))
                },
                column_gap: Val::Px(if is_compact { 4.0 } else { 8.0 }),
                justify_content: if is_wide {
                    JustifyContent::FlexStart
                } else {
                    JustifyContent::Center
                },
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
        ));
        if count == 0 {
            tool_btn.insert(NoButtonScale);
        }
        tool_btn.with_children(|btn| {
            if let Some(icon) = fjord_art.tool_icon(tool) {
                let icon_px = if is_compact {
                    26.0
                } else if is_wide {
                    30.0
                } else {
                    34.0
                };
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
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BorderRadius::all(Val::Px(6.0)),
                ));
            }
            btn.spawn((
                Text::new(tool_lbl),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(text_col),
            ));
        });
    }
}

fn spawn_tooltip_section(
    parent: &mut ChildBuilder,
    ctx: &GameContext,
    confirm_loki_shuffle: bool,
    in_valhalla: bool,
) {
    let i18n = &ctx.data.i18n;
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
    let show_cancel_tool = !in_valhalla && (ctx.armed_tool.is_some() || confirm_loki_shuffle);
    parent
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
    left_rail_scroll_y: f32,
    right_rail_scroll_y: f32,
    hud_mode: HudLayoutMode,
    active_drawer: super::Match3Drawer,
    safe: SafeAreaInsets,
    can_undo: bool,
    undo_used: bool,
    confirm_retreat: bool,
    confirm_loki_shuffle: bool,
    confirm_restart: bool,
    is_fullscreen: bool,
    in_valhalla: bool,
) {
    let i18n = &ctx.data.i18n;
    let Some(session) = &ctx.active_session else {
        return;
    };
    let is_compact = hud_mode.is_compact();
    let is_portrait = hud_mode == HudLayoutMode::Portrait;
    let is_wide = hud_mode.is_desktop_wide();

    commands
        .spawn((
            Match3Cleanup,
            Match3HudRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: if is_wide {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                justify_content: JustifyContent::SpaceBetween,
                align_items: if is_wide {
                    AlignItems::Stretch
                } else {
                    AlignItems::Center
                },
                padding: if is_portrait {
                    UiRect {
                        left: Val::Px(6.0 + safe.left),
                        right: Val::Px(6.0 + safe.right),
                        top: Val::Px(6.0 + safe.top),
                        bottom: Val::Px(6.0 + safe.bottom),
                    }
                } else {
                    UiRect {
                        left: Val::Px(12.0 + safe.left),
                        right: Val::Px(12.0 + safe.right),
                        top: Val::Px(12.0 + safe.top),
                        bottom: Val::Px(12.0 + safe.bottom),
                    }
                },
                ..default()
            },
        ))
        .with_children(|root| {
            if is_wide {
                // Desktop Wide Left Rail: Scrollable stats & objectives body + sticky status footer.
                root.spawn((
                    Match3HudBar,
                    Interaction::None,
                    FocusPolicy::Block,
                    sliced_panel_image(vfx),
                    Node {
                        width: Val::Px(DESKTOP_SIDE_RAIL_WIDTH_PX),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Stretch,
                        padding: UiRect::all(Val::Px(14.0)),
                        row_gap: Val::Px(10.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    panel_shadow(),
                ))
                .with_children(|left| {
                    left.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_grow: 1.0,
                        flex_shrink: 1.0,
                        min_height: Val::Px(0.0),
                        column_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|scroll_row| {
                        scroll_row
                            .spawn((
                                ScrollablePanel {
                                    kind: ScrollPanelKind::LeftRail,
                                },
                                ScrollPosition {
                                    offset_x: 0.0,
                                    offset_y: left_rail_scroll_y,
                                },
                                RelativeCursorPosition::default(),
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    flex_grow: 1.0,
                                    flex_shrink: 1.0,
                                    min_height: Val::Px(0.0),
                                    overflow: Overflow::scroll_y(),
                                    row_gap: Val::Px(10.0),
                                    align_items: AlignItems::Stretch,
                                    padding: UiRect::right(Val::Px(2.0)),
                                    ..default()
                                },
                            ))
                            .with_children(|top_sec| {
                                top_sec
                                    .spawn(Node {
                                        flex_direction: FlexDirection::Column,
                                        row_gap: Val::Px(8.0),
                                        align_items: AlignItems::Stretch,
                                        ..default()
                                    })
                                    .with_children(|stats| {
                                        spawn_moves_score_overdrive(
                                            stats, ctx, session, fjord_art, vfx, true,
                                        );
                                    });
                                spawn_hud_divider(top_sec);
                                let sec_key = if session.boss.is_some() {
                                    "match3.hud.section.boss"
                                } else {
                                    "match3.hud.section.objectives"
                                };
                                spawn_section_header(top_sec, fjord_art, i18n.t(sec_key));
                                top_sec
                                    .spawn(Node {
                                        flex_direction: FlexDirection::Column,
                                        row_gap: Val::Px(6.0),
                                        align_items: AlignItems::Stretch,
                                        ..default()
                                    })
                                    .with_children(|objs| {
                                        spawn_objectives_or_boss(
                                            objs, ctx, session, fjord_art, textures, vfx, hud_mode,
                                        );
                                    });
                                spawn_active_buffs_card(top_sec, i18n, session);
                            });
                        spawn_scrollbar_track(scroll_row, ScrollPanelKind::LeftRail);
                    });

                    left.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 0.0,
                        row_gap: Val::Px(6.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|status_col| {
                        spawn_ftue_coach_pill(
                            status_col,
                            i18n,
                            ctx.city.stats.total_victories,
                            session,
                            true,
                        );
                        spawn_status_banner(status_col, i18n, session);
                    });
                });

                // Desktop Wide Right Rail: Sticky Utility controls + scrollable Fury & Arsenal + sticky Tooltip footer.
                root.spawn((
                    Match3HudBar,
                    Interaction::None,
                    FocusPolicy::Block,
                    sliced_panel_image(vfx),
                    Node {
                        width: Val::Px(DESKTOP_SIDE_RAIL_WIDTH_PX),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Stretch,
                        padding: UiRect::all(Val::Px(14.0)),
                        row_gap: Val::Px(8.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    panel_shadow(),
                ))
                .with_children(|right| {
                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 0.0,
                            row_gap: Val::Px(8.0),
                            align_items: AlignItems::Stretch,
                            ..default()
                        })
                        .with_children(|hdr| {
                            hdr.spawn(Node {
                                column_gap: Val::Px(6.0),
                                row_gap: Val::Px(6.0),
                                flex_wrap: FlexWrap::Wrap,
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|u_row| {
                                spawn_utility_buttons(
                                    u_row,
                                    i18n,
                                    can_undo,
                                    undo_used,
                                    confirm_retreat,
                                    !in_valhalla,
                                );
                            });
                            spawn_hud_divider(hdr);
                        });

                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            min_height: Val::Px(0.0),
                            column_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|scroll_row| {
                            scroll_row
                                .spawn((
                                    ScrollablePanel {
                                        kind: ScrollPanelKind::RightRail,
                                    },
                                    ScrollPosition {
                                        offset_x: 0.0,
                                        offset_y: right_rail_scroll_y,
                                    },
                                    RelativeCursorPosition::default(),
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        flex_grow: 1.0,
                                        flex_shrink: 1.0,
                                        min_height: Val::Px(0.0),
                                        overflow: Overflow::scroll_y(),
                                        row_gap: Val::Px(8.0),
                                        align_items: AlignItems::Stretch,
                                        padding: UiRect::right(Val::Px(2.0)),
                                        ..default()
                                    },
                                ))
                                .with_children(|top_sec| {
                                    if !session.is_trial {
                                        spawn_fury_card(
                                            top_sec,
                                            i18n,
                                            session,
                                            fjord_art,
                                            textures,
                                            hud_mode,
                                            ctx.colorblind_mode,
                                        );
                                        spawn_hud_divider(top_sec);
                                    }
                                    spawn_section_header(
                                        top_sec,
                                        fjord_art,
                                        i18n.t("match3.hud.section.arsenal"),
                                    );
                                    top_sec
                                        .spawn(Node {
                                            flex_direction: FlexDirection::Column,
                                            row_gap: Val::Px(6.0),
                                            align_items: AlignItems::Stretch,
                                            ..default()
                                        })
                                        .with_children(|tools_col| {
                                            spawn_tools_section(
                                                tools_col,
                                                ctx,
                                                session,
                                                fjord_art,
                                                textures,
                                                hud_mode,
                                                confirm_loki_shuffle,
                                            );
                                        });
                                });
                            spawn_scrollbar_track(scroll_row, ScrollPanelKind::RightRail);
                        });

                    right
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 0.0,
                            row_gap: Val::Px(6.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|foot| {
                            spawn_tooltip_section(foot, ctx, confirm_loki_shuffle, in_valhalla);
                            foot.spawn((
                                Text::new(i18n.t("match3.hud.shortcuts_hint")),
                                TextFont {
                                    font_size: FONT_SMALL - 2.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_MUTED),
                            ));
                        });
                });
            } else if is_portrait {
                spawn_portrait_top_bar(root, ctx, session, fjord_art, textures, vfx, hud_mode);
                spawn_portrait_bottom_dock(
                    root,
                    ctx,
                    session,
                    fjord_art,
                    textures,
                    vfx,
                    hud_mode,
                    active_drawer,
                    confirm_loki_shuffle,
                    in_valhalla,
                );
                if active_drawer != super::Match3Drawer::None
                    && ctx.active_modal != ActiveModal::Help
                {
                    spawn_portrait_drawer_overlay(
                        root,
                        ctx,
                        session,
                        fjord_art,
                        vfx,
                        sound_bank,
                        safe,
                        active_drawer,
                        can_undo,
                        undo_used,
                        confirm_retreat,
                        confirm_loki_shuffle,
                        is_fullscreen,
                        in_valhalla,
                    );
                }
            } else {
                // Top HUD bar (Compact layout)
                root.spawn((
                    Match3HudBar,
                    Match3TopHudBar,
                    Interaction::None,
                    FocusPolicy::Block,
                    sliced_panel_image(vfx),
                    Node {
                        width: Val::Percent(100.0),
                        max_width: Val::Px(1680.0),
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: if is_compact {
                            UiRect::axes(Val::Px(8.0), Val::Px(6.0))
                        } else {
                            UiRect::axes(Val::Px(16.0), Val::Px(12.0))
                        },
                        column_gap: Val::Px(if is_compact { 8.0 } else { 14.0 }),
                        row_gap: Val::Px(if is_compact { 4.0 } else { 6.0 }),
                        ..default()
                    },
                    panel_shadow(),
                ))
                .with_children(|top| {
                    top.spawn(Node {
                        column_gap: Val::Px(if is_compact { 10.0 } else { 18.0 }),
                        row_gap: Val::Px(4.0),
                        flex_wrap: FlexWrap::Wrap,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|stats| {
                        spawn_moves_score_overdrive(stats, ctx, session, fjord_art, vfx, false);
                    });

                    top.spawn(Node {
                        column_gap: Val::Px(14.0),
                        flex_wrap: FlexWrap::Wrap,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|objs| {
                        spawn_objectives_or_boss(
                            objs, ctx, session, fjord_art, textures, vfx, hud_mode,
                        );
                    });

                    top.spawn(Node {
                        column_gap: Val::Px(8.0),
                        row_gap: Val::Px(4.0),
                        flex_wrap: FlexWrap::Wrap,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|right| {
                        spawn_status_banner(right, i18n, session);
                        spawn_utility_buttons(
                            right,
                            i18n,
                            can_undo,
                            undo_used,
                            confirm_retreat,
                            !in_valhalla,
                        );
                    });

                    spawn_ftue_coach_pill(top, i18n, ctx.city.stats.total_victories, session, true);
                });

                // Bottom active tools bar (Compact & Portrait layouts)
                root.spawn((
                    Match3HudBar,
                    Match3BottomHudBar,
                    Interaction::None,
                    FocusPolicy::Block,
                    sliced_panel_image(vfx),
                    Node {
                        width: Val::Percent(100.0),
                        max_width: Val::Px(1680.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(if is_compact { 4.0 } else { 6.0 }),
                        padding: if is_compact {
                            UiRect::axes(Val::Px(8.0), Val::Px(6.0))
                        } else {
                            UiRect::axes(Val::Px(16.0), Val::Px(10.0))
                        },
                        ..default()
                    },
                    panel_shadow(),
                ))
                .with_children(|bottom| {
                    spawn_tooltip_section(bottom, ctx, confirm_loki_shuffle, in_valhalla);
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
                            spawn_tools_section(
                                row,
                                ctx,
                                session,
                                fjord_art,
                                textures,
                                hud_mode,
                                confirm_loki_shuffle,
                            );
                        });
                });
            }

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
                        padding: UiRect {
                            left: Val::Px(14.0 + safe.left),
                            right: Val::Px(14.0 + safe.right),
                            top: Val::Px(14.0 + safe.top),
                            bottom: Val::Px(14.0 + safe.bottom),
                        },
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
                                Text::new(i18n.t("match3.pause.title")),
                                display_font(fjord_art, FONT_HEADING),
                                TextColor(COLOR_ACCENT_GOLD),
                            ));
                            hdr.spawn((
                                Button,
                                Match3UiAction::ToggleHelp,
                                Node {
                                    min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                    min_width: Val::Px(MIN_TOUCH_TARGET_PX),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
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
                                    if session.time_left_secs.is_some() {
                                        body.spawn((
                                            Text::new(i18n.t("match3.hud.paused_banner")),
                                            TextFont {
                                                font_size: FONT_BODY,
                                                ..default()
                                            },
                                            TextColor(COLOR_ACCENT_TEAL_TEXT),
                                        ));
                                    }
                                    let is_daily = session.level.building_id
                                        == wyrdstone_core::level::DAILY_TRIAL_BUILDING_ID;
                                    let is_exp = session.level.building_id
                                        == wyrdstone_core::expedition::EXPEDITION_BUILDING_ID;
                                    let is_gauntlet = session.level.building_id
                                        == wyrdstone_core::GAUNTLET_BUILDING_ID;
                                    let has_energy =
                                        ctx.city.energy >= ctx.data.balance.energy_cost_per_level;
                                    let can_restart = !is_daily
                                        && !is_exp
                                        && !is_gauntlet
                                        && (session.is_trial || has_energy);
                                    body.spawn(Node {
                                        flex_wrap: FlexWrap::Wrap,
                                        column_gap: Val::Px(8.0),
                                        row_gap: Val::Px(6.0),
                                        ..default()
                                    })
                                    .with_children(
                                        |act_row| {
                                            act_row
                                                .spawn((
                                                    Button,
                                                    Match3UiAction::ToggleHelp,
                                                    Node {
                                                        flex_grow: 1.0,
                                                        padding: UiRect::axes(
                                                            Val::Px(14.0),
                                                            Val::Px(8.0),
                                                        ),
                                                        min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                                                        justify_content: JustifyContent::Center,
                                                        align_items: AlignItems::Center,
                                                        border: UiRect::all(Val::Px(1.5)),
                                                        ..default()
                                                    },
                                                    BackgroundColor(COLOR_ACCENT_GREEN),
                                                    BorderColor(COLOR_ACCENT_GOLD),
                                                    BorderRadius::all(Val::Px(RADIUS_SM)),
                                                ))
                                                .with_children(|btn| {
                                                    btn.spawn((
                                                        Text::new(i18n.t("match3.pause.resume")),
                                                        TextFont {
                                                            font_size: FONT_SMALL,
                                                            ..default()
                                                        },
                                                        TextColor(COLOR_TEXT_ON_ACCENT),
                                                    ));
                                                });
                                            if can_restart {
                                                let r_lbl = if confirm_restart {
                                                    i18n.t("match3.hud.restart_confirm").to_string()
                                                } else if session.is_trial {
                                                    i18n.t("match3.hud.restart").to_string()
                                                } else {
                                                    i18n.t_fmt(
                                                        "match3.hud.restart_cost",
                                                        &[(
                                                            "cost",
                                                            ctx.data
                                                                .balance
                                                                .energy_cost_per_level
                                                                .to_string(),
                                                        )],
                                                    )
                                                };
                                                act_row
                                                    .spawn((
                                                        Button,
                                                        Match3UiAction::RestartBoard,
                                                        Node {
                                                            flex_grow: 1.0,
                                                            padding: UiRect::axes(
                                                                Val::Px(12.0),
                                                                Val::Px(8.0),
                                                            ),
                                                            min_height: Val::Px(
                                                                MIN_TOUCH_TARGET_PX,
                                                            ),
                                                            justify_content: JustifyContent::Center,
                                                            align_items: AlignItems::Center,
                                                            border: UiRect::all(Val::Px(1.5)),
                                                            ..default()
                                                        },
                                                        BackgroundColor(if confirm_restart {
                                                            COLOR_ACCENT_CRIMSON
                                                        } else {
                                                            COLOR_PANEL_CARD
                                                        }),
                                                        BorderColor(COLOR_ACCENT_GOLD),
                                                        BorderRadius::all(Val::Px(RADIUS_SM)),
                                                    ))
                                                    .with_children(|btn| {
                                                        btn.spawn((
                                                            Text::new(r_lbl),
                                                            TextFont {
                                                                font_size: FONT_SMALL,
                                                                ..default()
                                                            },
                                                            TextColor(COLOR_TEXT_PRIMARY),
                                                        ));
                                                    });
                                            }
                                        },
                                    );

                                    spawn_active_buffs_card(body, i18n, session);

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
                                            crate::audio::bgm_volume_percent(sound_bank.bgm_volume);
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
                                        #[cfg_attr(target_os = "android", allow(unused_mut))]
                                        let mut quick_btns: Vec<(
                                            String,
                                            Match3UiAction,
                                            Color,
                                        )> = vec![
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
                                        ];
                                        #[cfg(not(target_os = "android"))]
                                        if crate::platform::is_fullscreen_supported() {
                                            let fs_lbl = if is_fullscreen {
                                                i18n.t("app.display.windowed")
                                            } else {
                                                i18n.t("app.display.fullscreen")
                                            };
                                            quick_btns.push((
                                                fs_lbl.to_string(),
                                                Match3UiAction::ToggleFullscreen,
                                                if is_fullscreen {
                                                    COLOR_ACCENT_TEAL
                                                } else {
                                                    COLOR_PANEL_CARD
                                                },
                                            ));
                                        }
                                        for (lbl, act, bg_col) in quick_btns {
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

                                    crate::city_ui::modals::spawn_help_topic_cards(
                                        body, i18n, fjord_art, vfx,
                                    );
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

#[allow(clippy::too_many_arguments)]
fn spawn_portrait_top_bar(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    session: &LevelSession,
    fjord_art: &FjordTextures,
    textures: &RuneTextures,
    vfx: &VfxTextures,
    hud_mode: HudLayoutMode,
) {
    let i18n = &ctx.data.i18n;
    root.spawn((
        Match3HudBar,
        Match3TopHudBar,
        Interaction::None,
        FocusPolicy::Block,
        sliced_panel_image(vfx),
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
            ..default()
        },
        panel_shadow(),
    ))
    .with_children(|top| {
        top.spawn(Node {
            width: Val::Percent(100.0),
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            row_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|stats| {
            spawn_moves_score_overdrive(stats, ctx, session, fjord_art, vfx, false);
        });

        top.spawn(Node {
            width: Val::Percent(100.0),
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            row_gap: Val::Px(3.0),
            ..default()
        })
        .with_children(|objs| {
            spawn_objectives_or_boss(objs, ctx, session, fjord_art, textures, vfx, hud_mode);
            spawn_status_banner(objs, i18n, session);
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn spawn_portrait_bottom_dock(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    session: &LevelSession,
    fjord_art: &FjordTextures,
    textures: &RuneTextures,
    vfx: &VfxTextures,
    hud_mode: HudLayoutMode,
    active_drawer: super::Match3Drawer,
    confirm_loki_shuffle: bool,
    in_valhalla: bool,
) {
    let i18n = &ctx.data.i18n;
    let total_tools: u32 = [
        ActiveTool::Mjolnir,
        ActiveTool::SurtrEmber,
        ActiveTool::ThorBolt,
        ActiveTool::BifrostPrism,
        ActiveTool::DrakkarCharge,
        ActiveTool::LokiShuffle,
    ]
    .iter()
    .map(|tool| ctx.city.tools.count(*tool))
    .sum();
    let tool_armed = ctx.armed_tool.is_some() || confirm_loki_shuffle;
    let tools_open = active_drawer == super::Match3Drawer::Tools;
    let actions_open = active_drawer == super::Match3Drawer::Actions;

    root.spawn((
        Match3HudBar,
        Match3BottomHudBar,
        Interaction::None,
        FocusPolicy::Block,
        sliced_panel_image(vfx),
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: Val::Px(4.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(5.0)),
            ..default()
        },
        panel_shadow(),
    ))
    .with_children(|bottom| {
        spawn_tooltip_section(bottom, ctx, confirm_loki_shuffle, in_valhalla);
        if active_drawer == super::Match3Drawer::None && !tool_armed && !in_valhalla {
            if let Some(ftue_key) = ftue_coach_key(ctx.city.stats.total_victories, session) {
                bottom.spawn((
                    Text::new(i18n.t(ftue_key)),
                    TextFont {
                        font_size: FONT_SMALL - 1.5,
                        ..default()
                    },
                    TextColor(COLOR_ACCENT_GOLD),
                    TextLayout::new_with_justify(JustifyText::Center),
                ));
            }
        }
        bottom
            .spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|row| {
                if session.is_trial {
                    row.spawn((
                        Button,
                        Match3UiAction::RestartBoard,
                        Node {
                            flex_grow: 1.0,
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(COLOR_ACCENT_TEAL),
                        BorderRadius::all(Val::Px(8.0)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(i18n.t("match3.hud.restart")),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_ON_ACCENT),
                        ));
                    });
                } else {
                    row.spawn(Node {
                        flex_grow: 1.0,
                        flex_shrink: 1.0,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        ..default()
                    })
                    .with_children(|fury_slot| {
                        spawn_fury_card(
                            fury_slot,
                            i18n,
                            session,
                            fjord_art,
                            textures,
                            hud_mode,
                            ctx.colorblind_mode,
                        );
                    });

                    let tools_bg = if tool_armed {
                        COLOR_ACCENT_GOLD
                    } else if tools_open {
                        COLOR_ACCENT_TEAL
                    } else {
                        COLOR_PANEL_CARD
                    };
                    let tools_fg = if tool_armed {
                        COLOR_TEXT_DARK
                    } else if tools_open {
                        COLOR_TEXT_ON_ACCENT
                    } else {
                        COLOR_TEXT_PRIMARY
                    };
                    row.spawn((
                        Button,
                        Match3UiAction::ToggleDrawer(super::Match3Drawer::Tools),
                        Node {
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                            min_width: Val::Px(MIN_TOUCH_TARGET_PX),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BackgroundColor(tools_bg),
                        BorderColor(if tool_armed || tools_open {
                            COLOR_ACCENT_GOLD
                        } else {
                            COLOR_PANEL_BORDER
                        }),
                        BorderRadius::all(Val::Px(8.0)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(i18n.t_fmt(
                                "match3.hud.tools_drawer",
                                &[("count", total_tools.to_string())],
                            )),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(tools_fg),
                        ));
                    });
                }

                row.spawn((
                    Button,
                    Match3UiAction::ToggleDrawer(super::Match3Drawer::Actions),
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                        min_width: Val::Px(MIN_TOUCH_TARGET_PX),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    BackgroundColor(if actions_open {
                        COLOR_ACCENT_TEAL
                    } else {
                        COLOR_PANEL_CARD
                    }),
                    BorderColor(if actions_open {
                        COLOR_ACCENT_GOLD
                    } else {
                        COLOR_PANEL_BORDER
                    }),
                    BorderRadius::all(Val::Px(8.0)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(i18n.t("match3.hud.menu_drawer")),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(if actions_open {
                            COLOR_TEXT_ON_ACCENT
                        } else {
                            COLOR_TEXT_PRIMARY
                        }),
                    ));
                });
            });
    });
}

#[allow(clippy::too_many_arguments)]
fn spawn_portrait_drawer_overlay(
    root: &mut ChildBuilder,
    ctx: &GameContext,
    session: &LevelSession,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    sound_bank: &SoundBank,
    safe: SafeAreaInsets,
    active_drawer: super::Match3Drawer,
    can_undo: bool,
    undo_used: bool,
    confirm_retreat: bool,
    confirm_loki_shuffle: bool,
    is_fullscreen: bool,
    in_valhalla: bool,
) {
    let i18n = &ctx.data.i18n;
    #[cfg(target_os = "android")]
    let _ = is_fullscreen;

    // Full-screen translucent scrim tapping outside dismisses the open drawer cleanly.
    root.spawn((
        Button,
        NoButtonScale,
        Match3UiAction::CloseDrawer,
        FocusPolicy::Block,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.64)),
        ZIndex(70),
    ));

    let title_key = match active_drawer {
        super::Match3Drawer::Tools => "match3.hud.drawer_tools_title",
        super::Match3Drawer::Actions | super::Match3Drawer::None => {
            "match3.hud.drawer_actions_title"
        }
    };
    root.spawn((
        FocusPolicy::Block,
        Interaction::default(),
        sliced_panel_image(vfx),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0 + safe.left),
            right: Val::Px(8.0 + safe.right),
            bottom: Val::Px(74.0 + safe.bottom),
            max_height: Val::Vh(72.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(12.0)),
            row_gap: Val::Px(8.0),
            overflow: Overflow::clip(),
            ..default()
        },
        modal_shadow(),
        ZIndex(75),
    ))
    .with_children(|drawer| {
        drawer
            .spawn((
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    padding: UiRect::bottom(Val::Px(6.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BorderColor(Color::srgba(0.93, 0.74, 0.29, 0.35)),
            ))
            .with_children(|hdr| {
                hdr.spawn((
                    Text::new(i18n.t(title_key)),
                    display_font(fjord_art, FONT_BODY),
                    TextColor(COLOR_ACCENT_GOLD),
                ));
                hdr.spawn((
                    Button,
                    Match3UiAction::CloseDrawer,
                    Node {
                        min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                        min_width: Val::Px(MIN_TOUCH_TARGET_PX),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
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

        match active_drawer {
            super::Match3Drawer::Tools => {
                spawn_tooltip_section(drawer, ctx, confirm_loki_shuffle, in_valhalla);
                let tools = [
                    (ActiveTool::Mjolnir, "match3.tool.mjolnir"),
                    (ActiveTool::SurtrEmber, "match3.tool.surtr_ember"),
                    (ActiveTool::ThorBolt, "match3.tool.thor_bolt"),
                    (ActiveTool::BifrostPrism, "match3.tool.bifrost_prism"),
                    (ActiveTool::DrakkarCharge, "match3.tool.drakkar_charge"),
                    (ActiveTool::LokiShuffle, "match3.tool.loki_shuffle"),
                ];
                drawer
                    .spawn(Node {
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(8.0),
                        row_gap: Val::Px(8.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Stretch,
                        ..default()
                    })
                    .with_children(|grid| {
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
                            let fg = if armed {
                                COLOR_TEXT_DARK
                            } else if count == 0 {
                                COLOR_TEXT_MUTED
                            } else {
                                COLOR_TEXT_PRIMARY
                            };
                            let lbl = if tool == ActiveTool::LokiShuffle && confirm_loki_shuffle {
                                i18n.t("match3.tool.confirm_loki").to_string()
                            } else {
                                i18n.t_fmt(key, &[("count", count.to_string())])
                            };
                            let mut btn_cmd = grid.spawn((
                                Button,
                                Match3UiAction::SelectTool(tool),
                                Node {
                                    flex_grow: 1.0,
                                    min_width: Val::Px(136.0),
                                    min_height: Val::Px(48.0),
                                    padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                                    column_gap: Val::Px(8.0),
                                    justify_content: JustifyContent::FlexStart,
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
                            ));
                            if count == 0 {
                                btn_cmd.insert(NoButtonScale);
                            }
                            btn_cmd.with_children(|btn| {
                                if let Some(icon) = fjord_art.tool_icon(tool) {
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
                                            width: Val::Px(28.0),
                                            height: Val::Px(28.0),
                                            flex_shrink: 0.0,
                                            ..default()
                                        },
                                        BorderRadius::all(Val::Px(6.0)),
                                    ));
                                }
                                btn.spawn((
                                    Text::new(lbl),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(fg),
                                ));
                            });
                        }
                    });
            }
            super::Match3Drawer::Actions | super::Match3Drawer::None => {
                let mut action_btns: Vec<(String, Match3UiAction, Color, Color, bool)> = Vec::new();
                if !in_valhalla {
                    action_btns.push((
                        i18n.t("match3.hud.hint").to_string(),
                        Match3UiAction::RequestHint,
                        COLOR_PANEL_CARD,
                        COLOR_ACCENT_TEAL_TEXT,
                        false,
                    ));
                }
                let undo_lbl = if undo_used {
                    i18n.t("match3.hud.undo_used")
                } else {
                    i18n.t("match3.hud.undo_short")
                };
                action_btns.push((
                    undo_lbl.to_string(),
                    Match3UiAction::Undo,
                    if can_undo {
                        COLOR_ACCENT_TEAL
                    } else {
                        COLOR_BUTTON_DISABLED
                    },
                    if can_undo {
                        COLOR_TEXT_ON_ACCENT
                    } else {
                        COLOR_TEXT_MUTED
                    },
                    !can_undo,
                ));
                action_btns.push((
                    i18n.t("match3.hud.pause_short").to_string(),
                    Match3UiAction::ToggleHelp,
                    COLOR_PANEL_CARD,
                    COLOR_ACCENT_GOLD,
                    false,
                ));
                action_btns.push((
                    if sound_bank.muted {
                        i18n.t("app.audio.off").to_string()
                    } else {
                        i18n.t("app.audio.on").to_string()
                    },
                    Match3UiAction::ToggleAudio,
                    if sound_bank.muted {
                        COLOR_BUTTON_DISABLED
                    } else {
                        COLOR_PANEL_CARD
                    },
                    COLOR_TEXT_PRIMARY,
                    false,
                ));
                #[cfg(not(target_os = "android"))]
                if crate::platform::is_fullscreen_supported() {
                    action_btns.push((
                        if is_fullscreen {
                            i18n.t("app.display.windowed").to_string()
                        } else {
                            i18n.t("app.display.fullscreen").to_string()
                        },
                        Match3UiAction::ToggleFullscreen,
                        if is_fullscreen {
                            COLOR_ACCENT_TEAL
                        } else {
                            COLOR_PANEL_CARD
                        },
                        if is_fullscreen {
                            COLOR_TEXT_ON_ACCENT
                        } else {
                            COLOR_TEXT_PRIMARY
                        },
                        false,
                    ));
                }
                action_btns.push((
                    if confirm_retreat {
                        i18n.t("match3.hud.retreat_confirm").to_string()
                    } else {
                        i18n.t("match3.hud.back").to_string()
                    },
                    Match3UiAction::Retreat,
                    if confirm_retreat {
                        Color::srgb(0.92, 0.20, 0.22)
                    } else {
                        COLOR_ACCENT_CRIMSON
                    },
                    COLOR_TEXT_PRIMARY,
                    false,
                ));

                drawer
                    .spawn(Node {
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(8.0),
                        row_gap: Val::Px(8.0),
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    })
                    .with_children(|grid| {
                        for (lbl, act, bg, fg, disabled) in action_btns {
                            let mut btn_cmd = grid.spawn((
                                Button,
                                act,
                                Node {
                                    flex_grow: 1.0,
                                    min_width: Val::Px(134.0),
                                    min_height: Val::Px(46.0),
                                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.5)),
                                    ..default()
                                },
                                BackgroundColor(bg),
                                BorderColor(COLOR_PANEL_BORDER),
                                BorderRadius::all(Val::Px(8.0)),
                            ));
                            if disabled {
                                btn_cmd.insert(NoButtonScale);
                            }
                            btn_cmd.with_children(|btn| {
                                btn.spawn((
                                    Text::new(lbl),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(fg),
                                ));
                            });
                        }
                    });
                spawn_active_buffs_card(drawer, i18n, session);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape() -> HudShape {
        HudShape {
            low_moves: false,
            projected_stars: 3,
            has_boss: false,
            boss_phase2: false,
            boss_hit_flash: false,
            boss_riposte_imminent: false,
            boss_exposed_turns: 0,
            boss_weakness: None,
            banner: HudBanner::None,
            fury_ready: false,
            overdrive_active: false,
            overdrive_turns_left: 0,
            is_blitz: false,
            is_trial: false,
            blitz_multiplier: 1,
            blitz_frozen: false,
            blitz_medal: BlitzMedal::None,
            objectives_complete: vec![false, false],
            ftue_coach_key: None,
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
    fn boss_hud_updates_in_place_until_phase_flash_or_weakness_changes() {
        let mut boss = shape();
        boss.has_boss = true;
        boss.boss_weakness = Some(RuneType::Fire);
        assert!(can_update_in_place(Some(&boss), &boss));

        let mut phase2 = boss.clone();
        phase2.boss_phase2 = true;
        assert!(!can_update_in_place(Some(&boss), &phase2));

        let mut flashed = boss.clone();
        flashed.boss_hit_flash = true;
        assert!(!can_update_in_place(Some(&boss), &flashed));

        let mut rotated_weakness = boss.clone();
        rotated_weakness.boss_weakness = Some(RuneType::Ice);
        assert!(!can_update_in_place(Some(&boss), &rotated_weakness));

        let mut imminent = boss.clone();
        imminent.boss_riposte_imminent = true;
        assert!(!can_update_in_place(Some(&boss), &imminent));

        let mut fewer_stars = shape();
        fewer_stars.projected_stars = 2;
        assert!(!can_update_in_place(Some(&shape()), &fewer_stars));
    }

    #[test]
    fn fury_charge_pct_clamps_zero_partial_full_and_zero_max() {
        assert!((fury_charge_pct_values(0, 100) - 0.0).abs() < f32::EPSILON);
        assert!((fury_charge_pct_values(45, 100) - 45.0).abs() < f32::EPSILON);
        assert!((fury_charge_pct_values(150, 100) - 100.0).abs() < f32::EPSILON);
        assert!((fury_charge_pct_values(10, 0) - 100.0).abs() < f32::EPSILON);
    }
}
