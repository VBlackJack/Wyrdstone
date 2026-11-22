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

use crate::audio::{play_sfx, SfxKind, SoundBank};
use crate::fjord_art::FjordTextures;
use crate::runes_art::RuneTextures;
use crate::state::{AppState, GameContext};
use crate::theme::*;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use std::collections::{HashMap, HashSet};
use wyrdstone_core::{
    ActiveTool, Booster, CellPos, FallMove, LevelOutcome, Objective, PreparedClear,
};

const PARTICLES_PER_CLEARED_TILE: usize = 6;
const PARTICLE_LIFETIME_SECS: f32 = 0.34;
const PARTICLE_SPEED_PX: f32 = 155.0;
const PARTICLE_SIZE_PX: f32 = 7.0;
const HINT_IDLE_SECS: f32 = 5.0;
const FLOATING_TEXT_LIFETIME_SECS: f32 = 0.85;
const FLOATING_TEXT_RISE_SPEED_PX: f32 = 58.0;
const SHAKE_DURATION_SECS: f32 = 0.18;
const SHAKE_AMPLITUDE_PX: f32 = 6.5;

#[derive(Component)]
pub struct Match3Entity;

#[derive(Component)]
pub struct Match3Backdrop;

#[derive(Component)]
pub struct Match3HudRoot;

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
        // Painted Norse sanctuary backdrop scaled to cover the viewport.
        let bg_w = window.width().max(window.height() * (16.0 / 9.0));
        let bg_h = window.height().max(window.width() * (9.0 / 16.0));
        commands.spawn((
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

    spawn_match3_hud(&mut commands, &ctx);
    commands.insert_resource(input_state);
}

/// Cleans up all Match-3 sprites, backdrop, particles, floating text, HUD, and camera offset on exit.
pub fn cleanup_match3_scene(
    mut commands: Commands,
    sprites: Query<Entity, With<Match3Entity>>,
    backdrops: Query<Entity, With<Match3Backdrop>>,
    huds: Query<Entity, With<Match3HudRoot>>,
    particles: Query<Entity, With<BurstParticle>>,
    popups: Query<Entity, With<FloatingScore>>,
    mut cameras: Query<&mut Transform, With<Camera2d>>,
) {
    for e in &sprites {
        commands.entity(e).despawn_recursive();
    }
    for e in &backdrops {
        commands.entity(e).despawn_recursive();
    }
    for e in &huds {
        commands.entity(e).despawn_recursive();
    }
    for e in &particles {
        commands.entity(e).despawn_recursive();
    }
    for e in &popups {
        commands.entity(e).despawn_recursive();
    }
    for mut cam in &mut cameras {
        cam.translation.x = 0.0;
        cam.translation.y = 0.0;
    }
    commands.remove_resource::<Match3InputState>();
}

fn compute_board_layout(
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

fn grid_to_world_f(col: f32, row: f32, layout: &Match3InputState) -> Vec2 {
    Vec2::new(
        layout.board_origin.x + col * layout.cell_step,
        layout.board_origin.y - row * layout.cell_step,
    )
}

fn cell_to_world(pos: CellPos, layout: &Match3InputState) -> Vec2 {
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

fn spawn_board_sprites(
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

    // Board frame backdrop (semi-transparent dark stone so sanctuary art breathes around it)
    commands.spawn((
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

            // Cell background tile
            commands.spawn((
                Match3Entity,
                Sprite {
                    image: textures.cell_bg.clone(),
                    custom_size: Some(cell_size),
                    ..default()
                },
                Transform::from_xyz(world.x, world.y, Z_CELL_BG),
            ));

            // Ice plate layer
            if cell.ice_layers > 0 {
                let ice_tex = if cell.ice_layers >= 2 {
                    textures.ice_layer2.clone()
                } else {
                    textures.ice_layer1.clone()
                };
                commands.spawn((
                    Match3Entity,
                    Sprite {
                        image: ice_tex,
                        custom_size: Some(cell_size),
                        ..default()
                    },
                    Transform::from_xyz(world.x, world.y, Z_ICE_OVERLAY),
                ));
            }

            // Frost crate obstacle OR Buried Relic OR Rune sprite (or Bifrost orb)
            if cell.crate_layers > 0 {
                let crate_tex = if cell.crate_layers >= 2 {
                    textures.crate_layer2.clone()
                } else {
                    textures.crate_layer1.clone()
                };
                commands.spawn((
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
                    Match3Entity,
                    BoardTileVisual { pos },
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
                        Match3Entity,
                        BoardTileVisual { pos },
                        Sprite {
                            image: booster_tex,
                            custom_size: Some(tile_size),
                            ..default()
                        },
                        Transform::from_xyz(tile_world.x, tile_world.y, Z_BOOSTER_BADGE),
                    ));
                }
            }

            // Fenrir chain overlay
            if cell.chained {
                commands.spawn((
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

    // Selection ring if a cell is selected
    if let Some(sel) = layout.selected {
        let w = cell_to_world(sel, layout);
        commands.spawn((
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

fn rebuild_board_now(
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

fn spawn_clear_particles(
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

fn spawn_match3_hud(commands: &mut Commands, ctx: &GameContext) {
    let i18n = &ctx.data.i18n;
    let Some(session) = &ctx.active_session else {
        return;
    };

    commands
        .spawn((
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
                            Text::new(i18n.t_fmt(
                                "match3.hud.timer",
                                &[("secs", whole.to_string())],
                            )),
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
                        Text::new(i18n.t_fmt(
                            "match3.hud.score",
                            &[("score", session.score.to_string())],
                        )),
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
                    ..default()
                })
                .with_children(|objs| {
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
                });

                top.spawn(Node {
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|right| {
                    if session.last_shuffled {
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
                        ..default()
                    })
                    .with_children(|row| {
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
                            row.spawn((
                                Button,
                                Match3UiAction::SelectTool(tool),
                                Node {
                                    padding: UiRect::axes(Val::Px(14.0), Val::Px(9.0)),
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
                                btn.spawn((
                                    Text::new(i18n.t_fmt(key, &[("count", count.to_string())])),
                                    TextFont {
                                        font_size: FONT_BODY,
                                        ..default()
                                    },
                                    TextColor(text_col),
                                ));
                            });
                        }
                    });
            });

            // Optional Wyrdstone Guide overlay inside Match-3
            if ctx.help_modal_open {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.80)),
                ))
                .with_children(|bg| {
                    bg.spawn((
                        Node {
                            width: Val::Px(540.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(22.0)),
                            row_gap: Val::Px(10.0),
                            border: UiRect::all(Val::Px(2.0)),
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

                        for (t_key, b_key) in [
                            ("city.panel.help.boosters_title", "city.panel.help.boosters_body"),
                            ("city.panel.help.obstacles_title", "city.panel.help.obstacles_body"),
                            ("city.panel.help.village_title", "city.panel.help.village_body"),
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

/// Handles HUD button interactions (Retreat and Active Tools).
pub fn handle_match3_hud_buttons(
    mut ctx: ResMut<GameContext>,
    mut input_state: ResMut<Match3InputState>,
    mut next_state: ResMut<NextState<AppState>>,
    buttons: Query<(&Interaction, &Match3UiAction), Changed<Interaction>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            Match3UiAction::Retreat => {
                ctx.active_session = None;
                ctx.armed_tool = None;
                ctx.help_modal_open = false;
                next_state.set(AppState::FjordCity);
                return;
            }
            Match3UiAction::ToggleHelp => {
                ctx.help_modal_open = !ctx.help_modal_open;
                input_state.dirty_hud = true;
            }
            Match3UiAction::SelectTool(tool) => {
                if !input_state.is_idle() || ctx.city.tools.count(tool) == 0 {
                    continue;
                }
                if tool == ActiveTool::LokiShuffle {
                    let balance = ctx.data.balance.clone();
                    if ctx.city.tools.consume(ActiveTool::LokiShuffle) {
                        let ctx_ref = ctx.as_mut();
                        if let Some(session) = ctx_ref.active_session.as_mut() {
                            session.play_tool(
                                ActiveTool::LokiShuffle,
                                None,
                                &balance,
                                &mut ctx_ref.rng,
                            );
                        }
                        ctx.armed_tool = None;
                        ctx.persist();
                        input_state.dirty_board = true;
                        input_state.dirty_hud = true;
                    }
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
    if !input_state.is_idle() || ctx.help_modal_open {
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
                // Armed active tool triggers an animated clear phase directly.
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
                            let clear_dur = ctx.data.balance.clear_anim_seconds;
                            input_state.selected = None;
                            input_state.shake_secs = SHAKE_DURATION_SECS;
                            input_state.dirty_hud = true;
                            input_state.phase = BoardAnimPhase::Clearing {
                                prepared: prep,
                                combo: 1,
                                elapsed: 0.0,
                                duration: clear_dur,
                                particles_spawned: false,
                            };
                        }
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
                        play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                        start_swap_animation(prev, cell, &ctx, &mut input_state);
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
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                start_swap_animation(origin_cell, nb, &ctx, &mut input_state);
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
) {
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
        return;
    }
    input_state.phase = BoardAnimPhase::Swapping {
        a,
        b,
        elapsed: 0.0,
        duration: ctx.data.balance.swap_anim_seconds,
    };
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
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let dt = time.delta_secs();

    // Tick optional level countdown timer.
    let timer_changed = ctx
        .active_session
        .as_mut()
        .map(|s| s.tick_timer(dt))
        .unwrap_or(false);
    if timer_changed {
        input_state.dirty_hud = true;
        check_level_finished(&mut ctx, &mut next_state);
    }

    // Take ownership of phase to transition cleanly without borrow conflicts.
    let phase = std::mem::take(&mut input_state.phase);
    match phase {
        BoardAnimPhase::Idle => {
            // Auto-hint after 5 seconds of inactivity so the player never feels stuck.
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
                    input_state.dirty_hud = true;
                    input_state.phase = BoardAnimPhase::Clearing {
                        prepared: prep,
                        combo: 1,
                        elapsed: 0.0,
                        duration: ctx.data.balance.clear_anim_seconds,
                        particles_spawned: false,
                    };
                } else {
                    // Invalid swap: smoothly bounce both tiles back.
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
            let cleared_set: HashSet<CellPos> = prepared.cells.iter().copied().collect();

            // Pulse up briefly (0..0.35), then shrink & fade cleanly (0.35..1.0).
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
                let mut fall_map = HashMap::new();
                {
                    let ctx_ref = ctx.as_mut();
                    if let Some(session) = ctx_ref.active_session.as_mut() {
                        let step = session.board.commit_clear_step(&prepared, combo, &balance);
                        gained_score = step.score_gained;
                        session.apply_step_progress(&step);
                        let fall_moves: Vec<FallMove> =
                            session.board.apply_gravity_and_refill_tracked(&mut ctx_ref.rng);
                        for fm in fall_moves {
                            fall_map.insert(CellPos::new(fm.col, fm.to_row), fm.from_row);
                        }
                    }
                }
                if gained_score > 0 {
                    let popup_label = if combo >= 2 {
                        format!("+{} (x{})", gained_score, combo)
                    } else {
                        format!("+{}", gained_score)
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
                // First check if any Buried Relics reached the bottom row of their column!
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
                            let fms =
                                session.board.apply_gravity_and_refill_tracked(&mut ctx_ref.rng);
                            for fm in fms {
                                relic_fall_map.insert(CellPos::new(fm.col, fm.to_row), fm.from_row);
                            }
                        }
                    }
                }
                if !dropped_positions.is_empty() {
                    play_sfx(&mut commands, &sound_bank, SfxKind::RelicDrop);
                    if let Some(session) = ctx.active_session.as_ref() {
                        spawn_clear_particles(&mut commands, &dropped_positions, session, &input_state);
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

                // Check for follow-up cascade!
                let next_cascade = ctx
                    .active_session
                    .as_ref()
                    .and_then(|s| s.board.prepare_cascade_step());
                if let Some(next_prep) = next_cascade {
                    if let Some(session) = ctx.active_session.as_mut() {
                        session.last_combo = combo + 1;
                    }
                    input_state.dirty_hud = true;
                    input_state.phase = BoardAnimPhase::Clearing {
                        prepared: next_prep,
                        combo: combo + 1,
                        elapsed: 0.0,
                        duration: ctx.data.balance.clear_anim_seconds,
                        particles_spawned: false,
                    };
                } else {
                    // Turn finished: check deadlock shuffle and Victory/Defeat.
                    let balance = ctx.data.balance.clone();
                    let ctx_ref = ctx.as_mut();
                    let mut did_shuffle = false;
                    if let Some(session) = ctx_ref.active_session.as_mut() {
                        if !session.board.has_valid_moves() {
                            session.board.shuffle_free_runes(&mut ctx_ref.rng);
                            session.last_shuffled = true;
                            did_shuffle = true;
                        }
                        session.finalize_turn(&balance);
                    }
                    if did_shuffle {
                        input_state.dirty_board = true;
                        input_state.dirty_hud = true;
                    }
                    check_level_finished(&mut ctx, &mut next_state);
                    input_state.phase = BoardAnimPhase::Idle;
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
    }
}

fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn check_level_finished(ctx: &mut GameContext, next_state: &mut NextState<AppState>) {
    let finished = ctx.active_session.as_ref().and_then(|s| match s.outcome {
        LevelOutcome::InProgress => None,
        outcome => Some((
            outcome,
            s.level.building_id.clone(),
            s.earned_rewards(),
            s.score,
        )),
    });

    if let Some((outcome, building_id, rewards, score)) = finished {
        ctx.last_outcome = Some(outcome);
        ctx.last_rewards = rewards;
        ctx.last_score = score;
        if outcome == LevelOutcome::Victory {
            ctx.city.record_level_victory(&building_id, &rewards);
            let collections = ctx.data.collections.clone();
            ctx.last_discovered_relic_key = ctx.city.discover_next_relic(&collections);
        } else {
            ctx.last_discovered_relic_key = None;
        }
        ctx.help_modal_open = false;
        ctx.persist();
        next_state.set(AppState::LevelResult);
    }
}

/// Rebuilds board sprites and/or HUD when dirty flags are raised.
pub fn sync_match3_visuals(
    mut commands: Commands,
    ctx: Res<GameContext>,
    textures: Res<RuneTextures>,
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
        spawn_match3_hud(&mut commands, &ctx);
        input_state.dirty_hud = false;
    }
}

/// Moves and fades out burst particles, floating score popups, camera shake, and window resize.
#[allow(clippy::too_many_arguments)]
pub fn animate_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut input_state: ResMut<Match3InputState>,
    mut resize_events: EventReader<WindowResized>,
    mut backdrops: Query<&mut Sprite, (With<Match3Backdrop>, Without<BurstParticle>)>,
    mut particles: Query<(Entity, &mut BurstParticle, &mut Transform, &mut Sprite), Without<Camera2d>>,
    mut popups: Query<(Entity, &mut FloatingScore, &mut Transform, &mut TextColor), (Without<BurstParticle>, Without<Camera2d>)>,
    mut cameras: Query<&mut Transform, With<Camera2d>>,
) {
    let dt = time.delta_secs();

    // Adapt sanctuary backdrop and board layout on window resize / mobile rotation.
    for ev in resize_events.read() {
        let bg_w = ev.width.max(ev.height * (16.0 / 9.0));
        let bg_h = ev.height.max(ev.width * (9.0 / 16.0));
        for mut bg in &mut backdrops {
            bg.custom_size = Some(Vec2::new(bg_w, bg_h));
        }
        input_state.dirty_board = true;
    }

    // Burst spark particles.
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

    // Rising floating score popups.
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

    // Tactile camera screen shake on heavy blasts.
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
