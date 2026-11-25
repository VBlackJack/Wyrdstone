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

//! Victory and defeat modal screen after completing a Match-3 level.

use crate::audio::{play_sfx, SfxKind, SoundBank};
use crate::fjord_art::FjordTextures;
use crate::scroll::{
    spawn_scrollbar_track, PressArm, ScrollPanelKind, ScrollablePanel, UiScrollMemory,
};
use crate::state::{today_day_index, ActiveModal, AppState, GameContext};
use crate::theme::*;
use crate::ui_style::{
    button_shadow, display_font, gold_glow_shadow, modal_shadow, sliced_panel_image, NoButtonScale,
};
use crate::vfx_art::VfxTextures;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use wyrdstone_core::{
    last_stand_cost, weekly_medal_name_key, LevelOutcome, Objective, LAST_STAND_EXTRA_MOVES,
    LAST_STAND_EXTRA_SECS, MAX_DAILY_STREAK_CYCLE, SKALD_PASS_TIERS, SKALD_PASS_XP_PER_TIER,
};

#[derive(Component)]
pub struct ResultRoot;

/// Sequential pop-in animation for the 1..=3 Victory mastery stars.
#[derive(Component)]
pub struct ResultStarAnim {
    pub delay: f32,
    pub elapsed: f32,
    pub earned: bool,
    pub star_idx: u8,
    pub sfx_played: bool,
}

/// Locale template and placeholder for the animated score line.
const SCORE_TEMPLATE_KEY: &str = "result.score.line";
const SCORE_TEMPLATE_ARG: &str = "score";
/// Locale template and placeholder for the animated "+N" resource reward pills.
const REWARD_TEMPLATE_KEY: &str = "match3.popup.score";
const REWARD_TEMPLATE_ARG: &str = "n";

/// Smooth ease-out numeric roll-up animation for Victory score and resource reward pills.
#[derive(Component)]
pub struct RollUpNumber {
    pub target: u32,
    /// Locale key of the template that wraps the animated number.
    pub template_key: &'static str,
    /// Placeholder name of the number inside the template.
    pub arg_name: &'static str,
    pub elapsed: f32,
    pub duration: f32,
    pub done: bool,
}

#[derive(Component, Clone)]
pub enum ResultAction {
    ReturnToFjord,
    NextLevel(String),
    Replay,
    UseLastStand,
    OpenFjordModal(ActiveModal),
    OpenUpgradeBuilding(String),
    DrinkMead,
}

/// Computes `(extra_moves_needed, next_star_tier)` toward the next star tier, or `None` for 3 stars
/// or levels without a move budget.
pub(crate) fn star_threshold_hint_args(
    stars: u8,
    moves_left: u32,
    initial_moves: u32,
    two_star_ratio: f32,
    three_star_ratio: f32,
) -> Option<(u32, u8)> {
    if stars >= 3 || initial_moves == 0 {
        return None;
    }
    let next_star = if stars < 2 { 2 } else { 3 };
    let target_ratio = if next_star == 2 {
        two_star_ratio
    } else {
        three_star_ratio
    };
    let mut req_moves = (initial_moves as f32 * target_ratio).ceil() as u32;
    while req_moves <= initial_moves && (req_moves as f32 / initial_moves as f32) < target_ratio {
        req_moves += 1;
    }
    let need = req_moves.saturating_sub(moves_left).max(1);
    Some((need, next_star))
}

fn find_next_level_id(ctx: &GameContext) -> Option<String> {
    let cur_id = ctx.active_level_id.as_deref()?;
    if cur_id.starts_with("trial_") {
        let cur_idx = ctx.data.trials.iter().position(|t| t.id == cur_id)?;
        return ctx
            .data
            .trials
            .iter()
            .skip(cur_idx + 1)
            .map(|t| t.id.clone())
            .next();
    }
    if cur_id.starts_with("daily_")
        || cur_id.starts_with("weekly_")
        || cur_id.starts_with("boss_")
        || cur_id.starts_with("expedition_")
        || cur_id.starts_with("gauntlet_")
        || cur_id == wyrdstone_core::BLITZ_BUILDING_ID
    {
        return None;
    }
    let cur_level = ctx.data.level_by_id(cur_id)?;
    let bldg_idx = ctx
        .data
        .buildings
        .iter()
        .position(|b| b.id == cur_level.building_id)?;
    let cur_bldg = &ctx.data.buildings[bldg_idx];
    let stars = ctx.city.building_mastery_stars(&cur_bldg.id);

    // 1. Prefer the next unlocked building's unbeaten standard level.
    for b in ctx.data.buildings.iter().skip(bldg_idx + 1) {
        if ctx.city.is_building_unlocked(b) && ctx.city.level_stars(&b.level_id) == 0 {
            return Some(b.level_id.clone());
        }
    }
    // 2. Otherwise step up difficulty on the current building if unlocked.
    if cur_id == cur_bldg.level_id && stars >= 1 {
        if let Some(m_id) = &cur_bldg.mastery_level_id {
            return Some(m_id.clone());
        }
    }
    if cur_bldg.mastery_level_id.as_deref() == Some(cur_id) && stars >= 3 {
        if let Some(l_id) = &cur_bldg.legendary_level_id {
            return Some(l_id.clone());
        }
    }
    // 3. Fallback to any next unlocked building's standard level.
    for b in ctx.data.buildings.iter().skip(bldg_idx + 1) {
        if ctx.city.is_building_unlocked(b) {
            return Some(b.level_id.clone());
        }
    }
    None
}

/// Spawns the Victory / Defeat summary card.
pub fn setup_result_ui(
    mut commands: Commands,
    ctx: Res<GameContext>,
    fjord_art: Res<FjordTextures>,
    vfx: Res<VfxTextures>,
    sound_bank: Res<SoundBank>,
    mut scroll_mem: ResMut<UiScrollMemory>,
) {
    scroll_mem.modal_offset_y = 0.0;
    let is_victory = ctx.last_outcome == Some(LevelOutcome::Victory);
    // Victory SFX already plays at the start of VictoryCelebration; only play Defeat SFX here.
    if !is_victory {
        play_sfx(&mut commands, &sound_bank, SfxKind::Defeat);
    }
    spawn_result_ui(&mut commands, &ctx, &fjord_art, &vfx);
}

