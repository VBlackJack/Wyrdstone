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
    BoosterPulse, BoosterShimmer, BurstParticle, FloatingScore, FlyingOrb, FuryReadyPulse,
    LowMovesPulse, Match3Backdrop, Match3Cleanup, Match3Entity, Match3InputState, Match3TimerText,
    SanctuaryEmber, SanctuaryTorchGlow, UndoSnapshot, FLOATING_TEXT_LIFETIME_SECS,
    FLOATING_TEXT_RISE_SPEED_PX, HINT_IDLE_SECS, PARTICLES_PER_CLEARED_TILE, PARTICLE_GRAVITY_PX,
    PARTICLE_LIFETIME_SECS, PARTICLE_SIZE_PX, PARTICLE_SPEED_PX, SANCTUARY_EMBER_HALF_H_RATIO,
    SANCTUARY_EMBER_HALF_W_RATIO, SHAKE_AMPLITUDE_PX, SHAKE_DURATION_SECS,
};
use crate::audio::{play_sfx, SfxKind, SoundBank};
use crate::runes_art::RuneTextures;
use crate::state::{ActiveModal, AppState, GameContext};
use crate::theme::*;
use crate::vfx_art::VfxTextures;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use std::collections::{HashMap, HashSet};
use wyrdstone_core::{
    ActiveTool, Booster, CellPos, LevelOutcome, PatronGod, TurnStep, VICTORY_SKIP_MAX_DETONATIONS,
};

type BackdropQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Sprite,
    (
        With<Match3Backdrop>,
        Without<BurstParticle>,
        Without<BlastBeam>,
        Without<SanctuaryTorchGlow>,
        Without<SanctuaryEmber>,
        Without<FlyingOrb>,
        Without<BoosterShimmer>,
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
    (
        Without<Camera2d>,
        Without<BlastBeam>,
        Without<SanctuaryTorchGlow>,
        Without<SanctuaryEmber>,
        Without<FlyingOrb>,
        Without<BoosterShimmer>,
    ),
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
    (
        Without<Camera2d>,
        Without<BurstParticle>,
        Without<SanctuaryTorchGlow>,
        Without<SanctuaryEmber>,
        Without<FlyingOrb>,
        Without<BoosterShimmer>,
    ),
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
        Without<SanctuaryTorchGlow>,
        Without<SanctuaryEmber>,
        Without<FlyingOrb>,
        Without<BoosterShimmer>,
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
        Without<SanctuaryTorchGlow>,
        Without<SanctuaryEmber>,
        Without<FlyingOrb>,
        Without<BoosterShimmer>,
    ),
>;
type TorchGlowQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static SanctuaryTorchGlow,
        &'static mut Transform,
        &'static mut Sprite,
    ),
    (
        Without<Camera2d>,
        Without<BurstParticle>,
        Without<BlastBeam>,
        Without<FloatingScore>,
        Without<SanctuaryEmber>,
        Without<Match3Backdrop>,
        Without<FlyingOrb>,
        Without<BoosterShimmer>,
    ),
>;
type SanctuaryEmberQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut SanctuaryEmber,
        &'static mut Transform,
        &'static mut Sprite,
    ),
    (
        Without<Camera2d>,
        Without<BurstParticle>,
        Without<BlastBeam>,
        Without<FloatingScore>,
        Without<SanctuaryTorchGlow>,
        Without<Match3Backdrop>,
        Without<FlyingOrb>,
        Without<BoosterShimmer>,
    ),
>;
type FlyingOrbQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut FlyingOrb,
        &'static mut Transform,
        &'static mut Sprite,
    ),
    (
        Without<Camera2d>,
        Without<BurstParticle>,
        Without<BlastBeam>,
        Without<FloatingScore>,
        Without<SanctuaryTorchGlow>,
        Without<SanctuaryEmber>,
        Without<Match3Backdrop>,
        Without<BoosterShimmer>,
    ),
>;
type ShimmerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static BoosterShimmer,
        &'static mut Transform,
        &'static mut Sprite,
    ),
    (
        Without<Camera2d>,
        Without<BurstParticle>,
        Without<BlastBeam>,
        Without<FloatingScore>,
        Without<SanctuaryTorchGlow>,
        Without<SanctuaryEmber>,
        Without<Match3Backdrop>,
        Without<FlyingOrb>,
        Without<BoosterPulse>,
    ),
>;

const MAX_FLYING_ORBS_PER_WAVE: usize = 10;

/// Multiplier applied to the clear animation duration when boosters fire in the same clear.
const BOOSTER_CLEAR_DURATION_FACTOR: f32 = 1.85;
/// Base duration of the victory celebration, in seconds, before the per-booster bonus.
const VICTORY_BASE_SECS: f32 = 2.5;
/// Extra celebration seconds added per remaining booster converted at level end.
const VICTORY_SECS_PER_BOOSTER: f32 = 0.62;
/// Upper bound of the victory celebration duration, in seconds.
const VICTORY_MAX_SECS: f32 = 7.5;
/// Delay before the first victory burst, in seconds.
const VICTORY_FIRST_BURST_DELAY_SECS: f32 = 0.32;
/// Delay between consecutive victory bursts, in seconds.
const VICTORY_BURST_INTERVAL_SECS: f32 = 0.58;
/// Minimum combo count that earns the first combo callout.
const COMBO_TIER_2: u32 = 2;
/// Combo count of the second combo callout tier.
const COMBO_TIER_3: u32 = 3;
/// Combo count of the third combo callout tier.
const COMBO_TIER_4: u32 = 4;
/// Combo count from which the top combo callout tier applies.
const COMBO_TIER_5: u32 = 5;
/// Score popup font size without a combo.
const COMBO_POPUP_SIZE_BASE: f32 = 24.0;
/// Score popup font size at combo tier 2.
const COMBO_POPUP_SIZE_TIER_2: f32 = 28.0;
/// Score popup font size at combo tier 3.
const COMBO_POPUP_SIZE_TIER_3: f32 = 32.0;
/// Score popup font size at combo tier 4 and above.
const COMBO_POPUP_SIZE_TOP: f32 = 36.0;

/// Spawns curved Bezier energy orbs flying from cleared tiles up to the HUD objectives or Boss bar.
#[allow(clippy::too_many_arguments)]
fn spawn_flying_orbs(
    commands: &mut Commands,
    vfx: &VfxTextures,
    cells: &[CellPos],
    session: &wyrdstone_core::LevelSession,
    layout: &Match3InputState,
    win_w: f32,
    win_h: f32,
    base_duration: f32,
) {
    if cells.is_empty() {
        return;
    }
    let has_boss = session.boss.is_some();
    let step = (cells.len() / MAX_FLYING_ORBS_PER_WAVE).max(1);
    let obj_target = Vec2::new(-win_w * 0.14, win_h * 0.43);
    let boss_target = Vec2::new(win_w * 0.16, win_h * 0.43);

    for (idx, &pos) in cells
        .iter()
        .step_by(step)
        .take(MAX_FLYING_ORBS_PER_WAVE)
        .enumerate()
    {
        let start = cell_to_world(pos, layout);
        let to_boss = has_boss && idx % 2 == 1;
        let target = if to_boss { boss_target } else { obj_target };
        let color = if to_boss {
            Color::srgba(1.0, 0.56, 0.24, 0.96)
        } else {
            session
                .board
                .get(pos)
                .and_then(|c| c.rune)
                .map(|r| {
                    let [rr, gg, bb] = rune_base_rgb(r);
                    Color::srgba(
                        (rr as f32 / 255.0 * 1.25).min(1.0),
                        (gg as f32 / 255.0 * 1.25).min(1.0),
                        (bb as f32 / 255.0 * 1.25).min(1.0),
                        0.96,
                    )
                })
                .unwrap_or(COLOR_ACCENT_GOLD)
        };
        let mid = start.lerp(target, 0.5);
        let arc_side = if idx % 2 == 0 { -1.0 } else { 1.0 };
        let control = mid + Vec2::new(arc_side * (65.0 + (idx % 3) as f32 * 28.0), 42.0);
        let duration = (base_duration * (0.88 + (idx % 4) as f32 * 0.08)).max(0.22);

        commands.spawn((
            Match3Cleanup,
            FlyingOrb {
                start,
                control,
                target,
                elapsed: 0.0,
                duration,
                color,
            },
            Sprite {
                image: vfx.spark_glow.clone(),
                color,
                custom_size: Some(Vec2::splat(26.0)),
                ..default()
            },
            Transform::from_xyz(start.x, start.y, Z_SELECTION_RING + 2.2),
        ));
    }
}

