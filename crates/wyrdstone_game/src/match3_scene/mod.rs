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

use crate::audio::{
    cycle_bgm_volume, cycle_sfx_volume, play_sfx, toggle_mute, BgmTrack, SfxKind, SoundBank,
};
use crate::fjord_art::FjordTextures;
use crate::platform::{HapticEvent, HapticKind, HudLayoutMode, SafeAreaInsets};
use crate::runes_art::RuneTextures;
use crate::scroll::UiScrollMemory;
use crate::state::{ActiveModal, AppState, GameContext};
use crate::theme::*;
use crate::vfx_art::VfxTextures;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::{HashMap, HashSet};
use wyrdstone_core::{
    format_patron_desc, ActiveTool, Booster, CellPos, ClearPreview, DailyBountyKind, LevelOutcome,
    LevelSession, Turn, TurnStep,
};

pub(super) const PARTICLES_PER_CLEARED_TILE: usize = 9;
pub(super) const PARTICLE_LIFETIME_SECS: f32 = 0.44;
pub(super) const PARTICLE_SPEED_PX: f32 = 170.0;
pub(super) const PARTICLE_SIZE_PX: f32 = 18.0;
pub(super) const PARTICLE_GRAVITY_PX: f32 = 220.0;
pub(super) const SANCTUARY_EMBER_COUNT: usize = 16;
/// Ember drift band half width as a fraction of the window width.
pub(super) const SANCTUARY_EMBER_HALF_W_RATIO: f32 = 0.48;
/// Ember rise band half height as a fraction of the window height.
pub(super) const SANCTUARY_EMBER_HALF_H_RATIO: f32 = 0.52;
pub(super) const HINT_IDLE_SECS: f32 = 5.0;
/// Moves left at or below which the moves counter switches to its warning style.
pub(super) const LOW_MOVES_THRESHOLD: u32 = 3;
pub(super) const FLOATING_TEXT_LIFETIME_SECS: f32 = 0.85;
pub(super) const FLOATING_TEXT_RISE_SPEED_PX: f32 = 58.0;
pub(super) const SHAKE_DURATION_SECS: f32 = 0.18;
pub(super) const SHAKE_AMPLITUDE_PX: f32 = 6.5;

/// Unified cleanup marker for all entities belonging to the Match-3 scene.
#[derive(Component)]
pub struct Match3Cleanup;

#[derive(Component)]
pub struct Match3Entity;

/// Static board backdrop, cell grid backgrounds, and portal rings (rebuilt only on setup/resize).
#[derive(Component)]
pub struct BoardStaticEntity;

#[derive(Component)]
pub struct Match3Backdrop;

#[derive(Component)]
pub struct Match3HudRoot;

/// Marker on the top and bottom HUD bars used to block pointer/touch clicks from reaching the board.
#[derive(Component)]
pub(super) struct Match3HudBar;

/// Marker on the HUD countdown text so timer ticks update in-place without rebuilding the HUD.
#[derive(Component)]
pub struct Match3TimerText;

/// Marker on the HUD moves counter text so cascade steps update it in-place.
#[derive(Component)]
pub struct Match3MovesText;

/// Marker on the HUD score text so cascade steps update it in-place.
#[derive(Component)]
pub struct Match3ScoreText;

/// Marker on one objective pill text (index into the session objectives) for in-place updates.
#[derive(Component)]
pub struct Match3ObjectiveText(pub usize);

/// Marker on the HUD combo banner text so cascade steps update it in-place.
#[derive(Component)]
pub struct Match3ComboText;

/// Marker on the Patron Fury button label so fury charge updates in-place.
#[derive(Component)]
pub struct Match3FuryText;

/// Marker on the Boss HP / riposte / weakness text so unchanged-phase combat steps update in-place.
#[derive(Component)]
pub struct Match3BossHpText;

/// Marker on the Boss HP fill bar so combat damage updates bar width in-place.
#[derive(Component)]
pub struct Match3BossHpFill;

/// Marker on the Ragnarok Overdrive gauge / banner text for in-place updates.
#[derive(Component)]
pub struct Match3OverdriveText;

/// Marker on the bottom HUD tooltip/hint text so hovering tools/Fury updates in-place.
#[derive(Component)]
pub struct Match3TooltipText;

pub(super) fn tool_hint_key(tool: ActiveTool) -> &'static str {
    match tool {
        ActiveTool::Mjolnir => "match3.tool.hint.mjolnir",
        ActiveTool::SurtrEmber => "match3.tool.hint.surtr_ember",
        ActiveTool::LokiShuffle => "match3.tool.hint.loki_shuffle",
        ActiveTool::ThorBolt => "match3.tool.hint.thor_bolt",
        ActiveTool::BifrostPrism => "match3.tool.hint.bifrost_prism",
        ActiveTool::DrakkarCharge => "match3.tool.hint.drakkar_charge",
    }
}

/// Marks a movable rune/booster sprite bound to a logical grid cell.
#[derive(Component)]
pub struct BoardTileVisual {
    pub pos: CellPos,
}

#[derive(Component)]
pub struct SelectionMarker;

/// Visible focus ring for keyboard grid navigation (Arrow keys + Enter).
#[derive(Component)]
pub struct KeyboardCursorMarker;

/// Translucent highlight on the adjacent target cell while dragging a rune before release.
#[derive(Component)]
pub struct DragTargetHighlight;

/// Lightweight burst particle spawned when runes clear.
#[derive(Component)]
pub struct BurstParticle {
    pub velocity: Vec2,
    pub elapsed: f32,
    pub lifetime: f32,
    pub spin: f32,
}

/// Floating "+score" or "Combo xN!" text rising above cleared matches.
#[derive(Component)]
pub struct FloatingScore {
    pub elapsed: f32,
    pub lifetime: f32,
}

/// Drop-shadow child text attached to a `FloatingScore` popup so its alpha fades in sync.
#[derive(Component)]
pub struct FloatingScoreShadow {
    pub base_alpha: f32,
}

/// Expanding lightning beam, shockwave ring, or runic mandala VFX for boosters and active tools.
#[derive(Component)]
pub struct BlastBeam {
    pub elapsed: f32,
    pub lifetime: f32,
    pub rotation_speed: f32,
    pub start_scale: f32,
    pub end_scale: f32,
}

/// Flickering warm light halo aligned to the wall torches in `match3_bg.png`.
#[derive(Component)]
pub struct SanctuaryTorchGlow {
    pub norm_x: f32,
    pub norm_y: f32,
    pub phase: f32,
    pub base_size: f32,
}

/// Slow rising runic ember/dust mote drifting across the Match-3 sanctuary backdrop.
#[derive(Component)]
pub struct SanctuaryEmber {
    pub base_x: f32,
    pub y: f32,
    pub speed_y: f32,
    pub phase: f32,
    pub half_h: f32,
    /// Half width of the drift band used when the ember was spawned.
    pub half_w: f32,
    /// Normalized horizontal lane in `-1.0..=1.0` (side * lane), rescaled on window resize.
    pub lane: f32,
}

/// Luminous orb flying along a quadratic Bezier curve from a cleared rune to the HUD objectives or Boss.
#[derive(Component)]
pub struct FlyingOrb {
    pub start: Vec2,
    pub control: Vec2,
    pub target: Vec2,
    pub elapsed: f32,
    pub duration: f32,
    pub color: Color,
}

/// Diagonal specular sheen sweeping periodically over boosters and Bifrost Shards.
#[derive(Component)]
pub struct BoosterShimmer {
    pub phase: f32,
}

/// Marker on the Match-3 Patron Fury button when 100% charged so its golden rim pulses.
#[derive(Component)]
pub struct FuryReadyPulse;

#[derive(Component)]
pub struct LowMovesPulse;

/// Gently pulses placed board boosters so they stand out crisply.
#[derive(Component)]
pub struct BoosterPulse;

/// Translucent AoE preview tile shown when hovering a cell with an armed active tool.
#[derive(Component)]
pub struct ToolHoverOverlay;

#[derive(Component)]
pub enum Match3UiAction {
    Retreat,
    Undo,
    ToggleHelp,
    SelectTool(ActiveTool),
    CancelTool,
    TriggerFury,
    ToggleAudio,
    CycleBgmVolume,
    CycleSfxVolume,
    ToggleColorblind,
    ToggleFastAnim,
    ToggleReducedMotion,
    RestartBoard,
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
    /// The turn is carried by every resolving phase and is never dropped before it reports its end.
    SuperComboCharge {
        turn: Turn,
        preview: ClearPreview,
        elapsed: f32,
        duration: f32,
        beams_spawned: bool,
    },
    Clearing {
        turn: Turn,
        preview: ClearPreview,
        cleared_set: HashSet<CellPos>,
        elapsed: f32,
        duration: f32,
        particles_spawned: bool,
    },
    Falling {
        turn: Turn,
        moves: HashMap<CellPos, i32>,
        elapsed: f32,
        duration: f32,
    },
    VictoryCelebration {
        elapsed: f32,
        duration: f32,
        next_burst_at: f32,
        burst_index: u32,
    },
    DefeatBeat {
        elapsed: f32,
        duration: f32,
        out_of_time: bool,
    },
}

/// Tracks board layout metrics, pointer/touch gesture state, hints, shake, and animation phase.
#[derive(Resource, Default)]
pub struct Match3InputState {
    pub selected: Option<CellPos>,
    pub kbd_cursor: Option<CellPos>,
    pub last_input_was_kbd: bool,
    pub last_cursor_pos: Option<Vec2>,
    pub drag_origin_cell: Option<CellPos>,
    pub drag_origin_screen: Option<Vec2>,
    pub drag_highlight_cell: Option<CellPos>,
    pub active_touch_id: Option<u64>,
    pub tile_size: f32,
    pub cell_step: f32,
    pub board_origin: Vec2,
    pub board_center_y: f32,
    pub hud_mode: HudLayoutMode,
    pub dirty_static_board: bool,
    pub dirty_board: bool,
    pub dirty_hud: bool,
    /// Cascade-step HUD refresh: texts are rewritten in place when the HUD shape is unchanged,
    /// otherwise the HUD is rebuilt. A raised `dirty_hud` always wins.
    pub dirty_hud_soft: bool,
    /// Shape of the HUD as last built or refreshed, used to decide between in-place and rebuild.
    pub hud_shape: Option<hud::HudShape>,
    pub undo_snapshot: Option<UndoSnapshot>,
    pub undo_used: bool,
    pub confirm_retreat: bool,
    pub confirm_loki_shuffle: bool,
    pub confirm_restart: bool,
    pub skip_valhalla: bool,
    pub reduced_motion: bool,
    pub idle_secs: f32,
    pub hint_pair: Option<(CellPos, CellPos)>,
    pub hint_checked: bool,
    pub hover_cell: Option<CellPos>,
    pub touch_tool_cell: Option<CellPos>,
    pub buffered_swap: Option<(CellPos, CellPos)>,
    pub drag_cancelled: bool,
    /// Raised when the board was rebuilt: the hover and drag overlay entities (not Match3Entity)
    /// may still exist for a cell that no longer matches, so the pointer handler, their only
    /// despawner, must clear and redraw them on its next idle frame, then lower the flag.
    pub overlays_stale: bool,
    pub shake_secs: f32,
    pub boss_hit_flash_secs: f32,
    pub phase: BoardAnimPhase,
}

