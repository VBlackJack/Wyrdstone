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

//! Match-3 2D board rendering, HUD, mouse + Android touch input, and active tools.

use crate::runes_art::RuneTextures;
use crate::state::{AppState, GameContext};
use crate::theme::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use wyrdstone_core::{ActiveTool, Booster, CellPos, LevelOutcome, Objective};

#[derive(Component)]
pub struct Match3Entity;

#[derive(Component)]
pub struct Match3HudRoot;

#[derive(Component)]
pub struct BoardTileVisual;

#[derive(Component)]
pub struct SelectionMarker;

#[derive(Component)]
pub enum Match3UiAction {
    Retreat,
    SelectTool(ActiveTool),
}

/// Tracks board layout metrics and pointer/touch gesture state.
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
}

/// Initializes the Match-3 scene (2D sprites + HUD overlay).
pub fn setup_match3_scene(
    mut commands: Commands,
    ctx: Res<GameContext>,
    textures: Res<RuneTextures>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let mut input_state = Match3InputState {
        dirty_board: false,
        dirty_hud: false,
        ..default()
    };

    if let (Some(session), Ok(window)) = (&ctx.active_session, windows.get_single()) {
        compute_board_layout(
            window.width(),
            window.height(),
            session.board.cols(),
            session.board.rows(),
            &mut input_state,
        );
        spawn_board_sprites(&mut commands, session, &textures, &input_state);
    }

    spawn_match3_hud(&mut commands, &ctx);
    commands.insert_resource(input_state);
}

