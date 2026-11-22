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

//! Match-3 animation state machine, VFX beams, burst particles, and level outcome transitions.

use super::{
    cell_to_world, grid_to_world_f, rebuild_board_now, BlastBeam, BoardAnimPhase, BoardTileVisual,
    BoosterPulse, BurstParticle, FloatingScore, Match3Backdrop, Match3Cleanup, Match3Entity,
    Match3InputState, Match3TimerText, FLOATING_TEXT_LIFETIME_SECS, FLOATING_TEXT_RISE_SPEED_PX,
    HINT_IDLE_SECS, PARTICLES_PER_CLEARED_TILE, PARTICLE_LIFETIME_SECS, PARTICLE_SIZE_PX,
    PARTICLE_SPEED_PX, SHAKE_AMPLITUDE_PX, SHAKE_DURATION_SECS,
};
use crate::audio::{play_sfx, SfxKind, SoundBank};
use crate::runes_art::RuneTextures;
use crate::state::{ActiveModal, AppState, GameContext};
use crate::theme::*;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use std::collections::{HashMap, HashSet};
use wyrdstone_core::{ActiveTool, CellPos, FallMove, LevelOutcome};

type BackdropQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Sprite,
    (
        With<Match3Backdrop>,
        Without<BurstParticle>,
        Without<BlastBeam>,
    ),
>;
type ParticleQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut BurstParticle,
        &'static mut Transform,
        &'static mut Sprite,
    ),
    (Without<Camera2d>, Without<BlastBeam>),
>;
type BeamQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut BlastBeam,
        &'static mut Transform,
        &'static mut Sprite,
    ),
    (Without<Camera2d>, Without<BurstParticle>),
>;
type PopupQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut FloatingScore,
        &'static mut Transform,
        &'static mut TextColor,
    ),
    (
        Without<BurstParticle>,
        Without<Camera2d>,
        Without<BlastBeam>,
    ),
>;
type BoosterPulseQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Transform,
    (
        With<BoosterPulse>,
        Without<BurstParticle>,
        Without<BlastBeam>,
        Without<FloatingScore>,
        Without<Camera2d>,
    ),
>;

pub(super) fn spawn_clear_particles(
    commands: &mut Commands,
    cells: &[CellPos],
    session: &wyrdstone_core::LevelSession,
    layout: &Match3InputState,
) {
    for &pos in cells {
        let center = cell_to_world(pos, layout);
        let color = session
            .board
            .get(pos)
            .and_then(|c| c.rune)
            .map(|r| {
                let [rr, gg, bb] = rune_base_rgb(r);
                Color::srgb(
                    (rr as f32 / 255.0).min(1.0) * 1.15,
                    (gg as f32 / 255.0).min(1.0) * 1.15,
                    (bb as f32 / 255.0).min(1.0) * 1.15,
                )
            })
            .unwrap_or(COLOR_ACCENT_GOLD);

        for i in 0..PARTICLES_PER_CLEARED_TILE {
            let angle = (i as f32 / PARTICLES_PER_CLEARED_TILE as f32) * std::f32::consts::TAU
                + (pos.col as f32 * 0.37);
            let dir = Vec2::new(angle.cos(), angle.sin());
            commands.spawn((
                Match3Cleanup,
                BurstParticle {
                    velocity: dir * PARTICLE_SPEED_PX,
                    elapsed: 0.0,
                    lifetime: PARTICLE_LIFETIME_SECS,
                },
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(PARTICLE_SIZE_PX)),
                    ..default()
                },
                Transform::from_xyz(center.x, center.y, Z_SELECTION_RING + 1.0),
            ));
        }
    }
}

fn spawn_blast_beam(commands: &mut Commands, center: Vec2, size: Vec2, color: Color) {
    commands.spawn((
        Match3Cleanup,
        BlastBeam {
            elapsed: 0.0,
            lifetime: 0.30,
        },
        Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_xyz(center.x, center.y, Z_SELECTION_RING + 1.5),
    ));
}

