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

//! Interactive painted Fjord village map, building inspector drawer, passive harvests, and modals.

mod drawers;
mod inspector;
mod modals;

use crate::audio::{
    cycle_bgm_volume, cycle_sfx_volume, play_sfx, toggle_mute, BgmTrack, SfxKind, SoundBank,
};
use crate::fjord_art::FjordTextures;
use crate::scroll::{PressArm, UiScrollMemory};
use crate::state::{
    now_unix_secs, today_day_index, ActiveModal, AppState, ChapterDialogue, GameContext,
};
use crate::theme::*;
use crate::ui_style::{
    button_shadow, display_font, gold_glow_shadow, modal_shadow, panel_shadow, sliced_panel_image,
    spawn_notice_toast, NoButtonScale, NOTICE_TOAST_DURATION_SECS,
};
use crate::vfx_art::VfxTextures;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy::window::{PrimaryWindow, WindowResized};
use wyrdstone_core::{
    format_rank_perk, ActiveTool, BuildingDef, I18nCatalog, PatronGod, QuestCondition,
};

const CELEBRATION_DURATION_SECS: f32 = 3.2;
const BACKUP_NOTICE_DURATION_SECS: f32 = 6.0;
const SNOWFLAKE_COUNT: usize = 22;
const HARVEST_POLL_SECS: f32 = 5.0;
/// Native aspect ratio of `assets/sprites/fjord_map.png` (1280x720 = 16:9).
/// Keeping the interactive Fjord map locked to this exact ratio guarantees identical
/// normalized building positions across every landscape resolution.
const FJORD_MAP_ASPECT: f32 = 1280.0 / 720.0;

#[derive(Component)]
pub struct CityRoot;

/// Full-viewport dimmed cover copy of `fjord_map.png` behind the 16:9 interactive canvas
/// so ultrawide or tall windows never display harsh flat letterbox bars.
#[derive(Component)]
pub struct FjordMapBackdrop;

/// Computes exact `(width, height)` for the interactive `FjordMapContainer`.
/// Landscape mode always preserves the exact 16:9 `fjord_map.png` ratio centered inside the viewport.
pub(crate) fn compute_fjord_canvas_size(win_w: f32, win_h: f32, is_portrait: bool) -> (f32, f32) {
    if is_portrait {
        return (win_w.max(1.0), win_h.max(1.0));
    }
    let w = win_w.min(win_h * FJORD_MAP_ASPECT).max(1.0);
    let h = (w / FJORD_MAP_ASPECT).max(1.0);
    (w, h)
}

fn compute_backdrop_cover_size(win_w: f32, win_h: f32) -> (f32, f32) {
    let w = win_w.max(win_h * FJORD_MAP_ASPECT).max(1.0);
    let h = (w / FJORD_MAP_ASPECT).max(1.0);
    (w, h)
}

/// Marker on the Fjord map image container used to compute exact pixel anchor coordinates.
#[derive(Component)]
pub struct FjordMapContainer;

/// Stores normalized terrain anchor coordinates (`0..=100%`) and tier scale for resolution-independent placement.
#[derive(Component)]
pub struct MapBuildingPin {
    pub map_x_pct: f32,
    pub map_y_pct: f32,
    pub portrait_left_pct: Option<f32>,
    pub portrait_top_pct: Option<f32>,
    pub tier_scale: f32,
}

/// Marker on the building sprite stage inside a `MapBuildingPin` so its size scales proportionally with the map.
#[derive(Component)]
pub struct MapBuildingStage {
    pub tier_scale: f32,
}

/// Marker on the building pin targeted by the active Saga quest so its border pulses gold.
#[derive(Component)]
pub struct QuestBeaconPin;

/// Animated Aurora Borealis ribbon drifting softly across the Fjord sky.
#[derive(Component)]
pub struct FjordAurora {
    pub base_left_pct: f32,
    pub phase: f32,
    pub base_alpha: f32,
}

/// Gentle falling snowflake drifting across the Fjord map.
#[derive(Component)]
pub struct FjordSnowflake {
    pub base_x_pct: f32,
    pub base_y_pct: f32,
    pub x_pct: f32,
    pub y_pct: f32,
    pub speed_y: f32,
    pub phase: f32,
}

/// Rising warm ember spark glowing on rebuilt (`tier > 0`) Fjord buildings.
#[derive(Component)]
pub struct FjordEmber {
    pub base_x_pct: f32,
    pub base_y_pct: f32,
    pub y_pct: f32,
    pub phase: f32,
}

/// Gently rippling aurora reflection over the central Fjord bay water.
#[derive(Component)]
pub struct FjordWaterShimmer {
    pub base_left_pct: f32,
    pub phase: f32,
    pub base_alpha: f32,
}

/// Soft chimney smoke puff drifting above rebuilt Forge, Meadery, and Great Hall roofs.
#[derive(Component)]
pub struct FjordSmokePuff {
    pub base_y_pct: f32,
    pub y_pct: f32,
    pub phase: f32,
}

/// Flickering signal fire beacon halo atop a rebuilt Watchtower.
#[derive(Component)]
pub struct FjordBeaconGlow {
    pub phase: f32,
}

/// Elastic rise-from-ground animation played on a building right after construction or upgrade.
#[derive(Component)]
pub struct BuildingBuildAnim {
    pub elapsed: f32,
    pub duration: f32,
}

/// Expanding golden celebration ring spawned over a newly built or upgraded Fjord building.
#[derive(Component)]
pub struct BuildingBuildRing {
    pub elapsed: f32,
    pub duration: f32,
}

/// Marker on the top bar's Endurance pill text so periodic energy regen & countdown update in-place.
#[derive(Component)]
pub struct CityEnergyText;

/// Marker on the building inspector's passive tribute countdown so it ticks in-place every second.
#[derive(Component)]
pub struct InspectorHarvestTimerText;

/// Exclusive slide-out right drawer in the Fjord hub.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CityDrawer {
    #[default]
    None,
    Modes,
    Clan,
    Menu,
}

/// Tracks City layout mode, displayed endurance, audio state, and periodic harvest readiness checks.
#[derive(Resource)]
pub struct CityLayoutMode {
    pub is_portrait: bool,
    pub active_drawer: CityDrawer,
    pub inspector_open: bool,
    pub last_displayed_energy: u32,
    pub last_ready_harvests: usize,
    pub last_muted: bool,
    pub harvest_poll_timer: Timer,
    pub ui_clock_timer: Timer,
    pub just_built_id: Option<String>,
    /// Last notice key rendered by the hub toast, used to detect changes and restart its timer.
    pub last_notice_key: Option<String>,
    /// Seconds left before the current notice toast is cleared.
    pub notice_secs_left: f32,
    pub pending_chapter_delay_secs: f32,
    pub last_map_size: Vec2,
    pub last_canvas_window_size: Vec2,
    pub window_size: Vec2,
}

#[derive(Component)]
pub enum CityAction {
    SelectBuilding(String),
    CloseInspector,
    SelectDifficulty(u8),
    CyclePatron,
    CycleAmulet,
    ClaimLoginTribute,
    CollectHarvest(String),
    CollectAllHarvests,
    PlayBuildingLevel(String),
    PlayDailyTrial,
    StartExpedition,
    StartValhallaBlitz,
    StartTrial(String),
    StartGauntlet,
    PickExpeditionBoon(usize),
    ToggleExpeditionOath,
    EndExpedition,
    FightBoss(String),
    SelectPatron(PatronGod),
    ClaimAchievement(String),
    ClaimAllAchievements,
    UpgradeBuilding(String),
    DrinkMead,
    ClaimQuest(String),
    FocusActiveQuest,
    ToggleDrawer(CityDrawer),
    ToggleModal(ActiveModal),
    CloseModal,
    ClaimCollection(String),
    ClaimAllCollections,
    CraftTool(ActiveTool),
    BrewWarDraught(wyrdstone_core::WarDraught),
    CraftOrEquipAmulet(String),
    EquipSecondaryAmulet(String),
    UpgradeAmulet(String),
    AdvanceTutorial,
    ToggleColorblind,
    ToggleFastAnim,
    ToggleReducedMotion,
    CycleBgmVolume,
    CycleSfxVolume,
    ExportBackup,
    ImportBackup,
    ToggleLanguage,
    ToggleAudio,
    BackToTitle,
}

/// Spawns the Fjord City hub UI.
#[allow(clippy::too_many_arguments)]
pub fn setup_city_ui(
    mut commands: Commands,
    mut ctx: ResMut<GameContext>,
    fjord_art: Res<FjordTextures>,
    vfx: Res<VfxTextures>,
    sound_bank: Res<SoundBank>,
    scroll_mem: Res<UiScrollMemory>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let (win_w, win_h) = windows
        .get_single()
        .map(|w| (w.width(), w.height()))
        .unwrap_or((WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_HEIGHT));
    let is_portrait = win_w < win_h;
    if ctx.active_modal == ActiveModal::None {
        if ctx.city.stats.total_victories >= 1
            && ctx.city.is_login_tribute_available(today_day_index())
        {
            ctx.active_modal = ActiveModal::LoginCalendar;
        } else if ctx.pending_dialogue.is_some() {
            ctx.active_modal = ActiveModal::ChapterDialogue;
        }
    }
    let inspector_open = std::mem::take(&mut ctx.open_inspector_on_city_enter);
    let ready_harvests = count_ready_harvests(&ctx, now_unix_secs());
    commands.insert_resource(CityLayoutMode {
        is_portrait,
        active_drawer: CityDrawer::None,
        inspector_open,
        last_displayed_energy: ctx.city.energy,
        last_ready_harvests: ready_harvests,
        last_muted: sound_bank.muted,
        harvest_poll_timer: Timer::from_seconds(HARVEST_POLL_SECS, TimerMode::Repeating),
        ui_clock_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        just_built_id: None,
        last_notice_key: ctx.status_notice_key.clone(),
        notice_secs_left: NOTICE_TOAST_DURATION_SECS,
        pending_chapter_delay_secs: 0.0,
        last_map_size: Vec2::ZERO,
        last_canvas_window_size: Vec2::new(win_w, win_h),
        window_size: Vec2::new(win_w, win_h),
    });
    spawn_city_tree(
        &mut commands,
        &ctx,
        &fjord_art,
        &vfx,
        &sound_bank,
        &scroll_mem,
        is_portrait,
        win_w,
        win_h,
        CityDrawer::None,
        inspector_open,
        None,
    );
}

/// Cleans up the Fjord City UI on state exit.
pub fn cleanup_city_ui(mut commands: Commands, roots: Query<Entity, With<CityRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<CityLayoutMode>();
}

pub(super) fn format_countdown(i18n: &I18nCatalog, secs: u64) -> String {
    let mins = secs / 60;
    let rem = secs % 60;
    if mins > 0 {
        i18n.t_fmt(
            "city.time.min_sec",
            &[("mins", mins.to_string()), ("secs", format!("{rem:02}"))],
        )
    } else {
        i18n.t_fmt("city.time.sec", &[("secs", rem.to_string())])
    }
}

/// Formats the "title - reward summary" line shown in claim celebrations.
fn celebration_claim_line(i18n: &I18nCatalog, title: String, summary: String) -> String {
    i18n.t_fmt(
        "city.celebration.claim_line",
        &[("title", title), ("summary", summary)],
    )
}

/// Formats one resource pill text ("Wood: 120") through the shared locale template.
fn resource_pill_text(i18n: &I18nCatalog, label_key: &str, amount: u32) -> String {
    i18n.t_fmt(
        "city.header.resource_pill",
        &[
            ("label", i18n.t(label_key).to_string()),
            ("amount", amount.to_string()),
        ],
    )
}

fn format_energy_label(ctx: &GameContext) -> String {
    let i18n = &ctx.data.i18n;
    let max_energy = ctx.city.max_energy(&ctx.data.balance);
    if ctx.city.energy < max_energy {
        let speed_mult = 1.0
            + (ctx
                .city
                .compute_bonuses(&ctx.data.balance)
                .energy_regen_speed_pct as f32
                / 100.0);
        let rem_secs = (ctx.energy_regen_timer.remaining_secs() / speed_mult.max(0.1))
            .ceil()
            .max(1.0) as u64;
        i18n.t_fmt(
            "city.header.energy_regen",
            &[
                ("cur", ctx.city.energy.to_string()),
                ("max", max_energy.to_string()),
                ("time", format_countdown(i18n, rem_secs)),
            ],
        )
    } else {
        i18n.t_fmt(
            "city.header.energy_full",
            &[
                ("cur", ctx.city.energy.to_string()),
                ("max", max_energy.to_string()),
            ],
        )
    }
}

fn count_ready_harvests(ctx: &GameContext, now: u64) -> usize {
    ctx.data
        .buildings
        .iter()
        .filter(|b| ctx.city.is_harvest_ready(&b.id, now, &ctx.data.balance))
        .count()
}