fn spawn_result_ui(
    commands: &mut Commands,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
) {
    let i18n = &ctx.data.i18n;
    let is_victory = ctx.last_outcome == Some(LevelOutcome::Victory);

    let is_daily = ctx
        .active_level_id
        .as_deref()
        .map(|id| id.starts_with("daily_"))
        .unwrap_or(false);
    let can_replay = !is_daily || (!is_victory && ctx.city.is_daily_available(today_day_index()));

    let title = if is_victory {
        i18n.t("result.victory.title")
    } else {
        i18n.t("result.defeat.title")
    };
    let subtitle = if is_victory {
        i18n.t("result.victory.subtitle")
    } else if ctx.last_defeat_out_of_time {
        i18n.t("result.defeat.out_of_time")
    } else {
        i18n.t("result.defeat.out_of_moves")
    };
    let accent = if is_victory {
        COLOR_ACCENT_GOLD
    } else {
        COLOR_ACCENT_CRIMSON_TEXT
    };

    commands
        .spawn((
            ResultRoot,
            ImageNode::new(fjord_art.fjord_map.clone()),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|bg| {
            bg.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.03, 0.05, 0.09, 0.76)),
            ))
            .with_children(|root| {
                let card_shadow = if is_victory {
                    gold_glow_shadow()
                } else {
                    modal_shadow()
                };
                root.spawn((
                    sliced_panel_image(vfx),
                    Node {
                        width: Val::Px(500.0),
                        max_width: Val::Percent(94.0),
                        max_height: Val::Percent(92.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(24.0)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    card_shadow,
                ))
                .with_children(|shell| {
                    shell
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            min_height: Val::Px(0.0),
                            width: Val::Percent(100.0),
                            column_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|scroll_row| {
                            scroll_row
                                .spawn((
                                    ScrollablePanel {
                                        kind: ScrollPanelKind::Modal,
                                    },
                                    RelativeCursorPosition::default(),
                                    ScrollPosition {
                                        offset_y: 0.0,
                                        ..default()
                                    },
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        align_items: AlignItems::Center,
                                        flex_grow: 1.0,
                                        flex_shrink: 1.0,
                                        min_height: Val::Px(0.0),
                                        overflow: Overflow::scroll_y(),
                                        row_gap: Val::Px(12.0),
                                        padding: UiRect::right(Val::Px(4.0)),
                                        ..default()
                                    },
                                ))
                                .with_children(|card| {
                                    card.spawn((
                                        Text::new(title),
                                        display_font(fjord_art, FONT_TITLE_MD + 10.0),
                                        TextColor(accent),
                                    ));

                                    card.spawn((
                                        Text::new(subtitle),
                                        TextFont {
                                            font_size: FONT_BODY,
                                            ..default()
                                        },
                                        TextColor(COLOR_TEXT_PRIMARY),
                                    ));

                                    let initial_score = if is_victory && !ctx.reduced_motion {
                                        0
                                    } else {
                                        ctx.last_score
                                    };
                                    let mut score_cmd = card.spawn((
                                        Text::new(i18n.t_fmt(
                                            SCORE_TEMPLATE_KEY,
                                            &[(SCORE_TEMPLATE_ARG, initial_score.to_string())],
                                        )),
                                        TextFont {
                                            font_size: FONT_HEADING,
                                            ..default()
                                        },
                                        TextColor(COLOR_TEXT_PRIMARY),
                                    ));
                                    if is_victory && !ctx.reduced_motion && ctx.last_score > 0 {
                                        score_cmd.insert(RollUpNumber {
                                            target: ctx.last_score,
                                            template_key: SCORE_TEMPLATE_KEY,
                                            arg_name: SCORE_TEMPLATE_ARG,
                                            elapsed: 0.0,
                                            duration: 0.85,
                                            done: false,
                                        });
                                    }

                                    if is_victory {
                                        // Animated 3-star mastery showcase.
                                        card.spawn(Node {
                                            column_gap: Val::Px(14.0),
                                            align_items: AlignItems::Center,
                                            margin: UiRect::vertical(Val::Px(4.0)),
                                            ..default()
                                        })
                                        .with_children(
                                            |stars_row| {
                                                for idx in 0..3u8 {
                                                    let earned = idx < ctx.last_stars;
                                                    let tint = if earned {
                                                        Color::WHITE
                                                    } else {
                                                        Color::srgba(0.28, 0.34, 0.44, 0.55)
                                                    };
                                                    stars_row.spawn((
                                                        ResultStarAnim {
                                                            delay: 0.16 + idx as f32 * 0.24,
                                                            elapsed: 0.0,
                                                            earned,
                                                            star_idx: idx,
                                                            sfx_played: ctx.reduced_motion,
                                                        },
                                                        ImageNode::new(vfx.icon_star.clone())
                                                            .with_color(tint),
                                                        Node {
                                                            width: Val::Px(46.0),
                                                            height: Val::Px(46.0),
                                                            ..default()
                                                        },
                                                        Transform::from_scale(if !earned {
                                                            Vec3::splat(0.88)
                                                        } else if ctx.reduced_motion {
                                                            Vec3::ONE
                                                        } else {
                                                            Vec3::splat(0.01)
                                                        }),
                                                    ));
                                                }
                                            },
                                        );
                                        if let Some(medal) = ctx.last_blitz_medal {
                                            card.spawn((
                                                Text::new(i18n.t_fmt(
                                                    "result.blitz.medal",
                                                    &[(
                                                        "medal",
                                                        i18n.t(medal.name_key()).to_string(),
                                                    )],
                                                )),
                                                TextFont {
                                                    font_size: FONT_BODY,
                                                    ..default()
                                                },
                                                TextColor(COLOR_ACCENT_GOLD),
                                            ));
                                            if ctx.last_blitz_new_best {
                                                card.spawn((
                                                    Text::new(i18n.t("result.blitz.new_best")),
                                                    TextFont {
                                                        font_size: FONT_SMALL,
                                                        ..default()
                                                    },
                                                    TextColor(COLOR_ACCENT_GREEN_TEXT),
                                                ));
                                            }
                                        } else if let Some(medal) = ctx.last_weekly_medal {
                                            card.spawn((
                                                Text::new(
                                                    i18n.t_fmt(
                                                        "result.weekly.medal",
                                                        &[(
                                                            "medal",
                                                            i18n.t(weekly_medal_name_key(medal))
                                                                .to_string(),
                                                        )],
                                                    ),
                                                ),
                                                TextFont {
                                                    font_size: FONT_BODY,
                                                    ..default()
                                                },
                                                TextColor(COLOR_ACCENT_GOLD),
                                            ));
                                            if ctx.last_weekly_new_best {
                                                card.spawn((
                                                    Text::new(i18n.t("result.weekly.new_best")),
                                                    TextFont {
                                                        font_size: FONT_SMALL,
                                                        ..default()
                                                    },
                                                    TextColor(COLOR_ACCENT_GREEN_TEXT),
                                                ));
                                            }
                                        } else if let Some(hit_par) = ctx.last_trial_par {
                                            let badge_key = if hit_par {
                                                "result.trial.par_achieved"
                                            } else {
                                                "result.trial.par_missed"
                                            };
                                            card.spawn((
                                                Text::new(i18n.t(badge_key)),
                                                TextFont {
                                                    font_size: FONT_BODY,
                                                    ..default()
                                                },
                                                TextColor(if hit_par {
                                                    COLOR_ACCENT_GOLD
                                                } else {
                                                    COLOR_ACCENT_TEAL_TEXT
                                                }),
                                            ));
                                        } else if let Some((reached, total)) =
                                            ctx.last_gauntlet_stages
                                        {
                                            card.spawn((
                                                Text::new(i18n.t_fmt(
                                                    "result.gauntlet.cleared",
                                                    &[
                                                        ("cur", reached.to_string()),
                                                        ("total", total.to_string()),
                                                    ],
                                                )),
                                                TextFont {
                                                    font_size: FONT_BODY,
                                                    ..default()
                                                },
                                                TextColor(COLOR_ACCENT_GOLD),
                                            ));
                                        } else if let Some(session) = &ctx.active_session {
                                            if ctx.last_stars >= 3 {
                                                card.spawn((
                                                    Text::new(i18n.t("result.stars.perfect")),
                                                    TextFont {
                                                        font_size: FONT_SMALL,
                                                        ..default()
                                                    },
                                                    TextColor(COLOR_ACCENT_GOLD),
                                                ));
                                            } else if let Some((need, next)) =
                                                star_threshold_hint_args(
                                                    ctx.last_stars,
                                                    session.moves_left,
                                                    session.initial_moves,
                                                    ctx.data.balance.session.two_star_moves_ratio,
                                                    ctx.data.balance.session.three_star_moves_ratio,
                                                )
                                            {
                                                card.spawn((
                                                    Text::new(i18n.t_fmt(
                                                        "result.stars.next_hint",
                                                        &[
                                                            (
                                                                "moves",
                                                                session.moves_left.to_string(),
                                                            ),
                                                            ("need", need.to_string()),
                                                            ("next", next.to_string()),
                                                        ],
                                                    )),
                                                    TextFont {
                                                        font_size: FONT_SMALL,
                                                        ..default()
                                                    },
                                                    TextColor(COLOR_TEXT_MUTED),
                                                ));
                                            }
                                        }
                                        if ctx.city.stats.win_streak >= 1 {
                                            let flame = ctx.city.valhalla_flame_tier();
                                            card.spawn((
                                                Text::new(i18n.t_fmt(
                                                    "result.flame.active",
                                                    &[
                                                        (
                                                            "tier",
                                                            i18n.t(flame.name_key()).to_string(),
                                                        ),
                                                        (
                                                            "streak",
                                                            ctx.city.stats.win_streak.to_string(),
                                                        ),
                                                    ],
                                                )),
                                                TextFont {
                                                    font_size: FONT_SMALL,
                                                    ..default()
                                                },
                                                TextColor(COLOR_ACCENT_GOLD),
                                            ));
                                        }
                                        if let Some(streak) = ctx.last_daily_streak {
                                            card.spawn((
                                                Text::new(i18n.t_fmt(
                                                    "result.daily.streak",
                                                    &[
                                                        ("days", streak.to_string()),
                                                        ("max", MAX_DAILY_STREAK_CYCLE.to_string()),
                                                    ],
                                                )),
                                                TextFont {
                                                    font_size: FONT_BODY,
                                                    ..default()
                                                },
                                                TextColor(COLOR_ACCENT_GREEN_TEXT),
                                            ));
                                        }
                                        let r = ctx.last_rewards;
                                        card.spawn((
                                            Text::new(i18n.t("result.rewards.title")),
                                            TextFont {
                                                font_size: FONT_BODY,
                                                ..default()
                                            },
                                            TextColor(COLOR_ACCENT_GOLD),
                                        ));
                                        // Visual resource reward pills with icons.
                                        card.spawn(Node {
                                            flex_wrap: FlexWrap::Wrap,
                                            justify_content: JustifyContent::Center,
                                            column_gap: Val::Px(10.0),
                                            row_gap: Val::Px(6.0),
                                            ..default()
                                        })
                                        .with_children(
                                            |rew_row| {
                                                let items = [
                                                    (
                                                        vfx.icon_wood.clone(),
                                                        r.wood,
                                                        Color::srgb(0.44, 0.80, 0.52),
                                                    ),
                                                    (
                                                        vfx.icon_stone.clone(),
                                                        r.stone,
                                                        Color::srgb(0.76, 0.74, 0.88),
                                                    ),
                                                    (
                                                        vfx.icon_gold.clone(),
                                                        r.gold,
                                                        COLOR_ACCENT_GOLD,
                                                    ),
                                                    (
                                                        vfx.icon_mead.clone(),
                                                        r.mead,
                                                        Color::srgb(0.95, 0.66, 0.30),
                                                    ),
                                                ];
                                                for (icon, amount, col) in items {
                                                    if amount == 0 {
                                                        continue;
                                                    }
                                                    rew_row
                                                        .spawn((
                                                            Node {
                                                                padding: UiRect::axes(
                                                                    Val::Px(10.0),
                                                                    Val::Px(5.0),
                                                                ),
                                                                column_gap: Val::Px(5.0),
                                                                align_items: AlignItems::Center,
                                                                border: UiRect::all(Val::Px(1.0)),
                                                                ..default()
                                                            },
                                                            BackgroundColor(COLOR_PANEL_CARD),
                                                            BorderColor(COLOR_PANEL_BORDER),
                                                            BorderRadius::all(Val::Px(8.0)),
                                                        ))
                                                        .with_children(|pill| {
                                                            pill.spawn((
                                                                ImageNode::new(icon),
                                                                Node {
                                                                    width: Val::Px(18.0),
                                                                    height: Val::Px(18.0),
                                                                    ..default()
                                                                },
                                                            ));
                                                            let initial_amt = if ctx.reduced_motion
                                                            {
                                                                amount
                                                            } else {
                                                                0
                                                            };
                                                            let mut amt_cmd = pill.spawn((
                                                                Text::new(i18n.t_fmt(
                                                                    REWARD_TEMPLATE_KEY,
                                                                    &[(
                                                                        REWARD_TEMPLATE_ARG,
                                                                        initial_amt.to_string(),
                                                                    )],
                                                                )),
                                                                TextFont {
                                                                    font_size: FONT_BODY,
                                                                    ..default()
                                                                },
                                                                TextColor(col),
                                                            ));
                                                            if !ctx.reduced_motion {
                                                                amt_cmd.insert(RollUpNumber {
                                                                    target: amount,
                                                                    template_key:
                                                                        REWARD_TEMPLATE_KEY,
                                                                    arg_name: REWARD_TEMPLATE_ARG,
                                                                    elapsed: 0.0,
                                                                    duration: 0.72,
                                                                    done: false,
                                                                });
                                                            }
                                                        });
                                                }
                                                if ctx.last_glory_reward > 0 {
                                                    rew_row
                                                        .spawn((
                                                            Node {
                                                                padding: UiRect::axes(
                                                                    Val::Px(10.0),
                                                                    Val::Px(5.0),
                                                                ),
                                                                column_gap: Val::Px(5.0),
                                                                align_items: AlignItems::Center,
                                                                border: UiRect::all(Val::Px(1.0)),
                                                                ..default()
                                                            },
                                                            BackgroundColor(COLOR_PANEL_CARD),
                                                            BorderColor(COLOR_ACCENT_GOLD),
                                                            BorderRadius::all(Val::Px(8.0)),
                                                        ))
                                                        .with_children(|pill| {
                                                            pill.spawn((
                                                                ImageNode::new(
                                                                    vfx.icon_star.clone(),
                                                                ),
                                                                Node {
                                                                    width: Val::Px(18.0),
                                                                    height: Val::Px(18.0),
                                                                    ..default()
                                                                },
                                                            ));
                                                            pill.spawn((
                                                                Text::new(
                                                                    i18n.t_fmt(
                                                                        "city.panel.reward.glory",
                                                                        &[(
                                                                            "glory",
                                                                            ctx.last_glory_reward
                                                                                .to_string(),
                                                                        )],
                                                                    ),
                                                                ),
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
                                        // Show breakdown of active reward multipliers (moves, clan/amulet/blessing, Divine Fury)
                                        if let Some(session) = &ctx.active_session {
                                            let (moves_pct, village_pct, fury_pct) =
                                                session.reward_bonus_breakdown(&ctx.data.balance);
                                            let mut bonus_tags = Vec::new();
                                            if moves_pct > 0 {
                                                bonus_tags.push(i18n.t_fmt(
                                                    "result.bonus.moves",
                                                    &[("pct", moves_pct.to_string())],
                                                ));
                                            }
                                            if village_pct > 0 {
                                                bonus_tags.push(i18n.t_fmt(
                                                    "result.bonus.village",
                                                    &[("pct", village_pct.to_string())],
                                                ));
                                            }
                                            if fury_pct > 0 {
                                                bonus_tags.push(i18n.t_fmt(
                                                    "result.bonus.fury",
                                                    &[("pct", fury_pct.to_string())],
                                                ));
                                            }
                                            if !bonus_tags.is_empty() {
                                                card.spawn((
                                                    Text::new(
                                                        bonus_tags
                                                            .join(i18n.t("ui.separator.pipe")),
                                                    ),
                                                    TextFont {
                                                        font_size: FONT_SMALL,
                                                        ..default()
                                                    },
                                                    TextColor(COLOR_ACCENT_TEAL_TEXT),
                                                ));
                                            }
                                        }
                                        if let Some(relic_key) = &ctx.last_discovered_relic_key {
                                            card.spawn((
                                                Text::new(i18n.t_fmt(
                                                    "result.relic.discovered",
                                                    &[("relic", i18n.t(relic_key).to_string())],
                                                )),
                                                TextFont {
                                                    font_size: FONT_BODY,
                                                    ..default()
                                                },
                                                TextColor(COLOR_ACCENT_TEAL_TEXT),
                                            ));
                                        }
                                        if ctx.last_skald_xp_gained > 0 {
                                            let pass = &ctx.city.skald_pass;
                                            let tier = pass.current_tier();
                                            let fill_pct = if tier >= SKALD_PASS_TIERS {
                                                100.0
                                            } else {
                                                ((pass.xp % SKALD_PASS_XP_PER_TIER) as f32
                                                    / SKALD_PASS_XP_PER_TIER as f32
                                                    * 100.0)
                                                    .clamp(0.0, 100.0)
                                            };
                                            let has_claim = pass.claimable_count() > 0;
                                            card.spawn((
                                                Node {
                                                    width: Val::Percent(100.0),
                                                    flex_direction: FlexDirection::Column,
                                                    row_gap: Val::Px(5.0),
                                                    padding: UiRect::axes(
                                                        Val::Px(12.0),
                                                        Val::Px(8.0),
                                                    ),
                                                    border: UiRect::all(Val::Px(1.5)),
                                                    ..default()
                                                },
                                                BackgroundColor(COLOR_PANEL_CARD),
                                                BorderColor(if has_claim {
                                                    COLOR_ACCENT_GOLD
                                                } else {
                                                    COLOR_PANEL_BORDER
                                                }),
                                                BorderRadius::all(Val::Px(8.0)),
                                            ))
                                            .with_children(|sp| {
                                                sp.spawn((
                                                    Text::new(
                                                        i18n.t_fmt(
                                                            "result.skald.progress",
                                                            &[
                                                                ("tier", tier.to_string()),
                                                                (
                                                                    "max",
                                                                    SKALD_PASS_TIERS.to_string(),
                                                                ),
                                                                (
                                                                    "xp",
                                                                    ctx.last_skald_xp_gained
                                                                        .to_string(),
                                                                ),
                                                            ],
                                                        ),
                                                    ),
                                                    TextFont {
                                                        font_size: FONT_SMALL,
                                                        ..default()
                                                    },
                                                    TextColor(if has_claim {
                                                        COLOR_ACCENT_GOLD
                                                    } else {
                                                        COLOR_TEXT_PRIMARY
                                                    }),
                                                ));
                                                sp.spawn((
                                                    Node {
                                                        width: Val::Percent(100.0),
                                                        height: Val::Px(8.0),
                                                        overflow: Overflow::clip(),
                                                        ..default()
                                                    },
                                                    BackgroundColor(Color::srgba(
                                                        0.08, 0.11, 0.18, 0.90,
                                                    )),
                                                    BorderRadius::all(Val::Px(4.0)),
                                                ))
                                                .with_children(|bar| {
                                                    bar.spawn((
                                                        Node {
                                                            width: Val::Percent(fill_pct),
                                                            height: Val::Percent(100.0),
                                                            ..default()
                                                        },
                                                        BackgroundColor(COLOR_ACCENT_GOLD),
                                                    ));
                                                });
                                            });
                                        }
                                        // Highlight any newly claimable Fjord progression (Quest, Feats, Building Upgrade).
                                        let quest_ready = ctx
                                            .city
                                            .active_quest(&ctx.data.quests)
                                            .map(|q| ctx.city.is_quest_ready(q))
                                            .unwrap_or(false);
                                        let claimable_feats = ctx.city.claimable_achievement_count(
                                            &ctx.data.achievements,
                                            &ctx.data.buildings,
                                            ctx.data.bosses.len(),
                                        );
                                        let first_upgradeable_id = ctx
                                            .data
                                            .buildings
                                            .iter()
                                            .find(|b| {
                                                ctx.city.is_building_unlocked(b)
                                                    && ctx
                                                        .city
                                                        .next_upgrade_cost(b)
                                                        .map(|c| ctx.city.resources.can_afford(c))
                                                        .unwrap_or(false)
                                            })
                                            .map(|b| b.id.clone());
                                        let mut ready_badges: Vec<(String, Color, ResultAction)> =
                                            Vec::new();
                                        if quest_ready {
                                            ready_badges.push((
                                                i18n.t("result.ready.quest").to_string(),
                                                COLOR_ACCENT_GOLD,
                                                ResultAction::ReturnToFjord,
                                            ));
                                        }
                                        if claimable_feats > 0 {
                                            ready_badges.push((
                                                i18n.t_fmt(
                                                    "result.ready.feats",
                                                    &[("count", claimable_feats.to_string())],
                                                ),
                                                COLOR_ACCENT_GREEN,
                                                ResultAction::OpenFjordModal(
                                                    ActiveModal::Achievements,
                                                ),
                                            ));
                                        }
                                        if let Some(bldg_id) = first_upgradeable_id {
                                            ready_badges.push((
                                                i18n.t("result.ready.upgrade").to_string(),
                                                COLOR_ACCENT_TEAL,
                                                ResultAction::OpenUpgradeBuilding(bldg_id),
                                            ));
                                        }
                                        let claimable_bounties =
                                            ctx.city.claimable_bounties_and_pass_count();
                                        if claimable_bounties > 0 {
                                            ready_badges.push((
                                                i18n.t_fmt(
                                                    "result.ready.bounties",
                                                    &[("count", claimable_bounties.to_string())],
                                                ),
                                                COLOR_ACCENT_GOLD,
                                                ResultAction::OpenFjordModal(ActiveModal::Bounties),
                                            ));
                                        }
                                        if !ready_badges.is_empty() {
                                            card.spawn(Node {
                                                flex_wrap: FlexWrap::Wrap,
                                                justify_content: JustifyContent::Center,
                                                column_gap: Val::Px(8.0),
                                                row_gap: Val::Px(6.0),
                                                margin: UiRect::top(Val::Px(2.0)),
                                                ..default()
                                            })
                                            .with_children(|brow| {
                                                for (lbl, col, act) in ready_badges {
                                                    brow.spawn((
                                                        Button,
                                                        act,
                                                        Node {
                                                            min_height: Val::Px(32.0),
                                                            padding: UiRect::axes(
                                                                Val::Px(12.0),
                                                                Val::Px(5.0),
                                                            ),
                                                            justify_content: JustifyContent::Center,
                                                            align_items: AlignItems::Center,
                                                            border: UiRect::all(Val::Px(1.5)),
                                                            ..default()
                                                        },
                                                        BackgroundColor(COLOR_PANEL_CARD),
                                                        BorderColor(col),
                                                        BorderRadius::all(Val::Px(8.0)),
                                                    ))
                                                    .with_children(|pill| {
                                                        pill.spawn((
                                                            Text::new(lbl),
                                                            TextFont {
                                                                font_size: FONT_SMALL,
                                                                ..default()
                                                            },
                                                            TextColor(col),
                                                        ));
                                                    });
                                                }
                                            });
                                        }
                                    } else if let Some(session) = &ctx.active_session {
                                        // Defeat progress summary so the player sees how close they were.
                                        card.spawn((
                                            Node {
                                                flex_direction: FlexDirection::Column,
                                                align_items: AlignItems::Center,
                                                padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                                                row_gap: Val::Px(4.0),
                                                border: UiRect::all(Val::Px(1.0)),
                                                ..default()
                                            },
                                            BackgroundColor(COLOR_PANEL_CARD),
                                            BorderColor(COLOR_PANEL_BORDER),
                                            BorderRadius::all(Val::Px(8.0)),
                                        ))
                                        .with_children(
                                            |prog_box| {
                                                prog_box.spawn((
                                                    Text::new(i18n.t("result.defeat.progress")),
                                                    TextFont {
                                                        font_size: FONT_SMALL,
                                                        ..default()
                                                    },
                                                    TextColor(COLOR_TEXT_MUTED),
                                                ));
                                                if let Some(boss) = &session.boss {
                                                    let pct = ((boss.hp as f32
                                                        / boss.def.max_hp.max(1) as f32)
                                                        * 100.0)
                                                        .round()
                                                        as u32;
                                                    prog_box.spawn((
                                                        Text::new(
                                                            i18n.t_fmt(
                                                                "result.defeat.boss_hp",
                                                                &[
                                                                    (
                                                                        "boss",
                                                                        i18n.t(&boss.def.name_key)
                                                                            .to_string(),
                                                                    ),
                                                                    ("hp", boss.hp.to_string()),
                                                                    (
                                                                        "max",
                                                                        boss.def.max_hp.to_string(),
                                                                    ),
                                                                    ("pct", pct.to_string()),
                                                                ],
                                                            ),
                                                        ),
                                                        TextFont {
                                                            font_size: FONT_BODY,
                                                            ..default()
                                                        },
                                                        TextColor(COLOR_ACCENT_CRIMSON_TEXT),
                                                    ));
                                                } else {
                                                    for prog in &session.objectives {
                                                        let label = match &prog.objective {
                                                            Objective::ClearIce { .. } => i18n
                                                                .t_fmt(
                                                                    "match3.objective.clear_ice",
                                                                    &[
                                                                        (
                                                                            "done",
                                                                            prog.done.to_string(),
                                                                        ),
                                                                        (
                                                                            "target",
                                                                            prog.target.to_string(),
                                                                        ),
                                                                    ],
                                                                ),
                                                            Objective::CollectRune {
                                                                rune, ..
                                                            } => i18n.t_fmt(
                                                                "match3.objective.collect_rune",
                                                                &[
                                                                    (
                                                                        "rune",
                                                                        i18n.t(rune.i18n_key())
                                                                            .to_string(),
                                                                    ),
                                                                    ("done", prog.done.to_string()),
                                                                    (
                                                                        "target",
                                                                        prog.target.to_string(),
                                                                    ),
                                                                ],
                                                            ),
                                                            Objective::ReachScore { .. } => i18n
                                                                .t_fmt(
                                                                    "match3.objective.reach_score",
                                                                    &[
                                                                        (
                                                                            "done",
                                                                            prog.done.to_string(),
                                                                        ),
                                                                        (
                                                                            "target",
                                                                            prog.target.to_string(),
                                                                        ),
                                                                    ],
                                                                ),
                                                            Objective::DropRelics { .. } => i18n
                                                                .t_fmt(
                                                                    "match3.objective.drop_relics",
                                                                    &[
                                                                        (
                                                                            "done",
                                                                            prog.done.to_string(),
                                                                        ),
                                                                        (
                                                                            "target",
                                                                            prog.target.to_string(),
                                                                        ),
                                                                    ],
                                                                ),
                                                        };
                                                        prog_box.spawn((
                                                            Text::new(label),
                                                            TextFont {
                                                                font_size: FONT_SMALL,
                                                                ..default()
                                                            },
                                                            TextColor(if prog.is_complete() {
                                                                COLOR_ACCENT_GREEN_TEXT
                                                            } else {
                                                                COLOR_TEXT_PRIMARY
                                                            }),
                                                        ));
                                                        let pct = (prog.done as f32
                                                            / prog.target.max(1) as f32
                                                            * 100.0)
                                                            .clamp(0.0, 100.0);
                                                        prog_box
                                                            .spawn((
                                                                Node {
                                                                    width: Val::Px(220.0),
                                                                    height: Val::Px(6.0),
                                                                    margin: UiRect::bottom(
                                                                        Val::Px(3.0),
                                                                    ),
                                                                    overflow: Overflow::clip(),
                                                                    ..default()
                                                                },
                                                                BackgroundColor(Color::srgba(
                                                                    0.08, 0.11, 0.18, 0.90,
                                                                )),
                                                                BorderRadius::all(Val::Px(3.0)),
                                                            ))
                                                            .with_children(|bar| {
                                                                bar.spawn((
                                                                    Node {
                                                                        width: Val::Percent(pct),
                                                                        height: Val::Percent(100.0),
                                                                        ..default()
                                                                    },
                                                                    BackgroundColor(
                                                                        if prog.is_complete() {
                                                                            COLOR_ACCENT_GREEN
                                                                        } else {
                                                                            COLOR_ACCENT_TEAL
                                                                        },
                                                                    ),
                                                                    BorderRadius::all(Val::Px(3.0)),
                                                                ));
                                                            });
                                                    }
                                                }
                                            },
                                        );
                                        if session.can_use_last_stand() {
                                            let ls_cost = last_stand_cost();
                                            let can_afford_ls =
                                                ctx.city.resources.can_afford(&ls_cost);
                                            let ls_desc = if ctx.last_defeat_out_of_time {
                                                i18n.t_fmt(
                                                    "result.last_stand.desc_time",
                                                    &[(
                                                        "secs",
                                                        (LAST_STAND_EXTRA_SECS as u32).to_string(),
                                                    )],
                                                )
                                            } else {
                                                i18n.t_fmt(
                                                    "result.last_stand.desc_moves",
                                                    &[(
                                                        "moves",
                                                        LAST_STAND_EXTRA_MOVES.to_string(),
                                                    )],
                                                )
                                            };
                                            card.spawn((
                                                Node {
                                                    flex_direction: FlexDirection::Column,
                                                    align_items: AlignItems::Center,
                                                    row_gap: Val::Px(6.0),
                                                    padding: UiRect::axes(
                                                        Val::Px(14.0),
                                                        Val::Px(10.0),
                                                    ),
                                                    border: UiRect::all(Val::Px(1.5)),
                                                    ..default()
                                                },
                                                BackgroundColor(Color::srgba(
                                                    0.18, 0.14, 0.08, 0.94,
                                                )),
                                                BorderColor(COLOR_ACCENT_GOLD),
                                                BorderRadius::all(Val::Px(10.0)),
                                            ))
                                            .with_children(|ls_box| {
                                                ls_box.spawn((
                                                    Text::new(i18n.t("result.last_stand.title")),
                                                    display_font(fjord_art, FONT_BODY),
                                                    TextColor(COLOR_ACCENT_GOLD),
                                                ));
                                                ls_box.spawn((
                                                    Text::new(ls_desc),
                                                    TextFont {
                                                        font_size: FONT_SMALL,
                                                        ..default()
                                                    },
                                                    TextColor(COLOR_TEXT_PRIMARY),
                                                ));
                                                if ctx.last_stand_pre_defeat_streak > 0 {
                                                    ls_box.spawn((
                                                        Text::new(i18n.t_fmt(
                                                            "result.last_stand.keep_flame",
                                                            &[(
                                                                "streak",
                                                                ctx.last_stand_pre_defeat_streak
                                                                    .to_string(),
                                                            )],
                                                        )),
                                                        TextFont {
                                                            font_size: FONT_SMALL - 1.0,
                                                            ..default()
                                                        },
                                                        TextColor(COLOR_ACCENT_GREEN_TEXT),
                                                    ));
                                                }
                                                let btn_lbl = i18n.t_fmt(
                                                    "result.last_stand.button",
                                                    &[
                                                        ("gold", ls_cost.gold.to_string()),
                                                        ("mead", ls_cost.mead.to_string()),
                                                    ],
                                                );
                                                spawn_result_btn(
                                                    ls_box,
                                                    &btn_lbl,
                                                    can_afford_ls
                                                        .then_some(ResultAction::UseLastStand),
                                                    if can_afford_ls {
                                                        COLOR_ACCENT_GOLD
                                                    } else {
                                                        COLOR_BUTTON_DISABLED
                                                    },
                                                    if can_afford_ls {
                                                        COLOR_TEXT_DARK
                                                    } else {
                                                        COLOR_TEXT_MUTED
                                                    },
                                                    can_afford_ls,
                                                );
                                            });
                                        }
                                    }

                                    let energy_cost = ctx.data.balance.energy_cost_per_level;
                                    let has_energy = ctx.city.energy >= energy_cost;
                                    let max_energy = ctx.city.max_energy(&ctx.data.balance);
                                    let next_level_id = if is_victory {
                                        find_next_level_id(ctx)
                                    } else {
                                        None
                                    };
                                    let can_drink_mead = !has_energy
                                        && ctx.city.resources.mead
                                            >= ctx.data.balance.mead_refill_cost;

                                    card.spawn(Node {
                                        column_gap: Val::Px(10.0),
                                        row_gap: Val::Px(6.0),
                                        flex_wrap: FlexWrap::Wrap,
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        margin: UiRect::top(Val::Px(4.0)),
                                        ..default()
                                    })
                                    .with_children(|erow| {
                                        erow.spawn((
                                            Text::new(i18n.t_fmt(
                                                "city.header.energy_full",
                                                &[
                                                    ("cur", ctx.city.energy.to_string()),
                                                    ("max", max_energy.to_string()),
                                                ],
                                            )),
                                            TextFont {
                                                font_size: FONT_SMALL,
                                                ..default()
                                            },
                                            TextColor(if has_energy {
                                                COLOR_ACCENT_TEAL_TEXT
                                            } else {
                                                COLOR_ACCENT_CRIMSON_TEXT
                                            }),
                                        ));
                                        if can_drink_mead {
                                            let mead_lbl = i18n.t_fmt(
                                                "city.button.drink_mead",
                                                &[
                                                    (
                                                        "energy",
                                                        ctx.data
                                                            .balance
                                                            .mead_refill_energy
                                                            .to_string(),
                                                    ),
                                                    (
                                                        "cost",
                                                        ctx.data
                                                            .balance
                                                            .mead_refill_cost
                                                            .to_string(),
                                                    ),
                                                ],
                                            );
                                            erow.spawn((
                                                Button,
                                                ResultAction::DrinkMead,
                                                Node {
                                                    min_height: Val::Px(34.0),
                                                    padding: UiRect::axes(
                                                        Val::Px(12.0),
                                                        Val::Px(5.0),
                                                    ),
                                                    justify_content: JustifyContent::Center,
                                                    align_items: AlignItems::Center,
                                                    border: UiRect::all(Val::Px(1.5)),
                                                    ..default()
                                                },
                                                BackgroundColor(Color::srgb(0.56, 0.36, 0.14)),
                                                BorderColor(COLOR_ACCENT_GOLD),
                                                BorderRadius::all(Val::Px(8.0)),
                                            ))
                                            .with_children(|btn| {
                                                btn.spawn((
                                                    Text::new(mead_lbl),
                                                    TextFont {
                                                        font_size: FONT_SMALL,
                                                        ..default()
                                                    },
                                                    TextColor(COLOR_TEXT_PRIMARY),
                                                ));
                                            });
                                        }
                                    });

                                    let is_trial = ctx
                                        .active_level_id
                                        .as_deref()
                                        .is_some_and(|id| id.starts_with("trial_"));
                                    let primary_is_next = is_victory && next_level_id.is_some();
                                    let replay_free = is_daily || is_trial;
                                    let replay_ok = can_replay && (replay_free || has_energy);
                                    let primary_is_replay = !is_victory && replay_ok;
                                    let primary_is_fjord = !primary_is_next && !primary_is_replay;

                                    card.spawn(Node {
                                        column_gap: Val::Px(12.0),
                                        row_gap: Val::Px(8.0),
                                        flex_wrap: FlexWrap::Wrap,
                                        justify_content: JustifyContent::Center,
                                        margin: UiRect::top(Val::Px(6.0)),
                                        ..default()
                                    })
                                    .with_children(|row| {
                                        if let Some(next_id) = next_level_id {
                                            let next_ok = is_trial || has_energy;
                                            let next_lbl = if is_trial {
                                                i18n.t("result.button.next_trial").to_string()
                                            } else {
                                                i18n.t_fmt(
                                                    "result.button.next_level",
                                                    &[("cost", energy_cost.to_string())],
                                                )
                                            };
                                            spawn_result_btn(
                                                row,
                                                &next_lbl,
                                                next_ok.then_some(ResultAction::NextLevel(next_id)),
                                                if next_ok {
                                                    COLOR_ACCENT_GREEN
                                                } else {
                                                    COLOR_BUTTON_DISABLED
                                                },
                                                if next_ok {
                                                    COLOR_TEXT_ON_ACCENT
                                                } else {
                                                    COLOR_TEXT_MUTED
                                                },
                                                primary_is_next && next_ok,
                                            );
                                        }
                                        spawn_result_btn(
                                            row,
                                            i18n.t("result.button.fjord"),
                                            Some(ResultAction::ReturnToFjord),
                                            COLOR_ACCENT_GOLD,
                                            COLOR_TEXT_DARK,
                                            primary_is_fjord,
                                        );
                                        if can_replay {
                                            let replay_lbl = if replay_free {
                                                i18n.t("result.button.replay").to_string()
                                            } else {
                                                i18n.t_fmt(
                                                    "result.button.replay_cost",
                                                    &[("cost", energy_cost.to_string())],
                                                )
                                            };
                                            spawn_result_btn(
                                                row,
                                                &replay_lbl,
                                                replay_ok.then_some(ResultAction::Replay),
                                                if replay_ok {
                                                    COLOR_ACCENT_TEAL
                                                } else {
                                                    COLOR_BUTTON_DISABLED
                                                },
                                                if replay_ok {
                                                    COLOR_TEXT_ON_ACCENT
                                                } else {
                                                    COLOR_TEXT_MUTED
                                                },
                                                primary_is_replay,
                                            );
                                        }
                                    });
                                });
                            spawn_scrollbar_track(scroll_row, ScrollPanelKind::Modal);
                        });
                });
            });
        });
}

fn spawn_result_btn(
    parent: &mut ChildBuilder,
    label: &str,
    action: Option<ResultAction>,
    bg: Color,
    text_color: Color,
    is_primary: bool,
) {
    let mut cmd = parent.spawn((
        Button,
        Node {
            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
            padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
            border: UiRect::all(Val::Px(2.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(bg),
        BorderColor(if is_primary {
            COLOR_ACCENT_GOLD_HOVER
        } else {
            Color::NONE
        }),
        BorderRadius::all(Val::Px(8.0)),
        button_shadow(),
    ));
    if let Some(act) = action {
        cmd.insert(act);
    } else {
        cmd.insert(NoButtonScale);
    }
    cmd.with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont {
                font_size: FONT_BODY,
                ..default()
            },
            TextColor(text_color),
        ));
    });
}

/// Animates sequential elastic pop-in of earned mastery stars and numeric roll-ups on Victory.
pub fn animate_result_stars(
    mut commands: Commands,
    time: Res<Time>,
    ctx: Res<GameContext>,
    sound_bank: Res<SoundBank>,
    mut stars: Query<(&mut ResultStarAnim, &mut Transform)>,
    mut rollups: Query<(&mut RollUpNumber, &mut Text)>,
) {
    if ctx.reduced_motion {
        for (anim, mut tf) in &mut stars {
            if anim.earned {
                tf.scale = Vec3::ONE;
            }
        }
        for (mut roll, mut text) in &mut rollups {
            if !roll.done {
                roll.done = true;
                text.0 = ctx.data.i18n.t_fmt(
                    roll.template_key,
                    &[(roll.arg_name, roll.target.to_string())],
                );
            }
        }
        return;
    }
    let dt = time.delta_secs();
    for (mut roll, mut text) in &mut rollups {
        if roll.done {
            continue;
        }
        roll.elapsed += dt;
        let t = (roll.elapsed / roll.duration).clamp(0.0, 1.0);
        // Smooth ease-out cubic count-up.
        let eased = 1.0 - (1.0 - t).powi(3);
        let cur = if t >= 1.0 {
            roll.done = true;
            roll.target
        } else {
            (roll.target as f32 * eased).round() as u32
        };
        text.0 = ctx
            .data
            .i18n
            .t_fmt(roll.template_key, &[(roll.arg_name, cur.to_string())]);
    }
    for (mut anim, mut tf) in &mut stars {
        if !anim.earned {
            continue;
        }
        anim.elapsed += dt;
        if !anim.sfx_played && anim.elapsed >= anim.delay {
            anim.sfx_played = true;
            play_sfx(&mut commands, &sound_bank, SfxKind::StarPop(anim.star_idx));
        }
        let t = ((anim.elapsed - anim.delay) / 0.36).clamp(0.0, 1.0);
        if t <= 0.0 {
            tf.scale = Vec3::splat(0.01);
        } else {
            // Elastic pop overshoot: 0 -> 1.25 -> 1.0
            let scale = if t < 0.65 {
                let u = t / 0.65;
                (1.0 - (1.0 - u) * (1.0 - u)) * 1.24
            } else {
                let u = (t - 0.65) / 0.35;
                1.24 - 0.24 * u
            };
            tf.scale = Vec3::splat(scale);
        }
    }
}

/// Cleans up result UI entities.
pub fn cleanup_result_ui(mut commands: Commands, roots: Query<Entity, With<ResultRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

fn try_replay_active_level(ctx: &mut GameContext) -> Result<(), &'static str> {
    let Some(level_id) = ctx.active_level_id.clone() else {
        return Err("city.error.level_not_found");
    };
    if level_id.starts_with("daily_") {
        if ctx.city.is_daily_available(today_day_index()) {
            ctx.start_daily_trial()
        } else {
            Err("city.panel.daily.done")
        }
    } else if level_id.starts_with("weekly_") {
        ctx.start_weekly_challenge()
    } else if level_id.starts_with("trial_") {
        ctx.start_trial(&level_id)
    } else if level_id.starts_with("gauntlet_") {
        ctx.start_gauntlet()
    } else if level_id.starts_with("boss_") {
        ctx.start_boss_raid(&level_id)
    } else if level_id.starts_with("expedition_") {
        ctx.start_expedition()
    } else if level_id == wyrdstone_core::BLITZ_BUILDING_ID {
        ctx.start_valhalla_blitz()
    } else {
        ctx.start_level(&level_id)
    }
}

fn execute_result_action(
    commands: &mut Commands,
    sound_bank: &SoundBank,
    action: &ResultAction,
    ctx: &mut GameContext,
    next_state: &mut NextState<AppState>,
) -> bool {
    match action {
        ResultAction::ReturnToFjord => {
            next_state.set(AppState::FjordCity);
            false
        }
        ResultAction::NextLevel(next_id) => {
            let res = if next_id.starts_with("trial_") {
                ctx.start_trial(next_id)
            } else {
                ctx.start_level(next_id)
            };
            match res {
                Ok(()) => {
                    next_state.set(AppState::Match3);
                }
                Err(err_key) => {
                    ctx.status_notice_key = Some(err_key.to_string());
                    next_state.set(AppState::FjordCity);
                }
            }
            false
        }
        ResultAction::Replay => {
            match try_replay_active_level(ctx) {
                Ok(()) => {
                    next_state.set(AppState::Match3);
                }
                Err(err_key) => {
                    ctx.status_notice_key = Some(err_key.to_string());
                    next_state.set(AppState::FjordCity);
                }
            }
            false
        }
        ResultAction::UseLastStand => match ctx.try_use_last_stand() {
            Ok(()) => {
                play_sfx(commands, sound_bank, SfxKind::FuryUnleash);
                next_state.set(AppState::Match3);
                false
            }
            Err(err_key) => {
                ctx.status_notice_key = Some(err_key.to_string());
                play_sfx(commands, sound_bank, SfxKind::Invalid);
                true
            }
        },
        ResultAction::OpenFjordModal(modal) => {
            ctx.active_modal = *modal;
            next_state.set(AppState::FjordCity);
            false
        }
        ResultAction::OpenUpgradeBuilding(id) => {
            ctx.selected_building_id = id.clone();
            ctx.open_inspector_on_city_enter = true;
            next_state.set(AppState::FjordCity);
            false
        }
        ResultAction::DrinkMead => {
            if ctx.city.try_drink_mead(&ctx.data.balance).is_ok() {
                ctx.persist();
                true
            } else {
                false
            }
        }
    }
}

/// Handles Return to Fjord, Next Board, Play Again buttons, and keyboard navigation (`Tab`/`Arrows`/`Enter`/`Escape`).
#[allow(clippy::too_many_arguments)]
pub fn handle_result_buttons(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut ctx: ResMut<GameContext>,
    mut next_state: ResMut<NextState<AppState>>,
    touches: Res<Touches>,
    scroll_mem: Res<UiScrollMemory>,
    (fjord_art, vfx, sound_bank): (Res<FjordTextures>, Res<VfxTextures>, Res<SoundBank>),
    (mut press_arm, mut focus_idx): (Local<PressArm>, Local<Option<usize>>),
    roots: Query<Entity, With<ResultRoot>>,
    buttons: Query<(Entity, &Interaction, &ResultAction), Changed<Interaction>>,
    mut focus_buttons: Query<(Entity, &ResultAction, &mut BorderColor)>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        *focus_idx = None;
        next_state.set(AppState::FjordCity);
        return;
    }

    let focus_step: Option<isize> = if keys.just_pressed(KeyCode::Tab) {
        if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            Some(-1)
        } else {
            Some(1)
        }
    } else if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::ArrowDown) {
        Some(1)
    } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::ArrowUp) {
        Some(-1)
    } else {
        None
    };

    if let Some(step) = focus_step {
        let count = focus_buttons.iter().count();
        if count > 0 {
            let next = match *focus_idx {
                Some(cur) => ((cur as isize + step).rem_euclid(count as isize)) as usize,
                None => {
                    if step >= 0 {
                        0
                    } else {
                        count - 1
                    }
                }
            };
            *focus_idx = Some(next);
            for (idx, (_, _, mut border)) in focus_buttons.iter_mut().enumerate() {
                border.0 = if idx == next {
                    COLOR_ACCENT_GOLD_HOVER
                } else {
                    Color::NONE
                };
            }
        }
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        if let Some(chosen) =
            focus_idx.and_then(|idx| focus_buttons.iter().nth(idx).map(|(_, act, _)| act.clone()))
        {
            *focus_idx = None;
            if execute_result_action(
                &mut commands,
                &sound_bank,
                &chosen,
                &mut ctx,
                &mut next_state,
            ) {
                play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                for root in &roots {
                    commands.entity(root).despawn_recursive();
                }
                spawn_result_ui(&mut commands, &ctx, &fjord_art, &vfx);
            }
            return;
        }
        *focus_idx = None;
        let has_energy = ctx.city.energy >= ctx.data.balance.energy_cost_per_level;
        let is_trial = ctx
            .active_level_id
            .as_deref()
            .is_some_and(|id| id.starts_with("trial_"));
        if ctx.last_outcome == Some(LevelOutcome::Victory) {
            if has_energy || is_trial {
                if let Some(next_id) = find_next_level_id(&ctx) {
                    execute_result_action(
                        &mut commands,
                        &sound_bank,
                        &ResultAction::NextLevel(next_id),
                        &mut ctx,
                        &mut next_state,
                    );
                    return;
                }
            }
        } else {
            let is_daily = ctx
                .active_level_id
                .as_deref()
                .is_some_and(|id| id.starts_with("daily_"));
            let can_replay = !is_daily || ctx.city.is_daily_available(today_day_index());
            if can_replay && (is_daily || is_trial || has_energy) {
                execute_result_action(
                    &mut commands,
                    &sound_bank,
                    &ResultAction::Replay,
                    &mut ctx,
                    &mut next_state,
                );
                return;
            }
        }
        next_state.set(AppState::FjordCity);
        return;
    }

    // Actions fire on release so a touch that starts a scroll never triggers a button.
    let tapped = press_arm.completed_taps(
        buttons.iter().map(|(entity, i, _)| (entity, *i)),
        &touches,
        &scroll_mem,
    );
    for (entity, _, action) in &buttons {
        if !tapped.contains(&entity) {
            continue;
        }
        *focus_idx = None;
        if execute_result_action(
            &mut commands,
            &sound_bank,
            action,
            &mut ctx,
            &mut next_state,
        ) {
            play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
            for root in &roots {
                commands.entity(root).despawn_recursive();
            }
            spawn_result_ui(&mut commands, &ctx, &fjord_art, &vfx);
        }
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::star_threshold_hint_args;

    #[test]
    fn star_threshold_hint_computes_exact_extra_moves_and_skips_perfect() {
        // 20 moves total, 2-star >= 0.15 (3 moves), 3-star >= 0.35 (7 moves).
        assert_eq!(star_threshold_hint_args(1, 1, 20, 0.15, 0.35), Some((2, 2)));
        assert_eq!(star_threshold_hint_args(2, 4, 20, 0.15, 0.35), Some((3, 3)));
        assert_eq!(star_threshold_hint_args(3, 10, 20, 0.15, 0.35), None);
        assert_eq!(star_threshold_hint_args(1, 0, 0, 0.15, 0.35), None);
    }
}