pub(super) fn spawn_loki_vfx(
    commands: &mut Commands,
    cols: usize,
    rows: usize,
    layout: &Match3InputState,
) {
    let board_w = cols as f32 * layout.cell_step;
    let board_h = rows as f32 * layout.cell_step;
    let center_y = (BOARD_BOTTOM_RESERVED_PX - BOARD_TOP_RESERVED_PX) * 0.5;
    spawn_blast_beam(
        commands,
        Vec2::new(0.0, center_y),
        Vec2::new(board_w * 0.92, board_h * 0.92),
        Color::srgba(0.62, 0.38, 0.95, 0.52),
    );
}

pub(super) fn spawn_tool_vfx(
    commands: &mut Commands,
    tool: ActiveTool,
    cell: CellPos,
    affected_cells: &[CellPos],
    cols: usize,
    rows: usize,
    layout: &Match3InputState,
) {
    let center = cell_to_world(cell, layout);
    let board_w = cols as f32 * layout.cell_step;
    let board_h = rows as f32 * layout.cell_step;
    let center_y = (BOARD_BOTTOM_RESERVED_PX - BOARD_TOP_RESERVED_PX) * 0.5;
    match tool {
        ActiveTool::ThorBolt => {
            spawn_blast_beam(
                commands,
                Vec2::new(0.0, center.y),
                Vec2::new(board_w, layout.tile_size * 0.52),
                Color::srgba(0.45, 0.92, 1.0, 0.92),
            );
        }
        ActiveTool::DrakkarCharge => {
            spawn_blast_beam(
                commands,
                Vec2::new(0.0, center.y),
                Vec2::new(board_w, layout.tile_size * 0.55),
                Color::srgba(1.0, 0.85, 0.32, 0.92),
            );
            spawn_blast_beam(
                commands,
                Vec2::new(center.x, center_y),
                Vec2::new(layout.tile_size * 0.55, board_h),
                Color::srgba(1.0, 0.85, 0.32, 0.92),
            );
        }
        ActiveTool::SurtrEmber => {
            spawn_blast_beam(
                commands,
                center,
                Vec2::splat(layout.cell_step * 3.1),
                Color::srgba(1.0, 0.48, 0.16, 0.78),
            );
        }
        ActiveTool::Mjolnir => {
            spawn_blast_beam(
                commands,
                center,
                Vec2::splat(layout.cell_step * 1.4),
                Color::srgba(0.65, 0.92, 1.0, 0.90),
            );
        }
        ActiveTool::BifrostPrism => {
            let prism_colors = [
                Color::srgba(0.98, 0.42, 0.62, 0.82),
                Color::srgba(0.42, 0.92, 1.0, 0.82),
                Color::srgba(0.98, 0.84, 0.34, 0.82),
                Color::srgba(0.68, 0.46, 1.0, 0.82),
            ];
            for (idx, &pos) in affected_cells.iter().enumerate() {
                let w = cell_to_world(pos, layout);
                spawn_blast_beam(
                    commands,
                    w,
                    Vec2::splat(layout.cell_step * 1.15),
                    prism_colors[idx % prism_colors.len()],
                );
            }
        }
        ActiveTool::LokiShuffle => {
            spawn_loki_vfx(commands, cols, rows, layout);
        }
    }
}

fn spawn_floating_score(
    commands: &mut Commands,
    cells: &[CellPos],
    label: String,
    color: Color,
    layout: &Match3InputState,
) {
    if cells.is_empty() {
        return;
    }
    let mut sum = Vec2::ZERO;
    for &p in cells {
        sum += cell_to_world(p, layout);
    }
    let center = sum / cells.len() as f32;
    commands.spawn((
        Match3Cleanup,
        FloatingScore {
            elapsed: 0.0,
            lifetime: FLOATING_TEXT_LIFETIME_SECS,
        },
        Text2d::new(label),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(color),
        Transform::from_xyz(center.x, center.y + 8.0, Z_SELECTION_RING + 2.0),
    ));
}