/// Formats the "X (N)" badge label when `count > 0`, falling back to the bare `base_key` label.
pub(super) fn badged_label(i18n: &I18nCatalog, base_key: &str, count: usize) -> String {
    if count > 0 {
        i18n.t_fmt(
            "city.button.label_with_count",
            &[
                ("label", i18n.t(base_key).to_string()),
                ("count", count.to_string()),
            ],
        )
    } else {
        i18n.t(base_key).to_string()
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_city_tree(
    commands: &mut Commands,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    sound_bank: &SoundBank,
    scroll_mem: &UiScrollMemory,
    is_portrait: bool,
    win_w: f32,
    win_h: f32,
    active_drawer: CityDrawer,
    inspector_open: bool,
    just_built_id: Option<&str>,
) {
    let i18n = &ctx.data.i18n;
    let res = ctx.city.resources;
    let max_energy = ctx.city.max_energy(&ctx.data.balance);
    let now = now_unix_secs();
    let (cover_w, cover_h) = compute_backdrop_cover_size(win_w, win_h);
    let (canvas_w, canvas_h) = compute_fjord_canvas_size(win_w, win_h, is_portrait);
    let ready_harvests = count_ready_harvests(ctx, now);
    let daily_ready = ctx.city.is_daily_available(today_day_index());
    let claimable_feats = ctx.city.claimable_achievement_count(
        &ctx.data.achievements,
        &ctx.data.buildings,
        ctx.data.bosses.len(),
    );
    let claimable_cols = ctx.city.claimable_collection_count(&ctx.data.collections);
    let tribute_ready = ctx.city.is_login_tribute_available(today_day_index());
    let right_panel_open = inspector_open || active_drawer != CityDrawer::None;

    commands
        .spawn((
            CityRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(COLOR_BG_DEEP),
        ))
        .with_children(|viewport| {
            // 0. Dimmed cover backdrop behind the exact-aspect interactive Fjord canvas.
            viewport
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        right: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    ZIndex(0),
                    PickingBehavior::IGNORE,
                ))
                .with_children(|bg_wrap| {
                    bg_wrap.spawn((
                        FjordMapBackdrop,
                        ImageNode::new(fjord_art.fjord_map.clone())
                            .with_color(Color::srgba(0.28, 0.34, 0.44, 1.0)),
                        Node {
                            width: Val::Px(cover_w),
                            height: Val::Px(cover_h),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        PickingBehavior::IGNORE,
                    ));
                });

            // 1. Centered 16:9 Interactive Fjord Map Canvas (pins stay pixel-locked across resolutions).
            viewport
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        right: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ZIndex(5),
                    PickingBehavior::IGNORE,
                ))
                .with_children(|stage| {
                    stage
                        .spawn((
                            FjordMapContainer,
                            Button,
                            NoButtonScale,
                            CityAction::CloseInspector,
                            ImageNode::new(fjord_art.fjord_map.clone()),
                            Node {
                                width: Val::Px(canvas_w),
                                height: Val::Px(canvas_h),
                                flex_shrink: 0.0,
                                border: UiRect::all(Val::Px(if is_portrait { 0.0 } else { 1.5 })),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BorderColor(Color::srgba(0.93, 0.74, 0.29, 0.28)),
                            BorderRadius::all(Val::Px(if is_portrait { 0.0 } else { 10.0 })),
                            panel_shadow(),
                        ))
                        .with_children(|map| {
                            // Animated Aurora Borealis ribbons glowing brighter with Clan Glory.
                            let aurora_intensity =
                                (0.14 + (ctx.city.glory as f32 / 1200.0)).min(0.38);
                            let aurora_defs = [
                                (
                                    8.0,
                                    2.0,
                                    48.0,
                                    14.0,
                                    Color::srgba(0.18, 0.92, 0.68, aurora_intensity),
                                    0.0,
                                ),
                                (
                                    38.0,
                                    5.0,
                                    52.0,
                                    12.0,
                                    Color::srgba(0.24, 0.72, 0.98, aurora_intensity * 0.85),
                                    2.1,
                                ),
                                (
                                    18.0,
                                    8.0,
                                    44.0,
                                    10.0,
                                    Color::srgba(0.62, 0.36, 0.94, aurora_intensity * 0.75),
                                    4.2,
                                ),
                            ];
                            for (left_p, top_p, w_p, h_p, col, phase) in aurora_defs {
                                map.spawn((
                                    FjordAurora {
                                        base_left_pct: left_p,
                                        phase,
                                        base_alpha: col.alpha(),
                                    },
                                    ImageNode::new(vfx.aurora_ribbon.clone()).with_color(col),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Percent(left_p),
                                        top: Val::Percent(top_p),
                                        width: Val::Percent(w_p),
                                        height: Val::Percent(h_p + 4.0),
                                        ..default()
                                    },
                                    PickingBehavior::IGNORE,
                                ));
                            }

                            // Rippling aurora reflections across the central Fjord bay water.
                            let water_defs = [
                                (
                                    34.0,
                                    31.0,
                                    26.0,
                                    12.0,
                                    Color::srgba(0.22, 0.90, 0.78, 0.24),
                                    0.4,
                                ),
                                (
                                    42.0,
                                    37.0,
                                    24.0,
                                    11.0,
                                    Color::srgba(0.32, 0.80, 0.98, 0.20),
                                    2.3,
                                ),
                                (
                                    48.0,
                                    44.0,
                                    20.0,
                                    9.0,
                                    Color::srgba(0.24, 0.92, 0.74, 0.18),
                                    4.1,
                                ),
                            ];
                            for (left_p, top_p, w_p, h_p, col, phase) in water_defs {
                                map.spawn((
                                    FjordWaterShimmer {
                                        base_left_pct: left_p,
                                        phase,
                                        base_alpha: col.alpha(),
                                    },
                                    ImageNode::new(vfx.water_glint.clone()).with_color(col),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Percent(left_p),
                                        top: Val::Percent(top_p),
                                        width: Val::Percent(w_p),
                                        height: Val::Percent(h_p),
                                        ..default()
                                    },
                                    PickingBehavior::IGNORE,
                                ));
                            }

                            for i in 0..SNOWFLAKE_COUNT {
                                let x_pct = ((i * 37 + 11) % 96) as f32 + 2.0;
                                let y_pct = ((i * 53 + 7) % 94) as f32 + 2.0;
                                let size = if i % 3 == 0 { 7.0 } else { 4.5 };
                                map.spawn((
                                    FjordSnowflake {
                                        base_x_pct: x_pct,
                                        base_y_pct: y_pct,
                                        x_pct,
                                        y_pct,
                                        speed_y: 4.5 + (i % 5) as f32 * 1.4,
                                        phase: i as f32 * 0.9,
                                    },
                                    ImageNode::new(vfx.soft_orb.clone())
                                        .with_color(Color::srgba(0.92, 0.97, 1.0, 0.62)),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Percent(x_pct),
                                        top: Val::Percent(y_pct),
                                        width: Val::Px(size),
                                        height: Val::Px(size),
                                        ..default()
                                    },
                                    PickingBehavior::IGNORE,
                                ));
                            }

                            let quest_target_bldg = ctx
                                .city
                                .active_quest(&ctx.data.quests)
                                .and_then(|q| match &q.condition {
                                    QuestCondition::WinBuildingLevel { building_id, .. }
                                    | QuestCondition::ReachBuildingTier { building_id, .. } => {
                                        Some(building_id.as_str())
                                    }
                                    QuestCondition::WinSpecificLevel { level_id } => ctx
                                        .data
                                        .level_by_id(level_id)
                                        .map(|l| l.building_id.as_str()),
                                    QuestCondition::CraftAnyAmulet
                                    | QuestCondition::ClaimAnyCollection
                                    | QuestCondition::CompleteDailyTrials { .. }
                                    | QuestCondition::DefeatBosses { .. }
                                    | QuestCondition::ReachExpeditionWave { .. } => None,
                                });
                            for b in &ctx.data.buildings {
                                let is_quest_target = quest_target_bldg == Some(b.id.as_str());
                                let is_just_built = just_built_id == Some(b.id.as_str());
                                spawn_map_building_pin(
                                    map,
                                    b,
                                    ctx,
                                    fjord_art,
                                    vfx,
                                    is_quest_target,
                                    is_just_built,
                                    is_portrait,
                                    canvas_w,
                                    canvas_h,
                                    now,
                                );
                            }
                        });
                });

            // 2. Slim Floating Top HUD Bar + Contextual Banners (pass-through column wrapper).
            let edge_x = if is_portrait { 6.0 } else { 14.0 };
            let edge_y = if is_portrait { 6.0 } else { 10.0 };
            viewport
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(edge_y),
                        left: Val::Px(edge_x),
                        right: Val::Px(edge_x),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    ZIndex(30),
                    PickingBehavior::IGNORE,
                ))
                .with_children(|top_col| {
                    top_col
                        .spawn((
                            FocusPolicy::Block,
                            Interaction::default(),
                            sliced_panel_image(vfx),
                            Node {
                                width: Val::Percent(100.0),
                                flex_wrap: FlexWrap::Wrap,
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                padding: if is_portrait {
                                    UiRect::axes(Val::Px(8.0), Val::Px(6.0))
                                } else {
                                    UiRect::axes(Val::Px(12.0), Val::Px(6.0))
                                },
                                row_gap: Val::Px(5.0),
                                column_gap: Val::Px(10.0),
                                ..default()
                            },
                            panel_shadow(),
                        ))
                        .with_children(|bar| {
                            // Left: Clan Identity & Rank
                            bar.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(1.0),
                                ..default()
                            })
                            .with_children(|col| {
                                col.spawn((
                                    Text::new(i18n.t("city.header.title")),
                                    display_font(
                                        fjord_art,
                                        if is_portrait {
                                            FONT_BODY
                                        } else {
                                            FONT_HEADING - 2.0
                                        },
                                    ),
                                    TextColor(COLOR_ACCENT_GOLD),
                                ));
                                let rank = ctx.city.clan_rank_with_balance(&ctx.data.balance);
                                let rank_label = i18n.t(rank.i18n_key());
                                let mastery_label = i18n.t_fmt(
                                    "city.header.mastery",
                                    &[("glory", ctx.city.glory.to_string())],
                                );
                                let subtitle_line = if is_portrait {
                                    i18n.t_fmt(
                                        "city.header.subtitle_compact",
                                        &[
                                            ("rank", rank_label.to_string()),
                                            ("mastery", mastery_label),
                                        ],
                                    )
                                } else {
                                    let perk_label =
                                        format_rank_perk(i18n, rank, &ctx.data.balance);
                                    i18n.t_fmt(
                                        "city.header.subtitle_full",
                                        &[
                                            ("rank", rank_label.to_string()),
                                            ("mastery", mastery_label),
                                            ("perk", perk_label),
                                        ],
                                    )
                                };
                                col.spawn((
                                    Text::new(subtitle_line),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_MUTED),
                                ));
                            });

                            // Center: Resource & Energy Pills
                            bar.spawn(Node {
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(6.0),
                                row_gap: Val::Px(4.0),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|pills| {
                                let res_lbl = |key: &str, amount: u32| {
                                    if is_portrait {
                                        amount.to_string()
                                    } else {
                                        resource_pill_text(i18n, key, amount)
                                    }
                                };
                                spawn_resource_pill_with_icon(
                                    pills,
                                    vfx.icon_wood.clone(),
                                    &res_lbl("resources.label.wood", res.wood),
                                    COLOR_RES_WOOD,
                                );
                                spawn_resource_pill_with_icon(
                                    pills,
                                    vfx.icon_stone.clone(),
                                    &res_lbl("resources.label.stone", res.stone),
                                    COLOR_RES_STONE,
                                );
                                spawn_resource_pill_with_icon(
                                    pills,
                                    vfx.icon_gold.clone(),
                                    &res_lbl("resources.label.gold", res.gold),
                                    COLOR_ACCENT_GOLD,
                                );
                                spawn_resource_pill_with_icon(
                                    pills,
                                    vfx.icon_mead.clone(),
                                    &res_lbl("resources.label.mead", res.mead),
                                    COLOR_RES_MEAD,
                                );
                                pills
                                    .spawn((
                                        Node {
                                            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                                            column_gap: Val::Px(5.0),
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(COLOR_PANEL_CARD),
                                        BorderColor(COLOR_PANEL_BORDER),
                                        BorderRadius::all(Val::Px(RADIUS_SM)),
                                    ))
                                    .with_children(|pill| {
                                        pill.spawn((
                                            ImageNode::new(vfx.icon_energy.clone()),
                                            Node {
                                                width: Val::Px(15.0),
                                                height: Val::Px(15.0),
                                                ..default()
                                            },
                                        ));
                                        pill.spawn((
                                            CityEnergyText,
                                            Text::new(format_energy_label(ctx)),
                                            TextFont {
                                                font_size: FONT_SMALL,
                                                ..default()
                                            },
                                            TextColor(COLOR_RES_ENERGY),
                                        ));
                                    });
                                if ctx.city.stats.win_streak
                                    >= ctx.data.balance.win_streak_booster_threshold
                                {
                                    spawn_resource_pill(
                                        pills,
                                        &i18n.t_fmt(
                                            "city.header.win_streak",
                                            &[
                                                ("streak", ctx.city.stats.win_streak.to_string()),
                                                (
                                                    "boosters",
                                                    ctx.data
                                                        .balance
                                                        .win_streak_starting_boosters
                                                        .to_string(),
                                                ),
                                            ],
                                        ),
                                        COLOR_ACCENT_GOLD,
                                    );
                                }
                                if ctx.city.energy < max_energy {
                                    let can_drink = res.mead >= ctx.data.balance.mead_refill_cost;
                                    let lbl = i18n.t_fmt(
                                        "city.button.drink_mead",
                                        &[
                                            (
                                                "energy",
                                                ctx.data.balance.mead_refill_energy.to_string(),
                                            ),
                                            ("cost", ctx.data.balance.mead_refill_cost.to_string()),
                                        ],
                                    );
                                    spawn_small_button(
                                        pills,
                                        &lbl,
                                        CityAction::DrinkMead,
                                        if can_drink {
                                            COLOR_ACCENT_GREEN
                                        } else {
                                            COLOR_BUTTON_DISABLED
                                        },
                                        can_drink,
                                    );
                                }
                            });

                            // Right: Clean Burger/Drawer Controls (Modes, Clan, Menu + contextual Collect All)
                            bar.spawn((
                                Node {
                                    column_gap: Val::Px(6.0),
                                    row_gap: Val::Px(4.0),
                                    flex_wrap: FlexWrap::Wrap,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(COLOR_HUD_GROUP_BG),
                                BorderColor(Color::srgba(0.93, 0.74, 0.29, 0.28)),
                                BorderRadius::all(Val::Px(RADIUS_MD)),
                            ))
                            .with_children(|actions| {
                                if ready_harvests >= 2 {
                                    spawn_small_button_colored(
                                        actions,
                                        &i18n.t_fmt(
                                            "city.harvest.collect_all",
                                            &[("count", ready_harvests.to_string())],
                                        ),
                                        CityAction::CollectAllHarvests,
                                        COLOR_ACCENT_GOLD,
                                        COLOR_TEXT_DARK,
                                        true,
                                    );
                                }
                                let modes_open = active_drawer == CityDrawer::Modes;
                                let modes_badge = usize::from(daily_ready);
                                let modes_lbl =
                                    badged_label(i18n, "city.drawer.modes.button", modes_badge);
                                spawn_small_button_colored(
                                    actions,
                                    &modes_lbl,
                                    CityAction::ToggleDrawer(CityDrawer::Modes),
                                    if modes_open {
                                        COLOR_ACCENT_TEAL
                                    } else if daily_ready {
                                        COLOR_ACCENT_GOLD
                                    } else {
                                        COLOR_PANEL_CARD
                                    },
                                    if !modes_open && daily_ready {
                                        COLOR_TEXT_DARK
                                    } else {
                                        COLOR_TEXT_PRIMARY
                                    },
                                    true,
                                );

                                let clan_open = active_drawer == CityDrawer::Clan;
                                let clan_badge = claimable_feats + claimable_cols;
                                let clan_lbl =
                                    badged_label(i18n, "city.drawer.clan.button", clan_badge);
                                spawn_small_button_colored(
                                    actions,
                                    &clan_lbl,
                                    CityAction::ToggleDrawer(CityDrawer::Clan),
                                    if clan_open {
                                        COLOR_ACCENT_TEAL
                                    } else if clan_badge > 0 {
                                        COLOR_ACCENT_GREEN
                                    } else {
                                        COLOR_PANEL_CARD
                                    },
                                    COLOR_TEXT_PRIMARY,
                                    true,
                                );

                                let menu_open = active_drawer == CityDrawer::Menu;
                                let menu_badge = usize::from(tribute_ready);
                                let menu_lbl = badged_label(i18n, "city.button.menu", menu_badge);
                                spawn_small_button_colored(
                                    actions,
                                    &menu_lbl,
                                    CityAction::ToggleDrawer(CityDrawer::Menu),
                                    if menu_open {
                                        COLOR_ACCENT_TEAL
                                    } else if tribute_ready {
                                        COLOR_ACCENT_GOLD
                                    } else {
                                        COLOR_PANEL_CARD
                                    },
                                    if !menu_open && tribute_ready {
                                        COLOR_TEXT_DARK
                                    } else {
                                        COLOR_TEXT_PRIMARY
                                    },
                                    true,
                                );
                            });
                        });

                    // Guided 3-step Tutorial Banner
                    let q1_active = ctx.tutorial_step == 0
                        && ctx
                            .city
                            .active_quest(&ctx.data.quests)
                            .is_some_and(|q| q.id == "q1");
                    if ctx.tutorial_step < 3 && !q1_active {
                        let step_key = match ctx.tutorial_step {
                            0 => "tutorial.step0",
                            1 => "tutorial.step1",
                            _ => "tutorial.step2",
                        };
                        let btn_key = if ctx.tutorial_step < 2 {
                            "tutorial.next"
                        } else {
                            "tutorial.done"
                        };
                        top_col
                            .spawn((
                                FocusPolicy::Block,
                                Interaction::default(),
                                Node {
                                    width: Val::Percent(100.0),
                                    max_width: Val::Px(640.0),
                                    align_self: AlignSelf::Center,
                                    justify_content: JustifyContent::SpaceBetween,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::axes(Val::Px(12.0), Val::Px(7.0)),
                                    column_gap: Val::Px(12.0),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.10, 0.22, 0.34, 0.96)),
                                BorderColor(COLOR_ACCENT_TEAL),
                                BorderRadius::all(Val::Px(8.0)),
                            ))
                            .with_children(|tut| {
                                tut.spawn((
                                    Text::new(i18n.t(step_key)),
                                    TextFont {
                                        font_size: FONT_SMALL,
                                        ..default()
                                    },
                                    TextColor(COLOR_TEXT_PRIMARY),
                                ));
                                spawn_small_button_colored(
                                    tut,
                                    i18n.t(btn_key),
                                    CityAction::AdvanceTutorial,
                                    COLOR_ACCENT_GOLD,
                                    COLOR_TEXT_DARK,
                                    true,
                                );
                            });
                    }

                    // Celebration Banner
                    if ctx.active_modal == ActiveModal::None {
                        if let Some(msg) = &ctx.celebration_message {
                            top_col
                                .spawn((
                                    FocusPolicy::Block,
                                    Interaction::default(),
                                    Node {
                                        align_self: AlignSelf::Center,
                                        justify_content: JustifyContent::Center,
                                        padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                                        border: UiRect::all(Val::Px(1.5)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.14, 0.42, 0.26, 0.95)),
                                    BorderColor(COLOR_ACCENT_GOLD),
                                    BorderRadius::all(Val::Px(8.0)),
                                ))
                                .with_children(|cel| {
                                    cel.spawn((
                                        Text::new(msg.clone()),
                                        TextFont {
                                            font_size: FONT_BODY,
                                            ..default()
                                        },
                                        TextColor(COLOR_TEXT_PRIMARY),
                                    ));
                                });
                        }
                    }
                });

            // 3. Compact Floating Bottom-Left Saga Quest Card
            if !(is_portrait && right_panel_open) {
                let quest_node = if is_portrait {
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(6.0),
                        right: Val::Px(6.0),
                        bottom: Val::Px(6.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        column_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    }
                } else {
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(14.0),
                        bottom: Val::Px(14.0),
                        max_width: Val::Px(460.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(7.0)),
                        column_gap: Val::Px(10.0),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    }
                };
                viewport
                    .spawn((
                        FocusPolicy::Block,
                        Interaction::default(),
                        quest_node,
                        BackgroundColor(Color::srgba(0.06, 0.09, 0.14, 0.90)),
                        BorderColor(COLOR_ACCENT_GOLD),
                        BorderRadius::all(Val::Px(10.0)),
                        panel_shadow(),
                        ZIndex(28),
                    ))
                    .with_children(|qbar| {
                        if let Some(quest) = ctx.city.active_quest(&ctx.data.quests) {
                            let ready = ctx.city.is_quest_ready(quest);
                            qbar.spawn(Node {
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(8.0),
                                flex_shrink: 1.0,
                                ..default()
                            })
                            .with_children(|left| {
                                if let Some(portrait) = fjord_art.portrait(&quest.speaker) {
                                    left.spawn((
                                        ImageNode::new(portrait),
                                        Node {
                                            width: Val::Px(38.0),
                                            height: Val::Px(38.0),
                                            flex_shrink: 0.0,
                                            border: UiRect::all(Val::Px(1.5)),
                                            ..default()
                                        },
                                        BorderColor(COLOR_ACCENT_GOLD),
                                        BorderRadius::all(Val::Px(6.0)),
                                    ));
                                }
                                left.spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(1.0),
                                    flex_shrink: 1.0,
                                    ..default()
                                })
                                .with_children(|info| {
                                    info.spawn((
                                        Text::new(i18n.t_fmt(
                                            "city.panel.quests.banner",
                                            &[
                                                (
                                                    "title",
                                                    i18n.t("city.panel.quests.title").to_string(),
                                                ),
                                                ("quest", i18n.t(&quest.title_key).to_string()),
                                            ],
                                        )),
                                        TextFont {
                                            font_size: FONT_SMALL,
                                            ..default()
                                        },
                                        TextColor(COLOR_ACCENT_GOLD),
                                    ));
                                    info.spawn((
                                        Text::new(i18n.t(&quest.desc_key)),
                                        TextFont {
                                            font_size: FONT_SMALL - 1.0,
                                            ..default()
                                        },
                                        TextColor(COLOR_TEXT_PRIMARY),
                                    ));
                                });
                            });

                            if ready {
                                spawn_small_button(
                                    qbar,
                                    i18n.t("city.panel.quests.complete"),
                                    CityAction::ClaimQuest(quest.id.clone()),
                                    COLOR_ACCENT_GREEN,
                                    true,
                                );
                            } else {
                                spawn_small_button(
                                    qbar,
                                    i18n.t("city.panel.quests.go"),
                                    CityAction::FocusActiveQuest,
                                    COLOR_ACCENT_TEAL,
                                    true,
                                );
                            }
                        } else {
                            qbar.spawn((
                                Text::new(i18n.t("city.panel.quests.done")),
                                TextFont {
                                    font_size: FONT_SMALL,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GOLD),
                            ));
                        }
                    });
            }

            // 4. Unified Slide-Out Drawer (Modes / Clan / Burger Menu / Building Inspector)
            if right_panel_open {
                let drawer_node = if is_portrait {
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(6.0),
                        right: Val::Px(6.0),
                        bottom: Val::Px(6.0),
                        height: Val::Percent(68.0),
                        min_height: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::all(Val::Px(14.0)),
                        row_gap: Val::Px(8.0),
                        overflow: Overflow::clip(),
                        ..default()
                    }
                } else {
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(82.0),
                        bottom: Val::Px(14.0),
                        right: Val::Px(14.0),
                        width: Val::Px(390.0),
                        max_width: Val::Percent(92.0),
                        min_height: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::all(Val::Px(15.0)),
                        row_gap: Val::Px(8.0),
                        overflow: Overflow::clip(),
                        ..default()
                    }
                };
                viewport
                    .spawn((
                        FocusPolicy::Block,
                        Interaction::default(),
                        sliced_panel_image(vfx),
                        drawer_node,
                        modal_shadow(),
                        ZIndex(40),
                    ))
                    .with_children(|drawer| match active_drawer {
                        CityDrawer::Modes => {
                            drawers::spawn_modes_drawer(
                                drawer,
                                ctx,
                                fjord_art,
                                scroll_mem.inspector_offset_y,
                            );
                        }
                        CityDrawer::Clan => {
                            drawers::spawn_clan_drawer(
                                drawer,
                                ctx,
                                fjord_art,
                                scroll_mem.inspector_offset_y,
                            );
                        }
                        CityDrawer::Menu => {
                            drawers::spawn_menu_drawer(
                                drawer,
                                ctx,
                                fjord_art,
                                sound_bank,
                                scroll_mem.inspector_offset_y,
                            );
                        }
                        CityDrawer::None => {
                            if let Some(selected) =
                                ctx.data.building_by_id(&ctx.selected_building_id)
                            {
                                inspector::spawn_building_inspector(
                                    drawer,
                                    selected,
                                    ctx,
                                    fjord_art,
                                    scroll_mem.inspector_offset_y,
                                );
                            }
                        }
                    });
            }

            // 4. Exclusive Modal Overlay
            let m_scroll = scroll_mem.modal_offset_y;
            match ctx.active_modal {
                ActiveModal::None => {}
                ActiveModal::Amulets => {
                    modals::spawn_amulets_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Collections => {
                    modals::spawn_collections_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Shop => {
                    modals::spawn_tool_shop_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Stats => {
                    modals::spawn_stats_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Settings => modals::spawn_settings_modal(
                    viewport, ctx, fjord_art, vfx, sound_bank, m_scroll,
                ),
                ActiveModal::Help => {
                    modals::spawn_help_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Raids => {
                    modals::spawn_raids_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Trials => {
                    modals::spawn_trials_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Patrons => {
                    modals::spawn_patrons_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Achievements => {
                    modals::spawn_achievements_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::ExpeditionBoons => {
                    modals::spawn_expedition_boons_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::LoginCalendar => {
                    modals::spawn_login_calendar_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::ChapterDialogue => {
                    modals::spawn_chapter_dialogue_modal(viewport, ctx, fjord_art, vfx, m_scroll)
                }
            }

            // 5. Hub-level notice toast (modals and the inspector render notices themselves).
            if ctx.active_modal == ActiveModal::None && !inspector_open {
                if let Some(notice_key) = &ctx.status_notice_key {
                    spawn_notice_toast(viewport, i18n.t(notice_key), None);
                }
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn spawn_map_building_pin(
    map: &mut ChildBuilder,
    b: &BuildingDef,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    is_quest_target: bool,
    is_just_built: bool,
    is_portrait: bool,
    canvas_w: f32,
    canvas_h: f32,
    now_secs: u64,
) {
    let i18n = &ctx.data.i18n;
    let tier = ctx.city.building_tier(&b.id);
    let unlocked = ctx.city.is_building_unlocked(b);
    let selected = ctx.selected_building_id == b.id;
    let harvest_ready = ctx
        .city
        .is_harvest_ready(&b.id, now_secs, &ctx.data.balance);
    let can_upgrade = unlocked
        && ctx
            .city
            .next_upgrade_cost(b)
            .map(|c| ctx.city.resources.can_afford(c))
            .unwrap_or(false);

    let border_col = if selected {
        COLOR_ACCENT_GOLD
    } else if harvest_ready {
        COLOR_ACCENT_GOLD_HOVER
    } else if can_upgrade {
        COLOR_ACCENT_GREEN
    } else if !unlocked {
        COLOR_BUTTON_DISABLED
    } else {
        COLOR_PANEL_BORDER
    };

    let img_tint = if !unlocked {
        Color::srgba(0.38, 0.42, 0.50, 0.85)
    } else if tier == 0 {
        Color::srgba(0.62, 0.70, 0.80, 0.95)
    } else {
        Color::WHITE
    };

    let tier_scale = match tier {
        0 => 0.56,
        1 => 0.88,
        2 => 1.04,
        _ => 1.20,
    };
    let pin_data = MapBuildingPin {
        map_x_pct: b.map_x_pct,
        map_y_pct: b.map_y_pct,
        portrait_left_pct: b.portrait_left_pct,
        portrait_top_pct: b.portrait_top_pct,
        tier_scale,
    };
    let (init_left, init_top, pin_w, sprite_w, sprite_h) =
        compute_pin_layout(&pin_data, canvas_w, canvas_h, is_portrait);

    let nameplate_shadow = if selected || harvest_ready || is_quest_target || tier >= 3 {
        gold_glow_shadow()
    } else {
        button_shadow()
    };
    let ground_glow_color = if selected || harvest_ready || is_quest_target || is_just_built {
        Color::srgba(0.98, 0.82, 0.30, 0.68)
    } else if tier >= 3 {
        Color::srgba(0.98, 0.78, 0.28, 0.48)
    } else if tier >= 2 {
        Color::srgba(0.95, 0.70, 0.26, 0.32)
    } else if tier == 1 {
        Color::srgba(0.95, 0.68, 0.24, 0.18)
    } else {
        Color::srgba(0.02, 0.04, 0.08, 0.72)
    };

    map.spawn((
        pin_data,
        Button,
        CityAction::SelectBuilding(b.id.clone()),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(init_left),
            top: Val::Px(init_top),
            width: Val::Px(pin_w),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(2.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
    ))
    .with_children(|pin| {
        // Cutout isometric building (or mossy stone ruin at Tier 0) standing directly on the Fjord terrain.
        pin.spawn((
            MapBuildingStage { tier_scale },
            Node {
                width: Val::Px(sprite_w),
                height: Val::Px(sprite_h),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
        ))
        .with_children(|stage| {
            // Soft elliptical ground contact shadow or golden selection/tier halo at the building base.
            stage.spawn((
                ImageNode::new(vfx.soft_orb.clone()).with_color(ground_glow_color),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(-4.0),
                    width: Val::Percent(106.0),
                    height: Val::Percent(42.0),
                    ..default()
                },
                PickingBehavior::IGNORE,
            ));

            // Warm radiant sanctuary aura behind Tier 2-3 rebuilt buildings so upgrades stand out.
            if tier >= 2 {
                let aura_alpha = if tier >= 3 { 0.34 } else { 0.18 };
                stage.spawn((
                    ImageNode::new(vfx.soft_orb.clone())
                        .with_color(Color::srgba(1.0, 0.80, 0.30, aura_alpha)),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(118.0),
                        height: Val::Percent(105.0),
                        ..default()
                    },
                    PickingBehavior::IGNORE,
                ));
            }

            if is_just_built {
                stage.spawn((
                    BuildingBuildRing {
                        elapsed: 0.0,
                        duration: 1.05,
                    },
                    ImageNode::new(vfx.shockwave_ring.clone())
                        .with_color(Color::srgba(1.0, 0.86, 0.34, 0.98)),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(155.0),
                        height: Val::Percent(155.0),
                        ..default()
                    },
                    Transform::from_scale(Vec3::splat(0.30)),
                    PickingBehavior::IGNORE,
                ));
                stage.spawn((
                    BuildingBuildRing {
                        elapsed: 0.0,
                        duration: 0.72,
                    },
                    ImageNode::new(vfx.shockwave_ring.clone())
                        .with_color(Color::srgba(0.45, 0.94, 0.82, 0.85)),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(125.0),
                        height: Val::Percent(125.0),
                        ..default()
                    },
                    Transform::from_scale(Vec3::splat(0.20)),
                    PickingBehavior::IGNORE,
                ));
            }

            let building_tex = if tier == 0 {
                Some(vfx.ruin_cairn.clone())
            } else {
                fjord_art.map_building_image(&b.id)
            };
            if let Some(img) = building_tex {
                let mut img_cmd = stage.spawn((
                    ImageNode {
                        image: img,
                        color: if tier == 0 {
                            if unlocked {
                                Color::srgba(0.92, 0.95, 1.0, 0.96)
                            } else {
                                Color::srgba(0.55, 0.60, 0.68, 0.80)
                            }
                        } else {
                            img_tint
                        },
                        ..default()
                    },
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                ));
                if is_just_built {
                    img_cmd.insert((
                        BuildingBuildAnim {
                            elapsed: 0.0,
                            duration: 0.95,
                        },
                        Transform::from_scale(Vec3::splat(0.20)),
                    ));
                }
                img_cmd.with_children(|img_box| {
                    if tier > 0 {
                        for k in 0..3usize {
                            let base_x = 24.0 + k as f32 * 24.0;
                            let start_y = 66.0 - (k as f32 * 16.0);
                            img_box.spawn((
                                FjordEmber {
                                    base_x_pct: base_x,
                                    base_y_pct: start_y,
                                    y_pct: start_y,
                                    phase: k as f32 * 2.1 + b.map_x_pct * 0.1,
                                },
                                ImageNode::new(vfx.soft_orb.clone())
                                    .with_color(Color::srgba(1.0, 0.78, 0.28, 0.85)),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Percent(base_x),
                                    top: Val::Percent(start_y),
                                    width: Val::Px(7.0),
                                    height: Val::Px(7.0),
                                    ..default()
                                },
                                PickingBehavior::IGNORE,
                            ));
                        }
                        if matches!(
                            b.id.as_str(),
                            "forge" | "meadery" | wyrdstone_core::HUB_BUILDING_ID
                        ) {
                            for s in 0..3usize {
                                let sx = 46.0 + (s as f32 - 1.0) * 6.0;
                                let sy = 14.0 - s as f32 * 10.0;
                                img_box.spawn((
                                    FjordSmokePuff {
                                        base_y_pct: sy,
                                        y_pct: sy,
                                        phase: s as f32 * 2.1 + b.map_x_pct * 0.08,
                                    },
                                    ImageNode::new(vfx.soft_orb.clone())
                                        .with_color(Color::srgba(0.84, 0.88, 0.94, 0.24)),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Percent(sx),
                                        top: Val::Percent(sy),
                                        width: Val::Px(16.0),
                                        height: Val::Px(16.0),
                                        ..default()
                                    },
                                    PickingBehavior::IGNORE,
                                ));
                            }
                        }
                        if b.id == "watchtower" {
                            img_box.spawn((
                                FjordBeaconGlow {
                                    phase: b.map_y_pct * 0.1,
                                },
                                ImageNode::new(vfx.soft_orb.clone())
                                    .with_color(Color::srgba(1.0, 0.66, 0.20, 0.58)),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Percent(22.0),
                                    top: Val::Percent(6.0),
                                    width: Val::Px(42.0),
                                    height: Val::Px(42.0),
                                    ..default()
                                },
                                PickingBehavior::IGNORE,
                            ));
                        }
                    }
                });
            }
        });

        // Compact Nordic nameplate ribbon below the building base.
        let mut plate_cmd = pin.spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                row_gap: Val::Px(2.0),
                border: UiRect::all(Val::Px(if selected || is_quest_target || harvest_ready {
                    2.0
                } else {
                    1.5
                })),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.09, 0.15, 0.90)),
            BorderColor(border_col),
            BorderRadius::all(Val::Px(8.0)),
            nameplate_shadow,
        ));
        if is_quest_target || harvest_ready {
            plate_cmd.insert(QuestBeaconPin);
        }
        plate_cmd.with_children(|plate| {
            plate
                .spawn(Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(4.0),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                })
                .with_children(|title_row| {
                    if is_quest_target {
                        title_row.spawn((
                            Text::new(i18n.t("city.panel.quests.tag")),
                            TextFont {
                                font_size: FONT_SMALL - 2.0,
                                ..default()
                            },
                            TextColor(COLOR_ACCENT_TEAL_TEXT),
                        ));
                    }
                    title_row.spawn((
                        Text::new(i18n.t(&b.name_key)),
                        TextFont {
                            font_size: FONT_SMALL,
                            ..default()
                        },
                        TextColor(if selected {
                            COLOR_ACCENT_GOLD
                        } else {
                            COLOR_TEXT_PRIMARY
                        }),
                    ));
                });
            if tier > 0 {
                plate
                    .spawn(Node {
                        column_gap: Val::Px(3.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|srow| {
                        for idx in 0..b.max_tier {
                            let filled = idx < tier;
                            srow.spawn((
                                ImageNode::new(vfx.icon_star.clone()).with_color(if filled {
                                    Color::WHITE
                                } else {
                                    Color::srgba(0.30, 0.36, 0.46, 0.45)
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

            if harvest_ready {
                plate
                    .spawn((
                        Button,
                        NoButtonScale,
                        CityAction::CollectHarvest(b.id.clone()),
                        Node {
                            width: Val::Percent(100.0),
                            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
                            padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(COLOR_ACCENT_GOLD),
                        BorderRadius::all(Val::Px(5.0)),
                    ))
                    .with_children(|hbtn| {
                        hbtn.spawn((
                            Text::new(i18n.t("city.harvest.ready")),
                            TextFont {
                                font_size: FONT_SMALL,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_DARK),
                        ));
                    });
                if can_upgrade {
                    plate.spawn((
                        Text::new(i18n.t("city.panel.building.status.ready")),
                        TextFont {
                            font_size: FONT_SMALL - 1.5,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GREEN_TEXT),
                    ));
                }
            } else {
                let badge = if !unlocked {
                    i18n.t("city.panel.building.badge.locked").to_string()
                } else if can_upgrade {
                    i18n.t_fmt(
                        "city.panel.building.badge.ready",
                        &[
                            (
                                "status",
                                i18n.t("city.panel.building.status.ready").to_string(),
                            ),
                            ("tier", tier.to_string()),
                            ("max", b.max_tier.to_string()),
                        ],
                    )
                } else if tier == 0 {
                    i18n.t("city.panel.building.status.ruined").to_string()
                } else {
                    i18n.t_fmt(
                        "city.panel.building.badge.tier",
                        &[("tier", tier.to_string()), ("max", b.max_tier.to_string())],
                    )
                };
                plate.spawn((
                    Text::new(badge),
                    TextFont {
                        font_size: FONT_SMALL,
                        ..default()
                    },
                    TextColor(if can_upgrade {
                        COLOR_ACCENT_GREEN_TEXT
                    } else if tier > 0 {
                        COLOR_ACCENT_GOLD
                    } else {
                        COLOR_TEXT_MUTED
                    }),
                ));
            }
        });
    });
}

pub(super) fn spawn_resource_pill_with_icon(
    parent: &mut ChildBuilder,
    icon: Handle<Image>,
    text: &str,
    text_color: Color,
) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                column_gap: Val::Px(5.0),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(COLOR_PANEL_CARD),
            BorderColor(COLOR_PANEL_BORDER),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|pill| {
            pill.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(16.0),
                    height: Val::Px(16.0),
                    ..default()
                },
            ));
            pill.spawn((
                Text::new(text),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(text_color),
            ));
        });
}

pub(crate) fn spawn_resource_pill(parent: &mut ChildBuilder, text: &str, text_color: Color) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(COLOR_PANEL_CARD),
            BorderColor(COLOR_PANEL_BORDER),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|pill| {
            pill.spawn((
                Text::new(text),
                TextFont {
                    font_size: FONT_SMALL,
                    ..default()
                },
                TextColor(text_color),
            ));
        });
}

/// Swaps light text for the dark accent text colour when the button sits on a green or teal
/// accent, where light text only reaches about 2.8:1 contrast.
pub(crate) fn readable_text_color(bg_color: Color, text_color: Color) -> Color {
    if text_color == COLOR_TEXT_PRIMARY
        && (bg_color == COLOR_ACCENT_GREEN || bg_color == COLOR_ACCENT_TEAL)
    {
        COLOR_TEXT_ON_ACCENT
    } else {
        text_color
    }
}

/// Fixed beacon glow level used instead of the pulse when reduced motion is on.
const BEACON_STATIC_GLOW: f32 = 0.5;

pub(crate) fn spawn_small_button(
    parent: &mut ChildBuilder,
    label: &str,
    action: CityAction,
    bg_color: Color,
    enabled: bool,
) {
    spawn_small_button_colored(parent, label, action, bg_color, COLOR_TEXT_PRIMARY, enabled);
}

pub(crate) fn spawn_small_button_colored(
    parent: &mut ChildBuilder,
    label: &str,
    action: CityAction,
    bg_color: Color,
    text_color: Color,
    enabled: bool,
) {
    let text_color = if !enabled {
        COLOR_TEXT_MUTED
    } else {
        readable_text_color(bg_color, text_color)
    };
    let border_col = if enabled {
        COLOR_PANEL_BORDER
    } else {
        Color::srgba(0.28, 0.34, 0.42, 0.65)
    };
    let mut cmd = parent.spawn((
        Button,
        action,
        Node {
            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
            min_width: Val::Px(MIN_TOUCH_TARGET_PX),
            padding: UiRect::axes(Val::Px(11.0), Val::Px(7.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(bg_color),
        BorderColor(border_col),
        BorderRadius::all(Val::Px(RADIUS_SM)),
    ));
    if !enabled {
        cmd.insert(NoButtonScale);
    }
    cmd.with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont {
                font_size: FONT_SMALL,
                ..default()
            },
            TextColor(text_color),
        ));
    });
}

pub(crate) fn spawn_full_button(
    parent: &mut ChildBuilder,
    label: &str,
    action: CityAction,
    bg_color: Color,
    text_color: Color,
    enabled: bool,
) {
    let text_color = if !enabled {
        COLOR_TEXT_MUTED
    } else {
        readable_text_color(bg_color, text_color)
    };
    let border_col = if enabled {
        COLOR_PANEL_BORDER
    } else {
        Color::srgba(0.28, 0.34, 0.42, 0.65)
    };
    let mut cmd = parent.spawn((
        Button,
        action,
        Node {
            flex_grow: 1.0,
            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(11.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.5)),
            ..default()
        },
        BackgroundColor(bg_color),
        BorderColor(border_col),
        BorderRadius::all(Val::Px(RADIUS_MD)),
    ));
    if !enabled {
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

type HarvestTimerQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<InspectorHarvestTimerText>, Without<CityEnergyText>),
>;

/// Processes button presses, passive harvest checks, and Escape modal-close shortcut.
#[allow(clippy::too_many_arguments)]
pub fn handle_city_buttons(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut ctx: ResMut<GameContext>,
    fjord_art: Res<FjordTextures>,
    vfx: Res<VfxTextures>,
    mut sound_bank: ResMut<SoundBank>,
    mut bgm_sinks: Query<&mut AudioSink, With<BgmTrack>>,
    mut layout_mode: ResMut<CityLayoutMode>,
    mut resize_events: EventReader<WindowResized>,
    mut next_state: ResMut<NextState<AppState>>,
    roots: Query<Entity, With<CityRoot>>,
    mut energy_texts: Query<&mut Text, (With<CityEnergyText>, Without<InspectorHarvestTimerText>)>,
    mut harvest_texts: HarvestTimerQuery,
    (touches, mut press_arm, mut scroll_mem): (
        Res<Touches>,
        Local<PressArm>,
        ResMut<UiScrollMemory>,
    ),
    buttons: Query<(Entity, &Interaction, &CityAction), Changed<Interaction>>,
) {
    let mut rebuild = false;

    if keys.just_pressed(KeyCode::Escape) {
        if ctx.active_modal != ActiveModal::None {
            if ctx.active_modal == ActiveModal::ChapterDialogue {
                ctx.pending_dialogue = None;
            }
            ctx.active_modal = ActiveModal::None;
            ctx.confirm_import_backup = false;
            ctx.confirm_end_expedition = false;
            ctx.status_notice_key = None;
            scroll_mem.modal_offset_y = 0.0;
            rebuild = true;
        } else if layout_mode.active_drawer != CityDrawer::None {
            layout_mode.active_drawer = CityDrawer::None;
            rebuild = true;
        } else if layout_mode.inspector_open {
            layout_mode.inspector_open = false;
            rebuild = true;
        } else {
            // Nothing is open: Escape (and the Android Back button) leaves the hub.
            next_state.set(AppState::Title);
            return;
        }
    }

    // Keyboard confirm inside ChapterDialogue and LoginCalendar (Enter / Space).
    if matches!(
        ctx.active_modal,
        ActiveModal::ChapterDialogue | ActiveModal::LoginCalendar
    ) && (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space))
    {
        if ctx.active_modal == ActiveModal::ChapterDialogue {
            ctx.pending_dialogue = None;
            ctx.active_modal = ActiveModal::None;
            scroll_mem.modal_offset_y = 0.0;
            play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
            rebuild = true;
        } else if !claim_login_tribute(&mut commands, &mut ctx, &sound_bank) {
            ctx.active_modal = ActiveModal::None;
            scroll_mem.modal_offset_y = 0.0;
            play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
            rebuild = true;
        } else {
            rebuild = true;
        }
    }

    // Keyboard shortcuts on the Fjord hub when no modal is open (1-6 buildings, Enter play, C harvest all).
    if ctx.active_modal == ActiveModal::None {
        let digit_keys = [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
        ];
        for (idx, kc) in digit_keys.iter().enumerate() {
            if keys.just_pressed(*kc) {
                if let Some(b) = ctx.data.buildings.get(idx) {
                    ctx.selected_building_id = b.id.clone();
                    ctx.selected_difficulty = 0;
                    ctx.status_notice_key = None;
                    layout_mode.active_drawer = CityDrawer::None;
                    layout_mode.inspector_open = true;
                    scroll_mem.inspector_offset_y = 0.0;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    rebuild = true;
                }
            }
        }
        if keys.just_pressed(KeyCode::KeyC) {
            let now = now_unix_secs();
            let (count, _) = {
                let s = &mut *ctx;
                s.city
                    .collect_all_harvests(&s.data.buildings, now, &s.data.balance)
            };
            if count > 0 {
                ctx.celebration_message = Some(ctx.data.i18n.t_fmt(
                    "city.harvest.collected_all",
                    &[("count", count.to_string())],
                ));
                ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                layout_mode.last_ready_harvests = 0;
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                rebuild = true;
            }
        }
        if keys.just_pressed(KeyCode::Enter) {
            if !layout_mode.inspector_open {
                layout_mode.inspector_open = true;
                scroll_mem.inspector_offset_y = 0.0;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            } else if let Some(b) = ctx.data.building_by_id(&ctx.selected_building_id).cloned() {
                if ctx.city.is_building_unlocked(&b) {
                    let stars = ctx.city.building_mastery_stars(&b.id);
                    let level_id = match ctx.selected_difficulty {
                        2 if stars >= 3 => b
                            .legendary_level_id
                            .clone()
                            .unwrap_or_else(|| b.level_id.clone()),
                        1 if stars >= 1 => b
                            .mastery_level_id
                            .clone()
                            .unwrap_or_else(|| b.level_id.clone()),
                        _ => b.level_id.clone(),
                    };
                    match ctx.start_level(&level_id) {
                        Ok(()) => {
                            if ctx.tutorial_step == 0 {
                                ctx.tutorial_step = 1;
                                ctx.persist();
                            }
                            next_state.set(AppState::Match3);
                            return;
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                            rebuild = true;
                        }
                    }
                } else {
                    ctx.status_notice_key = Some("city.panel.building.locked".to_string());
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    rebuild = true;
                }
            }
        }
    }

    // Hub toast: restart its timer on a new notice, clear the notice once the timer runs out.
    // The read-only save warning is persistent: it is re-asserted instead of timing out.
    let read_only_notice = ctx.save_read_only_notice();
    if ctx.status_notice_key.is_none() {
        ctx.status_notice_key = read_only_notice.map(str::to_string);
    }
    let hub_toast_shown = ctx.active_modal == ActiveModal::None && !layout_mode.inspector_open;
    if ctx.status_notice_key.is_some() && ctx.status_notice_key == layout_mode.last_notice_key {
        if !hub_toast_shown || rebuild {
            // Count down only while the toast is visible; a rebuild caused by setting the same
            // key again restarts the timer.
            layout_mode.notice_secs_left = NOTICE_TOAST_DURATION_SECS;
        } else {
            layout_mode.notice_secs_left -= time.delta_secs();
            if layout_mode.notice_secs_left <= 0.0
                && (read_only_notice.is_none()
                    || ctx.status_notice_key.as_deref() != read_only_notice)
            {
                ctx.status_notice_key = None;
            }
        }
    }
    if ctx.status_notice_key != layout_mode.last_notice_key {
        layout_mode.last_notice_key = ctx.status_notice_key.clone();
        layout_mode.notice_secs_left = NOTICE_TOAST_DURATION_SECS;
        rebuild = true;
    }

    if layout_mode.last_muted != sound_bank.muted {
        layout_mode.last_muted = sound_bank.muted;
        rebuild = true;
    }

    if ctx.celebration_timer_secs > 0.0 {
        ctx.celebration_timer_secs -= time.delta_secs();
        if ctx.celebration_timer_secs <= 0.0 {
            ctx.celebration_message = None;
            rebuild = true;
        }
    }

    // Open queued Chapter Dialogue once the building rise animation finishes and no other modal is open.
    if ctx.pending_dialogue.is_some() && ctx.active_modal == ActiveModal::None {
        if layout_mode.pending_chapter_delay_secs > 0.0 {
            layout_mode.pending_chapter_delay_secs -= time.delta_secs();
        }
        if layout_mode.pending_chapter_delay_secs <= 0.0 {
            ctx.active_modal = ActiveModal::ChapterDialogue;
            scroll_mem.modal_offset_y = 0.0;
            rebuild = true;
        }
    }

    if layout_mode.last_displayed_energy != ctx.city.energy {
        let max_energy = ctx.city.max_energy(&ctx.data.balance);
        let prev_below = layout_mode.last_displayed_energy < max_energy;
        let curr_below = ctx.city.energy < max_energy;
        layout_mode.last_displayed_energy = ctx.city.energy;
        if prev_below != curr_below {
            rebuild = true;
        } else {
            let formatted = format_energy_label(&ctx);
            for mut text in &mut energy_texts {
                text.0 = formatted.clone();
            }
        }
    }

    // Update live 1s countdowns for Endurance regen and selected building tribute in-place.
    if layout_mode
        .ui_clock_timer
        .tick(time.delta())
        .just_finished()
    {
        let max_energy = ctx.city.max_energy(&ctx.data.balance);
        if ctx.city.energy < max_energy {
            let formatted = format_energy_label(&ctx);
            for mut text in &mut energy_texts {
                text.0 = formatted.clone();
            }
        }
        if let Some(rem) = ctx.city.harvest_remaining_secs(
            &ctx.selected_building_id,
            now_unix_secs(),
            &ctx.data.balance,
        ) {
            let (lbl, col) = if rem == 0 {
                (
                    ctx.data
                        .i18n
                        .t("city.panel.building.harvest_ready")
                        .to_string(),
                    COLOR_ACCENT_GOLD,
                )
            } else {
                (
                    ctx.data.i18n.t_fmt(
                        "city.panel.building.harvest_in",
                        &[("time", format_countdown(&ctx.data.i18n, rem))],
                    ),
                    COLOR_ACCENT_TEAL_TEXT,
                )
            };
            for (mut text, mut text_col) in &mut harvest_texts {
                text.0 = lbl.clone();
                text_col.0 = col;
            }
        }
    }

    if layout_mode
        .harvest_poll_timer
        .tick(time.delta())
        .just_finished()
    {
        let ready = count_ready_harvests(&ctx, now_unix_secs());
        if ready != layout_mode.last_ready_harvests {
            layout_mode.last_ready_harvests = ready;
            rebuild = true;
        }
    }

    for ev in resize_events.read() {
        if !ev.width.is_finite() || !ev.height.is_finite() || ev.width < 100.0 || ev.height < 100.0
        {
            continue;
        }
        layout_mode.window_size = Vec2::new(ev.width, ev.height);
        let now_portrait = ev.width < ev.height;
        if now_portrait != layout_mode.is_portrait {
            layout_mode.is_portrait = now_portrait;
            layout_mode.last_map_size = Vec2::ZERO;
            rebuild = true;
        }
    }

    // Actions fire on release so a touch that starts a scroll never triggers a button.
    let tapped = press_arm.completed_taps(
        buttons.iter().map(|(entity, i, _)| (entity, *i)),
        &touches,
        &scroll_mem,
    );
    let tapped_actions: Vec<&CityAction> = buttons
        .iter()
        .filter(|(entity, _, _)| tapped.contains(entity))
        .map(|(_, _, action)| action)
        .collect();
    let has_harvest_press = tapped_actions
        .iter()
        .any(|a| matches!(a, CityAction::CollectHarvest(_)));
    let has_pin_press = tapped_actions.iter().any(|a| {
        matches!(
            a,
            CityAction::SelectBuilding(_) | CityAction::CollectHarvest(_)
        )
    });

    for action in tapped_actions {
        match action {
            CityAction::SelectBuilding(id) => {
                if has_harvest_press {
                    continue;
                }
                ctx.selected_building_id = id.clone();
                ctx.selected_difficulty = 0;
                ctx.status_notice_key = None;
                layout_mode.active_drawer = CityDrawer::None;
                layout_mode.inspector_open = true;
                scroll_mem.inspector_offset_y = 0.0;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::CloseInspector => {
                if has_pin_press {
                    continue;
                }
                if layout_mode.inspector_open || layout_mode.active_drawer != CityDrawer::None {
                    layout_mode.inspector_open = false;
                    layout_mode.active_drawer = CityDrawer::None;
                    ctx.status_notice_key = None;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    rebuild = true;
                }
            }
            CityAction::SelectDifficulty(diff) => {
                let allowed = if *diff == 0 {
                    true
                } else if let Some(b) = ctx.data.building_by_id(&ctx.selected_building_id) {
                    let stars = ctx.city.building_mastery_stars(&b.id);
                    ctx.city.is_building_unlocked(b)
                        && ((*diff == 1 && stars >= 1 && b.mastery_level_id.is_some())
                            || (*diff == 2 && stars >= 3 && b.legendary_level_id.is_some()))
                } else {
                    false
                };
                if allowed {
                    ctx.selected_difficulty = *diff;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    rebuild = true;
                }
            }
            CityAction::CyclePatron => {
                let idx = PatronGod::ALL
                    .iter()
                    .position(|&g| g == ctx.city.selected_patron)
                    .unwrap_or(0);
                ctx.city.selected_patron = PatronGod::ALL[(idx + 1) % PatronGod::ALL.len()];
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::CycleAmulet => {
                let crafted: Vec<String> = ctx
                    .data
                    .amulets
                    .iter()
                    .filter(|a| ctx.city.amulet_tier(&a.id) > 0)
                    .map(|a| a.id.clone())
                    .collect();
                if crafted.is_empty() {
                    ctx.active_modal = ActiveModal::Amulets;
                    scroll_mem.modal_offset_y = 0.0;
                } else {
                    let next_idx = ctx
                        .city
                        .equipped_amulet
                        .as_deref()
                        .and_then(|cur| crafted.iter().position(|id| id == cur))
                        .map(|i| (i + 1) % crafted.len())
                        .unwrap_or(0);
                    ctx.city.equipped_amulet = Some(crafted[next_idx].clone());
                    ctx.persist();
                }
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::ClaimLoginTribute => {
                if claim_login_tribute(&mut commands, &mut ctx, &sound_bank) {
                    rebuild = true;
                }
            }
            CityAction::CollectHarvest(building_id) => {
                if let Some(def) = ctx.data.building_by_id(building_id).cloned() {
                    let now = now_unix_secs();
                    let harvested = {
                        let s = &mut *ctx;
                        s.city
                            .try_collect_harvest(&def, now, &s.data.balance)
                            .is_some()
                    };
                    if harvested {
                        let bname = ctx.data.i18n.t(&def.name_key).to_string();
                        ctx.celebration_message = Some(
                            ctx.data
                                .i18n
                                .t_fmt("city.harvest.collected", &[("building", bname)]),
                        );
                        ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                        layout_mode.last_ready_harvests = count_ready_harvests(&ctx, now);
                        ctx.persist();
                        play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                        rebuild = true;
                    }
                }
            }
            CityAction::CollectAllHarvests => {
                let now = now_unix_secs();
                let (count, _) = {
                    let s = &mut *ctx;
                    s.city
                        .collect_all_harvests(&s.data.buildings, now, &s.data.balance)
                };
                if count > 0 {
                    ctx.celebration_message = Some(ctx.data.i18n.t_fmt(
                        "city.harvest.collected_all",
                        &[("count", count.to_string())],
                    ));
                    ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                    layout_mode.last_ready_harvests = 0;
                    ctx.persist();
                    play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                    rebuild = true;
                }
            }
            CityAction::StartExpedition => match ctx.start_or_resume_expedition() {
                Ok(true) => {
                    next_state.set(AppState::Match3);
                    return;
                }
                Ok(false) => {
                    scroll_mem.modal_offset_y = 0.0;
                    rebuild = true;
                }
                Err(err_key) => {
                    ctx.status_notice_key = Some(err_key.to_string());
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    rebuild = true;
                }
            },
            CityAction::StartValhallaBlitz => match ctx.start_valhalla_blitz() {
                Ok(()) => {
                    next_state.set(AppState::Match3);
                    return;
                }
                Err(err_key) => {
                    ctx.status_notice_key = Some(err_key.to_string());
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    rebuild = true;
                }
            },
            CityAction::StartTrial(trial_id) => match ctx.start_trial(trial_id) {
                Ok(()) => {
                    next_state.set(AppState::Match3);
                    return;
                }
                Err(err_key) => {
                    ctx.status_notice_key = Some(err_key.to_string());
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    rebuild = true;
                }
            },
            CityAction::StartGauntlet => match ctx.start_gauntlet() {
                Ok(()) => {
                    next_state.set(AppState::Match3);
                    return;
                }
                Err(err_key) => {
                    ctx.status_notice_key = Some(err_key.to_string());
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    rebuild = true;
                }
            },
            CityAction::PickExpeditionBoon(idx) => match ctx.continue_expedition_with_boon(*idx) {
                Ok(()) => {
                    next_state.set(AppState::Match3);
                    return;
                }
                Err(err_key) => {
                    ctx.status_notice_key = Some(err_key.to_string());
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    ctx.active_modal = ActiveModal::None;
                    scroll_mem.modal_offset_y = 0.0;
                    rebuild = true;
                }
            },
            CityAction::ToggleExpeditionOath => {
                if let Some(run) = ctx.active_expedition.as_mut() {
                    if run.pending_oath.is_some() {
                        run.oath_accepted = !run.oath_accepted;
                        play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                        ctx.persist();
                        rebuild = true;
                    }
                }
            }
            CityAction::EndExpedition => {
                if !ctx.confirm_end_expedition {
                    ctx.confirm_end_expedition = true;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    rebuild = true;
                    continue;
                }
                ctx.confirm_end_expedition = false;
                ctx.active_expedition = None;
                ctx.active_modal = ActiveModal::None;
                scroll_mem.modal_offset_y = 0.0;
                ctx.persist();
                rebuild = true;
            }
            CityAction::FightBoss(boss_id) => match ctx.start_boss_raid(boss_id) {
                Ok(()) => {
                    next_state.set(AppState::Match3);
                    return;
                }
                Err(err_key) => {
                    ctx.status_notice_key = Some(err_key.to_string());
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    rebuild = true;
                }
            },
            CityAction::SelectPatron(god) => {
                ctx.city.selected_patron = *god;
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                rebuild = true;
            }
            CityAction::ClaimAchievement(ach_id) => {
                let achievements = ctx.data.achievements.clone();
                let buildings = ctx.data.buildings.clone();
                let boss_count = ctx.data.bosses.len();
                if ctx
                    .city
                    .try_claim_achievement(ach_id, &achievements, &buildings, boss_count)
                {
                    if let Some(ach) = achievements.iter().find(|a| a.id == *ach_id) {
                        let summary =
                            modals::format_reward_summary(&ctx.data.i18n, ach.reward, ach.glory);
                        ctx.celebration_message = Some(celebration_claim_line(
                            &ctx.data.i18n,
                            ctx.data.i18n.t(&ach.title_key).to_string(),
                            summary,
                        ));
                        ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                    }
                    ctx.status_notice_key = None;
                    ctx.persist();
                    play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                    rebuild = true;
                }
            }
            CityAction::ClaimAllAchievements => {
                let achievements = ctx.data.achievements.clone();
                let buildings = ctx.data.buildings.clone();
                let boss_count = ctx.data.bosses.len();
                let mut claimed_any = false;
                let mut total_res = wyrdstone_core::ResourceBundle::default();
                let mut total_glory = 0u32;
                for ach in &achievements {
                    if ctx.city.try_claim_achievement(
                        &ach.id,
                        &achievements,
                        &buildings,
                        boss_count,
                    ) {
                        claimed_any = true;
                        total_res.add(&ach.reward);
                        total_glory += ach.glory;
                    }
                }
                if claimed_any {
                    ctx.celebration_message = Some(modals::format_reward_summary(
                        &ctx.data.i18n,
                        total_res,
                        total_glory,
                    ));
                    ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                    ctx.status_notice_key = None;
                    ctx.persist();
                    play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                    rebuild = true;
                }
            }
            CityAction::PlayBuildingLevel(level_id) => {
                let is_unlocked = ctx
                    .data
                    .level_by_id(level_id)
                    .and_then(|l| ctx.data.building_by_id(&l.building_id))
                    .map(|b| ctx.city.is_building_unlocked(b))
                    .unwrap_or(false);
                if !is_unlocked {
                    ctx.status_notice_key = Some("city.panel.building.locked".to_string());
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    rebuild = true;
                } else {
                    match ctx.start_level(level_id) {
                        Ok(()) => {
                            if ctx.tutorial_step == 0 {
                                ctx.tutorial_step = 1;
                                ctx.persist();
                            }
                            next_state.set(AppState::Match3);
                            return;
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                            rebuild = true;
                        }
                    }
                }
            }
            CityAction::PlayDailyTrial => {
                if ctx.city.is_daily_available(today_day_index()) {
                    match ctx.start_daily_trial() {
                        Ok(()) => {
                            next_state.set(AppState::Match3);
                            return;
                        }
                        Err(err_key) => ctx.status_notice_key = Some(err_key.to_string()),
                    }
                } else {
                    ctx.status_notice_key = Some("city.panel.daily.done".to_string());
                }
                play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                rebuild = true;
            }
            CityAction::UpgradeBuilding(building_id) => {
                if let Some(def) = ctx.data.building_by_id(building_id).cloned() {
                    let upgrade_res = {
                        let s = &mut *ctx;
                        s.city.try_upgrade_building(&def, &s.data.balance)
                    };
                    match upgrade_res {
                        Ok(new_tier) => {
                            ctx.status_notice_key = None;
                            if ctx.tutorial_step == 2 {
                                ctx.tutorial_step = 3;
                            }
                            let bname = ctx.data.i18n.t(&def.name_key).to_string();
                            ctx.celebration_message = Some(ctx.data.i18n.t_fmt(
                                "city.celebration.upgraded",
                                &[("building", bname), ("tier", new_tier.to_string())],
                            ));
                            ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                            layout_mode.last_ready_harvests =
                                count_ready_harvests(&ctx, now_unix_secs());
                            layout_mode.just_built_id = Some(def.id.clone());
                            // Close inspector so the player watches the building rise cleanly on the Fjord map.
                            layout_mode.inspector_open = false;
                            if new_tier == 1 && !ctx.city.seen_chapters.contains(&def.id) {
                                if let Some(dlg) = chapter_dialogue_for_building(&def.id) {
                                    ctx.city.seen_chapters.insert(def.id.clone());
                                    ctx.pending_dialogue = Some(dlg);
                                    layout_mode.pending_chapter_delay_secs = 1.15;
                                }
                            }
                            ctx.persist();
                            play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                        }
                    }
                    rebuild = true;
                }
            }
            CityAction::DrinkMead => {
                let drink_res = {
                    let s = &mut *ctx;
                    s.city.try_drink_mead(&s.data.balance)
                };
                match drink_res {
                    Ok(_) => {
                        ctx.status_notice_key = None;
                        ctx.persist();
                        play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                    }
                    Err(err_key) => {
                        ctx.status_notice_key = Some(err_key.to_string());
                        play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    }
                }
                rebuild = true;
            }
            CityAction::ClaimQuest(quest_id) => {
                if let Some(quest) = ctx.data.quests.iter().find(|q| q.id == *quest_id).cloned() {
                    let claimed = {
                        let s = &mut *ctx;
                        s.city.try_claim_quest(&quest, &s.data.balance)
                    };
                    if claimed {
                        let summary =
                            modals::format_reward_summary(&ctx.data.i18n, quest.reward, 0);
                        ctx.celebration_message = Some(celebration_claim_line(
                            &ctx.data.i18n,
                            ctx.data.i18n.t(&quest.title_key).to_string(),
                            summary,
                        ));
                        ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                        ctx.status_notice_key = None;
                        ctx.persist();
                        play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                        rebuild = true;
                    }
                }
            }
            CityAction::FocusActiveQuest => {
                if let Some(quest) = ctx.city.active_quest(&ctx.data.quests).cloned() {
                    ctx.status_notice_key = None;
                    match &quest.condition {
                        QuestCondition::WinBuildingLevel { building_id, .. }
                        | QuestCondition::ReachBuildingTier { building_id, .. } => {
                            ctx.selected_building_id = building_id.clone();
                            ctx.selected_difficulty = 0;
                            layout_mode.active_drawer = CityDrawer::None;
                            layout_mode.inspector_open = true;
                            scroll_mem.inspector_offset_y = 0.0;
                            play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                            rebuild = true;
                        }
                        QuestCondition::WinSpecificLevel { level_id } => {
                            if let Some(level) = ctx.data.level_by_id(level_id).cloned() {
                                let diff = ctx
                                    .data
                                    .building_by_id(&level.building_id)
                                    .map(|b| {
                                        if b.legendary_level_id.as_deref()
                                            == Some(level_id.as_str())
                                        {
                                            2
                                        } else if b.mastery_level_id.as_deref()
                                            == Some(level_id.as_str())
                                        {
                                            1
                                        } else {
                                            0
                                        }
                                    })
                                    .unwrap_or(0);
                                ctx.selected_building_id = level.building_id;
                                ctx.selected_difficulty = diff;
                                layout_mode.active_drawer = CityDrawer::None;
                                layout_mode.inspector_open = true;
                                scroll_mem.inspector_offset_y = 0.0;
                                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                                rebuild = true;
                            }
                        }
                        QuestCondition::CraftAnyAmulet => {
                            ctx.active_modal = ActiveModal::Amulets;
                            scroll_mem.modal_offset_y = 0.0;
                            play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                            rebuild = true;
                        }
                        QuestCondition::ClaimAnyCollection => {
                            ctx.active_modal = ActiveModal::Collections;
                            scroll_mem.modal_offset_y = 0.0;
                            play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                            rebuild = true;
                        }
                        QuestCondition::DefeatBosses { .. } => {
                            ctx.active_modal = ActiveModal::Raids;
                            scroll_mem.modal_offset_y = 0.0;
                            play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                            rebuild = true;
                        }
                        QuestCondition::CompleteDailyTrials { .. } => {
                            if ctx.city.is_daily_available(today_day_index())
                                && ctx.start_daily_trial().is_ok()
                            {
                                next_state.set(AppState::Match3);
                                return;
                            }
                            ctx.status_notice_key = Some("city.panel.daily.done".to_string());
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                            rebuild = true;
                        }
                        QuestCondition::ReachExpeditionWave { .. } => {
                            match ctx.start_or_resume_expedition() {
                                Ok(true) => {
                                    next_state.set(AppState::Match3);
                                    return;
                                }
                                Ok(false) => {
                                    scroll_mem.modal_offset_y = 0.0;
                                    rebuild = true;
                                }
                                Err(err_key) => {
                                    ctx.status_notice_key = Some(err_key.to_string());
                                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                                    rebuild = true;
                                }
                            }
                        }
                    }
                }
            }
            CityAction::ToggleDrawer(drawer) => {
                layout_mode.inspector_open = false;
                layout_mode.active_drawer = if layout_mode.active_drawer == *drawer {
                    CityDrawer::None
                } else {
                    *drawer
                };
                scroll_mem.inspector_offset_y = 0.0;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::ToggleModal(modal) => {
                if ctx.active_modal == ActiveModal::ChapterDialogue {
                    ctx.pending_dialogue = None;
                }
                layout_mode.active_drawer = CityDrawer::None;
                ctx.toggle_modal(*modal);
                scroll_mem.modal_offset_y = 0.0;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::CloseModal => {
                if ctx.active_modal != ActiveModal::None {
                    if ctx.active_modal == ActiveModal::ChapterDialogue {
                        ctx.pending_dialogue = None;
                    }
                    ctx.active_modal = ActiveModal::None;
                    ctx.confirm_import_backup = false;
                    ctx.confirm_end_expedition = false;
                    ctx.status_notice_key = None;
                    scroll_mem.modal_offset_y = 0.0;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    rebuild = true;
                }
            }
            CityAction::CraftTool(tool) => {
                if let Some(recipe) = ctx.data.tool_shop.iter().find(|r| r.tool == *tool).cloned() {
                    match ctx.city.try_craft_tool(&recipe) {
                        Ok(_) => {
                            let tool_label = ctx.data.i18n.t_fmt(
                                &recipe.name_key,
                                &[("count", ctx.city.tools.count(*tool).to_string())],
                            );
                            ctx.celebration_message = Some(
                                ctx.data
                                    .i18n
                                    .t_fmt("city.panel.shop.crafted", &[("tool", tool_label)]),
                            );
                            ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                            ctx.status_notice_key = None;
                            ctx.persist();
                            play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                        }
                    }
                    rebuild = true;
                }
            }
            CityAction::ClaimCollection(col_id) => {
                if let Some(col) = ctx
                    .data
                    .collections
                    .iter()
                    .find(|c| c.id == *col_id)
                    .cloned()
                {
                    let claimed = {
                        let s = &mut *ctx;
                        s.city.try_claim_collection(&col, &s.data.balance)
                    };
                    if claimed {
                        let summary =
                            modals::format_reward_summary(&ctx.data.i18n, col.reward_resources, 0);
                        ctx.celebration_message = Some(celebration_claim_line(
                            &ctx.data.i18n,
                            ctx.data.i18n.t(&col.name_key).to_string(),
                            summary,
                        ));
                        ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                        ctx.status_notice_key = None;
                        ctx.persist();
                        play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                        rebuild = true;
                    }
                }
            }
            CityAction::ClaimAllCollections => {
                let (claimed_any, total_res) = {
                    let s = &mut *ctx;
                    let mut claimed_any = false;
                    let mut total_res = wyrdstone_core::ResourceBundle::default();
                    for col in &s.data.collections {
                        if s.city.try_claim_collection(col, &s.data.balance) {
                            claimed_any = true;
                            total_res.add(&col.reward_resources);
                        }
                    }
                    (claimed_any, total_res)
                };
                if claimed_any {
                    ctx.celebration_message =
                        Some(modals::format_reward_summary(&ctx.data.i18n, total_res, 0));
                    ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                    ctx.status_notice_key = None;
                    ctx.persist();
                    play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                    rebuild = true;
                }
            }
            CityAction::CraftOrEquipAmulet(amulet_id) => {
                if ctx.city.equipped_amulet.as_deref() == Some(amulet_id.as_str()) {
                    continue;
                }
                if let Some(amulet) = ctx.data.amulet_by_id(amulet_id).cloned() {
                    let prev_primary = ctx.city.equipped_amulet.clone();
                    let was_secondary =
                        ctx.city.equipped_amulet_secondary.as_deref() == Some(amulet_id.as_str());
                    let res = {
                        let s = &mut *ctx;
                        s.city.try_craft_amulet(&amulet, &s.data.balance)
                    };
                    match res {
                        Ok(()) => {
                            if was_secondary {
                                ctx.city.equipped_amulet_secondary = prev_primary;
                            }
                            ctx.status_notice_key = None;
                            ctx.persist();
                            play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                        }
                    }
                    rebuild = true;
                }
            }
            CityAction::EquipSecondaryAmulet(amulet_id) => {
                match ctx.city.try_equip_secondary_amulet(amulet_id) {
                    Ok(()) => {
                        ctx.status_notice_key = None;
                        ctx.persist();
                        play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                    }
                    Err(err_key) => {
                        ctx.status_notice_key = Some(err_key.to_string());
                        play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    }
                }
                rebuild = true;
            }
            CityAction::BrewWarDraught(draught) => {
                match ctx.city.try_brew_war_draught(*draught) {
                    Ok(()) => {
                        ctx.celebration_message = Some(ctx.data.i18n.t_fmt(
                            "city.panel.shop.brewed",
                            &[("name", ctx.data.i18n.t(draught.name_key()).to_string())],
                        ));
                        ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                        ctx.status_notice_key = None;
                        ctx.persist();
                        play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                    }
                    Err(err_key) => {
                        ctx.status_notice_key = Some(err_key.to_string());
                        play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    }
                }
                rebuild = true;
            }
            CityAction::UpgradeAmulet(amulet_id) => {
                if let Some(amulet) = ctx.data.amulet_by_id(amulet_id).cloned() {
                    let res = {
                        let s = &mut *ctx;
                        s.city.try_upgrade_amulet(&amulet, &s.data.balance)
                    };
                    match res {
                        Ok(_) => {
                            ctx.status_notice_key = None;
                            ctx.persist();
                            play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                        }
                        Err(err_key) => {
                            ctx.status_notice_key = Some(err_key.to_string());
                            play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                        }
                    }
                    rebuild = true;
                }
            }
            CityAction::AdvanceTutorial => {
                ctx.tutorial_step = ctx.tutorial_step.saturating_add(1).min(3);
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::ToggleColorblind => {
                ctx.confirm_import_backup = false;
                ctx.colorblind_mode = !ctx.colorblind_mode;
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::ToggleFastAnim => {
                ctx.confirm_import_backup = false;
                ctx.fast_animations = !ctx.fast_animations;
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::ToggleReducedMotion => {
                ctx.confirm_import_backup = false;
                ctx.reduced_motion = !ctx.reduced_motion;
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::CycleBgmVolume => {
                ctx.confirm_import_backup = false;
                cycle_bgm_volume(&mut ctx, &mut sound_bank, &mut bgm_sinks);
                rebuild = true;
            }
            CityAction::CycleSfxVolume => {
                ctx.confirm_import_backup = false;
                cycle_sfx_volume(&mut ctx, &mut sound_bank);
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::ExportBackup => match ctx.export_backup() {
                Ok(dest_path) => {
                    ctx.confirm_import_backup = false;
                    ctx.status_notice_key = None;
                    ctx.celebration_message = Some(
                        ctx.data
                            .i18n
                            .t_fmt("city.panel.backup.exported_path", &[("path", dest_path)]),
                    );
                    ctx.celebration_timer_secs = BACKUP_NOTICE_DURATION_SECS;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                    rebuild = true;
                }
                Err(err_key) => {
                    ctx.status_notice_key = Some(err_key.to_string());
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    rebuild = true;
                }
            },
            CityAction::ImportBackup => {
                if !ctx.confirm_import_backup {
                    ctx.confirm_import_backup = true;
                    ctx.status_notice_key = None;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    rebuild = true;
                    continue;
                }
                ctx.confirm_import_backup = false;
                match ctx.import_backup() {
                    Ok(()) => {
                        sound_bank.muted = ctx.audio_muted;
                        sound_bank.bgm_volume = ctx.bgm_volume;
                        sound_bank.sfx_volume = ctx.sfx_volume;
                        let target_vol = if sound_bank.muted {
                            0.0
                        } else {
                            sound_bank.bgm_volume
                        };
                        for sink in bgm_sinks.iter_mut() {
                            sink.set_volume(target_vol);
                        }
                        layout_mode.last_muted = sound_bank.muted;
                        layout_mode.last_displayed_energy = ctx.city.energy;
                        layout_mode.last_ready_harvests =
                            count_ready_harvests(&ctx, now_unix_secs());
                        ctx.status_notice_key = None;
                        ctx.celebration_message =
                            Some(ctx.data.i18n.t("city.panel.backup.imported").to_string());
                        ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                        play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                        rebuild = true;
                    }
                    Err(err_key) => {
                        ctx.status_notice_key = Some(err_key.to_string());
                        play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                        rebuild = true;
                    }
                }
            }
            CityAction::ToggleLanguage => {
                ctx.confirm_import_backup = false;
                ctx.data.i18n.toggle_language();
                ctx.persist();
                rebuild = true;
            }
            CityAction::ToggleAudio => {
                ctx.confirm_import_backup = false;
                toggle_mute(&mut ctx, &mut sound_bank, &mut bgm_sinks);
                rebuild = true;
            }
            CityAction::BackToTitle => {
                ctx.active_modal = ActiveModal::None;
                next_state.set(AppState::Title);
                return;
            }
        }
    }

    if rebuild {
        layout_mode.last_map_size = Vec2::ZERO;
        layout_mode.last_canvas_window_size = layout_mode.window_size;
        for entity in &roots {
            commands.entity(entity).despawn_recursive();
        }
        let just_built = layout_mode.just_built_id.take();
        spawn_city_tree(
            &mut commands,
            &ctx,
            &fjord_art,
            &vfx,
            &sound_bank,
            &scroll_mem,
            layout_mode.is_portrait,
            layout_mode.window_size.x,
            layout_mode.window_size.y,
            layout_mode.active_drawer,
            layout_mode.inspector_open,
            just_built.as_deref(),
        );
    }
}

fn claim_login_tribute(
    commands: &mut Commands,
    ctx: &mut GameContext,
    sound_bank: &SoundBank,
) -> bool {
    let claim = ctx
        .city
        .try_claim_login_tribute_with(today_day_index(), &ctx.data.balance);
    if let Some((day, res, _tool)) = claim {
        let summary = modals::format_reward_summary(&ctx.data.i18n, res, 0);
        let title = ctx
            .data
            .i18n
            .t_fmt("city.panel.tribute.day", &[("day", day.to_string())]);
        ctx.celebration_message = Some(celebration_claim_line(&ctx.data.i18n, title, summary));
        ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
        ctx.status_notice_key = None;
        ctx.persist();
        play_sfx(commands, sound_bank, SfxKind::Victory);
        return true;
    }
    false
}

fn chapter_dialogue_for_building(building_id: &str) -> Option<ChapterDialogue> {
    let speaker = match building_id {
        wyrdstone_core::HUB_BUILDING_ID | "watchtower" => "jarl",
        "forge" | "meadery" => "brokk",
        "longship_dock" | "odin_shrine" => "volva",
        _ => return None,
    };
    Some(ChapterDialogue {
        speaker: speaker.to_string(),
        title_key: format!("chapter.{building_id}.title"),
        line1_key: format!("chapter.{building_id}.line1"),
        line2_key: format!("chapter.{building_id}.line2"),
    })
}

/// Animates gentle falling snow, aurora borealis ribbons, water reflections, rising building embers, smoke, build pops, and beacons.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn animate_city_ambience(
    mut commands: Commands,
    time: Res<Time>,
    ctx: Res<GameContext>,
    parent_nodes: Query<
        &ComputedNode,
        (
            Without<FjordSnowflake>,
            Without<FjordAurora>,
            Without<FjordEmber>,
            Without<FjordWaterShimmer>,
            Without<FjordSmokePuff>,
            Without<BuildingBuildAnim>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut snow_q: Query<
        (&mut FjordSnowflake, &Parent, &mut Transform),
        (
            Without<FjordAurora>,
            Without<FjordEmber>,
            Without<FjordWaterShimmer>,
            Without<FjordSmokePuff>,
            Without<FjordBeaconGlow>,
            Without<BuildingBuildAnim>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut aurora_q: Query<
        (&FjordAurora, &Parent, &mut Transform, &mut ImageNode),
        (
            Without<FjordSnowflake>,
            Without<FjordEmber>,
            Without<FjordWaterShimmer>,
            Without<FjordSmokePuff>,
            Without<FjordBeaconGlow>,
            Without<BuildingBuildAnim>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut ember_q: Query<
        (&mut FjordEmber, &Parent, &mut Transform, &mut ImageNode),
        (
            Without<FjordSnowflake>,
            Without<FjordAurora>,
            Without<FjordWaterShimmer>,
            Without<FjordSmokePuff>,
            Without<FjordBeaconGlow>,
            Without<BuildingBuildAnim>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut water_q: Query<
        (&FjordWaterShimmer, &Parent, &mut Transform, &mut ImageNode),
        (
            Without<FjordSnowflake>,
            Without<FjordAurora>,
            Without<FjordEmber>,
            Without<FjordSmokePuff>,
            Without<FjordBeaconGlow>,
            Without<BuildingBuildAnim>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut smoke_q: Query<
        (&mut FjordSmokePuff, &Parent, &mut Transform, &mut ImageNode),
        (
            Without<FjordSnowflake>,
            Without<FjordAurora>,
            Without<FjordEmber>,
            Without<FjordWaterShimmer>,
            Without<FjordBeaconGlow>,
            Without<BuildingBuildAnim>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut beacon_glow_q: Query<
        (&FjordBeaconGlow, &mut ImageNode),
        (
            Without<FjordSnowflake>,
            Without<FjordAurora>,
            Without<FjordEmber>,
            Without<FjordWaterShimmer>,
            Without<FjordSmokePuff>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut build_anim_q: Query<
        (Entity, &mut BuildingBuildAnim, &mut Transform),
        (
            Without<FjordSnowflake>,
            Without<FjordAurora>,
            Without<FjordEmber>,
            Without<FjordWaterShimmer>,
            Without<FjordSmokePuff>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut build_ring_q: Query<
        (
            Entity,
            &mut BuildingBuildRing,
            &mut Transform,
            &mut ImageNode,
        ),
        (
            Without<FjordSnowflake>,
            Without<FjordAurora>,
            Without<FjordEmber>,
            Without<FjordWaterShimmer>,
            Without<FjordSmokePuff>,
            Without<FjordBeaconGlow>,
            Without<BuildingBuildAnim>,
        ),
    >,
    mut beacon_q: Query<&mut BorderColor, With<QuestBeaconPin>>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();

    if !ctx.reduced_motion {
        for (mut flake, parent, mut tf) in &mut snow_q {
            flake.y_pct += flake.speed_y * dt;
            if flake.y_pct > 98.0 {
                flake.y_pct = 1.0;
            }
            let sway = (t * 1.1 + flake.phase).sin() * 1.6;
            let target_x = (flake.x_pct + sway).clamp(1.0, 98.0);
            let psize = parent_nodes
                .get(parent.get())
                .map(|n| n.size())
                .unwrap_or(Vec2::ZERO);
            tf.translation.x += (target_x - flake.base_x_pct) * 0.01 * psize.x;
            tf.translation.y += (flake.y_pct - flake.base_y_pct) * 0.01 * psize.y;
        }

        for (aurora, parent, mut tf, mut img) in &mut aurora_q {
            let drift = (t * 0.35 + aurora.phase).sin() * 5.0;
            let pulse = 0.72 + 0.28 * (t * 0.8 + aurora.phase).sin();
            let target_left = (aurora.base_left_pct + drift).clamp(0.0, 60.0);
            let pw = parent_nodes
                .get(parent.get())
                .map(|n| n.size().x)
                .unwrap_or(0.0);
            tf.translation.x += (target_left - aurora.base_left_pct) * 0.01 * pw;
            img.color = img
                .color
                .with_alpha((aurora.base_alpha * pulse).clamp(0.05, 0.45));
        }

        for (water, parent, mut tf, mut img) in &mut water_q {
            let ripple = (t * 0.65 + water.phase).sin() * 2.2;
            let glint = 0.65 + 0.35 * (t * 1.4 + water.phase * 1.3).cos();
            let target_left = (water.base_left_pct + ripple).clamp(15.0, 70.0);
            let pw = parent_nodes
                .get(parent.get())
                .map(|n| n.size().x)
                .unwrap_or(0.0);
            tf.translation.x += (target_left - water.base_left_pct) * 0.01 * pw;
            img.color = img
                .color
                .with_alpha((water.base_alpha * glint).clamp(0.05, 0.36));
        }

        for (mut ember, parent, mut tf, mut img) in &mut ember_q {
            ember.y_pct -= 22.0 * dt;
            if ember.y_pct < 6.0 {
                ember.y_pct = 86.0;
            }
            let sway = (t * 2.4 + ember.phase).sin() * 4.0;
            let alpha = ((ember.y_pct - 6.0) / 80.0).clamp(0.1, 0.9);
            let target_x = (ember.base_x_pct + sway).clamp(4.0, 94.0);
            let psize = parent_nodes
                .get(parent.get())
                .map(|n| n.size())
                .unwrap_or(Vec2::ZERO);
            tf.translation.x += (target_x - ember.base_x_pct) * 0.01 * psize.x;
            tf.translation.y += (ember.y_pct - ember.base_y_pct) * 0.01 * psize.y;
            img.color = img.color.with_alpha(alpha);
        }

        for (mut smoke, parent, mut tf, mut img) in &mut smoke_q {
            smoke.y_pct -= 12.0 * dt;
            if smoke.y_pct < -24.0 {
                smoke.y_pct = 18.0;
            }
            let life = ((18.0 - smoke.y_pct) / 42.0).clamp(0.0, 1.0);
            let sway = (t * 1.4 + smoke.phase).sin() * (4.0 + life * 8.0);
            let size = 12.0 + life * 16.0;
            let psize = parent_nodes
                .get(parent.get())
                .map(|n| n.size())
                .unwrap_or(Vec2::ZERO);
            tf.translation.x += sway * 0.01 * psize.x;
            tf.translation.y += (smoke.y_pct - smoke.base_y_pct) * 0.01 * psize.y;
            tf.scale = Vec3::splat(size / 16.0);
            img.color = img.color.with_alpha((1.0 - life) * 0.28);
        }

        for (b_glow, mut img) in &mut beacon_glow_q {
            let flicker =
                (t * 6.8 + b_glow.phase).sin() * 0.5 + (t * 13.4 + b_glow.phase * 1.5).cos() * 0.5;
            img.color = Color::srgba(
                1.0,
                0.66 + flicker * 0.08,
                0.20,
                (0.50 + flicker * 0.16).clamp(0.28, 0.72),
            );
        }
    }

    for (entity, mut anim, mut tf) in &mut build_anim_q {
        anim.elapsed += dt;
        let u = (anim.elapsed / anim.duration).clamp(0.0, 1.0);
        if u >= 1.0 || ctx.reduced_motion {
            tf.scale = Vec3::ONE;
            commands.entity(entity).remove::<BuildingBuildAnim>();
        } else {
            // Elastic construction pop: 0.25 -> 1.18 -> 1.0
            let scale = if u < 0.62 {
                let p = u / 0.62;
                0.25 + (1.0 - (1.0 - p) * (1.0 - p)) * 0.93
            } else {
                let p = (u - 0.62) / 0.38;
                1.18 - 0.18 * p
            };
            tf.scale = Vec3::splat(scale);
        }
    }

    for (entity, mut ring, mut tf, mut img) in &mut build_ring_q {
        ring.elapsed += dt;
        let u = (ring.elapsed / ring.duration).clamp(0.0, 1.0);
        if u >= 1.0 || ctx.reduced_motion {
            commands.entity(entity).despawn();
        } else {
            let eased = 1.0 - (1.0 - u) * (1.0 - u);
            tf.scale = Vec3::splat(0.35 + eased * 1.45);
            img.color = Color::srgba(1.0, 0.86, 0.34, (1.0 - u).powf(1.4));
        }
    }

    let glow = if ctx.reduced_motion {
        BEACON_STATIC_GLOW
    } else {
        0.5 + 0.5 * (t * 4.5).sin()
    };
    let beacon_col = Color::srgb(0.95 + 0.05 * glow, 0.72 + 0.25 * glow, 0.24 + 0.45 * glow);
    for mut border in &mut beacon_q {
        border.0 = beacon_col;
    }
}

/// Vertical padding reserved below each building sprite for its nameplate + optional harvest button.
const PIN_NAMEPLATE_RESERVE_PX: f32 = 96.0;

/// Computes responsive `(left, top, pin_w, sprite_w, sprite_h)` in map pixels for a `MapBuildingPin`.
pub(crate) fn compute_pin_layout(
    pin: &MapBuildingPin,
    map_w: f32,
    map_h: f32,
    is_portrait: bool,
) -> (f32, f32, f32, f32, f32) {
    if is_portrait {
        let pin_w = (map_w * 0.30).clamp(100.0, 124.0);
        let unit = (map_w * 0.22).min(map_h * 0.125).clamp(48.0, 88.0);
        let sprite_w = unit * pin.tier_scale;
        let sprite_h = sprite_w * 0.88;
        let left = match pin.portrait_left_pct {
            Some(pct) => (pct / 100.0) * map_w,
            None => (pin.map_x_pct / 100.0) * map_w - pin_w * 0.5,
        };
        let top = match pin.portrait_top_pct {
            Some(pct) => (pct / 100.0) * map_h,
            None => (pin.map_y_pct / 100.0) * map_h - sprite_h,
        };
        let max_left = (map_w - pin_w - 4.0).max(4.0);
        let max_top = (map_h - sprite_h - PIN_NAMEPLATE_RESERVE_PX).max(4.0);
        return (
            left.clamp(4.0, max_left),
            top.clamp(4.0, max_top),
            pin_w,
            sprite_w,
            sprite_h,
        );
    }
    let unit = (map_w * 0.115).min(map_h * 0.205).max(44.0);
    let pin_w = (unit * 1.06).clamp(108.0, 210.0);
    let sprite_w = unit * pin.tier_scale;
    let sprite_h = sprite_w * 0.88;
    let anchor_x = (pin.map_x_pct / 100.0) * map_w;
    let anchor_y = (pin.map_y_pct / 100.0) * map_h;
    let nameplate_reserve = (map_h * 0.11).clamp(52.0, 96.0);
    let max_left = (map_w - pin_w - 4.0).max(4.0);
    let max_top = (map_h - sprite_h - nameplate_reserve).max(4.0);
    (
        (anchor_x - pin_w * 0.5).clamp(4.0, max_left),
        (anchor_y - sprite_h).clamp(4.0, max_top),
        pin_w,
        sprite_w,
        sprite_h,
    )
}

#[allow(clippy::type_complexity)]
type CityBackdropNodeQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Node,
    (
        With<FjordMapBackdrop>,
        Without<FjordMapContainer>,
        Without<MapBuildingPin>,
        Without<MapBuildingStage>,
    ),
>;

#[allow(clippy::type_complexity)]
type CityMapContainerQuery<'w, 's> = Query<
    'w,
    's,
    (&'static ComputedNode, &'static mut Node),
    (
        With<FjordMapContainer>,
        Without<FjordMapBackdrop>,
        Without<MapBuildingPin>,
        Without<MapBuildingStage>,
    ),
>;

/// Keeps the 16:9 Fjord map canvas and every building's base center anchored to its exact terrain spot across all window resolutions.
#[allow(clippy::type_complexity)]
pub fn sync_map_building_layout(
    mut layout_mode: ResMut<CityLayoutMode>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut backdrop_q: CityBackdropNodeQuery,
    mut map_q: CityMapContainerQuery,
    mut pins: Query<
        (&MapBuildingPin, &mut Node),
        (
            Without<MapBuildingStage>,
            Without<FjordMapContainer>,
            Without<FjordMapBackdrop>,
        ),
    >,
    mut stages: Query<
        (&MapBuildingStage, &mut Node),
        (
            Without<MapBuildingPin>,
            Without<FjordMapContainer>,
            Without<FjordMapBackdrop>,
        ),
    >,
) {
    if let Ok(win) = windows.get_single() {
        let (win_w, win_h) = (win.width(), win.height());
        if win_w.is_finite()
            && win_h.is_finite()
            && win_w >= 100.0
            && win_h >= 100.0
            && ((win_w - layout_mode.last_canvas_window_size.x).abs() >= 1.0
                || (win_h - layout_mode.last_canvas_window_size.y).abs() >= 1.0)
        {
            layout_mode.last_canvas_window_size = Vec2::new(win_w, win_h);
            let (cover_w, cover_h) = compute_backdrop_cover_size(win_w, win_h);
            for mut bd_node in &mut backdrop_q {
                bd_node.width = Val::Px(cover_w);
                bd_node.height = Val::Px(cover_h);
            }
            let (canvas_w, canvas_h) =
                compute_fjord_canvas_size(win_w, win_h, layout_mode.is_portrait);
            for (_, mut map_node) in &mut map_q {
                map_node.width = Val::Px(canvas_w);
                map_node.height = Val::Px(canvas_h);
            }
        }
    }
    let Ok((map_computed, _)) = map_q.get_single() else {
        return;
    };
    let inv = map_computed.inverse_scale_factor();
    if !inv.is_finite() || inv <= 0.0 {
        return;
    }
    let map_w = map_computed.size().x * inv;
    let map_h = map_computed.size().y * inv;
    if !map_w.is_finite() || !map_h.is_finite() || map_w < 100.0 || map_h < 100.0 {
        return;
    }
    if (map_w - layout_mode.last_map_size.x).abs() < 1.0
        && (map_h - layout_mode.last_map_size.y).abs() < 1.0
    {
        return;
    }
    layout_mode.last_map_size = Vec2::new(map_w, map_h);

    for (pin, mut node) in &mut pins {
        let (target_left, target_top, pin_w, _, _) =
            compute_pin_layout(pin, map_w, map_h, layout_mode.is_portrait);
        if target_left.is_finite() && target_top.is_finite() && pin_w.is_finite() {
            node.left = Val::Px(target_left);
            node.top = Val::Px(target_top);
            node.width = Val::Px(pin_w);
        }
    }

    let unit = if layout_mode.is_portrait {
        (map_w * 0.22).min(map_h * 0.125).clamp(48.0, 88.0)
    } else {
        (map_w * 0.115).min(map_h * 0.205).max(44.0)
    };
    for (stage, mut node) in &mut stages {
        let target_w = unit * stage.tier_scale;
        let target_h = target_w * 0.88;
        if target_w.is_finite() && target_h.is_finite() {
            node.width = Val::Px(target_w);
            node.height = Val::Px(target_h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wyrdstone_core::{GameDataBundle, Language};

    const MIN_PORTRAIT_MAP_WIDTH_PX: f32 = 336.0;
    const MIN_PORTRAIT_MAP_HEIGHT_PX: f32 = 560.0;
    const MIN_LANDSCAPE_MAP_WIDTH_PX: f32 = 1336.0;
    const MIN_LANDSCAPE_MAP_HEIGHT_PX: f32 = 500.0;
    const NAMEPLATE_HEIGHT_PX: f32 = 44.0;

    #[test]
    fn portrait_pins_never_overlap() {
        let data = GameDataBundle::load_bundled(Language::En).expect("bundled data must parse");
        let boxes: Vec<(&str, f32, f32, f32, f32)> = data
            .buildings
            .iter()
            .map(|b| {
                let pin = MapBuildingPin {
                    map_x_pct: b.map_x_pct,
                    map_y_pct: b.map_y_pct,
                    portrait_left_pct: b.portrait_left_pct,
                    portrait_top_pct: b.portrait_top_pct,
                    tier_scale: 1.20,
                };
                let (left, top, pin_w, _, sprite_h) = compute_pin_layout(
                    &pin,
                    MIN_PORTRAIT_MAP_WIDTH_PX,
                    MIN_PORTRAIT_MAP_HEIGHT_PX,
                    true,
                );
                assert!(
                    left >= 4.0 && left + pin_w <= MIN_PORTRAIT_MAP_WIDTH_PX,
                    "portrait pin {} leaves map horizontally (left={left}, w={pin_w})",
                    b.id
                );
                assert!(
                    top >= 4.0
                        && top + sprite_h + NAMEPLATE_HEIGHT_PX <= MIN_PORTRAIT_MAP_HEIGHT_PX,
                    "portrait pin {} leaves map vertically (top={top}, h={})",
                    b.id,
                    sprite_h + NAMEPLATE_HEIGHT_PX
                );
                (
                    b.id.as_str(),
                    left,
                    top,
                    pin_w,
                    sprite_h + NAMEPLATE_HEIGHT_PX,
                )
            })
            .collect();

        for (i, a) in boxes.iter().enumerate() {
            for b in &boxes[i + 1..] {
                let separated_x = a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1;
                let separated_y = a.2 + a.4 <= b.2 || b.2 + b.4 <= a.2;
                assert!(
                    separated_x || separated_y,
                    "portrait Tier-3 pins {} and {} overlap: {:?} vs {:?}",
                    a.0,
                    b.0,
                    a,
                    b
                );
            }
        }
    }

    #[test]
    fn landscape_tier3_pins_stay_within_map_and_separated() {
        let data = GameDataBundle::load_bundled(Language::En).expect("bundled data must parse");
        let boxes: Vec<(&str, f32, f32, f32, f32)> = data
            .buildings
            .iter()
            .map(|b| {
                let pin = MapBuildingPin {
                    map_x_pct: b.map_x_pct,
                    map_y_pct: b.map_y_pct,
                    portrait_left_pct: b.portrait_left_pct,
                    portrait_top_pct: b.portrait_top_pct,
                    tier_scale: 1.20,
                };
                let (left, top, pin_w, _, sprite_h) = compute_pin_layout(
                    &pin,
                    MIN_LANDSCAPE_MAP_WIDTH_PX,
                    MIN_LANDSCAPE_MAP_HEIGHT_PX,
                    false,
                );
                assert!(
                    left >= 4.0 && left + pin_w <= MIN_LANDSCAPE_MAP_WIDTH_PX,
                    "landscape pin {} leaves map horizontally (left={left}, w={pin_w})",
                    b.id
                );
                assert!(
                    top >= 4.0
                        && top + sprite_h + NAMEPLATE_HEIGHT_PX <= MIN_LANDSCAPE_MAP_HEIGHT_PX,
                    "landscape pin {} leaves map vertically (top={top}, h={})",
                    b.id,
                    sprite_h + NAMEPLATE_HEIGHT_PX
                );
                (
                    b.id.as_str(),
                    left,
                    top,
                    pin_w,
                    sprite_h + NAMEPLATE_HEIGHT_PX,
                )
            })
            .collect();

        for (i, a) in boxes.iter().enumerate() {
            for b in &boxes[i + 1..] {
                let separated_x = a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1;
                let separated_y = a.2 + a.4 <= b.2 || b.2 + b.4 <= a.2;
                assert!(
                    separated_x || separated_y,
                    "landscape Tier-3 pins {} and {} overlap: {:?} vs {:?}",
                    a.0,
                    b.0,
                    a,
                    b
                );
            }
        }
    }

    #[test]
    fn landscape_buildings_keep_identical_relative_positions_across_resolutions() {
        let data = GameDataBundle::load_bundled(Language::En).expect("bundled data must parse");
        for &(win_w, win_h, exp_w, exp_h) in &[
            (1280.0_f32, 720.0_f32, 1280.0_f32, 720.0_f32),
            (1920.0, 1080.0, 1920.0, 1080.0),
            (2560.0, 1440.0, 2560.0, 1440.0),
            (3440.0, 1440.0, 2560.0, 1440.0),
        ] {
            let (cw, ch) = compute_fjord_canvas_size(win_w, win_h, false);
            assert!((cw - exp_w).abs() < 0.5, "{cw} != {exp_w}");
            assert!((ch - exp_h).abs() < 0.5, "{ch} != {exp_h}");
        }

        let (base_w, base_h) = compute_fjord_canvas_size(1920.0, 1080.0, false);
        let (hd_w, hd_h) = compute_fjord_canvas_size(1280.0, 720.0, false);
        let (uw_w, uw_h) = compute_fjord_canvas_size(3440.0, 1440.0, false);

        let mut boxes_2k = Vec::new();
        for b in &data.buildings {
            let pin = MapBuildingPin {
                map_x_pct: b.map_x_pct,
                map_y_pct: b.map_y_pct,
                portrait_left_pct: b.portrait_left_pct,
                portrait_top_pct: b.portrait_top_pct,
                tier_scale: 1.20,
            };
            let (l1, t1, pw1, sw1, sh1) = compute_pin_layout(&pin, base_w, base_h, false);
            let (l0, t0, pw0, sw0, sh0) = compute_pin_layout(&pin, hd_w, hd_h, false);
            let (l2, t2, pw2, sw2, sh2) = compute_pin_layout(&pin, uw_w, uw_h, false);

            let cx1 = (l1 + pw1 * 0.5) / base_w;
            let cx0 = (l0 + pw0 * 0.5) / hd_w;
            let cx2 = (l2 + pw2 * 0.5) / uw_w;
            let by1 = (t1 + sh1) / base_h;
            let by0 = (t0 + sh0) / hd_h;
            let by2 = (t2 + sh2) / uw_h;
            assert!((cx1 - cx0).abs() < 0.002 && (cx1 - cx2).abs() < 0.002);
            assert!((by1 - by0).abs() < 0.002 && (by1 - by2).abs() < 0.002);
            assert!((sw1 / base_w - sw0 / hd_w).abs() < 0.002);
            assert!((sw1 / base_w - sw2 / uw_w).abs() < 0.002);

            assert!(l2 >= 4.0 && l2 + pw2 <= uw_w);
            assert!(t2 >= 4.0 && t2 + sh2 + NAMEPLATE_HEIGHT_PX <= uw_h);
            boxes_2k.push((b.id.as_str(), l2, t2, pw2, sh2 + NAMEPLATE_HEIGHT_PX));
        }

        for (i, a) in boxes_2k.iter().enumerate() {
            for b in &boxes_2k[i + 1..] {
                let separated_x = a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1;
                let separated_y = a.2 + a.4 <= b.2 || b.2 + b.4 <= a.2;
                assert!(
                    separated_x || separated_y,
                    "ultrawide Tier-3 pins {} and {} overlap: {:?} vs {:?}",
                    a.0,
                    b.0,
                    a,
                    b
                );
            }
        }
    }
}
