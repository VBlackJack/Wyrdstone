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
use crate::scroll::{PressArm, UiScrollMemory};
use crate::state::{today_day_index, AppState, GameContext};
use crate::theme::*;
use crate::ui_style::{
    button_shadow, display_font, gold_glow_shadow, modal_shadow, sliced_panel_image, NoButtonScale,
};
use crate::vfx_art::VfxTextures;
use bevy::prelude::*;
use wyrdstone_core::{LevelOutcome, Objective, MAX_DAILY_STREAK_CYCLE};

#[derive(Component)]
pub struct ResultRoot;

/// Sequential pop-in animation for the 1..=3 Victory mastery stars.
#[derive(Component)]
pub struct ResultStarAnim {
    pub delay: f32,
    pub elapsed: f32,
    pub earned: bool,
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

#[derive(Component)]
pub enum ResultAction {
    ReturnToFjord,
    NextLevel(String),
    Replay,
}

fn find_next_level_id(ctx: &GameContext) -> Option<String> {
    let cur_id = ctx.active_level_id.as_deref()?;
    if cur_id.starts_with("daily_")
        || cur_id.starts_with("boss_")
        || cur_id.starts_with("expedition_")
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
) {
    let i18n = &ctx.data.i18n;
    let is_victory = ctx.last_outcome == Some(LevelOutcome::Victory);
    // Victory SFX already plays at the start of VictoryCelebration; only play Defeat SFX here.
    if !is_victory {
        play_sfx(&mut commands, &sound_bank, SfxKind::Defeat);
    }

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
    } else {
        i18n.t("result.defeat.subtitle")
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
                    sliced_panel_image(&vfx),
                    Node {
                        width: Val::Px(480.0),
                        max_width: Val::Percent(94.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(32.0)),
                        row_gap: Val::Px(14.0),
                        ..default()
                    },
                    card_shadow,
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new(title),
                        display_font(&fjord_art, FONT_TITLE_MD + 10.0),
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
                        .with_children(|stars_row| {
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
                                    },
                                    ImageNode::new(vfx.icon_star.clone()).with_color(tint),
                                    Node {
                                        width: Val::Px(46.0),
                                        height: Val::Px(46.0),
                                        ..default()
                                    },
                                    Transform::from_scale(if earned {
                                        Vec3::splat(0.01)
                                    } else {
                                        Vec3::splat(0.88)
                                    }),
                                ));
                            }
                        });
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
                                TextColor(COLOR_ACCENT_GREEN),
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
                        .with_children(|rew_row| {
                            let items = [
                                (vfx.icon_wood.clone(), r.wood, Color::srgb(0.44, 0.80, 0.52)),
                                (
                                    vfx.icon_stone.clone(),
                                    r.stone,
                                    Color::srgb(0.76, 0.74, 0.88),
                                ),
                                (vfx.icon_gold.clone(), r.gold, COLOR_ACCENT_GOLD),
                                (vfx.icon_mead.clone(), r.mead, Color::srgb(0.95, 0.66, 0.30)),
                            ];
                            for (icon, amount, col) in items {
                                if amount == 0 {
                                    continue;
                                }
                                rew_row
                                    .spawn((
                                        Node {
                                            padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
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
                                        let initial_amt =
                                            if ctx.reduced_motion { amount } else { 0 };
                                        let mut amt_cmd = pill.spawn((
                                            Text::new(i18n.t_fmt(
                                                REWARD_TEMPLATE_KEY,
                                                &[(REWARD_TEMPLATE_ARG, initial_amt.to_string())],
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
                                                template_key: REWARD_TEMPLATE_KEY,
                                                arg_name: REWARD_TEMPLATE_ARG,
                                                elapsed: 0.0,
                                                duration: 0.72,
                                                done: false,
                                            });
                                        }
                                    });
                            }
                        });
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
                                bonus_tags.push(
                                    i18n.t_fmt(
                                        "result.bonus.fury",
                                        &[("pct", fury_pct.to_string())],
                                    ),
                                );
                            }
                            if !bonus_tags.is_empty() {
                                card.spawn((
                                    Text::new(bonus_tags.join(i18n.t("ui.separator.pipe"))),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_TEAL),
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
                                TextColor(COLOR_ACCENT_TEAL),
                            ));
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
                        let upgrade_ready = ctx.data.buildings.iter().any(|b| {
                            ctx.city.is_building_unlocked(b)
                                && ctx
                                    .city
                                    .next_upgrade_cost(b)
                                    .map(|c| ctx.city.resources.can_afford(c))
                                    .unwrap_or(false)
                        });
                        let mut ready_badges: Vec<(String, Color)> = Vec::new();
                        if quest_ready {
                            ready_badges.push((
                                i18n.t("result.ready.quest").to_string(),
                                COLOR_ACCENT_GOLD,
                            ));
                        }
                        if claimable_feats > 0 {
                            ready_badges.push((
                                i18n.t_fmt(
                                    "result.ready.feats",
                                    &[("count", claimable_feats.to_string())],
                                ),
                                COLOR_ACCENT_GREEN,
                            ));
                        }
                        if upgrade_ready {
                            ready_badges.push((
                                i18n.t("result.ready.upgrade").to_string(),
                                COLOR_ACCENT_TEAL,
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
                                for (lbl, col) in ready_badges {
                                    brow.spawn((
                                        Node {
                                            padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
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
                        .with_children(|prog_box| {
                            prog_box.spawn((
                                Text::new(i18n.t("result.defeat.progress")),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT_MUTED),
                            ));
                            if let Some(boss) = &session.boss {
                                let pct = ((boss.hp as f32 / boss.def.max_hp.max(1) as f32) * 100.0)
                                    .round() as u32;
                                prog_box.spawn((
                                    Text::new(i18n.t_fmt(
                                        "result.defeat.boss_hp",
                                        &[
                                            ("boss", i18n.t(&boss.def.name_key).to_string()),
                                            ("hp", boss.hp.to_string()),
                                            ("max", boss.def.max_hp.to_string()),
                                            ("pct", pct.to_string()),
                                        ],
                                    )),
                                    TextFont {
                                        font_size: FONT_BODY,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_CRIMSON_TEXT),
                                ));
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
                                    prog_box.spawn((
                                        Text::new(label),
                                        TextFont {
                                            font_size: FONT_SMALL,
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
                    }

                    let energy_cost = ctx.data.balance.energy_cost_per_level;
                    let has_energy = ctx.city.energy >= energy_cost;
                    let next_level_id = if is_victory {
                        find_next_level_id(&ctx)
                    } else {
                        None
                    };

                    card.spawn(Node {
                        column_gap: Val::Px(12.0),
                        row_gap: Val::Px(8.0),
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        margin: UiRect::top(Val::Px(10.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        if let Some(next_id) = next_level_id {
                            let next_lbl = i18n.t_fmt(
                                "result.button.next_level",
                                &[("cost", energy_cost.to_string())],
                            );
                            spawn_result_btn(
                                row,
                                &next_lbl,
                                has_energy.then_some(ResultAction::NextLevel(next_id)),
                                if has_energy {
                                    COLOR_ACCENT_GREEN
                                } else {
                                    COLOR_BUTTON_DISABLED
                                },
                                if has_energy {
                                    COLOR_TEXT_ON_ACCENT
                                } else {
                                    COLOR_TEXT_MUTED
                                },
                            );
                        }
                        spawn_result_btn(
                            row,
                            i18n.t("result.button.fjord"),
                            Some(ResultAction::ReturnToFjord),
                            COLOR_ACCENT_GOLD,
                            COLOR_TEXT_DARK,
                        );
                        if can_replay {
                            let replay_free = is_daily;
                            let replay_ok = replay_free || has_energy;
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
                            );
                        }
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
) {
    let mut cmd = parent.spawn((
        Button,
        Node {
            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
            padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(bg),
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
    time: Res<Time>,
    ctx: Res<GameContext>,
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
    } else if level_id.starts_with("boss_") {
        ctx.start_boss_raid(&level_id)
    } else if level_id.starts_with("expedition_") {
        ctx.start_expedition()
    } else {
        ctx.start_level(&level_id)
    }
}

/// Handles Return to Fjord, Next Board, Play Again buttons, and keyboard shortcuts (`Enter`/`Escape`).
pub fn handle_result_buttons(
    keys: Res<ButtonInput<KeyCode>>,
    mut ctx: ResMut<GameContext>,
    mut next_state: ResMut<NextState<AppState>>,
    touches: Res<Touches>,
    scroll_mem: Res<UiScrollMemory>,
    mut press_arm: Local<PressArm>,
    buttons: Query<(Entity, &Interaction, &ResultAction), Changed<Interaction>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::FjordCity);
        return;
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        if ctx.last_outcome == Some(LevelOutcome::Victory) {
            if let Some(next_id) = find_next_level_id(&ctx) {
                if ctx.start_level(&next_id).is_ok() {
                    next_state.set(AppState::Match3);
                    return;
                }
            }
        } else if try_replay_active_level(&mut ctx).is_ok() {
            next_state.set(AppState::Match3);
            return;
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
        match action {
            ResultAction::ReturnToFjord => {
                next_state.set(AppState::FjordCity);
                return;
            }
            ResultAction::NextLevel(next_id) => {
                match ctx.start_level(next_id) {
                    Ok(()) => {
                        next_state.set(AppState::Match3);
                        return;
                    }
                    Err(err_key) => {
                        ctx.status_notice_key = Some(err_key.to_string());
                    }
                }
                next_state.set(AppState::FjordCity);
                return;
            }
            ResultAction::Replay => {
                match try_replay_active_level(&mut ctx) {
                    Ok(()) => {
                        next_state.set(AppState::Match3);
                        return;
                    }
                    Err(err_key) => {
                        ctx.status_notice_key = Some(err_key.to_string());
                    }
                }
                next_state.set(AppState::FjordCity);
                return;
            }
        }
    }
}