/// Session copy taken before a swap, plus the city stats that the swap may still change.
#[derive(Clone)]
pub struct UndoSnapshot {
    pub session: LevelSession,
    pub total_runes_cleared: u64,
    pub best_combo: u32,
}

impl Match3InputState {
    /// Drops the current hint and restarts the idle countdown. When a hint was showing, the board
    /// is flagged dirty so the pulsing tiles are rebuilt with their normal scale and colour.
    pub fn clear_hint(&mut self) {
        if self.hint_pair.take().is_some() {
            self.dirty_board = true;
        }
        self.hint_checked = false;
        self.idle_secs = 0.0;
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.phase, BoardAnimPhase::Idle)
    }

    pub fn is_compact_hud(&self) -> bool {
        self.hud_mode.is_compact()
    }

    pub fn can_undo(&self, ctx: &GameContext) -> bool {
        self.is_idle()
            && !self.undo_used
            && self.undo_snapshot.is_some()
            && ctx.active_session.as_ref().map(|s| s.outcome) == Some(LevelOutcome::InProgress)
    }
}

/// Initializes the Match-3 scene (painted sanctuary backdrop + 2D sprites + HUD overlay).
#[allow(clippy::too_many_arguments)]
pub fn setup_match3_scene(
    mut commands: Commands,
    ctx: Res<GameContext>,
    textures: Res<RuneTextures>,
    vfx: Res<VfxTextures>,
    fjord_art: Res<FjordTextures>,
    sound_bank: Res<SoundBank>,
    scroll_mem: Res<UiScrollMemory>,
    safe_area: Res<SafeAreaInsets>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let mut input_state = Match3InputState {
        reduced_motion: ctx.reduced_motion,
        ..default()
    };

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

        // Flickering warm wall torch halos aligned to the sanctuary sconces.
        let torch_defs = [
            (-0.332, -0.015, 0.0_f32, 185.0_f32),
            (0.332, -0.015, 2.4_f32, 185.0_f32),
            (0.415, -0.175, 4.1_f32, 160.0_f32),
        ];
        for (nx, ny, phase, base_size) in torch_defs {
            commands.spawn((
                Match3Cleanup,
                SanctuaryTorchGlow {
                    norm_x: nx,
                    norm_y: ny,
                    phase,
                    base_size,
                },
                Sprite {
                    image: vfx.soft_orb.clone(),
                    color: Color::srgba(1.0, 0.62, 0.22, 0.34),
                    custom_size: Some(Vec2::splat(base_size)),
                    ..default()
                },
                Transform::from_xyz(nx * bg_w, ny * bg_h, Z_BOARD_BG - 1.2),
            ));
        }

        // Gentle floating runic embers/motes drifting up the sanctuary arches.
        let half_w = window.width() * SANCTUARY_EMBER_HALF_W_RATIO;
        let half_h = window.height() * SANCTUARY_EMBER_HALF_H_RATIO;
        for i in 0..SANCTUARY_EMBER_COUNT {
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let lane = 0.55 + ((i * 19) % 45) as f32 / 100.0;
            let norm_lane = side * lane;
            let bx = norm_lane * half_w;
            let by = -half_h + ((i * 73) % 100) as f32 / 100.0 * (half_h * 2.0);
            let is_aurora = i % 3 == 0;
            let color = if is_aurora {
                Color::srgba(0.34, 0.94, 0.76, 0.32)
            } else {
                Color::srgba(1.0, 0.72, 0.30, 0.38)
            };
            let size = 7.0 + (i % 4) as f32 * 3.5;
            commands.spawn((
                Match3Cleanup,
                SanctuaryEmber {
                    base_x: bx,
                    y: by,
                    speed_y: 16.0 + (i % 5) as f32 * 5.5,
                    phase: i as f32 * 1.17,
                    half_h,
                    half_w,
                    lane: norm_lane,
                },
                Sprite {
                    image: vfx.soft_orb.clone(),
                    color,
                    custom_size: Some(Vec2::splat(size)),
                    ..default()
                },
                Transform::from_xyz(bx, by, Z_BOARD_BG - 0.8),
            ));
        }

        if let Some(session) = &ctx.active_session {
            compute_board_layout(
                window.width(),
                window.height(),
                session.board.cols(),
                session.board.rows(),
                *safe_area,
                &mut input_state,
            );
            spawn_board_static_sprites(&mut commands, session, &textures, &vfx, &input_state);
            spawn_board_sprites(
                &mut commands,
                session,
                &textures,
                &vfx,
                &input_state,
                None,
                ctx.colorblind_mode,
            );
        }
    }

    hud::spawn_match3_hud(
        &mut commands,
        &ctx,
        &fjord_art,
        &textures,
        &vfx,
        &sound_bank,
        scroll_mem.modal_offset_y,
        input_state.hud_mode,
        *safe_area,
        input_state.can_undo(&ctx),
        input_state.undo_used,
        input_state.confirm_retreat,
        input_state.confirm_loki_shuffle,
        input_state.confirm_restart,
        false,
    );
    commands.insert_resource(input_state);
}