/// Cleans up all Match-3 sprites, HUD, and input state on exit.
pub fn cleanup_match3_scene(
    mut commands: Commands,
    sprites: Query<Entity, With<Match3Entity>>,
    huds: Query<Entity, With<Match3HudRoot>>,
) {
    for e in &sprites {
        commands.entity(e).despawn_recursive();
    }
    for e in &huds {
        commands.entity(e).despawn_recursive();
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
    layout.board_origin = Vec2::new(-board_w * 0.5 + step * 0.5, board_h * 0.5 - step * 0.5 + center_y_offset);
}

fn cell_to_world(pos: CellPos, layout: &Match3InputState) -> Vec2 {
    Vec2::new(
        layout.board_origin.x + pos.col as f32 * layout.cell_step,
        layout.board_origin.y - pos.row as f32 * layout.cell_step,
    )
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
) {
    let cols = session.board.cols();
    let rows = session.board.rows();
    let board_w = cols as f32 * layout.cell_step + 16.0;
    let board_h = rows as f32 * layout.cell_step + 16.0;
    let center_y = (BOARD_BOTTOM_RESERVED_PX - BOARD_TOP_RESERVED_PX) * 0.5;

    // Board frame backdrop
    commands.spawn((
        Match3Entity,
        Sprite {
            color: COLOR_PANEL_DARK,
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

            // Rune sprite (or Bifrost orb)
            if cell.booster == Booster::BifrostShard {
                commands.spawn((
                    Match3Entity,
                    BoardTileVisual,
                    Sprite {
                        image: textures.booster_bifrost.clone(),
                        custom_size: Some(tile_size),
                        ..default()
                    },
                    Transform::from_xyz(world.x, world.y, Z_RUNE_SPRITE)
                        .with_scale(Vec3::splat(0.86)),
                ));
            } else if let Some(rune) = cell.rune {
                if let Some(tex) = textures.runes.get(&rune) {
                    commands.spawn((
                        Match3Entity,
                        BoardTileVisual,
                        Sprite {
                            image: tex.clone(),
                            custom_size: Some(tile_size),
                            ..default()
                        },
                        Transform::from_xyz(world.x, world.y, Z_RUNE_SPRITE)
                            .with_scale(Vec3::splat(0.88)),
                    ));
                }
                if let Some(booster_tex) = textures.booster_handle(cell.booster) {
                    commands.spawn((
                        Match3Entity,
                        Sprite {
                            image: booster_tex,
                            custom_size: Some(tile_size),
                            ..default()
                        },
                        Transform::from_xyz(world.x, world.y, Z_BOOSTER_BADGE),
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
                // Moves + Score
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

                // Objectives list
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

                // Combo / shuffle notice + Retreat button
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
                        column_gap: Val::Px(12.0),
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        let tools = [
                            (ActiveTool::Mjolnir, "match3.tool.mjolnir"),
                            (ActiveTool::SurtrEmber, "match3.tool.surtr_ember"),
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
                next_state.set(AppState::FjordCity);
                return;
            }
            Match3UiAction::SelectTool(tool) => {
                if ctx.city.tools.count(tool) == 0 {
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

/// Handles both desktop mouse (click + drag) and Android touch (tap + swipe) on the Match-3 board.
pub fn handle_board_pointer_input(
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut ctx: ResMut<GameContext>,
    mut input_state: ResMut<Match3InputState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
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

    // Touch input (Android)
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

    // Mouse input (Windows desktop)
    if pressed_screen.is_none() && input_state.active_touch_id.is_none() {
        if mouse.just_pressed(MouseButton::Left) {
            pressed_screen = window.cursor_position();
        } else if mouse.pressed(MouseButton::Left) {
            current_screen = window.cursor_position();
        } else if mouse.just_released(MouseButton::Left) {
            released = true;
        }
    }

    // Handle initial press
    if let Some(screen_pos) = pressed_screen {
        if let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, screen_pos) {
            if let Some(cell) = world_to_cell(world_pos, cols, rows, &input_state) {
                // If an active tool is armed, fire it on this cell!
                if let Some(tool) = ctx.armed_tool {
                    if ctx.city.tools.consume(tool) {
                        let balance = ctx.data.balance.clone();
                        let ctx_ref = ctx.as_mut();
                        if let Some(session) = ctx_ref.active_session.as_mut() {
                            session.play_tool(tool, Some(cell), &balance, &mut ctx_ref.rng);
                        }
                        ctx.armed_tool = None;
                        ctx.persist();
                        input_state.selected = None;
                        input_state.dirty_board = true;
                        input_state.dirty_hud = true;
                        check_level_finished(&mut ctx, &mut next_state);
                    }
                    return;
                }

                // If another adjacent cell was already selected, swap them!
                if let Some(prev) = input_state.selected {
                    if prev == cell {
                        input_state.selected = None;
                        input_state.dirty_board = true;
                        return;
                    }
                    if prev.is_adjacent(cell) {
                        execute_swap(prev, cell, &mut ctx, &mut input_state, &mut next_state);
                        return;
                    }
                }

                // Select cell & start potential swipe drag.
                input_state.selected = Some(cell);
                input_state.drag_origin_cell = Some(cell);
                input_state.drag_origin_screen = Some(screen_pos);
                input_state.dirty_board = true;
            }
        }
    }

    // Handle drag-swipe gesture
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
                execute_swap(origin_cell, nb, &mut ctx, &mut input_state, &mut next_state);
            }
        }
    }

    if released {
        input_state.drag_origin_cell = None;
        input_state.drag_origin_screen = None;
    }
}

fn execute_swap(
    a: CellPos,
    b: CellPos,
    ctx: &mut GameContext,
    input_state: &mut Match3InputState,
    next_state: &mut NextState<AppState>,
) {
    let balance = ctx.data.balance.clone();
    let outcome = match ctx.active_session.as_mut() {
        Some(session) => session.play_swap(a, b, &balance, &mut ctx.rng),
        None => return,
    };
    input_state.selected = None;
    input_state.drag_origin_cell = None;
    input_state.drag_origin_screen = None;
    input_state.dirty_board = true;
    if outcome.valid {
        input_state.dirty_hud = true;
        check_level_finished(ctx, next_state);
    }
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
        }
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
        for e in &sprites {
            commands.entity(e).despawn_recursive();
        }
        if let (Some(session), Ok(window)) = (&ctx.active_session, windows.get_single()) {
            compute_board_layout(
                window.width(),
                window.height(),
                session.board.cols(),
                session.board.rows(),
                &mut input_state,
            );
            spawn_board_sprites(&mut commands, session, &textures, &input_state);
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

/// Smoothly animates tile scale pop-in after spawns and matches.
pub fn animate_board_tiles(
    time: Res<Time>,
    mut tiles: Query<&mut Transform, With<BoardTileVisual>>,
) {
    let dt = (time.delta_secs() * TILE_LERP_SPEED).min(1.0);
    for mut tf in &mut tiles {
        tf.scale = tf.scale.lerp(Vec3::ONE, dt);
    }
}
