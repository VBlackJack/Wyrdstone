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

//! Match-3 2D board rendering, stepped animations, particles, HUD, and mouse + Android touch input.

mod anim;
mod hud;

pub use anim::{animate_particles, tick_board_animation};

use crate::audio::{play_sfx, SfxKind, SoundBank};
use crate::fjord_art::FjordTextures;
use crate::runes_art::RuneTextures;
use crate::state::{ActiveModal, AppState, GameContext};
use crate::theme::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::{HashMap, HashSet};
use wyrdstone_core::{ActiveTool, Booster, CellPos, PreparedClear};

pub(super) const PARTICLES_PER_CLEARED_TILE: usize = 6;
pub(super) const PARTICLE_LIFETIME_SECS: f32 = 0.34;
pub(super) const PARTICLE_SPEED_PX: f32 = 155.0;
pub(super) const PARTICLE_SIZE_PX: f32 = 7.0;
pub(super) const HINT_IDLE_SECS: f32 = 5.0;
pub(super) const FLOATING_TEXT_LIFETIME_SECS: f32 = 0.85;
pub(super) const FLOATING_TEXT_RISE_SPEED_PX: f32 = 58.0;
pub(super) const SHAKE_DURATION_SECS: f32 = 0.18;
pub(super) const SHAKE_AMPLITUDE_PX: f32 = 6.5;

/// Unified cleanup marker for all entities belonging to the Match-3 scene.
#[derive(Component)]
pub struct Match3Cleanup;

#[derive(Component)]
pub struct Match3Entity;

#[derive(Component)]
pub struct Match3Backdrop;

#[derive(Component)]
pub struct Match3HudRoot;

/// Marker on the HUD countdown text so timer ticks update in-place without rebuilding the HUD.
#[derive(Component)]
pub struct Match3TimerText;

/// Marks a movable rune/booster sprite bound to a logical grid cell.
#[derive(Component)]
pub struct BoardTileVisual {
    pub pos: CellPos,
}

#[derive(Component)]
pub struct SelectionMarker;

/// Lightweight burst particle spawned when runes clear.
#[derive(Component)]
pub struct BurstParticle {
    pub velocity: Vec2,
    pub elapsed: f32,
    pub lifetime: f32,
}

/// Floating "+score" or "Combo xN!" text rising above cleared matches.
#[derive(Component)]
pub struct FloatingScore {
    pub elapsed: f32,
    pub lifetime: f32,
}

/// Expanding lightning beam or fiery shockwave VFX for boosters and active tools.
#[derive(Component)]
pub struct BlastBeam {
    pub elapsed: f32,
    pub lifetime: f32,
}

/// Gently pulses placed board boosters so they stand out crisply.
#[derive(Component)]
pub struct BoosterPulse;

#[derive(Component)]
pub enum Match3UiAction {
    Retreat,
    ToggleHelp,
    SelectTool(ActiveTool),
}

/// Stepped animation phase machine so every swap, match, clear, fall, and cascade is readable.
#[derive(Default)]
pub enum BoardAnimPhase {
    #[default]
    Idle,
    Swapping {
        a: CellPos,
        b: CellPos,
        elapsed: f32,
        duration: f32,
    },
    SwapRevert {
        a: CellPos,
        b: CellPos,
        elapsed: f32,
        duration: f32,
    },
    Clearing {
        prepared: PreparedClear,
        cleared_set: HashSet<CellPos>,
        combo: u32,
        elapsed: f32,
        duration: f32,
        particles_spawned: bool,
    },
    Falling {
        moves: HashMap<CellPos, i32>,
        combo: u32,
        elapsed: f32,
        duration: f32,
    },
    VictoryCelebration {
        elapsed: f32,
        duration: f32,
        next_burst_at: f32,
        burst_index: u32,
    },
}

/// Tracks board layout metrics, pointer/touch gesture state, hints, shake, and animation phase.
#[derive(Resource, Default)]
pub struct Match3InputState {
    pub selected: Option<CellPos>,
    pub drag_origin_cell: Option<CellPos>,
    pub drag_origin_screen: Option<Vec2>,
    pub active_touch_id: Option<u64>,
    pub tile_size: f32,
    pub cell_step: f32,
    pub board_origin: Vec2,
    pub dirty_board: bool,
    pub dirty_hud: bool,
    pub idle_secs: f32,
    pub hint_pair: Option<(CellPos, CellPos)>,
    pub shake_secs: f32,
    pub phase: BoardAnimPhase,
}