/// Advances the stepped Match-3 animation state machine (swap -> clear -> fall -> cascade).
#[allow(clippy::too_many_arguments)]
pub fn tick_board_animation(
    mut commands: Commands,
    time: Res<Time>,
    mut ctx: ResMut<GameContext>,
    textures: Res<RuneTextures>,
    sound_bank: Res<SoundBank>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut input_state: ResMut<Match3InputState>,
    mut next_state: ResMut<NextState<AppState>>,
    all_sprites: Query<Entity, With<Match3Entity>>,
    mut tile_q: Query<(&BoardTileVisual, &mut Transform, &mut Sprite)>,
    mut timer_text_q: Query<(&mut Text, &mut TextColor), With<Match3TimerText>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let dt = time.delta_secs();

    // Tick optional level countdown timer (paused during VictoryCelebration or Help modal).
    let timer_paused = matches!(input_state.phase, BoardAnimPhase::VictoryCelebration { .. })
        || ctx.active_modal == ActiveModal::Help;
    if !timer_paused {
        let timer_changed = ctx
            .active_session
            .as_mut()
            .map(|s| s.tick_timer(dt))
            .unwrap_or(false);
        if timer_changed {
            if ctx.active_session.as_ref().map(|s| s.outcome) == Some(LevelOutcome::Defeat) {
                finalize_level_transition(&mut ctx, &mut next_state);
                return;
            }
            // Update timer text in-place instead of rebuilding the entire HUD every second.
            if let Some(secs) = ctx.active_session.as_ref().and_then(|s| s.time_left_secs) {
                let whole = secs.ceil() as u32;
                let formatted = ctx
                    .data
                    .i18n
                    .t_fmt("match3.hud.timer", &[("secs", whole.to_string())]);
                let color = if whole <= 15 {
                    COLOR_ACCENT_CRIMSON
                } else {
                    COLOR_ACCENT_TEAL
                };
                for (mut text, mut text_col) in &mut timer_text_q {
                    text.0 = formatted.clone();
                    text_col.0 = color;
                }
            }
        }
    }

    let phase = std::mem::take(&mut input_state.phase);
    match phase {
        BoardAnimPhase::Idle => {
            input_state.idle_secs += dt;
            if input_state.idle_secs >= HINT_IDLE_SECS {
                if input_state.hint_pair.is_none() {
                    input_state.hint_pair = ctx
                        .active_session
                        .as_ref()
                        .and_then(|s| s.board.find_hint_move());
                }
                if let Some((ha, hb)) = input_state.hint_pair {
                    let pulse = 1.0 + 0.11 * ((input_state.idle_secs * 5.0).sin() * 0.5 + 0.5);
                    let glow = Color::srgb(1.22, 1.18, 0.92);
                    for (tile, mut tf, mut sprite) in &mut tile_q {
                        if tile.pos == ha || tile.pos == hb {
                            tf.scale = Vec3::splat(pulse);
                            sprite.color = glow;
                        }
                    }
                }
            }
            input_state.phase = BoardAnimPhase::Idle;
        }
        BoardAnimPhase::Swapping {
            a,
            b,
            mut elapsed,
            duration,
        } => {
            elapsed += dt;
            let t = ease_out_quad((elapsed / duration).clamp(0.0, 1.0));
            let wa = cell_to_world(a, &input_state);
            let wb = cell_to_world(b, &input_state);
            for (tile, mut tf, _) in &mut tile_q {
                if tile.pos == a {
                    let p = wa.lerp(wb, t);
                    tf.translation.x = p.x;
                    tf.translation.y = p.y;
                } else if tile.pos == b {
                    let p = wb.lerp(wa, t);
                    tf.translation.x = p.x;
                    tf.translation.y = p.y;
                }
            }

            if elapsed >= duration {
                let prepared = ctx
                    .active_session
                    .as_mut()
                    .and_then(|s| s.board.begin_swap_step(a, b));
                if let Some(prep) = prepared {
                    if let Some(session) = ctx.active_session.as_mut() {
                        session.consume_move();
                        session.last_combo = 1;
                        session.last_shuffled = false;
                    }
                    if let Some(session) = ctx.active_session.as_ref() {
                        rebuild_board_now(
                            &mut commands,
                            session,
                            &textures,
                            window,
                            &mut input_state,
                            &all_sprites,
                            None,
                        );
                    }
                    let cleared_set: HashSet<CellPos> = prep.cells.iter().copied().collect();
                    input_state.dirty_hud = true;
                    input_state.phase = BoardAnimPhase::Clearing {
                        prepared: prep,
                        cleared_set,
                        combo: 1,
                        elapsed: 0.0,
                        duration: ctx.data.balance.clear_anim_seconds,
                        particles_spawned: false,
                    };
                } else {
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    input_state.phase = BoardAnimPhase::SwapRevert {
                        a,
                        b,
                        elapsed: 0.0,
                        duration,
                    };
                }
            } else {
                input_state.phase = BoardAnimPhase::Swapping {
                    a,
                    b,
                    elapsed,
                    duration,
                };
            }
        }
        BoardAnimPhase::SwapRevert {
            a,
            b,
            mut elapsed,
            duration,
        } => {
            elapsed += dt;
            let t = ease_out_quad((elapsed / duration).clamp(0.0, 1.0));
            let wa = cell_to_world(a, &input_state);
            let wb = cell_to_world(b, &input_state);
            for (tile, mut tf, _) in &mut tile_q {
                if tile.pos == a {
                    let p = wb.lerp(wa, t);
                    tf.translation.x = p.x;
                    tf.translation.y = p.y;
                } else if tile.pos == b {
                    let p = wa.lerp(wb, t);
                    tf.translation.x = p.x;
                    tf.translation.y = p.y;
                }
            }

            if elapsed >= duration {
                input_state.dirty_board = true;
                input_state.phase = BoardAnimPhase::Idle;
            } else {
                input_state.phase = BoardAnimPhase::SwapRevert {
                    a,
                    b,
                    elapsed,
                    duration,
                };
            }
        }
        BoardAnimPhase::Clearing {
            prepared,
            cleared_set,
            combo,
            mut elapsed,
            duration,
            mut particles_spawned,
        } => {
            if !particles_spawned {
                play_sfx(&mut commands, &sound_bank, SfxKind::MatchCombo(combo));
                if let Some(session) = ctx.active_session.as_ref() {
                    spawn_clear_particles(&mut commands, &prepared.cells, session, &input_state);
                }
                particles_spawned = true;
            }

            elapsed += dt;
            let raw_t = (elapsed / duration).clamp(0.0, 1.0);
            let scale = if raw_t < 0.35 {
                1.0 + (raw_t / 0.35) * 0.22
            } else {
                (1.22 * (1.0 - (raw_t - 0.35) / 0.65)).max(0.05)
            };
            let alpha = (1.0 - raw_t * 0.85).max(0.1);

            for (tile, mut tf, mut sprite) in &mut tile_q {
                if cleared_set.contains(&tile.pos) {
                    tf.scale = Vec3::splat(scale);
                    sprite.color = Color::srgba(1.35, 1.30, 1.05, alpha);
                }
            }

            if elapsed >= duration {
                let balance = ctx.data.balance.clone();
                let mut gained_score = 0u32;
                let mut cleared_total = 0u64;
                let mut fall_map = HashMap::new();
                {
                    let ctx_ref = ctx.as_mut();
                    if let Some(session) = ctx_ref.active_session.as_mut() {
                        let step = session.board.commit_clear_step(&prepared, combo, &balance);
                        gained_score = step.score_gained;
                        cleared_total = step.cleared_runes.values().map(|&v| v as u64).sum();
                        session.apply_step_progress(&step);
                        let fall_moves: Vec<FallMove> = session
                            .board
                            .apply_gravity_and_refill_tracked(&mut ctx_ref.rng);
                        for fm in fall_moves {
                            fall_map.insert(CellPos::new(fm.col, fm.to_row), fm.from_row);
                        }
                    }
                    ctx_ref.city.stats.total_runes_cleared += cleared_total;
                    ctx_ref.city.stats.best_combo = ctx_ref.city.stats.best_combo.max(combo);
                }
                if gained_score > 0 {
                    let i18n = &ctx.data.i18n;
                    let popup_label = if combo >= 5 {
                        i18n.t_fmt("match3.callout.combo5", &[("combo", combo.to_string())])
                    } else if combo == 4 {
                        i18n.t_fmt("match3.callout.combo4", &[("combo", combo.to_string())])
                    } else if combo == 3 {
                        i18n.t_fmt("match3.callout.combo3", &[("combo", combo.to_string())])
                    } else if combo == 2 {
                        i18n.t_fmt(
                            "match3.callout.combo2",
                            &[("score", gained_score.to_string())],
                        )
                    } else {
                        format!("+{gained_score}")
                    };
                    let popup_color = if combo >= 2 {
                        COLOR_ACCENT_GOLD
                    } else {
                        COLOR_TEXT_PRIMARY
                    };
                    spawn_floating_score(
                        &mut commands,
                        &prepared.cells,
                        popup_label,
                        popup_color,
                        &input_state,
                    );
                }
                if combo >= 3 || prepared.cells.len() >= 6 {
                    input_state.shake_secs = SHAKE_DURATION_SECS;
                }
                if let Some(session) = ctx.active_session.as_ref() {
                    rebuild_board_now(
                        &mut commands,
                        session,
                        &textures,
                        window,
                        &mut input_state,
                        &all_sprites,
                        Some(&fall_map),
                    );
                }
                input_state.idle_secs = 0.0;
                input_state.hint_pair = None;
                input_state.dirty_hud = true;
                input_state.phase = BoardAnimPhase::Falling {
                    moves: fall_map,
                    combo,
                    elapsed: 0.0,
                    duration: ctx.data.balance.fall_anim_seconds,
                };
            } else {
                input_state.phase = BoardAnimPhase::Clearing {
                    prepared,
                    cleared_set,
                    combo,
                    elapsed,
                    duration,
                    particles_spawned,
                };
            }
        }
        BoardAnimPhase::Falling {
            moves,
            combo,
            mut elapsed,
            duration,
        } => {
            elapsed += dt;
            let t = ease_out_quad((elapsed / duration).clamp(0.0, 1.0));
            for (tile, mut tf, _) in &mut tile_q {
                if let Some(&from_row) = moves.get(&tile.pos) {
                    let start = grid_to_world_f(tile.pos.col as f32, from_row as f32, &input_state);
                    let end = cell_to_world(tile.pos, &input_state);
                    let p = start.lerp(end, t);
                    tf.translation.x = p.x;
                    tf.translation.y = p.y;
                }
            }

            if elapsed >= duration {
                let balance = ctx.data.balance.clone();
                let mut relic_fall_map = HashMap::new();
                let mut dropped_positions = Vec::new();
                {
                    let ctx_ref = ctx.as_mut();
                    if let Some(session) = ctx_ref.active_session.as_mut() {
                        let relic_step = session.board.collect_bottom_relics(&balance);
                        if relic_step.relics_dropped > 0 {
                            dropped_positions = relic_step.cleared_positions.clone();
                            session.apply_step_progress(&relic_step);
                            let fms = session
                                .board
                                .apply_gravity_and_refill_tracked(&mut ctx_ref.rng);
                            for fm in fms {
                                relic_fall_map.insert(CellPos::new(fm.col, fm.to_row), fm.from_row);
                            }
                        }
                    }
                }
                if !dropped_positions.is_empty() {
                    play_sfx(&mut commands, &sound_bank, SfxKind::RelicDrop);
                    if let Some(session) = ctx.active_session.as_ref() {
                        spawn_clear_particles(
                            &mut commands,
                            &dropped_positions,
                            session,
                            &input_state,
                        );
                        rebuild_board_now(
                            &mut commands,
                            session,
                            &textures,
                            window,
                            &mut input_state,
                            &all_sprites,
                            Some(&relic_fall_map),
                        );
                    }
                    input_state.dirty_hud = true;
                    input_state.phase = BoardAnimPhase::Falling {
                        moves: relic_fall_map,
                        combo,
                        elapsed: 0.0,
                        duration: ctx.data.balance.fall_anim_seconds,
                    };
                    return;
                }

                let next_cascade = ctx
                    .active_session
                    .as_ref()
                    .and_then(|s| s.board.prepare_cascade_step());
                if let Some(next_prep) = next_cascade {
                    if let Some(session) = ctx.active_session.as_mut() {
                        session.last_combo = combo + 1;
                    }
                    let cleared_set: HashSet<CellPos> = next_prep.cells.iter().copied().collect();
                    input_state.dirty_hud = true;
                    input_state.phase = BoardAnimPhase::Clearing {
                        prepared: next_prep,
                        cleared_set,
                        combo: combo + 1,
                        elapsed: 0.0,
                        duration: ctx.data.balance.clear_anim_seconds,
                        particles_spawned: false,
                    };
                } else {
                    let balance = ctx.data.balance.clone();
                    let ctx_ref = ctx.as_mut();
                    let mut did_shuffle = false;
                    let mut frost_pos = None;
                    let mut boss_riposte_hits = Vec::new();
                    let mut boss_riposte_name: Option<String> = None;
                    if let Some(session) = ctx_ref.active_session.as_mut() {
                        frost_pos = session.apply_frost_spread_if_needed(&mut ctx_ref.rng);
                        boss_riposte_hits = session.tick_boss_riposte_if_needed(&mut ctx_ref.rng);
                        if !boss_riposte_hits.is_empty() {
                            boss_riposte_name = session
                                .boss
                                .as_ref()
                                .map(|b| ctx_ref.data.i18n.t(&b.def.name_key).to_string());
                        }
                        if !session.board.has_valid_moves() {
                            session.board.shuffle_free_runes(&mut ctx_ref.rng);
                            session.last_shuffled = true;
                            did_shuffle = true;
                        }
                        session.finalize_turn(&balance);
                    }
                    if !boss_riposte_hits.is_empty() {
                        play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                        for &rp in &boss_riposte_hits {
                            let w = cell_to_world(rp, &input_state);
                            spawn_blast_beam(
                                &mut commands,
                                w,
                                Vec2::splat(input_state.cell_step * 1.3),
                                Color::srgba(0.95, 0.22, 0.28, 0.88),
                            );
                        }
                        if let Some(bname) = boss_riposte_name {
                            let msg = ctx
                                .data
                                .i18n
                                .t_fmt("match3.boss.riposte", &[("boss", bname)]);
                            spawn_floating_score(
                                &mut commands,
                                &boss_riposte_hits,
                                msg,
                                COLOR_ACCENT_CRIMSON,
                                &input_state,
                            );
                        }
                        input_state.shake_secs = SHAKE_DURATION_SECS;
                        input_state.dirty_board = true;
                        input_state.dirty_hud = true;
                    }
                    if let Some(fp) = frost_pos {
                        let w = cell_to_world(fp, &input_state);
                        spawn_blast_beam(
                            &mut commands,
                            w,
                            Vec2::splat(input_state.cell_step * 1.2),
                            Color::srgba(0.45, 0.88, 1.0, 0.85),
                        );
                        input_state.dirty_board = true;
                        input_state.dirty_hud = true;
                    }
                    if did_shuffle {
                        input_state.dirty_board = true;
                        input_state.dirty_hud = true;
                    }
                    let outcome = ctx
                        .active_session
                        .as_ref()
                        .map(|s| s.outcome)
                        .unwrap_or(LevelOutcome::InProgress);
                    match outcome {
                        LevelOutcome::Victory => {
                            play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                            let banner = ctx.data.i18n.t("result.victory.title").to_string();
                            commands.spawn((
                                Match3Cleanup,
                                FloatingScore {
                                    elapsed: 0.0,
                                    lifetime: 1.65,
                                },
                                Text2d::new(banner),
                                TextFont {
                                    font_size: 46.0,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GOLD),
                                Transform::from_xyz(0.0, 10.0, Z_SELECTION_RING + 3.0),
                            ));
                            input_state.phase = BoardAnimPhase::VictoryCelebration {
                                elapsed: 0.0,
                                duration: 1.75,
                                next_burst_at: 0.14,
                                burst_index: 0,
                            };
                        }
                        LevelOutcome::Defeat => {
                            finalize_level_transition(&mut ctx, &mut next_state);
                            input_state.phase = BoardAnimPhase::Idle;
                        }
                        LevelOutcome::InProgress => {
                            input_state.phase = BoardAnimPhase::Idle;
                        }
                    }
                }
            } else {
                input_state.phase = BoardAnimPhase::Falling {
                    moves,
                    combo,
                    elapsed,
                    duration,
                };
            }
        }
        BoardAnimPhase::VictoryCelebration {
            mut elapsed,
            duration,
            mut next_burst_at,
            mut burst_index,
        } => {
            elapsed += dt;
            for (tile, mut tf, mut sprite) in &mut tile_q {
                let wave = ((elapsed * 9.0) - (tile.pos.col + tile.pos.row) as f32 * 0.45).sin();
                tf.scale = Vec3::splat(1.0 + 0.10 * wave.max(0.0));
                sprite.color =
                    Color::srgb(1.0 + 0.20 * wave.max(0.0), 1.0 + 0.16 * wave.max(0.0), 1.0);
            }

            if elapsed >= next_burst_at && elapsed < duration - 0.15 {
                next_burst_at += 0.22;
                burst_index += 1;
                if let Some(session) = ctx.active_session.as_ref() {
                    let cols = session.board.cols();
                    let rows = session.board.rows();
                    let mut burst_cells = Vec::new();
                    for k in 0..4usize {
                        let c = ((burst_index as usize * 3 + k * 5) % cols).min(cols - 1);
                        let r = ((burst_index as usize * 2 + k * 3) % rows).min(rows - 1);
                        let p = CellPos::new(c, r);
                        if session
                            .board
                            .get(p)
                            .map(|cell| !cell.blocked)
                            .unwrap_or(false)
                        {
                            burst_cells.push(p);
                        }
                    }
                    if !burst_cells.is_empty() {
                        spawn_clear_particles(&mut commands, &burst_cells, session, &input_state);
                        let bonus_pts = ctx.data.balance.remaining_move_bonus;
                        spawn_floating_score(
                            &mut commands,
                            &[burst_cells[0]],
                            format!("+{bonus_pts}"),
                            COLOR_ACCENT_GOLD,
                            &input_state,
                        );
                        play_sfx(&mut commands, &sound_bank, SfxKind::MatchCombo(burst_index));
                    }
                }
            }

            if elapsed >= duration {
                finalize_level_transition(&mut ctx, &mut next_state);
                input_state.phase = BoardAnimPhase::Idle;
            } else {
                input_state.phase = BoardAnimPhase::VictoryCelebration {
                    elapsed,
                    duration,
                    next_burst_at,
                    burst_index,
                };
            }
        }
    }
}

fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn finalize_level_transition(ctx: &mut GameContext, next_state: &mut NextState<AppState>) {
    let balance = ctx.data.balance.clone();
    let finished = ctx.active_session.as_ref().and_then(|s| match s.outcome {
        LevelOutcome::InProgress => None,
        outcome => Some((
            outcome,
            s.level.id.clone(),
            s.level.building_id.clone(),
            s.earned_rewards(&balance),
            s.score,
            s.earned_stars(),
            s.boss.as_ref().map(|b| b.def.clone()),
        )),
    });

    if let Some((outcome, level_id, building_id, rewards, score, stars, boss_def)) = finished {
        ctx.last_outcome = Some(outcome);
        ctx.last_rewards = rewards;
        ctx.last_score = score;
        ctx.last_stars = stars;
        ctx.last_daily_streak = None;
        ctx.city.stats.best_score = ctx.city.stats.best_score.max(score);
        if outcome == LevelOutcome::Victory {
            if let Some(boss) = boss_def {
                ctx.city.record_boss_victory(&boss);
                ctx.last_rewards = boss.reward_resources;
            } else if building_id == "expedition" {
                let wave = ctx.active_expedition.as_ref().map(|r| r.wave).unwrap_or(1);
                ctx.city.record_expedition_wave(wave, &rewards, &balance);
                let ctx_ref = ctx;
                if let Some(run) = ctx_ref.active_expedition.as_mut() {
                    run.roll_boon_choices(&mut ctx_ref.rng);
                }
                ctx_ref.active_modal = ActiveModal::ExpeditionBoons;
                ctx_ref.persist();
                next_state.set(AppState::FjordCity);
                return;
            } else if building_id == "daily_trial" {
                let streak = ctx.city.record_daily_victory(
                    crate::state::today_day_index(),
                    &rewards,
                    &balance,
                );
                ctx.last_daily_streak = Some(streak);
            } else {
                ctx.city
                    .record_level_victory(&building_id, &level_id, stars, &rewards, &balance);
            }
            let collections = ctx.data.collections.clone();
            ctx.last_discovered_relic_key = ctx
                .city
                .discover_next_relic_for_building(&building_id, &collections);
        } else {
            ctx.active_expedition = None;
            ctx.last_discovered_relic_key = None;
        }
        ctx.active_modal = ActiveModal::None;
        ctx.persist();
        next_state.set(AppState::LevelResult);
    }
}