pub(super) fn spawn_clear_particles(
    commands: &mut Commands,
    vfx: &VfxTextures,
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
                    (rr as f32 / 255.0 * 1.18).min(1.0),
                    (gg as f32 / 255.0 * 1.18).min(1.0),
                    (bb as f32 / 255.0 * 1.18).min(1.0),
                )
            })
            .unwrap_or(COLOR_ACCENT_GOLD);

        // Bright radial impact flash on the cleared tile.
        spawn_textured_vfx(
            commands,
            vfx.soft_orb.clone(),
            center,
            Vec2::splat(layout.tile_size * 1.25),
            color.with_alpha(0.78),
            0.22,
            0.0,
            0.0,
            0.65,
            1.35,
        );

        for i in 0..PARTICLES_PER_CLEARED_TILE {
            let angle = (i as f32 / PARTICLES_PER_CLEARED_TILE as f32) * std::f32::consts::TAU
                + (pos.col as f32 * 0.43 + pos.row as f32 * 0.29);
            let speed_scale = 0.68 + ((i * 37 + pos.col * 13) % 10) as f32 * 0.065;
            let size_scale = 0.72 + ((i * 19 + pos.row * 7) % 5) as f32 * 0.14;
            let dir = Vec2::new(angle.cos(), angle.sin());
            let spin = if i % 2 == 0 { 5.2 } else { -5.2 };
            commands.spawn((
                Match3Cleanup,
                BurstParticle {
                    velocity: dir * PARTICLE_SPEED_PX * speed_scale + Vec2::new(0.0, 42.0),
                    elapsed: 0.0,
                    lifetime: PARTICLE_LIFETIME_SECS * (0.85 + (i % 3) as f32 * 0.15),
                    spin,
                },
                Sprite {
                    image: vfx.spark_glow.clone(),
                    color,
                    custom_size: Some(Vec2::splat(PARTICLE_SIZE_PX * size_scale)),
                    ..default()
                },
                Transform::from_xyz(center.x, center.y, Z_SELECTION_RING + 1.0),
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_textured_vfx(
    commands: &mut Commands,
    image: Handle<Image>,
    center: Vec2,
    size: Vec2,
    color: Color,
    lifetime: f32,
    rotation_rad: f32,
    rotation_speed: f32,
    start_scale: f32,
    end_scale: f32,
) {
    commands.spawn((
        Match3Cleanup,
        BlastBeam {
            elapsed: 0.0,
            lifetime,
            rotation_speed,
            start_scale,
            end_scale,
        },
        Sprite {
            image,
            color,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_xyz(center.x, center.y, Z_SELECTION_RING + 1.5)
            .with_rotation(Quat::from_rotation_z(rotation_rad))
            .with_scale(Vec3::splat(start_scale)),
    ));
}

fn spawn_shockwave_ring(
    commands: &mut Commands,
    vfx: &VfxTextures,
    center: Vec2,
    size: Vec2,
    color: Color,
    lifetime: f32,
) {
    spawn_textured_vfx(
        commands,
        vfx.shockwave_ring.clone(),
        center,
        size,
        color,
        lifetime,
        0.0,
        0.0,
        0.42,
        1.42,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_lightning_strike(
    commands: &mut Commands,
    vfx: &VfxTextures,
    center: Vec2,
    beam_thickness: f32,
    beam_length: f32,
    horizontal: bool,
    color: Color,
    lifetime: f32,
) {
    let rotation = if horizontal {
        std::f32::consts::FRAC_PI_2
    } else {
        0.0
    };
    spawn_textured_vfx(
        commands,
        vfx.lightning_beam.clone(),
        center,
        Vec2::new(beam_thickness, beam_length),
        color,
        lifetime,
        rotation,
        0.0,
        0.92,
        1.12,
    );
}

/// Spawns dedicated high-impact VFX for an activated charged rune (Line, Bomb, or Bifrost Shard).
#[allow(clippy::too_many_arguments)]
fn spawn_booster_vfx_for(
    commands: &mut Commands,
    vfx: &VfxTextures,
    pos: CellPos,
    booster: Booster,
    cols: usize,
    rows: usize,
    layout: &Match3InputState,
) {
    let center = cell_to_world(pos, layout);
    let board_w = cols as f32 * layout.cell_step;
    let board_h = rows as f32 * layout.cell_step;
    let center_y = layout.board_center_y;

    match booster {
        Booster::None => {}
        Booster::LineHorizontal => {
            spawn_lightning_strike(
                commands,
                vfx,
                Vec2::new(0.0, center.y),
                layout.tile_size * 1.05,
                board_w * 1.08,
                true,
                Color::srgba(0.60, 0.96, 1.0, 0.98),
                0.68,
            );
            spawn_shockwave_ring(
                commands,
                vfx,
                center,
                Vec2::splat(layout.cell_step * 2.1),
                Color::srgba(1.0, 0.90, 0.42, 0.94),
                0.56,
            );
        }
        Booster::LineVertical => {
            spawn_lightning_strike(
                commands,
                vfx,
                Vec2::new(center.x, center_y),
                layout.tile_size * 1.05,
                board_h * 1.08,
                false,
                Color::srgba(0.60, 0.96, 1.0, 0.98),
                0.68,
            );
            spawn_shockwave_ring(
                commands,
                vfx,
                center,
                Vec2::splat(layout.cell_step * 2.1),
                Color::srgba(1.0, 0.90, 0.42, 0.94),
                0.56,
            );
        }
        Booster::Bomb => {
            spawn_shockwave_ring(
                commands,
                vfx,
                center,
                Vec2::splat(layout.cell_step * 3.8),
                Color::srgba(1.0, 0.54, 0.16, 0.98),
                0.72,
            );
            spawn_shockwave_ring(
                commands,
                vfx,
                center,
                Vec2::splat(layout.cell_step * 2.4),
                Color::srgba(1.0, 0.88, 0.36, 0.95),
                0.56,
            );
            spawn_textured_vfx(
                commands,
                vfx.soft_orb.clone(),
                center,
                Vec2::splat(layout.cell_step * 3.0),
                Color::srgba(1.0, 0.72, 0.24, 0.72),
                0.52,
                0.0,
                0.0,
                0.45,
                1.45,
            );
        }
        Booster::BifrostShard => {
            spawn_textured_vfx(
                commands,
                vfx.runic_vortex.clone(),
                center,
                Vec2::splat(board_w.min(board_h) * 0.86),
                Color::srgba(0.62, 0.88, 1.0, 0.84),
                0.82,
                0.0,
                3.4,
                0.45,
                1.32,
            );
            spawn_shockwave_ring(
                commands,
                vfx,
                center,
                Vec2::splat(layout.cell_step * 3.4),
                Color::srgba(0.98, 0.72, 0.96, 0.94),
                0.68,
            );
        }
    }
}

pub(super) fn spawn_patron_fury_vfx(
    commands: &mut Commands,
    vfx: &VfxTextures,
    patron: PatronGod,
    targets: &[CellPos],
    cols: usize,
    rows: usize,
    layout: &Match3InputState,
) {
    let board_w = cols as f32 * layout.cell_step;
    let board_h = rows as f32 * layout.cell_step;
    let center_y = layout.board_center_y;
    match patron {
        PatronGod::Thor => {
            // Vertical cyan lightning bolts + impact shockwaves on every targeted cell.
            for &pos in targets {
                let w = cell_to_world(pos, layout);
                spawn_lightning_strike(
                    commands,
                    vfx,
                    Vec2::new(w.x, center_y),
                    layout.tile_size * 0.95,
                    board_h * 1.06,
                    false,
                    Color::srgba(0.55, 0.95, 1.0, 0.98),
                    0.44,
                );
                spawn_shockwave_ring(
                    commands,
                    vfx,
                    w,
                    Vec2::splat(layout.cell_step * 1.45),
                    Color::srgba(0.65, 0.95, 1.0, 0.90),
                    0.38,
                );
            }
        }
        PatronGod::Freya => {
            // Radiant golden-rose runic mandala + starburst rings across cleared tiles.
            spawn_textured_vfx(
                commands,
                vfx.runic_vortex.clone(),
                Vec2::new(0.0, center_y),
                Vec2::splat(board_w.min(board_h) * 0.98),
                Color::srgba(1.0, 0.78, 0.56, 0.62),
                0.56,
                0.0,
                -1.4,
                0.55,
                1.25,
            );
            for &pos in targets {
                let w = cell_to_world(pos, layout);
                spawn_shockwave_ring(
                    commands,
                    vfx,
                    w,
                    Vec2::splat(layout.cell_step * 1.5),
                    Color::srgba(1.0, 0.84, 0.42, 0.92),
                    0.44,
                );
            }
        }
        PatronGod::Loki => {
            // Spinning violet trickster runic vortex + emerald shockwaves on spawned boosters.
            spawn_textured_vfx(
                commands,
                vfx.runic_vortex.clone(),
                Vec2::new(0.0, center_y),
                Vec2::splat(board_w.min(board_h) * 0.96),
                Color::srgba(0.68, 0.36, 1.0, 0.72),
                0.56,
                0.0,
                4.2,
                0.55,
                1.22,
            );
            for &pos in targets {
                let w = cell_to_world(pos, layout);
                spawn_shockwave_ring(
                    commands,
                    vfx,
                    w,
                    Vec2::splat(layout.cell_step * 1.65),
                    Color::srgba(0.32, 0.96, 0.68, 0.94),
                    0.46,
                );
            }
        }
        PatronGod::Odin => {
            // Rotating golden Allfather runic aura + solar-crimson shockwave on spawned Surtr Bomb.
            spawn_textured_vfx(
                commands,
                vfx.runic_vortex.clone(),
                Vec2::new(0.0, center_y),
                Vec2::splat(board_w.min(board_h) * 0.96),
                Color::srgba(1.0, 0.86, 0.34, 0.74),
                0.58,
                0.0,
                -2.4,
                0.55,
                1.26,
            );
            for &pos in targets {
                let w = cell_to_world(pos, layout);
                spawn_shockwave_ring(
                    commands,
                    vfx,
                    w,
                    Vec2::splat(layout.cell_step * 2.1),
                    Color::srgba(1.0, 0.58, 0.22, 0.95),
                    0.48,
                );
            }
        }
    }
}

pub(super) fn spawn_loki_vfx(
    commands: &mut Commands,
    vfx: &VfxTextures,
    cols: usize,
    rows: usize,
    layout: &Match3InputState,
) {
    let board_w = cols as f32 * layout.cell_step;
    let board_h = rows as f32 * layout.cell_step;
    let center_y = layout.board_center_y;
    spawn_textured_vfx(
        commands,
        vfx.runic_vortex.clone(),
        Vec2::new(0.0, center_y),
        Vec2::splat(board_w.min(board_h) * 1.02),
        Color::srgba(0.68, 0.44, 1.0, 0.78),
        0.46,
        0.0,
        4.4,
        0.55,
        1.20,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_tool_vfx(
    commands: &mut Commands,
    vfx: &VfxTextures,
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
    let center_y = layout.board_center_y;
    match tool {
        ActiveTool::ThorBolt => {
            spawn_lightning_strike(
                commands,
                vfx,
                Vec2::new(0.0, center.y),
                layout.tile_size * 0.95,
                board_w * 1.06,
                true,
                Color::srgba(0.50, 0.94, 1.0, 0.96),
                0.38,
            );
        }
        ActiveTool::DrakkarCharge => {
            spawn_lightning_strike(
                commands,
                vfx,
                Vec2::new(0.0, center.y),
                layout.tile_size * 0.95,
                board_w * 1.06,
                true,
                Color::srgba(1.0, 0.86, 0.34, 0.96),
                0.40,
            );
            spawn_lightning_strike(
                commands,
                vfx,
                Vec2::new(center.x, center_y),
                layout.tile_size * 0.95,
                board_h * 1.06,
                false,
                Color::srgba(1.0, 0.86, 0.34, 0.96),
                0.40,
            );
            spawn_shockwave_ring(
                commands,
                vfx,
                center,
                Vec2::splat(layout.cell_step * 2.2),
                Color::srgba(1.0, 0.92, 0.52, 0.92),
                0.38,
            );
        }
        ActiveTool::SurtrEmber => {
            spawn_shockwave_ring(
                commands,
                vfx,
                center,
                Vec2::splat(layout.cell_step * 3.4),
                Color::srgba(1.0, 0.50, 0.16, 0.95),
                0.44,
            );
            spawn_textured_vfx(
                commands,
                vfx.soft_orb.clone(),
                center,
                Vec2::splat(layout.cell_step * 2.6),
                Color::srgba(1.0, 0.70, 0.24, 0.65),
                0.32,
                0.0,
                0.0,
                0.5,
                1.35,
            );
        }
        ActiveTool::Mjolnir => {
            spawn_shockwave_ring(
                commands,
                vfx,
                center,
                Vec2::splat(layout.cell_step * 1.8),
                Color::srgba(0.68, 0.94, 1.0, 0.95),
                0.36,
            );
        }
        ActiveTool::BifrostPrism => {
            let prism_colors = [
                Color::srgba(0.98, 0.42, 0.62, 0.92),
                Color::srgba(0.42, 0.92, 1.0, 0.92),
                Color::srgba(0.98, 0.84, 0.34, 0.92),
                Color::srgba(0.68, 0.46, 1.0, 0.92),
            ];
            for (idx, &pos) in affected_cells.iter().enumerate() {
                let w = cell_to_world(pos, layout);
                spawn_shockwave_ring(
                    commands,
                    vfx,
                    w,
                    Vec2::splat(layout.cell_step * 1.45),
                    prism_colors[idx % prism_colors.len()],
                    0.38,
                );
            }
        }
        ActiveTool::LokiShuffle => {
            spawn_loki_vfx(commands, vfx, cols, rows, layout);
        }
    }
}

fn spawn_floating_score(
    commands: &mut Commands,
    cells: &[CellPos],
    label: String,
    color: Color,
    font_size: f32,
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
            font_size,
            ..default()
        },
        TextColor(color),
        Transform::from_xyz(center.x, center.y + 8.0, Z_SELECTION_RING + 2.0)
            .with_scale(Vec3::splat(0.65)),
    ));
}

/// Advances the stepped Match-3 animation state machine (swap -> clear -> fall -> cascade).
#[allow(clippy::too_many_arguments)]
pub fn tick_board_animation(
    mut commands: Commands,
    time: Res<Time>,
    mut ctx: ResMut<GameContext>,
    textures: Res<RuneTextures>,
    vfx: Res<VfxTextures>,
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

    // Clear boss portrait hit-flash shortly after damage lands.
    if input_state.boss_hit_flash_secs > 0.0 {
        input_state.boss_hit_flash_secs = (input_state.boss_hit_flash_secs - dt).max(0.0);
        if input_state.boss_hit_flash_secs == 0.0 {
            if let Some(session) = ctx.active_session.as_mut() {
                session.clear_boss_hit_marker();
            }
            input_state.dirty_hud = true;
        }
    }

    // Tick optional level countdown timer. It only runs while the board is idle: swap, clear,
    // fall and celebration animation time no longer counts against a timed level, and the Help
    // modal pauses it too.
    let timer_paused = !input_state.is_idle() || ctx.active_modal == ActiveModal::Help;
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
                    COLOR_ACCENT_CRIMSON_TEXT
                } else {
                    COLOR_ACCENT_TEAL_TEXT
                };
                for (mut text, mut text_col) in &mut timer_text_q {
                    text.0 = formatted.clone();
                    text_col.0 = color;
                }
            }
        }
    }

    if ctx.active_modal == ActiveModal::Help {
        return;
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
                    let pulse = if input_state.reduced_motion {
                        1.0
                    } else {
                        1.0 + 0.11 * ((input_state.idle_secs * 5.0).sin() * 0.5 + 0.5)
                    };
                    let glow = Color::srgb(1.0, 0.96, 0.72);
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
                let pre_swap = ctx.active_session.clone().map(|session| UndoSnapshot {
                    session,
                    total_runes_cleared: ctx.city.stats.total_runes_cleared,
                    best_combo: ctx.city.stats.best_combo,
                });
                let started = ctx.active_session.as_mut().map(|s| s.begin_swap(a, b));
                if let Some(Ok((turn, TurnStep::ClearPending(preview)))) = started {
                    if !input_state.undo_used {
                        input_state.undo_snapshot = pre_swap;
                    }
                    if let Some(session) = ctx.active_session.as_ref() {
                        rebuild_board_now(
                            &mut commands,
                            session,
                            &textures,
                            &vfx,
                            window,
                            &mut input_state,
                            &all_sprites,
                            None,
                            ctx.colorblind_mode,
                        );
                    }
                    input_state.dirty_hud_soft = true;
                    if !preview.prepared.converted_boosters.is_empty() {
                        input_state.phase = BoardAnimPhase::SuperComboCharge {
                            turn,
                            preview,
                            elapsed: 0.0,
                            duration: ctx.anim_duration(0.64),
                            beams_spawned: false,
                        };
                    } else {
                        let base_clear = if preview.fired_boosters.is_empty() {
                            ctx.data.balance.clear_anim_seconds
                        } else {
                            ctx.data.balance.clear_anim_seconds * BOOSTER_CLEAR_DURATION_FACTOR
                        };
                        let cleared_set: HashSet<CellPos> =
                            preview.prepared.cells.iter().copied().collect();
                        input_state.phase = BoardAnimPhase::Clearing {
                            turn,
                            preview,
                            cleared_set,
                            elapsed: 0.0,
                            duration: ctx.anim_duration(base_clear),
                            particles_spawned: false,
                        };
                    }
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
        BoardAnimPhase::SuperComboCharge {
            turn,
            preview,
            mut elapsed,
            duration,
            mut beams_spawned,
        } => {
            if !beams_spawned {
                play_sfx(&mut commands, &sound_bank, SfxKind::FuryUnleash);
                let colors = [
                    Color::srgba(0.98, 0.84, 0.32, 0.88),
                    Color::srgba(0.42, 0.92, 1.0, 0.88),
                    Color::srgba(0.98, 0.45, 0.68, 0.88),
                ];
                for (idx, &(pos, _)) in preview.prepared.converted_boosters.iter().enumerate() {
                    let w = cell_to_world(pos, &input_state);
                    spawn_shockwave_ring(
                        &mut commands,
                        &vfx,
                        w,
                        Vec2::splat(input_state.cell_step * 1.75),
                        colors[idx % colors.len()],
                        0.62,
                    );
                }
                input_state.shake_secs = SHAKE_DURATION_SECS;
                beams_spawned = true;
            }
            elapsed += dt;
            let pulse =
                1.0 + 0.18 * ((elapsed / duration).clamp(0.0, 1.0) * std::f32::consts::PI).sin();
            for (tile, mut tf, mut sprite) in &mut tile_q {
                if preview
                    .prepared
                    .converted_boosters
                    .iter()
                    .any(|&(p, _)| p == tile.pos)
                {
                    tf.scale = Vec3::splat(pulse);
                    sprite.color = Color::srgb(1.4, 1.28, 0.72);
                }
            }
            if elapsed >= duration {
                let cleared_set: HashSet<CellPos> =
                    preview.prepared.cells.iter().copied().collect();
                input_state.phase = BoardAnimPhase::Clearing {
                    turn,
                    preview,
                    cleared_set,
                    elapsed: 0.0,
                    duration: ctx.anim_duration(ctx.data.balance.clear_anim_seconds),
                    particles_spawned: false,
                };
            } else {
                input_state.phase = BoardAnimPhase::SuperComboCharge {
                    turn,
                    preview,
                    elapsed,
                    duration,
                    beams_spawned,
                };
            }
        }
        BoardAnimPhase::Clearing {
            mut turn,
            preview,
            cleared_set,
            mut elapsed,
            duration,
            mut particles_spawned,
        } => {
            if !particles_spawned {
                play_sfx(
                    &mut commands,
                    &sound_bank,
                    SfxKind::MatchCombo(preview.combo),
                );
                if let Some(session) = ctx.active_session.as_ref() {
                    let cols = session.board.cols();
                    let rows = session.board.rows();
                    for &(pos, booster) in &preview.fired_boosters {
                        spawn_booster_vfx_for(
                            &mut commands,
                            &vfx,
                            pos,
                            booster,
                            cols,
                            rows,
                            &input_state,
                        );
                    }
                    if !preview.fired_boosters.is_empty() {
                        play_sfx(&mut commands, &sound_bank, SfxKind::ToolBlast);
                        input_state.shake_secs = SHAKE_DURATION_SECS * 1.25;
                    }
                    spawn_clear_particles(
                        &mut commands,
                        &vfx,
                        &preview.prepared.cells,
                        session,
                        &input_state,
                    );
                    spawn_flying_orbs(
                        &mut commands,
                        &vfx,
                        &preview.prepared.cells,
                        session,
                        &input_state,
                        window.width(),
                        window.height(),
                        ctx.anim_duration(0.58),
                    );
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
                let step = {
                    let ctx_ref = ctx.as_mut();
                    ctx_ref
                        .active_session
                        .as_mut()
                        .and_then(|session| turn.advance(session, &balance, &mut ctx_ref.rng))
                };
                let Some(TurnStep::Cleared(report)) = step else {
                    debug_assert!(false, "a pending clear must report its committed step");
                    input_state.phase = BoardAnimPhase::Idle;
                    return;
                };
                ctx.city.stats.record_clear(report.stats);
                let combo = report.combo;
                let gained_score = report.score_awarded;
                let boss_damaged = report.boss.as_ref().is_some_and(|hit| hit.damage > 0);
                let boss_phase2_name = report
                    .boss
                    .as_ref()
                    .filter(|hit| hit.entered_phase2)
                    .map(|hit| ctx.data.i18n.t(&hit.name_key).to_string());
                let mut fall_map = HashMap::new();
                for fm in &report.fall {
                    fall_map.insert(CellPos::new(fm.col, fm.to_row), fm.from_row);
                }
                if boss_damaged {
                    input_state.boss_hit_flash_secs = 0.38;
                }
                if let Some(bname) = boss_phase2_name {
                    play_sfx(&mut commands, &sound_bank, SfxKind::BossRiposte);
                    let msg = ctx
                        .data
                        .i18n
                        .t_fmt("match3.boss.phase2", &[("boss", bname)]);
                    commands.spawn((
                        Match3Cleanup,
                        FloatingScore {
                            elapsed: 0.0,
                            lifetime: 1.35,
                        },
                        Text2d::new(msg),
                        TextFont {
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_CRIMSON_TEXT),
                        Transform::from_xyz(
                            0.0,
                            input_state.board_center_y + 20.0,
                            Z_SELECTION_RING + 3.0,
                        ),
                    ));
                    input_state.shake_secs = SHAKE_DURATION_SECS * 1.4;
                }
                if gained_score > 0 {
                    let i18n = &ctx.data.i18n;
                    let popup_label = if combo >= COMBO_TIER_5 {
                        i18n.t_fmt("match3.callout.combo5", &[("combo", combo.to_string())])
                    } else if combo == COMBO_TIER_4 {
                        i18n.t_fmt("match3.callout.combo4", &[("combo", combo.to_string())])
                    } else if combo == COMBO_TIER_3 {
                        i18n.t_fmt("match3.callout.combo3", &[("combo", combo.to_string())])
                    } else if combo == COMBO_TIER_2 {
                        i18n.t_fmt(
                            "match3.callout.combo2",
                            &[("score", gained_score.to_string())],
                        )
                    } else {
                        i18n.t_fmt("match3.popup.score", &[("n", gained_score.to_string())])
                    };
                    let popup_color = if combo >= COMBO_TIER_2 {
                        COLOR_ACCENT_GOLD
                    } else {
                        COLOR_TEXT_PRIMARY
                    };
                    let popup_size = match combo {
                        c if c < COMBO_TIER_2 => COMBO_POPUP_SIZE_BASE,
                        COMBO_TIER_2 => COMBO_POPUP_SIZE_TIER_2,
                        COMBO_TIER_3 => COMBO_POPUP_SIZE_TIER_3,
                        _ => COMBO_POPUP_SIZE_TOP,
                    };
                    spawn_floating_score(
                        &mut commands,
                        &report.cells,
                        popup_label,
                        popup_color,
                        popup_size,
                        &input_state,
                    );
                }
                if combo >= 3 || report.cells.len() >= 6 {
                    input_state.shake_secs = SHAKE_DURATION_SECS;
                }
                if let Some(session) = ctx.active_session.as_ref() {
                    rebuild_board_now(
                        &mut commands,
                        session,
                        &textures,
                        &vfx,
                        window,
                        &mut input_state,
                        &all_sprites,
                        Some(&fall_map),
                        ctx.colorblind_mode,
                    );
                }
                input_state.idle_secs = 0.0;
                input_state.hint_pair = None;
                input_state.dirty_hud_soft = true;
                input_state.phase = BoardAnimPhase::Falling {
                    turn,
                    moves: fall_map,
                    elapsed: 0.0,
                    duration: ctx.anim_duration(ctx.data.balance.fall_anim_seconds),
                };
            } else {
                input_state.phase = BoardAnimPhase::Clearing {
                    turn,
                    preview,
                    cleared_set,
                    elapsed,
                    duration,
                    particles_spawned,
                };
            }
        }
        BoardAnimPhase::Falling {
            mut turn,
            moves,
            mut elapsed,
            duration,
        } => {
            elapsed += dt;
            let raw_t = (elapsed / duration).clamp(0.0, 1.0);
            let t = ease_out_back_soft(raw_t);
            // Subtle squash-and-stretch as tiles accelerate then cushion into place.
            let impact_squash = if raw_t > 0.72 {
                let s = ((raw_t - 0.72) / 0.28 * std::f32::consts::PI).sin();
                s * 0.08
            } else {
                0.0
            };
            for (tile, mut tf, _) in &mut tile_q {
                if let Some(&from_row) = moves.get(&tile.pos) {
                    let start = grid_to_world_f(tile.pos.col as f32, from_row as f32, &input_state);
                    let end = cell_to_world(tile.pos, &input_state);
                    let p = start.lerp(end, t);
                    tf.translation.x = p.x;
                    tf.translation.y = p.y;
                    tf.scale = Vec3::new(1.0 + impact_squash, 1.0 - impact_squash * 0.85, 1.0);
                }
            }

            if elapsed >= duration {
                let balance = ctx.data.balance.clone();
                let consumes_turn = turn.consumes_turn();
                let step = {
                    let ctx_ref = ctx.as_mut();
                    ctx_ref
                        .active_session
                        .as_mut()
                        .and_then(|session| turn.advance(session, &balance, &mut ctx_ref.rng))
                };
                match step {
                    Some(TurnStep::RelicsDropped(relics)) => {
                        let mut relic_fall_map = HashMap::new();
                        for fm in &relics.fall {
                            relic_fall_map.insert(CellPos::new(fm.col, fm.to_row), fm.from_row);
                        }
                        play_sfx(&mut commands, &sound_bank, SfxKind::RelicDrop);
                        if let Some(session) = ctx.active_session.as_ref() {
                            spawn_clear_particles(
                                &mut commands,
                                &vfx,
                                &relics.positions,
                                session,
                                &input_state,
                            );
                            rebuild_board_now(
                                &mut commands,
                                session,
                                &textures,
                                &vfx,
                                window,
                                &mut input_state,
                                &all_sprites,
                                Some(&relic_fall_map),
                                ctx.colorblind_mode,
                            );
                        }
                        input_state.dirty_hud_soft = true;
                        input_state.phase = BoardAnimPhase::Falling {
                            turn,
                            moves: relic_fall_map,
                            elapsed: 0.0,
                            duration: ctx.anim_duration(ctx.data.balance.fall_anim_seconds),
                        };
                    }
                    Some(TurnStep::ClearPending(preview)) => {
                        let cleared_set: HashSet<CellPos> =
                            preview.prepared.cells.iter().copied().collect();
                        input_state.dirty_hud_soft = true;
                        input_state.phase = BoardAnimPhase::Clearing {
                            turn,
                            preview,
                            cleared_set,
                            elapsed: 0.0,
                            duration: ctx.anim_duration(ctx.data.balance.clear_anim_seconds),
                            particles_spawned: false,
                        };
                    }
                    Some(TurnStep::Ended(end)) => {
                        debug_assert!(turn.is_finished(), "an ended turn must be finished");
                        let did_shuffle = end.shuffled;
                        let frost_pos = end.frost;
                        let curse_hits = end.curse_hits;
                        let (boss_riposte_hits, boss_riposte_name) = match end.riposte {
                            Some((hits, name_key)) => {
                                (hits, Some(ctx.data.i18n.t(&name_key).to_string()))
                            }
                            None => (Vec::new(), None),
                        };
                        if !curse_hits.is_empty() {
                            play_sfx(&mut commands, &sound_bank, SfxKind::BossRiposte);
                            for &cp in &curse_hits {
                                let w = cell_to_world(cp, &input_state);
                                spawn_shockwave_ring(
                                    &mut commands,
                                    &vfx,
                                    w,
                                    Vec2::splat(input_state.cell_step * 1.55),
                                    Color::srgba(0.96, 0.18, 0.44, 0.94),
                                    0.40,
                                );
                            }
                            let msg = ctx.data.i18n.t("match3.curse.detonate").to_string();
                            spawn_floating_score(
                                &mut commands,
                                &curse_hits,
                                msg,
                                COLOR_ACCENT_CRIMSON_TEXT,
                                28.0,
                                &input_state,
                            );
                            input_state.shake_secs = SHAKE_DURATION_SECS;
                            input_state.dirty_board = true;
                        } else if consumes_turn {
                            // Refresh cursed rune countdown badges on board after a player turn.
                            if end.cursed_runes_remain {
                                input_state.dirty_board = true;
                            }
                        }
                        if !boss_riposte_hits.is_empty() {
                            play_sfx(&mut commands, &sound_bank, SfxKind::BossRiposte);
                            for &rp in &boss_riposte_hits {
                                let w = cell_to_world(rp, &input_state);
                                spawn_shockwave_ring(
                                    &mut commands,
                                    &vfx,
                                    w,
                                    Vec2::splat(input_state.cell_step * 1.6),
                                    Color::srgba(0.98, 0.24, 0.30, 0.94),
                                    0.40,
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
                                    COLOR_ACCENT_CRIMSON_TEXT,
                                    28.0,
                                    &input_state,
                                );
                            }
                            input_state.shake_secs = SHAKE_DURATION_SECS;
                            input_state.dirty_board = true;
                            input_state.dirty_hud = true;
                        }
                        if let Some(fp) = frost_pos {
                            let w = cell_to_world(fp, &input_state);
                            spawn_shockwave_ring(
                                &mut commands,
                                &vfx,
                                w,
                                Vec2::splat(input_state.cell_step * 1.45),
                                Color::srgba(0.45, 0.90, 1.0, 0.92),
                                0.38,
                            );
                            input_state.dirty_board = true;
                            input_state.dirty_hud = true;
                        }
                        if did_shuffle {
                            input_state.dirty_board = true;
                            input_state.dirty_hud = true;
                        }
                        match end.outcome {
                            LevelOutcome::Victory => {
                                play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                                let banner = ctx.data.i18n.t("result.victory.title").to_string();
                                commands.spawn((
                                    Match3Cleanup,
                                    FloatingScore {
                                        elapsed: 0.0,
                                        lifetime: 2.2,
                                    },
                                    Text2d::new(banner),
                                    TextFont {
                                        font_size: 48.0,
                                        ..default()
                                    },
                                    TextColor(COLOR_ACCENT_GOLD),
                                    Transform::from_xyz(0.0, 10.0, Z_SELECTION_RING + 3.0),
                                ));
                                let victory_secs = ctx.anim_duration(
                                    (VICTORY_BASE_SECS
                                        + end.remaining_boosters as f32 * VICTORY_SECS_PER_BOOSTER)
                                        .min(VICTORY_MAX_SECS),
                                );
                                input_state.dirty_hud = true;
                                input_state.phase = BoardAnimPhase::VictoryCelebration {
                                    elapsed: 0.0,
                                    duration: victory_secs,
                                    next_burst_at: ctx
                                        .anim_duration(VICTORY_FIRST_BURST_DELAY_SECS),
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
                    Some(TurnStep::Cleared(_)) | None => {
                        debug_assert!(
                            false,
                            "a fallen board must report relics, a cascade or the end"
                        );
                        input_state.phase = BoardAnimPhase::Idle;
                    }
                }
            } else {
                input_state.phase = BoardAnimPhase::Falling {
                    turn,
                    moves,
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
            if input_state.skip_valhalla {
                input_state.skip_valhalla = false;
                flush_remaining_bonus_boosters(ctx.as_mut());
                finalize_level_transition(&mut ctx, &mut next_state);
                input_state.phase = BoardAnimPhase::Idle;
                return;
            }
            elapsed += dt;
            if input_state.reduced_motion {
                for (_, mut tf, mut sprite) in &mut tile_q {
                    if tf.scale != Vec3::ONE {
                        tf.scale = Vec3::ONE;
                    }
                    if sprite.color != Color::WHITE {
                        sprite.color = Color::WHITE;
                    }
                }
            } else {
                for (tile, mut tf, mut sprite) in &mut tile_q {
                    let wave =
                        ((elapsed * 7.5) - (tile.pos.col + tile.pos.row) as f32 * 0.45).sin();
                    tf.scale = Vec3::splat(1.0 + 0.10 * wave.max(0.0));
                    sprite.color =
                        Color::srgb(1.0 + 0.20 * wave.max(0.0), 1.0 + 0.16 * wave.max(0.0), 1.0);
                }
            }

            if elapsed >= next_burst_at && elapsed < duration - 0.20 {
                burst_index += 1;
                // Priority 1: Sequentially detonate every remaining charged rune (booster) on the board for real bonus points!
                let balance = ctx.data.balance.clone();
                let blast = {
                    let ctx_ref = ctx.as_mut();
                    ctx_ref.active_session.as_mut().and_then(|session| {
                        session.detonate_next_bonus_booster(&balance, &mut ctx_ref.rng)
                    })
                };

                if let Some(blast) = blast {
                    ctx.city.stats.record_clear(blast.stats);
                    next_burst_at += ctx.anim_duration(VICTORY_BURST_INTERVAL_SECS);
                    let b_pos = blast.pos;
                    let b_kind = blast.booster;
                    let gained_pts = blast.score_awarded;
                    let cleared_cells = blast.cleared;
                    let (cols, rows) = ctx
                        .active_session
                        .as_ref()
                        .map(|s| (s.board.cols(), s.board.rows()))
                        .unwrap_or((8, 8));
                    spawn_booster_vfx_for(
                        &mut commands,
                        &vfx,
                        b_pos,
                        b_kind,
                        cols,
                        rows,
                        &input_state,
                    );
                    if let Some(session) = ctx.active_session.as_ref() {
                        if !cleared_cells.is_empty() {
                            spawn_clear_particles(
                                &mut commands,
                                &vfx,
                                &cleared_cells,
                                session,
                                &input_state,
                            );
                            spawn_flying_orbs(
                                &mut commands,
                                &vfx,
                                &cleared_cells,
                                session,
                                &input_state,
                                window.width(),
                                window.height(),
                                ctx.anim_duration(0.54),
                            );
                        }
                        rebuild_board_now(
                            &mut commands,
                            session,
                            &textures,
                            &vfx,
                            window,
                            &mut input_state,
                            &all_sprites,
                            None,
                            ctx.colorblind_mode,
                        );
                    }
                    if gained_pts > 0 {
                        spawn_floating_score(
                            &mut commands,
                            &[b_pos],
                            ctx.data
                                .i18n
                                .t_fmt("match3.popup.score", &[("n", gained_pts.to_string())]),
                            COLOR_ACCENT_GOLD,
                            34.0,
                            &input_state,
                        );
                    }
                    play_sfx(&mut commands, &sound_bank, SfxKind::ToolBlast);
                    input_state.shake_secs = SHAKE_DURATION_SECS * 1.15;
                    input_state.dirty_hud = true;
                } else {
                    // Priority 2: Valhalla Bonus golden lightning celebration strikes.
                    next_burst_at += ctx.anim_duration(0.42);
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
                            let board_h = rows as f32 * input_state.cell_step;
                            let center_y = input_state.board_center_y;
                            let strike_world = cell_to_world(burst_cells[0], &input_state);
                            spawn_lightning_strike(
                                &mut commands,
                                &vfx,
                                Vec2::new(strike_world.x, center_y),
                                input_state.tile_size * 0.92,
                                board_h * 1.06,
                                false,
                                Color::srgba(1.0, 0.88, 0.36, 0.96),
                                0.56,
                            );
                            spawn_clear_particles(
                                &mut commands,
                                &vfx,
                                &burst_cells,
                                session,
                                &input_state,
                            );
                            let bonus_pts = ctx.data.balance.remaining_move_bonus;
                            let label = if burst_index == 1 && session.moves_left > 0 {
                                ctx.data.i18n.t_fmt(
                                    "match3.valhalla.bonus",
                                    &[("pts", (session.moves_left * bonus_pts).to_string())],
                                )
                            } else {
                                ctx.data
                                    .i18n
                                    .t_fmt("match3.popup.score", &[("n", bonus_pts.to_string())])
                            };
                            spawn_floating_score(
                                &mut commands,
                                &[burst_cells[0]],
                                label,
                                COLOR_ACCENT_GOLD,
                                32.0,
                                &input_state,
                            );
                            play_sfx(&mut commands, &sound_bank, SfxKind::MatchCombo(burst_index));
                        }
                    }
                }
            }

            if elapsed >= duration {
                flush_remaining_bonus_boosters(ctx.as_mut());
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

fn flush_remaining_bonus_boosters(ctx: &mut GameContext) {
    let balance = ctx.data.balance.clone();
    let Some(session) = ctx.active_session.as_mut() else {
        return;
    };
    let blasts = session.detonate_bonus_boosters_detailed(
        &balance,
        &mut ctx.rng,
        VICTORY_SKIP_MAX_DETONATIONS,
    );
    for blast in blasts {
        ctx.city.stats.record_clear(blast.stats);
    }
}

fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Smooth fall curve with a very gentle cushion overshoot at the end.
fn ease_out_back_soft(t: f32) -> f32 {
    let c1 = 0.85_f32;
    let c3 = c1 + 1.0;
    let u = t - 1.0;
    1.0 + c3 * u * u * u + c1 * u * u
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
            s.earned_stars_with(&balance.session),
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
            if ctx.tutorial_step == 1 {
                ctx.tutorial_step = 2;
            }
            if let Some(boss) = boss_def {
                ctx.city.record_boss_victory_with_rewards(&boss, &rewards);
                ctx.last_rewards = rewards;
            } else if building_id == wyrdstone_core::expedition::EXPEDITION_BUILDING_ID {
                let wave = ctx.active_expedition.as_ref().map(|r| r.wave).unwrap_or(1);
                ctx.city.record_expedition_wave(wave, &rewards, &balance);
                let ctx_ref = ctx;
                if let Some(run) = ctx_ref.active_expedition.as_mut() {
                    run.roll_boon_choices_with(&balance.expedition, &mut ctx_ref.rng);
                }
                ctx_ref.active_modal = ActiveModal::ExpeditionBoons;
                ctx_ref.persist();
                next_state.set(AppState::FjordCity);
                return;
            } else if building_id == wyrdstone_core::level::DAILY_TRIAL_BUILDING_ID {
                let streak = ctx.city.record_daily_victory(
                    crate::state::today_day_index(),
                    &rewards,
                    &balance,
                );
                ctx.last_rewards =
                    wyrdstone_core::CityState::scale_daily_rewards_with(&rewards, streak, &balance);
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
            if building_id == wyrdstone_core::expedition::EXPEDITION_BUILDING_ID {
                ctx.active_expedition = None;
            }
            ctx.city.reset_win_streak();
            ctx.last_discovered_relic_key = None;
        }
        ctx.active_modal = ActiveModal::None;
        ctx.persist();
        next_state.set(AppState::LevelResult);
    }
}

#[allow(clippy::type_complexity)]
type PulseBorderQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut BorderColor,
        Option<&'static FuryReadyPulse>,
        Option<&'static LowMovesPulse>,
    ),
    Or<(With<FuryReadyPulse>, With<LowMovesPulse>)>,
>;

/// Moves and fades out burst particles, flying orbs, VFX beams, sanctuary ambience, shimmers, popups, camera shake, and window resize.
#[allow(clippy::too_many_arguments)]
pub fn animate_particles(
    mut commands: Commands,
    time: Res<Time>,
    vfx: Res<VfxTextures>,
    mut input_state: ResMut<Match3InputState>,
    mut resize_events: EventReader<WindowResized>,
    mut backdrops: BackdropQuery,
    mut particles: ParticleQuery,
    mut beams: BeamQuery,
    mut orbs: FlyingOrbQuery,
    mut shimmers: ShimmerQuery,
    mut popups: PopupQuery,
    mut boosters: BoosterPulseQuery,
    mut torches: TorchGlowQuery,
    mut embers: SanctuaryEmberQuery,
    mut pulse_borders: PulseBorderQuery,
    mut cameras: Query<&mut Transform, With<Camera2d>>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    for ev in resize_events.read() {
        let bg_w = ev.width.max(ev.height * (16.0 / 9.0));
        let bg_h = ev.height.max(ev.width * (9.0 / 16.0));
        for mut bg in &mut backdrops {
            bg.custom_size = Some(Vec2::new(bg_w, bg_h));
        }
        for (torch, mut tf, _) in &mut torches {
            tf.translation.x = torch.norm_x * bg_w;
            tf.translation.y = torch.norm_y * bg_h;
        }
        let ember_half_w = ev.width * SANCTUARY_EMBER_HALF_W_RATIO;
        let ember_half_h = ev.height * SANCTUARY_EMBER_HALF_H_RATIO;
        for (mut ember, mut tf, _) in &mut embers {
            ember.half_w = ember_half_w;
            ember.half_h = ember_half_h;
            ember.base_x = ember.lane * ember_half_w;
            ember.y = ember.y.clamp(-ember_half_h, ember_half_h);
            tf.translation.x = ember.base_x;
            tf.translation.y = ember.y;
        }
        let now_compact = ev.width < 760.0;
        if now_compact != input_state.is_compact_hud {
            input_state.is_compact_hud = now_compact;
            input_state.dirty_hud = true;
        }
        input_state.dirty_static_board = true;
        input_state.dirty_board = true;
    }

    if !input_state.reduced_motion {
        for (torch, _, mut sprite) in &mut torches {
            let flicker = (elapsed * 7.3 + torch.phase).sin() * 0.5
                + (elapsed * 13.1 + torch.phase * 1.7).cos() * 0.35
                + (elapsed * 21.7 + torch.phase * 0.6).sin() * 0.15;
            let alpha = (0.30 + flicker * 0.08).clamp(0.18, 0.44);
            let size = torch.base_size * (1.0 + flicker * 0.09);
            sprite.color = Color::srgba(1.0, 0.60 + flicker * 0.06, 0.20, alpha);
            sprite.custom_size = Some(Vec2::splat(size));
        }

        for (mut ember, mut tf, mut sprite) in &mut embers {
            ember.y += ember.speed_y * dt;
            if ember.y > ember.half_h {
                ember.y = -ember.half_h;
            }
            let drift = (elapsed * 1.2 + ember.phase).sin() * 22.0;
            tf.translation.x = ember.base_x + drift;
            tf.translation.y = ember.y;
            let norm_y = ((ember.y + ember.half_h) / (ember.half_h * 2.0)).clamp(0.0, 1.0);
            let fade = (norm_y * std::f32::consts::PI).sin().clamp(0.0, 1.0);
            sprite.color = sprite.color.with_alpha(fade * 0.42);
        }
    }

    for (entity, mut p, mut tf, mut sprite) in &mut particles {
        p.elapsed += dt;
        if p.elapsed >= p.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = p.elapsed / p.lifetime;
        p.velocity.y -= PARTICLE_GRAVITY_PX * dt;
        tf.translation.x += p.velocity.x * dt * (1.0 - progress * 0.45);
        tf.translation.y += p.velocity.y * dt;
        tf.rotate_z(p.spin * dt);
        let scale = if progress < 0.2 {
            0.7 + (progress / 0.2) * 0.45
        } else {
            (1.15 * (1.0 - (progress - 0.2) / 0.8)).max(0.08)
        };
        tf.scale = Vec3::splat(scale);
        sprite.color = sprite.color.with_alpha((1.0 - progress).powf(1.2));
    }

    for (entity, mut b, mut tf, mut sprite) in &mut beams {
        b.elapsed += dt;
        if b.elapsed >= b.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = b.elapsed / b.lifetime;
        let eased = ease_out_quad(progress);
        let scale = b.start_scale + (b.end_scale - b.start_scale) * eased;
        tf.scale = Vec3::splat(scale);
        if b.rotation_speed.abs() > f32::EPSILON {
            tf.rotate_z(b.rotation_speed * dt);
        }
        sprite.color = sprite.color.with_alpha((1.0 - progress).powf(1.5));
    }

    for (entity, mut orb, mut tf, mut sprite) in &mut orbs {
        orb.elapsed += dt;
        if orb.elapsed >= orb.duration {
            // Crisp mini shockwave impact where the orb lands on the HUD/Boss bar.
            spawn_shockwave_ring(
                &mut commands,
                &vfx,
                orb.target,
                Vec2::splat(48.0),
                orb.color.with_alpha(0.85),
                0.24,
            );
            commands.entity(entity).despawn();
            continue;
        }
        let u = (orb.elapsed / orb.duration).clamp(0.0, 1.0);
        let t = u * u * (3.0 - 2.0 * u);
        let inv = 1.0 - t;
        let pos = orb.start * (inv * inv) + orb.control * (2.0 * inv * t) + orb.target * (t * t);
        tf.translation.x = pos.x;
        tf.translation.y = pos.y;
        tf.rotate_z(7.5 * dt);
        let size_curve = 0.72 + (u * std::f32::consts::PI).sin() * 0.55;
        tf.scale = Vec3::splat(size_curve);
        sprite.color = orb.color.with_alpha((1.0 - u * 0.25).clamp(0.2, 1.0));
    }

    for (shimmer, mut tf, mut sprite) in &mut shimmers {
        if input_state.reduced_motion {
            sprite.color = Color::srgba(1.0, 0.96, 0.80, 0.0);
            continue;
        }
        // Periodic diagonal gleam sweeping over boosters every ~2.6 seconds.
        let cycle = (elapsed * 0.42 + shimmer.phase).fract();
        if cycle < 0.28 {
            let p = cycle / 0.28;
            let offset = (p - 0.5) * 18.0;
            tf.translation.x = offset;
            tf.translation.y = offset * 0.7;
            let alpha = (p * std::f32::consts::PI).sin() * 0.68;
            sprite.color = Color::srgba(1.0, 0.96, 0.80, alpha);
        } else {
            sprite.color = Color::srgba(1.0, 0.96, 0.80, 0.0);
        }
    }

    for (entity, mut f, mut tf, mut col) in &mut popups {
        f.elapsed += dt;
        if f.elapsed >= f.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = f.elapsed / f.lifetime;
        tf.translation.y += FLOATING_TEXT_RISE_SPEED_PX * dt * (1.0 - progress * 0.4);
        let pop_scale = if progress < 0.18 {
            0.65 + (progress / 0.18) * 0.50
        } else if progress < 0.34 {
            1.15 - ((progress - 0.18) / 0.16) * 0.15
        } else {
            1.0
        };
        tf.scale = Vec3::splat(pop_scale);
        let alpha = if progress < 0.65 {
            1.0
        } else {
            (1.0 - (progress - 0.65) / 0.35).max(0.0)
        };
        col.0 = col.0.with_alpha(alpha);
    }

    let fury_glow = if input_state.reduced_motion {
        0.65
    } else {
        0.5 + 0.5 * (elapsed * 5.2).sin()
    };
    let fury_border_col = Color::srgb(
        0.96 + 0.04 * fury_glow,
        0.74 + 0.24 * fury_glow,
        0.24 + 0.52 * fury_glow,
    );
    let low_moves_glow = if input_state.reduced_motion {
        0.65
    } else {
        0.5 + 0.5 * (elapsed * 7.5).sin()
    };
    let low_moves_col = Color::srgb(
        0.80 + 0.20 * low_moves_glow,
        0.16 + 0.22 * low_moves_glow,
        0.18 + 0.16 * low_moves_glow,
    );
    for (mut border, is_fury, is_low_moves) in &mut pulse_borders {
        if is_fury.is_some() {
            border.0 = fury_border_col;
        } else if is_low_moves.is_some() {
            border.0 = low_moves_col;
        }
    }

    if input_state.is_idle() {
        let pulse = if input_state.reduced_motion {
            1.0
        } else {
            1.0 + 0.08 * (elapsed * 4.2).sin()
        };
        for mut tf in &mut boosters {
            tf.scale = Vec3::splat(pulse);
        }
    }

    if input_state.shake_secs > 0.0 && !input_state.reduced_motion {
        input_state.shake_secs = (input_state.shake_secs - dt).max(0.0);
        let norm = (input_state.shake_secs / SHAKE_DURATION_SECS).clamp(0.0, 1.5);
        let intensity = norm * norm * SHAKE_AMPLITUDE_PX;
        let phase = elapsed * 62.0;
        for mut cam in &mut cameras {
            cam.translation.x = phase.sin() * intensity;
            cam.translation.y = (phase * 1.37).cos() * intensity;
        }
    } else {
        input_state.shake_secs = 0.0;
        for mut cam in &mut cameras {
            cam.translation.x = 0.0;
            cam.translation.y = 0.0;
        }
    }
}