impl Match3InputState {
    pub fn is_idle(&self) -> bool {
        matches!(self.phase, BoardAnimPhase::Idle)
    }
}

/// Initializes the Match-3 scene (painted sanctuary backdrop + 2D sprites + HUD overlay).
pub fn setup_match3_scene(
    mut commands: Commands,
    ctx: Res<GameContext>,
    textures: Res<RuneTextures>,
    fjord_art: Res<FjordTextures>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let mut input_state = Match3InputState::default();

    if let Ok(window) = windows.get_single() {
        let bg_w = window.width().max(window.height() * (16.0 / 9.0));
        let bg_h = window.height().max(window.width() * (9.0 / 16.0));
        commands.spawn((
            Match3Cleanup,
            Match3Backdrop,
            Sprite {
                image: fjord_art.match3_bg.clone(),
                custom_size: Some(Vec2::new(bg_w, bg_h)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, Z_BOARD_BG - 2.0),
        ));

        if let Some(session) = &ctx.active_session {
            compute_board_layout(
                window.width(),
                window.height(),
                session.board.cols(),
                session.board.rows(),
                &mut input_state,
            );
            spawn_board_sprites(&mut commands, session, &textures, &input_state, None);
        }
    }

    hud::spawn_match3_hud(&mut commands, &ctx, &fjord_art);
    commands.insert_resource(input_state);
}

/// Cleans up all Match-3 entities and resets camera offset on exit.
pub fn cleanup_match3_scene(
    mut commands: Commands,
    cleanup_q: Query<Entity, With<Match3Cleanup>>,
    mut cameras: Query<&mut Transform, With<Camera2d>>,
) {
    for entity in &cleanup_q {
        commands.entity(entity).despawn_recursive();
    }
    for mut cam in &mut cameras {
        cam.translation.x = 0.0;
        cam.translation.y = 0.0;
    }
    commands.remove_resource::<Match3InputState>();
}

pub(super) fn compute_board_layout(
    win_w: f32,
    win_h: f32,
    cols: usize,
    rows: usize,
    layout: &mut Match3InputState,
) {
    let avail_w = (win_w - BOARD_SIDE_MARGIN_PX * 2.0).max(200.0);
    let avail_h = (win_h - BOARD_TOP_RESERVED_PX - BOARD_BOTTOM_RESERVED_PX).max(200.0);
    let step = (avail_w / cols as f32)
        .min(avail_h / rows as f32)
        .clamp(MIN_TILE_SIZE_PX, MAX_TILE_SIZE_PX);
    layout.cell_step = step;
    layout.tile_size = step * (1.0 - TILE_GAP_RATIO);

    let board_w = step * cols as f32;
    let board_h = step * rows as f32;
    let center_y_offset = (BOARD_BOTTOM_RESERVED_PX - BOARD_TOP_RESERVED_PX) * 0.5;
    layout.board_origin = Vec2::new(
        -board_w * 0.5 + step * 0.5,
        board_h * 0.5 - step * 0.5 + center_y_offset,
    );
}

pub(super) fn grid_to_world_f(col: f32, row: f32, layout: &Match3InputState) -> Vec2 {
    Vec2::new(
        layout.board_origin.x + col * layout.cell_step,
        layout.board_origin.y - row * layout.cell_step,
    )
}

pub(super) fn cell_to_world(pos: CellPos, layout: &Match3InputState) -> Vec2 {
    grid_to_world_f(pos.col as f32, pos.row as f32, layout)
}

fn world_to_cell(
    world: Vec2,
    cols: usize,
    rows: usize,
    layout: &Match3InputState,
) -> Option<CellPos> {
    let col_f = ((world.x - layout.board_origin.x) / layout.cell_step).round();
    let row_f = ((layout.board_origin.y - world.y) / layout.cell_step).round();
    if col_f < 0.0 || row_f < 0.0 {
        return None;
    }
    let col = col_f as usize;
    let row = row_f as usize;
    if col >= cols || row >= rows {
        return None;
    }
    let center = cell_to_world(CellPos::new(col, row), layout);
    if (world - center).abs().max_element() <= layout.cell_step * 0.5 {
        Some(CellPos::new(col, row))
    } else {
        None
    }
}

pub(super) fn spawn_board_sprites(
    commands: &mut Commands,
    session: &wyrdstone_core::LevelSession,
    textures: &RuneTextures,
    layout: &Match3InputState,
    fall_starts: Option<&HashMap<CellPos, i32>>,
) {
    let cols = session.board.cols();
    let rows = session.board.rows();
    let board_w = cols as f32 * layout.cell_step + 16.0;
    let board_h = rows as f32 * layout.cell_step + 16.0;
    let center_y = (BOARD_BOTTOM_RESERVED_PX - BOARD_TOP_RESERVED_PX) * 0.5;

    commands.spawn((
        Match3Cleanup,
        Match3Entity,
        Sprite {
            color: Color::srgba(0.06, 0.09, 0.14, 0.84),
            custom_size: Some(Vec2::new(board_w, board_h)),
            ..default()
        },
        Transform::from_xyz(0.0, center_y, Z_BOARD_BG),
    ));

    for row in 0..rows {
        for col in 0..cols {
            let pos = CellPos::new(col, row);
            let Some(cell) = session.board.get(pos) else {
                continue;
            };
            if cell.blocked {
                continue;
            }
            let world = cell_to_world(pos, layout);
            let tile_world = match fall_starts.and_then(|m| m.get(&pos)) {
                Some(&from_row) => grid_to_world_f(col as f32, from_row as f32, layout),
                None => world,
            };
            let cell_size = Vec2::splat(layout.cell_step * 0.96);
            let tile_size = Vec2::splat(layout.tile_size);

            commands.spawn((
                Match3Cleanup,
                Match3Entity,
                Sprite {
                    image: textures.cell_bg.clone(),
                    custom_size: Some(cell_size),
                    ..default()
                },
                Transform::from_xyz(world.x, world.y, Z_CELL_BG),
            ));

            if cell.ice_layers > 0 {
                let ice_over = if cell.ice_layers >= 2 {
                    textures.ice_top_layer2.clone()
                } else {
                    textures.ice_top_layer1.clone()
                };
                commands.spawn((
                    Match3Cleanup,
                    Match3Entity,
                    Sprite {
                        image: ice_over,
                        custom_size: Some(Vec2::splat(layout.cell_step * 0.98)),
                        ..default()
                    },
                    Transform::from_xyz(world.x, world.y, Z_ICE_TOP_FROST),
                ));
            }

            if cell.crate_layers > 0 {
                let crate_tex = if cell.crate_layers >= 2 {
                    textures.crate_layer2.clone()
                } else {
                    textures.crate_layer1.clone()
                };
                commands.spawn((
                    Match3Cleanup,
                    Match3Entity,
                    BoardTileVisual { pos },
                    Sprite {
                        image: crate_tex,
                        custom_size: Some(tile_size),
                        ..default()
                    },
                    Transform::from_xyz(world.x, world.y, Z_RUNE_SPRITE),
                ));
            } else if cell.relic {
                commands.spawn((
                    Match3Cleanup,
                    Match3Entity,
                    BoardTileVisual { pos },
                    Sprite {
                        image: textures.relic_icon.clone(),
                        custom_size: Some(tile_size),
                        ..default()
                    },
                    Transform::from_xyz(tile_world.x, tile_world.y, Z_RUNE_SPRITE),
                ));
            } else if cell.booster == Booster::BifrostShard {
                commands.spawn((
                    Match3Cleanup,
                    Match3Entity,
                    BoardTileVisual { pos },
                    BoosterPulse,
                    Sprite {
                        image: textures.booster_bifrost.clone(),
                        custom_size: Some(tile_size),
                        ..default()
                    },
                    Transform::from_xyz(tile_world.x, tile_world.y, Z_RUNE_SPRITE),
                ));
            } else if let Some(rune) = cell.rune {
                if let Some(tex) = textures.runes.get(&rune) {
                    commands.spawn((
                        Match3Cleanup,
                        Match3Entity,
                        BoardTileVisual { pos },
                        Sprite {
                            image: tex.clone(),
                            custom_size: Some(tile_size),
                            ..default()
                        },
                        Transform::from_xyz(tile_world.x, tile_world.y, Z_RUNE_SPRITE),
                    ));
                }
                if let Some(booster_tex) = textures.booster_handle(cell.booster) {
                    commands.spawn((
                        Match3Cleanup,
                        Match3Entity,
                        BoardTileVisual { pos },
                        BoosterPulse,
                        Sprite {
                            image: booster_tex,
                            custom_size: Some(tile_size),
                            ..default()
                        },
                        Transform::from_xyz(tile_world.x, tile_world.y, Z_BOOSTER_BADGE),
                    ));
                }
            }

            if cell.chained {
                commands.spawn((
                    Match3Cleanup,
                    Match3Entity,
                    Sprite {
                        image: textures.chain_overlay.clone(),
                        custom_size: Some(tile_size),
                        ..default()
                    },
                    Transform::from_xyz(world.x, world.y, Z_CHAIN_OVERLAY),
                ));
            }
        }
    }

    for &(src, dst) in session.board.portals() {
        for p in [src, dst] {
            let w = cell_to_world(p, layout);
            commands.spawn((
                Match3Cleanup,
                Match3Entity,
                Sprite {
                    image: textures.portal_ring.clone(),
                    custom_size: Some(Vec2::splat(layout.cell_step * 0.98)),
                    ..default()
                },
                Transform::from_xyz(w.x, w.y, Z_CELL_BG + 0.5),
            ));
        }
    }

    if let Some(sel) = layout.selected {
        let w = cell_to_world(sel, layout);
        commands.spawn((
            Match3Cleanup,
            Match3Entity,
            SelectionMarker,
            Sprite {
                image: textures.selection_ring.clone(),
                custom_size: Some(Vec2::splat(layout.cell_step)),
                ..default()
            },
            Transform::from_xyz(w.x, w.y, Z_SELECTION_RING),
        ));
    }
}

pub(super) fn rebuild_board_now(
    commands: &mut Commands,
    session: &wyrdstone_core::LevelSession,
    textures: &RuneTextures,
    window: &Window,
    input_state: &mut Match3InputState,
    sprites: &Query<Entity, With<Match3Entity>>,
    fall_starts: Option<&HashMap<CellPos, i32>>,
) {
    for e in sprites {
        commands.entity(e).despawn_recursive();
    }
    compute_board_layout(
        window.width(),
        window.height(),
        session.board.cols(),
        session.board.rows(),
        input_state,
    );
    spawn_board_sprites(commands, session, textures, input_state, fall_starts);
}

fn execute_loki_shuffle(
    commands: &mut Commands,
    ctx: &mut GameContext,
    sound_bank: &SoundBank,
    input_state: &mut Match3InputState,
) {
    let balance = ctx.data.balance.clone();
    if ctx.city.tools.consume(ActiveTool::LokiShuffle) {
        let (cols, rows) = ctx
            .active_session
            .as_ref()
            .map(|s| (s.board.cols(), s.board.rows()))
            .unwrap_or((8, 8));
        if let Some(session) = ctx.active_session.as_mut() {
            session.play_tool(ActiveTool::LokiShuffle, None, &balance, &mut ctx.rng);
        }
        ctx.armed_tool = None;
        ctx.persist();
        play_sfx(commands, sound_bank, SfxKind::ToolBlast);
        anim::spawn_loki_vfx(commands, cols, rows, input_state);
        input_state.shake_secs = SHAKE_DURATION_SECS * 0.7;
        input_state.dirty_board = true;
        input_state.dirty_hud = true;
    }
}

/// Handles HUD button interactions and keyboard shortcuts (`1-6` tools, `Escape` cancel/close).
pub fn handle_match3_hud_buttons(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut ctx: ResMut<GameContext>,
    sound_bank: Res<SoundBank>,
    mut input_state: ResMut<Match3InputState>,
    mut next_state: ResMut<NextState<AppState>>,
    buttons: Query<(&Interaction, &Match3UiAction), Changed<Interaction>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if ctx.active_modal == ActiveModal::Help {
            ctx.active_modal = ActiveModal::None;
            input_state.dirty_hud = true;
            return;
        }
        if ctx.armed_tool.is_some() {
            ctx.armed_tool = None;
            input_state.dirty_hud = true;
            return;
        }
    }

    let key_tools = [
        (KeyCode::Digit1, ActiveTool::Mjolnir),
        (KeyCode::Digit2, ActiveTool::SurtrEmber),
        (KeyCode::Digit3, ActiveTool::ThorBolt),
        (KeyCode::Digit4, ActiveTool::BifrostPrism),
        (KeyCode::Digit5, ActiveTool::DrakkarCharge),
        (KeyCode::Digit6, ActiveTool::LokiShuffle),
    ];
    for (kc, tool) in key_tools {
        if keys.just_pressed(kc) && input_state.is_idle() && ctx.active_modal != ActiveModal::Help {
            if ctx.city.tools.count(tool) > 0 {
                if tool == ActiveTool::LokiShuffle {
                    execute_loki_shuffle(&mut commands, &mut ctx, &sound_bank, &mut input_state);
                } else {
                    ctx.armed_tool = if ctx.armed_tool == Some(tool) {
                        None
                    } else {
                        Some(tool)
                    };
                    input_state.selected = None;
                    input_state.dirty_board = true;
                    input_state.dirty_hud = true;
                }
            }
            return;
        }
    }

    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            Match3UiAction::Retreat => {
                ctx.active_session = None;
                ctx.armed_tool = None;
                ctx.active_modal = ActiveModal::None;
                next_state.set(AppState::FjordCity);
                return;
            }
            Match3UiAction::ToggleHelp => {
                ctx.toggle_modal(ActiveModal::Help);
                input_state.dirty_hud = true;
            }
            Match3UiAction::SelectTool(tool) => {
                if !input_state.is_idle() || ctx.city.tools.count(tool) == 0 {
                    continue;
                }
                if tool == ActiveTool::LokiShuffle {
                    execute_loki_shuffle(&mut commands, &mut ctx, &sound_bank, &mut input_state);
                } else if ctx.armed_tool == Some(tool) {
                    ctx.armed_tool = None;
                    input_state.dirty_hud = true;
                } else {
                    ctx.armed_tool = Some(tool);
                    input_state.selected = None;
                    input_state.dirty_board = true;
                    input_state.dirty_hud = true;
                }
            }
        }
    }
}