/// Moves and fades out burst particles, floating score popups, camera shake, and window resize.
#[allow(clippy::too_many_arguments)]
pub fn animate_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut input_state: ResMut<Match3InputState>,
    mut resize_events: EventReader<WindowResized>,
    mut backdrops: BackdropQuery,
    mut particles: ParticleQuery,
    mut beams: BeamQuery,
    mut popups: PopupQuery,
    mut boosters: BoosterPulseQuery,
    mut cameras: Query<&mut Transform, With<Camera2d>>,
) {
    let dt = time.delta_secs();

    for ev in resize_events.read() {
        let bg_w = ev.width.max(ev.height * (16.0 / 9.0));
        let bg_h = ev.height.max(ev.width * (9.0 / 16.0));
        for mut bg in &mut backdrops {
            bg.custom_size = Some(Vec2::new(bg_w, bg_h));
        }
        input_state.dirty_board = true;
    }

    for (entity, mut p, mut tf, mut sprite) in &mut particles {
        p.elapsed += dt;
        if p.elapsed >= p.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = p.elapsed / p.lifetime;
        tf.translation.x += p.velocity.x * dt * (1.0 - progress * 0.5);
        tf.translation.y += p.velocity.y * dt * (1.0 - progress * 0.5);
        tf.scale = Vec3::splat((1.0 - progress).max(0.1));
        sprite.color = sprite.color.with_alpha(1.0 - progress);
    }

    for (entity, mut b, mut tf, mut sprite) in &mut beams {
        b.elapsed += dt;
        if b.elapsed >= b.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = b.elapsed / b.lifetime;
        tf.scale = Vec3::splat(1.0 + progress * 0.25);
        sprite.color = sprite.color.with_alpha((1.0 - progress).powf(1.4));
    }

    for (entity, mut f, mut tf, mut col) in &mut popups {
        f.elapsed += dt;
        if f.elapsed >= f.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = f.elapsed / f.lifetime;
        tf.translation.y += FLOATING_TEXT_RISE_SPEED_PX * dt;
        col.0 = col.0.with_alpha((1.0 - progress).max(0.0));
    }

    if input_state.is_idle() {
        let pulse = 1.0 + 0.07 * (time.elapsed_secs() * 4.2).sin();
        for mut tf in &mut boosters {
            tf.scale = Vec3::splat(pulse);
        }
    }

    if input_state.shake_secs > 0.0 {
        input_state.shake_secs = (input_state.shake_secs - dt).max(0.0);
        let intensity = (input_state.shake_secs / SHAKE_DURATION_SECS) * SHAKE_AMPLITUDE_PX;
        let phase = time.elapsed_secs() * 58.0;
        for mut cam in &mut cameras {
            cam.translation.x = phase.sin() * intensity;
            cam.translation.y = (phase * 1.37).cos() * intensity;
        }
    } else {
        for mut cam in &mut cameras {
            cam.translation.x = 0.0;
            cam.translation.y = 0.0;
        }
    }
}