/// Cleans up all Match-3 entities and resets camera offset on exit.
pub fn cleanup_match3_scene(
    mut commands: Commands,
    cleanup_q: Query<Entity, With<Match3Cleanup>>,
    mut cameras: Query<&mut Transform, With<Camera2d>>,
) {
    for entity in &cleanup_q {
        commands.entity(entity).try_despawn_recursive();
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
    safe: SafeAreaInsets,
    layout: &mut Match3InputState,
) {
    let hud_mode = HudLayoutMode::from_viewport(win_w, win_h);
    layout.hud_mode = hud_mode;
    const BOARD_FRAME_PAD_PX: f32 = 28.0;
    let (avail_w, avail_h, top_reserved, bottom_reserved) = match hud_mode {
        HudLayoutMode::DesktopWide => {
            let rail_total = (DESKTOP_SIDE_RAIL_WIDTH_PX + 24.0) * 2.0 + safe.left + safe.right;
            let top_res = DESKTOP_BOARD_VERTICAL_MARGIN_PX + safe.top;
            let bot_res = DESKTOP_BOARD_VERTICAL_MARGIN_PX + safe.bottom;
            let aw = (win_w - rail_total - BOARD_FRAME_PAD_PX).max(180.0);
            let ah = (win_h - top_res - bot_res - BOARD_FRAME_PAD_PX).max(180.0);
            (aw, ah, top_res, bot_res)
        }
        HudLayoutMode::Compact => {
            let top_res = BOARD_TOP_RESERVED_PX + safe.top;
            let bot_res = BOARD_BOTTOM_RESERVED_PX + safe.bottom;
            let side_margin = BOARD_SIDE_MARGIN_PX + safe.left.max(safe.right);
            let aw = (win_w - side_margin * 2.0 - BOARD_FRAME_PAD_PX).max(180.0);
            let ah = (win_h - top_res - bot_res - BOARD_FRAME_PAD_PX).max(180.0);
            (aw, ah, top_res, bot_res)
        }
        HudLayoutMode::Portrait => {
            let narrow = win_w < 500.0;
            let top_res = (if narrow { 178.0 } else { 152.0 }) + safe.top;
            let bot_res = (if narrow { 174.0 } else { 148.0 }) + safe.bottom;
            let side_margin = 12.0 + safe.left.max(safe.right);
            let aw = (win_w - side_margin * 2.0 - BOARD_FRAME_PAD_PX).max(180.0);
            let ah = (win_h - top_res - bot_res - BOARD_FRAME_PAD_PX).max(180.0);
            (aw, ah, top_res, bot_res)
        }
    };
    let step = (avail_w / cols as f32)
        .min(avail_h / rows as f32)
        .clamp(MIN_TILE_SIZE_PX, MAX_TILE_SIZE_PX);
    layout.cell_step = step;
    layout.tile_size = step * (1.0 - TILE_GAP_RATIO);

    let board_w = step * cols as f32;
    let board_h = step * rows as f32;
    let center_y_offset = (bottom_reserved - top_reserved) * 0.5;
    layout.board_center_y = center_y_offset;
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

/// Spawns the static board backdrop, playable cell grid, and portal rings (rebuilt only on setup/resize).
pub(super) fn spawn_board_static_sprites(
    commands: &mut Commands,
    session: &wyrdstone_core::LevelSession,
    textures: &RuneTextures,
    vfx: &VfxTextures,
    layout: &Match3InputState,
) {
    let cols = session.board.cols();
    let rows = session.board.rows();
    let board_w = cols as f32 * layout.cell_step + 28.0;
    let board_h = rows as f32 * layout.cell_step + 28.0;
    let center_y = layout.board_center_y;

    // Soft ambient shadow behind the sculpted stone board frame.
    commands.spawn((
        Match3Cleanup,
        BoardStaticEntity,
        Sprite {
            color: Color::srgba(0.02, 0.03, 0.06, 0.58),
            custom_size: Some(Vec2::new(board_w + 18.0, board_h + 18.0)),
            ..default()
        },
        Transform::from_xyz(0.0, center_y - 5.0, Z_BOARD_BG - 0.2),
    ));

    commands.spawn((
        Match3Cleanup,
        BoardStaticEntity,
        Sprite {
            image: vfx.board_frame.clone(),
            custom_size: Some(Vec2::new(board_w, board_h)),
            ..default()
        },
        Transform::from_xyz(0.0, center_y, Z_BOARD_BG),
    ));

    let cell_size = Vec2::splat(layout.cell_step * 0.96);
    for row in 0..rows {
        for col in 0..cols {
            let pos = CellPos::new(col, row);
            if session.board.get(pos).map(|c| c.blocked).unwrap_or(true) {
                continue;
            }
            let world = cell_to_world(pos, layout);
            commands.spawn((
                Match3Cleanup,
                BoardStaticEntity,
                Sprite {
                    image: textures.cell_bg.clone(),
                    custom_size: Some(cell_size),
                    ..default()
                },
                Transform::from_xyz(world.x, world.y, Z_CELL_BG),
            ));
        }
    }

    for &(src, dst) in session.board.portals() {
        for p in [src, dst] {
            let w = cell_to_world(p, layout);
            commands.spawn((
                Match3Cleanup,
                BoardStaticEntity,
                Sprite {
                    image: textures.portal_ring.clone(),
                    custom_size: Some(Vec2::splat(layout.cell_step * 0.98)),
                    ..default()
                },
                Transform::from_xyz(w.x, w.y, Z_CELL_BG + 0.5),
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_board_sprites(
    commands: &mut Commands,
    session: &wyrdstone_core::LevelSession,
    textures: &RuneTextures,
    vfx: &VfxTextures,
    layout: &Match3InputState,
    fall_starts: Option<&HashMap<CellPos, i32>>,
    colorblind_mode: bool,
) {
    let cols = session.board.cols();
    let rows = session.board.rows();

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
            let tile_size = Vec2::splat(layout.tile_size);

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

            if cell.fog {
                commands.spawn((
                    Match3Cleanup,
                    Match3Entity,
                    Sprite {
                        image: textures.fog_overlay.clone(),
                        custom_size: Some(Vec2::splat(layout.cell_step * 0.98)),
                        ..default()
                    },
                    Transform::from_xyz(world.x, world.y, Z_CHAIN_OVERLAY + 0.5),
                ));
                continue;
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
                let phase = (pos.col as f32 * 0.23 + pos.row as f32 * 0.37).fract();
                commands
                    .spawn((
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
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            BoosterShimmer { phase },
                            Sprite {
                                image: vfx.shimmer_band.clone(),
                                color: Color::srgba(1.0, 0.96, 0.80, 0.0),
                                custom_size: Some(tile_size * 0.94),
                                ..default()
                            },
                            Transform::from_xyz(0.0, 0.0, 0.6),
                        ));
                    });
            } else if let Some(rune) = cell.rune {
                if let Some(tex) = textures.runes.get(&rune) {
                    let mut rune_cmd = commands.spawn((
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
                    if cell.curse_turns > 0 {
                        let urgent = cell.curse_turns == 1;
                        let badge_size = layout.tile_size * 0.38;
                        let badge_font = (layout.tile_size * 0.28).clamp(11.0, 18.0);
                        let badge_offset = layout.tile_size * 0.28;
                        // Render curse countdown badge at the top-right corner above Chain overlays.
                        let badge_z = Z_CHAIN_OVERLAY - Z_RUNE_SPRITE + 0.3;
                        rune_cmd.with_children(|parent| {
                            parent.spawn((
                                Sprite {
                                    color: if urgent {
                                        Color::srgba(0.92, 0.12, 0.26, 0.56)
                                    } else {
                                        Color::srgba(0.86, 0.14, 0.34, 0.34)
                                    },
                                    custom_size: Some(tile_size * 1.04),
                                    ..default()
                                },
                                Transform::from_xyz(0.0, 0.0, 0.4),
                            ));
                            let mut badge = parent.spawn((
                                Sprite {
                                    color: if urgent {
                                        Color::srgba(0.78, 0.10, 0.18, 0.96)
                                    } else {
                                        Color::srgba(0.06, 0.08, 0.12, 0.92)
                                    },
                                    custom_size: Some(Vec2::splat(badge_size)),
                                    ..default()
                                },
                                Transform::from_xyz(badge_offset, badge_offset, badge_z),
                            ));
                            if urgent {
                                badge.insert(BoosterPulse);
                            }
                            badge.with_children(|b| {
                                b.spawn((
                                    Text2d::new(cell.curse_turns.to_string()),
                                    TextFont {
                                        font_size: badge_font,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(1.0, 0.94, 0.96)),
                                    Transform::from_xyz(0.0, 0.0, 0.05),
                                ));
                            });
                        });
                    }
                }
                if colorblind_mode {
                    if let Some(badge) = textures.colorblind_badges.get(&rune) {
                        commands.spawn((
                            Match3Cleanup,
                            Match3Entity,
                            BoardTileVisual { pos },
                            Sprite {
                                image: badge.clone(),
                                custom_size: Some(tile_size),
                                ..default()
                            },
                            Transform::from_xyz(tile_world.x, tile_world.y, Z_BOOSTER_BADGE + 0.2),
                        ));
                    }
                }
                if let Some(booster_tex) = textures.booster_handle(cell.booster) {
                    let phase = (pos.col as f32 * 0.29 + pos.row as f32 * 0.41).fract();
                    commands
                        .spawn((
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
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                BoosterShimmer { phase },
                                Sprite {
                                    image: vfx.shimmer_band.clone(),
                                    color: Color::srgba(1.0, 0.96, 0.80, 0.0),
                                    custom_size: Some(tile_size * 0.92),
                                    ..default()
                                },
                                Transform::from_xyz(0.0, 0.0, 0.15),
                            ));
                        });
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

    // Highlight adjacent booster pairs and draw connecting super-combo energy bridges.
    let pairs = session.board.find_booster_combo_pairs();
    let mut combo_cells = HashSet::new();
    for &(a, b) in &pairs {
        combo_cells.insert(a);
        combo_cells.insert(b);
        let wa = cell_to_world(a, layout);
        let wb = cell_to_world(b, layout);
        let mid = wa.lerp(wb, 0.5);
        let delta = wb - wa;
        let len = delta.length().max(1.0);
        let angle = delta.y.atan2(delta.x);
        commands.spawn((
            Match3Cleanup,
            Match3Entity,
            BoosterPulse,
            Sprite {
                color: Color::srgba(1.0, 0.92, 0.44, 0.88),
                custom_size: Some(Vec2::new(len * 0.68, (layout.tile_size * 0.12).max(5.0))),
                ..default()
            },
            Transform::from_xyz(mid.x, mid.y, Z_BOOSTER_BADGE + 0.1)
                .with_rotation(Quat::from_rotation_z(angle)),
        ));
    }
    for pos in combo_cells {
        let w = cell_to_world(pos, layout);
        commands.spawn((
            Match3Cleanup,
            Match3Entity,
            BoosterPulse,
            Sprite {
                color: Color::srgba(1.0, 0.86, 0.34, 0.36),
                custom_size: Some(Vec2::splat(layout.cell_step * 0.96)),
                ..default()
            },
            Transform::from_xyz(w.x, w.y, Z_CELL_BG + 0.7),
        ));
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

    if let Some(cur) = layout.kbd_cursor {
        let w = cell_to_world(cur, layout);
        commands.spawn((
            Match3Cleanup,
            Match3Entity,
            KeyboardCursorMarker,
            Sprite {
                color: Color::srgba(0.42, 0.92, 1.0, 0.28),
                custom_size: Some(Vec2::splat(layout.cell_step * 0.96)),
                ..default()
            },
            Transform::from_xyz(w.x, w.y, Z_SELECTION_RING - 0.1),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn rebuild_board_now(
    commands: &mut Commands,
    session: &wyrdstone_core::LevelSession,
    textures: &RuneTextures,
    vfx: &VfxTextures,
    window: &Window,
    safe: SafeAreaInsets,
    input_state: &mut Match3InputState,
    sprites: &Query<Entity, With<Match3Entity>>,
    fall_starts: Option<&HashMap<CellPos, i32>>,
    colorblind_mode: bool,
) {
    for e in sprites {
        commands.entity(e).try_despawn_recursive();
    }
    // Hover and drag overlays are not Match3Entity (the pointer handler owns their lifecycle, so a
    // rebuild never double-despawns them): forget their cells so the handler redraws them.
    input_state.hover_cell = None;
    input_state.drag_highlight_cell = None;
    input_state.overlays_stale = true;
    compute_board_layout(
        window.width(),
        window.height(),
        session.board.cols(),
        session.board.rows(),
        safe,
        input_state,
    );
    spawn_board_sprites(
        commands,
        session,
        textures,
        vfx,
        input_state,
        fall_starts,
        colorblind_mode,
    );
}

fn spawn_invalid_cell_flash(commands: &mut Commands, pos: CellPos, layout: &Match3InputState) {
    let w = cell_to_world(pos, layout);
    commands.spawn((
        Match3Cleanup,
        BlastBeam {
            elapsed: 0.0,
            lifetime: 0.28,
            rotation_speed: 0.0,
            start_scale: 1.0,
            end_scale: 1.12,
        },
        Sprite {
            color: Color::srgba(0.95, 0.22, 0.26, 0.62),
            custom_size: Some(Vec2::splat(layout.cell_step * 0.94)),
            ..default()
        },
        Transform::from_xyz(w.x, w.y, Z_SELECTION_RING + 0.3),
    ));
}

fn update_selection_marker(
    commands: &mut Commands,
    textures: &RuneTextures,
    layout: &Match3InputState,
    markers: &Query<Entity, With<SelectionMarker>>,
) {
    for e in markers {
        commands.entity(e).try_despawn();
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

fn update_kbd_cursor_marker(
    commands: &mut Commands,
    layout: &Match3InputState,
    markers: &Query<Entity, With<KeyboardCursorMarker>>,
) {
    for e in markers {
        commands.entity(e).try_despawn();
    }
    if let Some(cur) = layout.kbd_cursor {
        let w = cell_to_world(cur, layout);
        commands.spawn((
            Match3Cleanup,
            Match3Entity,
            KeyboardCursorMarker,
            Sprite {
                color: Color::srgba(0.42, 0.92, 1.0, 0.28),
                custom_size: Some(Vec2::splat(layout.cell_step * 0.96)),
                ..default()
            },
            Transform::from_xyz(w.x, w.y, Z_SELECTION_RING - 0.1),
        ));
    }
}

/// Why a targeted tool turn did not start.
enum ToolStartError {
    /// The target is not valid for this tool: nothing was consumed.
    Invalid,
    /// The target is valid but the inventory holds no charge of the tool.
    NoStock,
}

/// Previews the tool on `cell`, consumes one charge from the inventory, then starts the turn.
fn start_tool_turn(
    ctx: &mut GameContext,
    tool: ActiveTool,
    cell: CellPos,
) -> Result<(Turn, ClearPreview), ToolStartError> {
    let valid = ctx
        .active_session
        .as_ref()
        .filter(|s| s.outcome == LevelOutcome::InProgress)
        .and_then(|s| s.board.prepare_tool_step(tool, cell))
        .is_some();
    if !valid {
        return Err(ToolStartError::Invalid);
    }
    if !ctx.city.tools.consume(tool) {
        return Err(ToolStartError::NoStock);
    }
    match ctx
        .active_session
        .as_mut()
        .map(|session| session.begin_tool(tool, cell))
    {
        Some(Ok((turn, TurnStep::ClearPending(preview)))) => Ok((turn, preview)),
        _ => Err(ToolStartError::Invalid),
    }
}

fn execute_undo(
    commands: &mut Commands,
    ctx: &mut GameContext,
    sound_bank: &SoundBank,
    input_state: &mut Match3InputState,
) {
    if !input_state.can_undo(ctx) {
        return;
    }
    if let Some(snap) = input_state.undo_snapshot.take() {
        let mut session = snap.session;
        session.time_left_secs = ctx.active_session.as_ref().and_then(|s| s.time_left_secs);
        ctx.active_session = Some(session);
        ctx.city.stats.total_runes_cleared = snap.total_runes_cleared;
        ctx.city.stats.best_combo = snap.best_combo;
        ctx.armed_tool = None;
        input_state.undo_used = true;
        input_state.selected = None;
        input_state.clear_hint();
        input_state.boss_hit_flash_secs = 0.0;
        input_state.dirty_board = true;
        input_state.dirty_hud = true;
        play_sfx(commands, sound_bank, SfxKind::Swap);
    }
}

fn execute_loki_shuffle(
    commands: &mut Commands,
    ctx: &mut GameContext,
    vfx: &VfxTextures,
    sound_bank: &SoundBank,
    input_state: &mut Match3InputState,
) {
    let can_shuffle = ctx
        .active_session
        .as_ref()
        .is_some_and(|s| s.outcome == LevelOutcome::InProgress);
    if !can_shuffle {
        return;
    }
    if ctx.city.tools.consume(ActiveTool::LokiShuffle) {
        let (cols, rows) = ctx
            .active_session
            .as_ref()
            .map(|s| (s.board.cols(), s.board.rows()))
            .unwrap_or((8, 8));
        {
            let ctx_ref = &mut *ctx;
            if let Some(session) = ctx_ref.active_session.as_mut() {
                session.loki_shuffle(&ctx_ref.data.balance, &mut ctx_ref.rng);
            }
        }
        ctx.armed_tool = None;
        ctx.persist();
        input_state.undo_snapshot = None;
        input_state.clear_hint();
        play_sfx(commands, sound_bank, SfxKind::ToolBlast);
        anim::spawn_loki_vfx(commands, vfx, cols, rows, input_state);
        input_state.shake_secs = SHAKE_DURATION_SECS * 0.7;
        input_state.dirty_board = true;
        input_state.dirty_hud = true;
    }
}

fn execute_patron_fury(
    commands: &mut Commands,
    ctx: &mut GameContext,
    vfx: &VfxTextures,
    sound_bank: &SoundBank,
    input_state: &mut Match3InputState,
) {
    let Some((can_fire, cols, rows)) = ctx
        .active_session
        .as_ref()
        .map(|s| (s.can_trigger_fury(), s.board.cols(), s.board.rows()))
    else {
        return;
    };
    if !can_fire || !input_state.is_idle() {
        if input_state.is_idle() {
            play_sfx(commands, sound_bank, SfxKind::Invalid);
        }
        return;
    }
    let ctx_ref = ctx;
    let session_tuning = ctx_ref.data.balance.session;
    let Some(outcome) = ctx_ref
        .active_session
        .as_mut()
        .and_then(|session| session.trigger_fury(&session_tuning, &mut ctx_ref.rng))
    else {
        return;
    };
    ctx_ref
        .city
        .daily_bounties
        .ensure_for_day(crate::state::today_day_index());
    ctx_ref
        .city
        .daily_bounties
        .record_progress(DailyBountyKind::TriggerFury, 1);
    input_state.undo_snapshot = None;
    input_state.clear_hint();
    play_sfx(commands, sound_bank, SfxKind::FuryUnleash);
    anim::spawn_patron_fury_vfx(
        commands,
        vfx,
        outcome.patron,
        &outcome.vfx_targets,
        cols,
        rows,
        input_state,
    );
    input_state.shake_secs = SHAKE_DURATION_SECS;
    input_state.dirty_hud = true;
    if let Some((turn, TurnStep::ClearPending(preview))) = outcome.turn {
        let cleared_set: HashSet<CellPos> = preview.prepared.cells.iter().copied().collect();
        let clear_dur = ctx_ref.anim_duration(ctx_ref.data.balance.clear_anim_seconds);
        input_state.selected = None;
        input_state.phase = BoardAnimPhase::Clearing {
            turn,
            preview,
            cleared_set,
            elapsed: 0.0,
            duration: clear_dur,
            particles_spawned: false,
        };
    } else {
        // Loki (2 boosters) or Odin (+2 moves & bomb): refresh board immediately.
        if ctx_ref.active_session.as_mut().is_some_and(|s| {
            let triggered = s.just_triggered_overdrive;
            s.just_triggered_overdrive = false;
            triggered
        }) {
            anim::spawn_floating_score(
                commands,
                &[CellPos::new(cols / 2, rows / 2)],
                ctx_ref.data.i18n.t("match3.callout.overdrive").to_string(),
                COLOR_ACCENT_GOLD,
                34.0,
                input_state,
            );
        }
        input_state.dirty_board = true;
    }
}

/// Handles HUD button interactions and keyboard shortcuts (`1-6`, `Space`/`F`, `H`, `Backspace` Undo, Arrows+`Enter`, `Escape`).
#[allow(clippy::too_many_arguments)]
pub fn handle_match3_hud_buttons(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut ctx: ResMut<GameContext>,
    textures: Res<RuneTextures>,
    vfx: Res<VfxTextures>,
    (mut sound_bank, mut haptics): (ResMut<SoundBank>, EventWriter<HapticEvent>),
    mut bgm_sinks: Query<&mut AudioSink, With<BgmTrack>>,
    mut scroll_mem: ResMut<UiScrollMemory>,
    mut input_state: ResMut<Match3InputState>,
    mut next_state: ResMut<NextState<AppState>>,
    selection_markers: Query<Entity, With<SelectionMarker>>,
    kbd_markers: Query<Entity, With<KeyboardCursorMarker>>,
    mut tooltip_q: Query<(&mut Text, &mut Node), With<Match3TooltipText>>,
    (touches, mut press_arm): (Res<Touches>, Local<crate::scroll::PressArm>),
    buttons: Query<(Entity, &Interaction, &Match3UiAction), Changed<Interaction>>,
) {
    // Allow players to fast-forward the Valhalla Bonus or Defeat beat with Space/Enter/Escape.
    if matches!(
        input_state.phase,
        BoardAnimPhase::VictoryCelebration { .. } | BoardAnimPhase::DefeatBeat { .. }
    ) {
        if keys.just_pressed(KeyCode::Space)
            || keys.just_pressed(KeyCode::Enter)
            || keys.just_pressed(KeyCode::Escape)
        {
            input_state.skip_valhalla = true;
        }
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        if ctx.active_modal == ActiveModal::Help {
            ctx.active_modal = ActiveModal::None;
            input_state.confirm_restart = false;
            scroll_mem.modal_offset_y = 0.0;
            input_state.dirty_hud = true;
            return;
        }
        if ctx.armed_tool.is_some()
            || input_state.confirm_retreat
            || input_state.confirm_loki_shuffle
        {
            ctx.armed_tool = None;
            input_state.confirm_retreat = false;
            input_state.confirm_loki_shuffle = false;
            input_state.overlays_stale = true;
            input_state.dirty_hud = true;
            return;
        }
        ctx.active_modal = ActiveModal::Help;
        ctx.armed_tool = None;
        input_state.overlays_stale = true;
        scroll_mem.modal_offset_y = 0.0;
        input_state.dirty_hud = true;
        return;
    }

    if keys.just_pressed(KeyCode::Backspace)
        && input_state.is_idle()
        && ctx.active_modal != ActiveModal::Help
    {
        if input_state.confirm_retreat || input_state.confirm_loki_shuffle {
            input_state.confirm_retreat = false;
            input_state.confirm_loki_shuffle = false;
            input_state.dirty_hud = true;
        }
        execute_undo(&mut commands, &mut ctx, &sound_bank, &mut input_state);
        return;
    }

    // Keyboard grid navigation (Arrow keys + Enter) for full accessibility.
    if input_state.is_idle() && ctx.active_modal != ActiveModal::Help {
        let arrow_delta = if keys.just_pressed(KeyCode::ArrowUp) {
            Some((0i32, -1i32))
        } else if keys.just_pressed(KeyCode::ArrowDown) {
            Some((0, 1))
        } else if keys.just_pressed(KeyCode::ArrowLeft) {
            Some((-1, 0))
        } else if keys.just_pressed(KeyCode::ArrowRight) {
            Some((1, 0))
        } else {
            None
        };
        if let (Some((dc, dr)), Some(session)) = (arrow_delta, &ctx.active_session) {
            let cols = session.board.cols();
            let rows = session.board.rows();
            let start = input_state.kbd_cursor.unwrap_or_else(|| {
                let mut best = CellPos::new(cols / 2, rows / 2);
                if session.board.get(best).map(|c| c.blocked).unwrap_or(true) {
                    for r in 0..rows {
                        for c in 0..cols {
                            let p = CellPos::new(c, r);
                            if session
                                .board
                                .get(p)
                                .map(|cell| !cell.blocked)
                                .unwrap_or(false)
                            {
                                best = p;
                                break;
                            }
                        }
                    }
                }
                best
            });
            let mut nc = start.col as i32 + dc;
            let mut nr = start.row as i32 + dr;
            let mut landed = start;
            while nc >= 0 && nc < cols as i32 && nr >= 0 && nr < rows as i32 {
                let candidate = CellPos::new(nc as usize, nr as usize);
                if session.board.get(candidate).is_some_and(|c| !c.blocked) {
                    landed = candidate;
                    break;
                }
                nc += dc;
                nr += dr;
            }
            input_state.kbd_cursor = Some(landed);
            input_state.last_input_was_kbd = true;
            update_kbd_cursor_marker(&mut commands, &input_state, &kbd_markers);
            return;
        }
        if keys.just_pressed(KeyCode::Enter) {
            if let Some(cell) = input_state.kbd_cursor {
                if let Some(tool) = ctx.armed_tool {
                    match start_tool_turn(&mut ctx, tool, cell) {
                        Ok((turn, preview)) => {
                            let (cols, rows) = ctx
                                .active_session
                                .as_ref()
                                .map(|s| (s.board.cols(), s.board.rows()))
                                .unwrap_or((8, 8));
                            ctx.armed_tool = None;
                            ctx.persist();
                            input_state.undo_snapshot = None;
                            input_state.clear_hint();
                            play_sfx(&mut commands, &sound_bank, SfxKind::ToolBlast);
                            anim::spawn_tool_vfx(
                                &mut commands,
                                &vfx,
                                tool,
                                cell,
                                &preview.prepared.cells,
                                cols,
                                rows,
                                &input_state,
                            );
                            let clear_dur = ctx.anim_duration(ctx.data.balance.clear_anim_seconds);
                            let cleared_set: HashSet<CellPos> =
                                preview.prepared.cells.iter().copied().collect();
                            input_state.selected = None;
                            input_state.shake_secs = SHAKE_DURATION_SECS;
                            input_state.dirty_hud = true;
                            haptics.send(HapticEvent(HapticKind::BoosterBlast));
                            input_state.phase = BoardAnimPhase::Clearing {
                                turn,
                                preview,
                                cleared_set,
                                elapsed: 0.0,
                                duration: clear_dur,
                                particles_spawned: false,
                            };
                        }
                        Err(ToolStartError::NoStock) => {}
                        Err(ToolStartError::Invalid) => {
                            spawn_invalid_cell_flash(&mut commands, cell, &input_state);
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                            haptics.send(HapticEvent(HapticKind::Error));
                        }
                    }
                    return;
                }
                if let Some(prev) = input_state.selected {
                    if prev == cell {
                        input_state.selected = None;
                        update_selection_marker(
                            &mut commands,
                            &textures,
                            &input_state,
                            &selection_markers,
                        );
                    } else if prev.is_adjacent(cell) {
                        if start_swap_animation(prev, cell, &ctx, &mut input_state) {
                            update_selection_marker(
                                &mut commands,
                                &textures,
                                &input_state,
                                &selection_markers,
                            );
                            play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                            haptics.send(HapticEvent(HapticKind::LightTap));
                        } else {
                            update_selection_marker(
                                &mut commands,
                                &textures,
                                &input_state,
                                &selection_markers,
                            );
                            spawn_invalid_cell_flash(&mut commands, prev, &input_state);
                            spawn_invalid_cell_flash(&mut commands, cell, &input_state);
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                            haptics.send(HapticEvent(HapticKind::Error));
                        }
                    } else {
                        input_state.selected = Some(cell);
                        update_selection_marker(
                            &mut commands,
                            &textures,
                            &input_state,
                            &selection_markers,
                        );
                    }
                } else {
                    input_state.selected = Some(cell);
                    update_selection_marker(
                        &mut commands,
                        &textures,
                        &input_state,
                        &selection_markers,
                    );
                }
                return;
            }
        }
    }

    if keys.just_pressed(KeyCode::KeyH)
        && input_state.is_idle()
        && ctx.active_modal != ActiveModal::Help
    {
        input_state.clear_hint();
        input_state.idle_secs = HINT_IDLE_SECS;
        input_state.hint_pair = ctx
            .active_session
            .as_ref()
            .and_then(|s| s.board.find_hint_move());
        return;
    }

    if (keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::KeyF))
        && input_state.is_idle()
        && ctx.active_modal != ActiveModal::Help
        && !ctx.active_session.as_ref().is_some_and(|s| s.is_trial)
    {
        if input_state.confirm_retreat || input_state.confirm_loki_shuffle {
            input_state.confirm_retreat = false;
            input_state.confirm_loki_shuffle = false;
            input_state.dirty_hud = true;
        }
        execute_patron_fury(&mut commands, &mut ctx, &vfx, &sound_bank, &mut input_state);
        if matches!(input_state.phase, BoardAnimPhase::Clearing { .. }) {
            haptics.send(HapticEvent(HapticKind::BoosterBlast));
        }
        return;
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
        if keys.just_pressed(kc)
            && input_state.is_idle()
            && ctx.active_modal != ActiveModal::Help
            && !ctx.active_session.as_ref().is_some_and(|s| s.is_trial)
        {
            if ctx.city.tools.count(tool) > 0 {
                input_state.confirm_retreat = false;
                if tool == ActiveTool::LokiShuffle {
                    if input_state.confirm_loki_shuffle {
                        input_state.confirm_loki_shuffle = false;
                        execute_loki_shuffle(
                            &mut commands,
                            &mut ctx,
                            &vfx,
                            &sound_bank,
                            &mut input_state,
                        );
                    } else {
                        ctx.armed_tool = None;
                        input_state.confirm_loki_shuffle = true;
                        input_state.overlays_stale = true;
                        play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                        input_state.dirty_hud = true;
                    }
                } else {
                    input_state.confirm_loki_shuffle = false;
                    ctx.armed_tool = if ctx.armed_tool == Some(tool) {
                        None
                    } else {
                        Some(tool)
                    };
                    input_state.selected = None;
                    input_state.overlays_stale = true;
                    input_state.dirty_hud = true;
                }
            } else {
                play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
            }
            return;
        }
    }

    let help_open = ctx.active_modal == ActiveModal::Help;

    for (_, interaction, action) in &buttons {
        // Live in-place tooltip on hover for tools and Patron Divine Fury (no HUD rebuild).
        if !help_open && ctx.armed_tool.is_none() && !input_state.confirm_loki_shuffle {
            match *interaction {
                Interaction::Hovered => {
                    let tip_txt = match *action {
                        Match3UiAction::SelectTool(tool) => {
                            Some(ctx.data.i18n.t(tool_hint_key(tool)).to_string())
                        }
                        Match3UiAction::TriggerFury => ctx.active_session.as_ref().map(|s| {
                            format_patron_desc(&ctx.data.i18n, s.patron, &ctx.data.balance.session)
                        }),
                        _ => None,
                    };
                    if let Some(txt) = tip_txt {
                        for (mut t, mut node) in &mut tooltip_q {
                            t.0 = txt.clone();
                            node.display = Display::Flex;
                        }
                    }
                }
                Interaction::None => {
                    if matches!(
                        action,
                        Match3UiAction::SelectTool(_) | Match3UiAction::TriggerFury
                    ) {
                        for (mut t, mut node) in &mut tooltip_q {
                            t.0.clear();
                            node.display = Display::None;
                        }
                    }
                }
                Interaction::Pressed => {}
            }
        }
    }

    // Actions fire on release so a touch that starts a scroll in Help never triggers a HUD button.
    let tapped = press_arm.completed_taps(
        buttons.iter().map(|(entity, i, _)| (entity, *i)),
        &touches,
        &scroll_mem,
    );
    for (entity, _, action) in &buttons {
        if !tapped.contains(&entity) {
            continue;
        }
        if help_open
            && matches!(
                action,
                Match3UiAction::Retreat
                    | Match3UiAction::Undo
                    | Match3UiAction::SelectTool(_)
                    | Match3UiAction::CancelTool
                    | Match3UiAction::TriggerFury
            )
        {
            continue;
        }
        match *action {
            Match3UiAction::Retreat => {
                if matches!(input_state.phase, BoardAnimPhase::VictoryCelebration { .. }) {
                    continue;
                }
                if !input_state.confirm_retreat {
                    input_state.confirm_retreat = true;
                    input_state.confirm_loki_shuffle = false;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    input_state.dirty_hud = true;
                    continue;
                }
                let was_expedition = ctx
                    .active_session
                    .as_ref()
                    .map(|s| {
                        s.level.building_id == wyrdstone_core::expedition::EXPEDITION_BUILDING_ID
                    })
                    .unwrap_or(false);
                if was_expedition {
                    ctx.active_expedition = None;
                }
                if ctx
                    .active_session
                    .as_ref()
                    .is_some_and(|s| s.level.building_id == wyrdstone_core::GAUNTLET_BUILDING_ID)
                {
                    ctx.active_gauntlet = None;
                }
                let is_trial_retreat = ctx.active_session.as_ref().is_some_and(|s| s.is_trial);
                if !is_trial_retreat {
                    ctx.city.reset_win_streak();
                }
                ctx.persist();
                ctx.active_session = None;
                ctx.armed_tool = None;
                ctx.active_modal = ActiveModal::None;
                scroll_mem.modal_offset_y = 0.0;
                next_state.set(AppState::FjordCity);
                return;
            }
            Match3UiAction::Undo => {
                input_state.confirm_retreat = false;
                input_state.confirm_loki_shuffle = false;
                execute_undo(&mut commands, &mut ctx, &sound_bank, &mut input_state);
            }
            Match3UiAction::ToggleHelp => {
                input_state.confirm_retreat = false;
                input_state.confirm_loki_shuffle = false;
                input_state.confirm_restart = false;
                ctx.toggle_modal(ActiveModal::Help);
                if ctx.active_modal == ActiveModal::Help {
                    ctx.armed_tool = None;
                    input_state.touch_tool_cell = None;
                    input_state.overlays_stale = true;
                }
                scroll_mem.modal_offset_y = 0.0;
                input_state.dirty_hud = true;
            }
            Match3UiAction::ToggleAudio => {
                toggle_mute(&mut ctx, &mut sound_bank, &mut bgm_sinks);
                input_state.dirty_hud = true;
            }
            Match3UiAction::CycleBgmVolume => {
                cycle_bgm_volume(&mut ctx, &mut sound_bank, &mut bgm_sinks);
                input_state.dirty_hud = true;
            }
            Match3UiAction::CycleSfxVolume => {
                cycle_sfx_volume(&mut ctx, &mut sound_bank);
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                input_state.dirty_hud = true;
            }
            Match3UiAction::ToggleColorblind => {
                ctx.colorblind_mode = !ctx.colorblind_mode;
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                input_state.dirty_board = true;
                input_state.dirty_hud = true;
            }
            Match3UiAction::ToggleFastAnim => {
                ctx.fast_animations = !ctx.fast_animations;
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                input_state.dirty_hud = true;
            }
            Match3UiAction::ToggleReducedMotion => {
                ctx.reduced_motion = !ctx.reduced_motion;
                input_state.reduced_motion = ctx.reduced_motion;
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                input_state.dirty_hud = true;
            }
            Match3UiAction::RestartBoard => {
                let is_trial_now = ctx.active_session.as_ref().is_some_and(|s| s.is_trial);
                if !is_trial_now && !input_state.confirm_restart {
                    input_state.confirm_restart = true;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    input_state.dirty_hud = true;
                    continue;
                }
                input_state.confirm_restart = false;
                if let Some(level_id) = ctx.active_level_id.clone() {
                    let res = if is_trial_now {
                        ctx.start_trial(&level_id)
                    } else if level_id.starts_with("gauntlet_") {
                        ctx.start_gauntlet()
                    } else if level_id.starts_with("boss_") {
                        ctx.start_boss_raid(&level_id)
                    } else {
                        ctx.start_level(&level_id)
                    };
                    if res.is_ok() {
                        *input_state = Match3InputState {
                            reduced_motion: ctx.reduced_motion,
                            hud_mode: input_state.hud_mode,
                            dirty_static_board: true,
                            dirty_board: true,
                            dirty_hud: true,
                            ..default()
                        };
                        ctx.active_modal = ActiveModal::None;
                        scroll_mem.modal_offset_y = 0.0;
                        play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    } else {
                        play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                        haptics.send(HapticEvent(HapticKind::Error));
                        input_state.dirty_hud = true;
                    }
                }
            }
            Match3UiAction::TriggerFury => {
                input_state.confirm_retreat = false;
                input_state.confirm_loki_shuffle = false;
                let ready = ctx
                    .active_session
                    .as_ref()
                    .map(|s| s.can_trigger_fury())
                    .unwrap_or(false);
                if !ready {
                    if let Some(patron) = ctx.active_session.as_ref().map(|s| s.patron) {
                        let txt =
                            format_patron_desc(&ctx.data.i18n, patron, &ctx.data.balance.session);
                        for (mut t, mut node) in &mut tooltip_q {
                            t.0 = txt.clone();
                            node.display = Display::Flex;
                        }
                    }
                    haptics.send(HapticEvent(HapticKind::Error));
                }
                execute_patron_fury(&mut commands, &mut ctx, &vfx, &sound_bank, &mut input_state);
                if ready {
                    haptics.send(HapticEvent(HapticKind::BoosterBlast));
                }
            }
            Match3UiAction::SelectTool(tool) => {
                if !input_state.is_idle() {
                    continue;
                }
                if ctx.city.tools.count(tool) == 0 {
                    let txt = ctx.data.i18n.t(tool_hint_key(tool)).to_string();
                    for (mut t, mut node) in &mut tooltip_q {
                        t.0 = txt.clone();
                        node.display = Display::Flex;
                    }
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    haptics.send(HapticEvent(HapticKind::Error));
                    continue;
                }
                input_state.confirm_retreat = false;
                if tool == ActiveTool::LokiShuffle {
                    if input_state.confirm_loki_shuffle {
                        input_state.confirm_loki_shuffle = false;
                        execute_loki_shuffle(
                            &mut commands,
                            &mut ctx,
                            &vfx,
                            &sound_bank,
                            &mut input_state,
                        );
                        haptics.send(HapticEvent(HapticKind::BoosterBlast));
                    } else {
                        ctx.armed_tool = None;
                        input_state.confirm_loki_shuffle = true;
                        input_state.overlays_stale = true;
                        play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                        input_state.dirty_hud = true;
                    }
                } else if ctx.armed_tool == Some(tool) {
                    input_state.confirm_loki_shuffle = false;
                    ctx.armed_tool = None;
                    input_state.overlays_stale = true;
                    input_state.dirty_hud = true;
                } else {
                    input_state.confirm_loki_shuffle = false;
                    ctx.armed_tool = Some(tool);
                    input_state.selected = None;
                    input_state.overlays_stale = true;
                    input_state.dirty_hud = true;
                }
            }
            Match3UiAction::CancelTool => {
                ctx.armed_tool = None;
                input_state.confirm_loki_shuffle = false;
                input_state.overlays_stale = true;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                input_state.dirty_hud = true;
            }
        }
    }
}

type TooltipTextQuery<'w, 's> =
    Query<'w, 's, (&'static mut Text, &'static mut Node), With<Match3TooltipText>>;

type HudBlockerQueries<'w, 's> = (
    Query<'w, 's, &'static Interaction, (Changed<Interaction>, With<Button>)>,
    Query<'w, 's, &'static Interaction, (With<Match3HudBar>, Without<Button>)>,
    TooltipTextQuery<'w, 's>,
);

/// Handles both desktop mouse (click + drag) and Android touch (tap + swipe) when the board is idle.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn handle_board_pointer_input(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut ctx: ResMut<GameContext>,
    textures: Res<RuneTextures>,
    vfx: Res<VfxTextures>,
    (sound_bank, mut haptics): (Res<SoundBank>, EventWriter<HapticEvent>),
    mut input_state: ResMut<Match3InputState>,
    hover_overlays: Query<Entity, With<ToolHoverOverlay>>,
    drag_highlights: Query<Entity, With<DragTargetHighlight>>,
    selection_markers: Query<Entity, With<SelectionMarker>>,
    kbd_markers: Query<Entity, With<KeyboardCursorMarker>>,
    (ui_buttons, hud_bars, mut tooltip_q): HudBlockerQueries,
) {
    if matches!(
        input_state.phase,
        BoardAnimPhase::VictoryCelebration { .. } | BoardAnimPhase::DefeatBeat { .. }
    ) {
        if (mouse.just_pressed(MouseButton::Left) || touches.iter_just_pressed().next().is_some())
            && !ui_buttons.iter().any(|i| *i == Interaction::Pressed)
            && !hud_bars.iter().any(|i| *i == Interaction::Pressed)
        {
            input_state.skip_valhalla = true;
        }
        return;
    }
    if !input_state.is_idle() || ctx.active_modal == ActiveModal::Help {
        for e in &hover_overlays {
            commands.entity(e).try_despawn();
        }
        for e in &drag_highlights {
            commands.entity(e).try_despawn();
        }
        input_state.hover_cell = None;
        input_state.drag_highlight_cell = None;
        input_state.overlays_stale = false;

        // Buffer an upcoming swipe during late Falling / SwapRevert frames so rapid chaining never drops input.
        if ctx.active_modal != ActiveModal::Help
            && ctx.armed_tool.is_none()
            && matches!(
                input_state.phase,
                BoardAnimPhase::Falling { .. } | BoardAnimPhase::SwapRevert { .. }
            )
            && !ui_buttons.iter().any(|i| *i == Interaction::Pressed)
            && !hud_bars.iter().any(|i| *i == Interaction::Pressed)
        {
            if let (Ok(window), Ok((camera, cam_transform)), Some(session)) = (
                windows.get_single(),
                camera_q.get_single(),
                &ctx.active_session,
            ) {
                let (cols, rows) = (session.board.cols(), session.board.rows());
                let swipe_threshold = ctx
                    .data
                    .balance
                    .swipe_threshold_px
                    .max(input_state.cell_step * 0.36);
                let mut buf_press: Option<Vec2> = None;
                let mut buf_curr: Option<Vec2> = None;
                let mut buf_rel = false;
                if let Some(touch) = touches.iter_just_pressed().next() {
                    input_state.active_touch_id = Some(touch.id());
                    buf_press = Some(touch.position());
                } else if let Some(tid) = input_state.active_touch_id {
                    if let Some(touch) = touches.get_pressed(tid) {
                        buf_curr = Some(touch.position());
                    } else {
                        input_state.active_touch_id = None;
                        buf_rel = true;
                    }
                }
                if buf_press.is_none() && input_state.active_touch_id.is_none() {
                    if mouse.just_pressed(MouseButton::Left) {
                        buf_press = window.cursor_position();
                    } else if mouse.pressed(MouseButton::Left) {
                        buf_curr = window.cursor_position();
                    } else if mouse.just_released(MouseButton::Left) {
                        buf_rel = true;
                    }
                }
                if let Some(sp) = buf_press {
                    if let Ok(wp) = camera.viewport_to_world_2d(cam_transform, sp) {
                        if let Some(cell) = world_to_cell(wp, cols, rows, &input_state) {
                            input_state.drag_origin_cell = Some(cell);
                            input_state.drag_origin_screen = Some(sp);
                        }
                    }
                } else if let (Some(curr), Some(origin_cell), Some(origin_screen)) = (
                    buf_curr,
                    input_state.drag_origin_cell,
                    input_state.drag_origin_screen,
                ) {
                    let delta = curr - origin_screen;
                    if delta.length() >= swipe_threshold {
                        let nb = if delta.x.abs() >= delta.y.abs() {
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
                        if let Some(nb) = nb {
                            input_state.buffered_swap = Some((origin_cell, nb));
                        }
                        input_state.drag_origin_cell = None;
                        input_state.drag_origin_screen = None;
                    }
                }
                if buf_rel {
                    input_state.drag_origin_cell = None;
                    input_state.drag_origin_screen = None;
                }
            }
        }
        return;
    }
    // A HUD button or HUD bar pressed this frame owns the click: never let it fall through to the board.
    if ui_buttons.iter().any(|i| *i == Interaction::Pressed)
        || hud_bars.iter().any(|i| *i == Interaction::Pressed)
    {
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

    // Live hover highlight: AoE preview when a tool is armed (tracks mouse or keyboard cursor
    // depending on which input was used most recently), or soft luminous rune halo when idle.
    let cursor_pos = window.cursor_position();
    if cursor_pos != input_state.last_cursor_pos {
        input_state.last_cursor_pos = cursor_pos;
        if cursor_pos.is_some() {
            input_state.last_input_was_kbd = false;
        }
    }
    let pointer_hovered = cursor_pos
        .and_then(|sp| camera.viewport_to_world_2d(cam_transform, sp).ok())
        .and_then(|wp| world_to_cell(wp, cols, rows, &input_state));
    let hovered = if ctx.armed_tool.is_some() {
        if let Some(tc) = input_state.touch_tool_cell {
            Some(tc)
        } else if input_state.last_input_was_kbd {
            input_state.kbd_cursor.or(pointer_hovered)
        } else {
            pointer_hovered.or(input_state.kbd_cursor)
        }
    } else {
        input_state.touch_tool_cell = None;
        pointer_hovered
    };
    // A board rebuild leaves overlays behind even when the hovered cell did not change (or is
    // now empty): redraw them from scratch and drop any stale drag highlight.
    let overlays_stale = std::mem::take(&mut input_state.overlays_stale);
    if overlays_stale {
        for e in &drag_highlights {
            commands.entity(e).try_despawn();
        }
    }
    if hovered != input_state.hover_cell || overlays_stale {
        input_state.hover_cell = hovered;
        for e in &hover_overlays {
            commands.entity(e).try_despawn();
        }
        if let (Some(tool), Some(cell), Some(session)) =
            (ctx.armed_tool, hovered, &ctx.active_session)
        {
            for p in session.board.tool_preview_cells(tool, cell) {
                let w = cell_to_world(p, &input_state);
                commands.spawn((
                    Match3Cleanup,
                    ToolHoverOverlay,
                    Sprite {
                        color: Color::srgba(0.98, 0.82, 0.28, 0.36),
                        custom_size: Some(Vec2::splat(input_state.cell_step * 0.94)),
                        ..default()
                    },
                    Transform::from_xyz(w.x, w.y, Z_SELECTION_RING - 0.2),
                ));
            }
        } else if let (None, Some(cell), Some(session)) =
            (ctx.armed_tool, hovered, &ctx.active_session)
        {
            let playable = session
                .board
                .get(cell)
                .map(|c| !c.blocked && !c.fog)
                .unwrap_or(false);
            if playable {
                let w = cell_to_world(cell, &input_state);
                commands.spawn((
                    Match3Cleanup,
                    ToolHoverOverlay,
                    Sprite {
                        image: vfx.soft_orb.clone(),
                        color: Color::srgba(1.0, 0.92, 0.62, 0.30),
                        custom_size: Some(Vec2::splat(input_state.cell_step * 1.18)),
                        ..default()
                    },
                    Transform::from_xyz(w.x, w.y, Z_CELL_BG + 0.8),
                ));
            }
        }
    }

    let swipe_threshold = ctx
        .data
        .balance
        .swipe_threshold_px
        .max(input_state.cell_step * 0.36);
    let mut pressed_screen: Option<Vec2> = None;
    let mut current_screen: Option<Vec2> = None;
    let mut released = false;
    let mut is_touch_event = false;

    if let Some(touch) = touches.iter_just_pressed().next() {
        input_state.active_touch_id = Some(touch.id());
        pressed_screen = Some(touch.position());
        is_touch_event = true;
    } else if let Some(tid) = input_state.active_touch_id {
        is_touch_event = true;
        if let Some(touch) = touches.get_pressed(tid) {
            current_screen = Some(touch.position());
        } else {
            input_state.active_touch_id = None;
            released = true;
        }
    }

    if pressed_screen.is_none() && input_state.active_touch_id.is_none() && !is_touch_event {
        if mouse.just_pressed(MouseButton::Left) {
            pressed_screen = window.cursor_position();
        } else if mouse.pressed(MouseButton::Left) {
            current_screen = window.cursor_position();
        } else if mouse.just_released(MouseButton::Left) {
            released = true;
        }
    }

    // Touch tool targeting: preview AoE while finger drags, strike cleanly on release (or drag off-board to cancel).
    if ctx.armed_tool.is_some() && is_touch_event {
        if let Some(sp) = pressed_screen.or(current_screen) {
            input_state.clear_hint();
            if input_state.kbd_cursor.is_some() {
                input_state.kbd_cursor = None;
                update_kbd_cursor_marker(&mut commands, &input_state, &kbd_markers);
            }
            let cell = camera
                .viewport_to_world_2d(cam_transform, sp)
                .ok()
                .and_then(|wp| world_to_cell(wp, cols, rows, &input_state));
            input_state.touch_tool_cell = cell;
        }
        if released {
            let target_cell = input_state.touch_tool_cell.take();
            if let (Some(tool), Some(cell)) = (ctx.armed_tool, target_cell) {
                if input_state.confirm_retreat
                    || input_state.confirm_loki_shuffle
                    || input_state.confirm_restart
                {
                    input_state.confirm_retreat = false;
                    input_state.confirm_loki_shuffle = false;
                    input_state.confirm_restart = false;
                    input_state.dirty_hud = true;
                }
                match start_tool_turn(&mut ctx, tool, cell) {
                    Ok((turn, preview)) => {
                        ctx.armed_tool = None;
                        ctx.persist();
                        input_state.undo_snapshot = None;
                        play_sfx(&mut commands, &sound_bank, SfxKind::ToolBlast);
                        anim::spawn_tool_vfx(
                            &mut commands,
                            &vfx,
                            tool,
                            cell,
                            &preview.prepared.cells,
                            cols,
                            rows,
                            &input_state,
                        );
                        let clear_dur = ctx.anim_duration(ctx.data.balance.clear_anim_seconds);
                        let cleared_set: HashSet<CellPos> =
                            preview.prepared.cells.iter().copied().collect();
                        input_state.selected = None;
                        input_state.shake_secs = SHAKE_DURATION_SECS;
                        input_state.dirty_hud = true;
                        haptics.send(HapticEvent(HapticKind::BoosterBlast));
                        input_state.phase = BoardAnimPhase::Clearing {
                            turn,
                            preview,
                            cleared_set,
                            elapsed: 0.0,
                            duration: clear_dur,
                            particles_spawned: false,
                        };
                    }
                    Err(ToolStartError::NoStock) => {}
                    Err(ToolStartError::Invalid) => {
                        spawn_invalid_cell_flash(&mut commands, cell, &input_state);
                        play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                        haptics.send(HapticEvent(HapticKind::Error));
                    }
                }
            }
        }
        return;
    }

    if let Some(screen_pos) = pressed_screen {
        input_state.drag_cancelled = false;
        input_state.clear_hint();
        if input_state.kbd_cursor.is_some() {
            input_state.kbd_cursor = None;
            update_kbd_cursor_marker(&mut commands, &input_state, &kbd_markers);
        }
        if let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, screen_pos) {
            if let Some(cell) = world_to_cell(world_pos, cols, rows, &input_state) {
                if input_state.confirm_retreat
                    || input_state.confirm_loki_shuffle
                    || input_state.confirm_restart
                {
                    input_state.confirm_retreat = false;
                    input_state.confirm_loki_shuffle = false;
                    input_state.confirm_restart = false;
                    input_state.dirty_hud = true;
                }
                if let Some(tool) = ctx.armed_tool {
                    match start_tool_turn(&mut ctx, tool, cell) {
                        Ok((turn, preview)) => {
                            ctx.armed_tool = None;
                            ctx.persist();
                            input_state.undo_snapshot = None;
                            play_sfx(&mut commands, &sound_bank, SfxKind::ToolBlast);
                            anim::spawn_tool_vfx(
                                &mut commands,
                                &vfx,
                                tool,
                                cell,
                                &preview.prepared.cells,
                                cols,
                                rows,
                                &input_state,
                            );
                            let clear_dur = ctx.anim_duration(ctx.data.balance.clear_anim_seconds);
                            let cleared_set: HashSet<CellPos> =
                                preview.prepared.cells.iter().copied().collect();
                            input_state.selected = None;
                            input_state.shake_secs = SHAKE_DURATION_SECS;
                            input_state.dirty_hud = true;
                            haptics.send(HapticEvent(HapticKind::BoosterBlast));
                            input_state.phase = BoardAnimPhase::Clearing {
                                turn,
                                preview,
                                cleared_set,
                                elapsed: 0.0,
                                duration: clear_dur,
                                particles_spawned: false,
                            };
                        }
                        Err(ToolStartError::NoStock) => {}
                        Err(ToolStartError::Invalid) => {
                            spawn_invalid_cell_flash(&mut commands, cell, &input_state);
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                            haptics.send(HapticEvent(HapticKind::Error));
                        }
                    }
                    return;
                }

                if let Some(prev) = input_state.selected {
                    if prev == cell {
                        input_state.selected = None;
                        update_selection_marker(
                            &mut commands,
                            &textures,
                            &input_state,
                            &selection_markers,
                        );
                        return;
                    }
                    if prev.is_adjacent(cell) {
                        if start_swap_animation(prev, cell, &ctx, &mut input_state) {
                            update_selection_marker(
                                &mut commands,
                                &textures,
                                &input_state,
                                &selection_markers,
                            );
                            play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                            haptics.send(HapticEvent(HapticKind::LightTap));
                        } else {
                            update_selection_marker(
                                &mut commands,
                                &textures,
                                &input_state,
                                &selection_markers,
                            );
                            spawn_invalid_cell_flash(&mut commands, prev, &input_state);
                            spawn_invalid_cell_flash(&mut commands, cell, &input_state);
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                            haptics.send(HapticEvent(HapticKind::Error));
                        }
                        return;
                    }
                }

                let cell_state = ctx.active_session.as_ref().and_then(|s| s.board.get(cell));
                let playable = cell_state.is_some_and(|c| !c.blocked && c.has_movable_piece());
                if playable {
                    input_state.selected = Some(cell);
                    input_state.drag_origin_cell = Some(cell);
                    input_state.drag_origin_screen = Some(screen_pos);
                    update_selection_marker(
                        &mut commands,
                        &textures,
                        &input_state,
                        &selection_markers,
                    );
                } else if cell_state.is_some_and(|c| !c.blocked) {
                    input_state.selected = None;
                    update_selection_marker(
                        &mut commands,
                        &textures,
                        &input_state,
                        &selection_markers,
                    );
                    spawn_invalid_cell_flash(&mut commands, cell, &input_state);
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    haptics.send(HapticEvent(HapticKind::Error));
                }
            }
        }
    }

    if let (Some(curr), Some(origin_cell), Some(origin_screen)) = (
        current_screen,
        input_state.drag_origin_cell,
        input_state.drag_origin_screen,
    ) {
        let delta = curr - origin_screen;
        if delta.length() >= swipe_threshold * 0.4 {
            input_state.drag_cancelled = true;
        }
        let candidate_target = if delta.length() >= swipe_threshold * 0.4 {
            if delta.x.abs() >= delta.y.abs() {
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
            }
        } else {
            None
        };

        if delta.length() >= swipe_threshold {
            input_state.drag_cancelled = false;
            input_state.drag_origin_cell = None;
            input_state.drag_origin_screen = None;
            input_state.drag_highlight_cell = None;
            for e in &drag_highlights {
                commands.entity(e).try_despawn();
            }
            if ctx.armed_tool.is_none() && !input_state.confirm_loki_shuffle {
                for (mut t, mut node) in &mut tooltip_q {
                    t.0.clear();
                    node.display = Display::None;
                }
            }
            if let Some(nb) = candidate_target {
                if start_swap_animation(origin_cell, nb, &ctx, &mut input_state) {
                    update_selection_marker(
                        &mut commands,
                        &textures,
                        &input_state,
                        &selection_markers,
                    );
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    haptics.send(HapticEvent(HapticKind::LightTap));
                } else {
                    update_selection_marker(
                        &mut commands,
                        &textures,
                        &input_state,
                        &selection_markers,
                    );
                    spawn_invalid_cell_flash(&mut commands, origin_cell, &input_state);
                    spawn_invalid_cell_flash(&mut commands, nb, &input_state);
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    haptics.send(HapticEvent(HapticKind::Error));
                }
            }
        } else if candidate_target != input_state.drag_highlight_cell {
            input_state.drag_highlight_cell = candidate_target;
            for e in &drag_highlights {
                commands.entity(e).try_despawn();
            }
            if let Some(hl) = candidate_target {
                let fusion = ctx
                    .active_session
                    .as_ref()
                    .and_then(|s| s.board.preview_booster_fusion(origin_cell, hl));
                if let Some((fusion_kind, cells)) = fusion {
                    for p in cells {
                        let w = cell_to_world(p, &input_state);
                        commands.spawn((
                            Match3Cleanup,
                            DragTargetHighlight,
                            Sprite {
                                color: Color::srgba(1.0, 0.84, 0.24, 0.42),
                                custom_size: Some(Vec2::splat(input_state.cell_step * 0.94)),
                                ..default()
                            },
                            Transform::from_xyz(w.x, w.y, Z_SELECTION_RING - 0.15),
                        ));
                    }
                    let tip = ctx.data.i18n.t(fusion_kind.name_key()).to_string();
                    for (mut t, mut node) in &mut tooltip_q {
                        t.0 = tip.clone();
                        node.display = Display::Flex;
                    }
                } else {
                    let w = cell_to_world(hl, &input_state);
                    commands.spawn((
                        Match3Cleanup,
                        DragTargetHighlight,
                        Sprite {
                            color: Color::srgba(0.45, 0.90, 1.0, 0.32),
                            custom_size: Some(Vec2::splat(input_state.cell_step * 0.94)),
                            ..default()
                        },
                        Transform::from_xyz(w.x, w.y, Z_SELECTION_RING - 0.15),
                    ));
                    if ctx.armed_tool.is_none() && !input_state.confirm_loki_shuffle {
                        for (mut t, mut node) in &mut tooltip_q {
                            t.0.clear();
                            node.display = Display::None;
                        }
                    }
                }
            } else if ctx.armed_tool.is_none() && !input_state.confirm_loki_shuffle {
                for (mut t, mut node) in &mut tooltip_q {
                    t.0.clear();
                    node.display = Display::None;
                }
            }
        }
    }

    if released {
        if std::mem::take(&mut input_state.drag_cancelled) {
            input_state.selected = None;
            update_selection_marker(&mut commands, &textures, &input_state, &selection_markers);
        }
        input_state.drag_origin_cell = None;
        input_state.drag_origin_screen = None;
        input_state.drag_highlight_cell = None;
        for e in &drag_highlights {
            commands.entity(e).try_despawn();
        }
        if ctx.armed_tool.is_none() && !input_state.confirm_loki_shuffle {
            for (mut t, mut node) in &mut tooltip_q {
                t.0.clear();
                node.display = Display::None;
            }
        }
    }
}

pub(super) fn start_swap_animation(
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
    input_state.drag_highlight_cell = None;
    if !can_swap {
        return false;
    }
    input_state.phase = BoardAnimPhase::Swapping {
        a,
        b,
        elapsed: 0.0,
        duration: ctx.anim_duration(ctx.data.balance.swap_anim_seconds),
    };
    true
}

/// Rebuilds static/dynamic board sprites and/or HUD when dirty flags are raised.
#[allow(clippy::too_many_arguments)]
pub fn sync_match3_visuals(
    mut commands: Commands,
    ctx: Res<GameContext>,
    textures: Res<RuneTextures>,
    vfx: Res<VfxTextures>,
    fjord_art: Res<FjordTextures>,
    sound_bank: Res<SoundBank>,
    (scroll_mem, safe_area): (Res<UiScrollMemory>, Res<SafeAreaInsets>),
    windows: Query<&Window, With<PrimaryWindow>>,
    mut input_state: ResMut<Match3InputState>,
    static_sprites: Query<Entity, With<BoardStaticEntity>>,
    sprites: Query<Entity, With<Match3Entity>>,
    huds: Query<Entity, With<Match3HudRoot>>,
    mut hud_texts: hud::HudTextQueries,
    mut was_idle: Local<Option<bool>>,
) {
    // The HUD derives Idle-dependent state (Undo, tools); refresh it on any Idle edge.
    let idle_now = input_state.is_idle();
    match *was_idle {
        None => *was_idle = Some(idle_now),
        Some(prev) if prev != idle_now => {
            *was_idle = Some(idle_now);
            input_state.dirty_hud = true;
        }
        _ => {}
    }
    // The HUD shows the mute state and volume labels; refresh when the sound bank changes.
    if sound_bank.is_changed() {
        input_state.dirty_hud = true;
    }
    if input_state.dirty_static_board {
        if let (Some(session), Ok(window)) = (&ctx.active_session, windows.get_single()) {
            for e in &static_sprites {
                commands.entity(e).try_despawn_recursive();
            }
            compute_board_layout(
                window.width(),
                window.height(),
                session.board.cols(),
                session.board.rows(),
                *safe_area,
                &mut input_state,
            );
            spawn_board_static_sprites(&mut commands, session, &textures, &vfx, &input_state);
        }
        input_state.dirty_static_board = false;
    }

    if input_state.dirty_board {
        if let (Some(session), Ok(window)) = (&ctx.active_session, windows.get_single()) {
            rebuild_board_now(
                &mut commands,
                session,
                &textures,
                &vfx,
                window,
                *safe_area,
                &mut input_state,
                &sprites,
                None,
                ctx.colorblind_mode,
            );
        }
        input_state.dirty_board = false;
    }

    if input_state.dirty_hud_soft && !input_state.dirty_hud {
        input_state.dirty_hud_soft = false;
        let new_shape = ctx
            .active_session
            .as_ref()
            .map(|s| hud::hud_shape(s, &ctx.data.balance.session, ctx.city.stats.total_victories));
        match new_shape {
            Some(shape) if hud::can_update_in_place(input_state.hud_shape.as_ref(), &shape) => {
                hud_texts.refresh(&ctx, input_state.is_compact_hud());
            }
            _ => input_state.dirty_hud = true,
        }
    }
    input_state.dirty_hud_soft = false;

    if input_state.dirty_hud {
        input_state.hud_shape = ctx
            .active_session
            .as_ref()
            .map(|s| hud::hud_shape(s, &ctx.data.balance.session, ctx.city.stats.total_victories));
        for e in &huds {
            commands.entity(e).try_despawn_recursive();
        }
        hud::spawn_match3_hud(
            &mut commands,
            &ctx,
            &fjord_art,
            &textures,
            &vfx,
            &sound_bank,
            scroll_mem.modal_offset_y,
            input_state.hud_mode,
            *safe_area,
            input_state.can_undo(&ctx),
            input_state.undo_used,
            input_state.confirm_retreat,
            input_state.confirm_loki_shuffle,
            input_state.confirm_restart,
            matches!(input_state.phase, BoardAnimPhase::VictoryCelebration { .. }),
        );
        input_state.dirty_hud = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_hint_flags_board_dirty_only_when_a_hint_was_showing() {
        let mut state = Match3InputState {
            hint_pair: Some((CellPos::new(0, 0), CellPos::new(1, 0))),
            idle_secs: 7.0,
            ..default()
        };
        state.clear_hint();
        assert!(state.hint_pair.is_none());
        assert!(state.dirty_board);
        assert_eq!(state.idle_secs, 0.0);

        state.dirty_board = false;
        state.idle_secs = 3.0;
        state.clear_hint();
        assert!(!state.dirty_board, "no hint showing: nothing to rebuild");
        assert_eq!(state.idle_secs, 0.0);
    }

    #[test]
    fn overlays_stale_flag_is_consumed_exactly_once() {
        let mut state = Match3InputState::default();
        assert!(!state.overlays_stale);
        state.overlays_stale = true;
        let hover_changed = false;
        assert!(hover_changed || std::mem::take(&mut state.overlays_stale));
        assert!(!state.overlays_stale, "flag must be lowered after use");
        assert!(!std::mem::take(&mut state.overlays_stale));
    }

    #[test]
    fn clear_hint_resets_hint_checked() {
        let mut state = Match3InputState {
            hint_checked: true,
            idle_secs: 6.0,
            ..default()
        };
        state.clear_hint();
        assert!(!state.hint_checked);
        assert_eq!(state.idle_secs, 0.0);
    }

    #[test]
    fn desktop_wide_board_fits_between_side_rails_and_uses_vertical_space() {
        let mut state = Match3InputState::default();
        let safe = SafeAreaInsets::default();
        compute_board_layout(1920.0, 1080.0, 8, 8, safe, &mut state);
        assert_eq!(state.hud_mode, HudLayoutMode::DesktopWide);
        assert!(!state.is_compact_hud());
        let board_w = state.cell_step * 8.0 + 28.0;
        let rail_clearance = 1920.0 - (DESKTOP_SIDE_RAIL_WIDTH_PX + 24.0) * 2.0;
        assert!(
            board_w <= rail_clearance + 0.5,
            "board_w ({board_w}) exceeds central band ({rail_clearance})"
        );
        assert!(
            state.cell_step >= 115.0,
            "1080p desktop board should reach near-max tile size, got {}",
            state.cell_step
        );
    }

    #[test]
    fn portrait_board_fits_phone_viewport_with_safe_area() {
        let mut state = Match3InputState::default();
        let safe = SafeAreaInsets {
            top: 28.0,
            bottom: 20.0,
            left: 0.0,
            right: 0.0,
        };
        compute_board_layout(360.0, 780.0, 8, 8, safe, &mut state);
        assert_eq!(state.hud_mode, HudLayoutMode::Portrait);
        assert!(state.is_compact_hud());
        let board_w = state.cell_step * 8.0 + 28.0;
        assert!(board_w <= 360.0 - 24.0 + 0.5);
        assert!(state.cell_step >= MIN_TILE_SIZE_PX);
    }
}