/// Handles both desktop mouse (click + drag) and Android touch (tap + swipe) when the board is idle.
#[allow(clippy::too_many_arguments)]
pub fn handle_board_pointer_input(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut ctx: ResMut<GameContext>,
    sound_bank: Res<SoundBank>,
    mut input_state: ResMut<Match3InputState>,
) {
    if !input_state.is_idle() || ctx.active_modal == ActiveModal::Help {
        return;
    }
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, cam_transform)) = camera_q.get_single() else {
        return;
    };
    let (cols, rows) = match &ctx.active_session {
        Some(s) => (s.board.cols(), s.board.rows()),
        None => return,
    };

    let swipe_threshold = ctx.data.balance.swipe_threshold_px;
    let mut pressed_screen: Option<Vec2> = None;
    let mut current_screen: Option<Vec2> = None;
    let mut released = false;

    if let Some(touch) = touches.iter_just_pressed().next() {
        input_state.active_touch_id = Some(touch.id());
        pressed_screen = Some(touch.position());
    } else if let Some(tid) = input_state.active_touch_id {
        if let Some(touch) = touches.get_pressed(tid) {
            current_screen = Some(touch.position());
        } else {
            input_state.active_touch_id = None;
            released = true;
        }
    }

    if pressed_screen.is_none() && input_state.active_touch_id.is_none() {
        if mouse.just_pressed(MouseButton::Left) {
            pressed_screen = window.cursor_position();
        } else if mouse.pressed(MouseButton::Left) {
            current_screen = window.cursor_position();
        } else if mouse.just_released(MouseButton::Left) {
            released = true;
        }
    }

    if let Some(screen_pos) = pressed_screen {
        input_state.idle_secs = 0.0;
        input_state.hint_pair = None;
        if let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, screen_pos) {
            if let Some(cell) = world_to_cell(world_pos, cols, rows, &input_state) {
                if let Some(tool) = ctx.armed_tool {
                    let prepared = ctx
                        .active_session
                        .as_ref()
                        .and_then(|s| s.board.prepare_tool_step(tool, cell));
                    if let Some(prep) = prepared {
                        if ctx.city.tools.consume(tool) {
                            ctx.armed_tool = None;
                            ctx.persist();
                            play_sfx(&mut commands, &sound_bank, SfxKind::ToolBlast);
                            anim::spawn_tool_vfx(
                                &mut commands,
                                tool,
                                cell,
                                &prep.cells,
                                cols,
                                rows,
                                &input_state,
                            );
                            let clear_dur = ctx.data.balance.clear_anim_seconds;
                            let cleared_set: HashSet<CellPos> =
                                prep.cells.iter().copied().collect();
                            input_state.selected = None;
                            input_state.shake_secs = SHAKE_DURATION_SECS;
                            input_state.dirty_hud = true;
                            input_state.phase = BoardAnimPhase::Clearing {
                                prepared: prep,
                                cleared_set,
                                combo: 1,
                                elapsed: 0.0,
                                duration: clear_dur,
                                particles_spawned: false,
                            };
                        }
                    } else {
                        play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    }
                    return;
                }

                if let Some(prev) = input_state.selected {
                    if prev == cell {
                        input_state.selected = None;
                        input_state.dirty_board = true;
                        return;
                    }
                    if prev.is_adjacent(cell) {
                        if start_swap_animation(prev, cell, &ctx, &mut input_state) {
                            play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                        } else {
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                        }
                        return;
                    }
                }

                input_state.selected = Some(cell);
                input_state.drag_origin_cell = Some(cell);
                input_state.drag_origin_screen = Some(screen_pos);
                input_state.dirty_board = true;
            }
        }
    }

    if let (Some(curr), Some(origin_cell), Some(origin_screen)) = (
        current_screen,
        input_state.drag_origin_cell,
        input_state.drag_origin_screen,
    ) {
        let delta = curr - origin_screen;
        if delta.length() >= swipe_threshold {
            let target = if delta.x.abs() >= delta.y.abs() {
                if delta.x > 0.0 && origin_cell.col + 1 < cols {
                    Some(CellPos::new(origin_cell.col + 1, origin_cell.row))
                } else if delta.x < 0.0 && origin_cell.col > 0 {
                    Some(CellPos::new(origin_cell.col - 1, origin_cell.row))
                } else {
                    None
                }
            } else if delta.y > 0.0 && origin_cell.row + 1 < rows {
                Some(CellPos::new(origin_cell.col, origin_cell.row + 1))
            } else if delta.y < 0.0 && origin_cell.row > 0 {
                Some(CellPos::new(origin_cell.col, origin_cell.row - 1))
            } else {
                None
            };

            input_state.drag_origin_cell = None;
            input_state.drag_origin_screen = None;
            if let Some(nb) = target {
                if start_swap_animation(origin_cell, nb, &ctx, &mut input_state) {
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                } else {
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                }
            }
        }
    }

    if released {
        input_state.drag_origin_cell = None;
        input_state.drag_origin_screen = None;
    }
}

fn start_swap_animation(
    a: CellPos,
    b: CellPos,
    ctx: &GameContext,
    input_state: &mut Match3InputState,
) -> bool {
    let can_swap = ctx
        .active_session
        .as_ref()
        .map(|s| s.board.can_initiate_swap(a, b))
        .unwrap_or(false);
    input_state.selected = None;
    input_state.drag_origin_cell = None;
    input_state.drag_origin_screen = None;
    if !can_swap {
        input_state.dirty_board = true;
        return false;
    }
    input_state.phase = BoardAnimPhase::Swapping {
        a,
        b,
        elapsed: 0.0,
        duration: ctx.data.balance.swap_anim_seconds,
    };
    true
}

/// Rebuilds board sprites and/or HUD when dirty flags are raised.
#[allow(clippy::too_many_arguments)]
pub fn sync_match3_visuals(
    mut commands: Commands,
    ctx: Res<GameContext>,
    textures: Res<RuneTextures>,
    fjord_art: Res<FjordTextures>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut input_state: ResMut<Match3InputState>,
    sprites: Query<Entity, With<Match3Entity>>,
    huds: Query<Entity, With<Match3HudRoot>>,
) {
    if input_state.dirty_board {
        if let (Some(session), Ok(window)) = (&ctx.active_session, windows.get_single()) {
            rebuild_board_now(
                &mut commands,
                session,
                &textures,
                window,
                &mut input_state,
                &sprites,
                None,
            );
        }
        input_state.dirty_board = false;
    }

    if input_state.dirty_hud {
        for e in &huds {
            commands.entity(e).despawn_recursive();
        }
        hud::spawn_match3_hud(&mut commands, &ctx, &fjord_art);
        input_state.dirty_hud = false;
    }
}
