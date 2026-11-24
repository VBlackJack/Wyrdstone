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
use wyrdstone_core::{ActiveTool, BuildingDef, I18nCatalog, PatronGod, QuestCondition};

const CELEBRATION_DURATION_SECS: f32 = 3.2;
const BACKUP_NOTICE_DURATION_SECS: f32 = 6.0;
const SNOWFLAKE_COUNT: usize = 22;
const HARVEST_POLL_SECS: f32 = 5.0;

#[derive(Component)]
pub struct CityRoot;

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
    pub x_pct: f32,
    pub y_pct: f32,
    pub speed_y: f32,
    pub phase: f32,
}

/// Rising warm ember spark glowing on rebuilt (`tier > 0`) Fjord buildings.
#[derive(Component)]
pub struct FjordEmber {
    pub base_x_pct: f32,
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
    pub base_x_pct: f32,
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

/// Tracks City layout mode, displayed endurance, audio state, and periodic harvest readiness checks.
#[derive(Resource)]
pub struct CityLayoutMode {
    pub is_portrait: bool,
    pub menu_open: bool,
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
    PickExpeditionBoon(usize),
    EndExpedition,
    FightBoss(String),
    SelectPatron(PatronGod),
    ClaimAchievement(String),
    ClaimAllAchievements,
    UpgradeBuilding(String),
    DrinkMead,
    ClaimQuest(String),
    FocusActiveQuest,
    ToggleMenu,
    ToggleModal(ActiveModal),
    CloseModal,
    ClaimCollection(String),
    ClaimAllCollections,
    CraftTool(ActiveTool),
    CraftOrEquipAmulet(String),
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
    let is_portrait = windows
        .get_single()
        .map(|w| w.width() < w.height())
        .unwrap_or(false);
    if ctx.active_modal == ActiveModal::None
        && ctx.city.is_login_tribute_available(today_day_index())
    {
        ctx.active_modal = ActiveModal::LoginCalendar;
    }
    let ready_harvests = count_ready_harvests(&ctx, now_unix_secs());
    commands.insert_resource(CityLayoutMode {
        is_portrait,
        menu_open: false,
        inspector_open: false,
        last_displayed_energy: ctx.city.energy,
        last_ready_harvests: ready_harvests,
        last_muted: sound_bank.muted,
        harvest_poll_timer: Timer::from_seconds(HARVEST_POLL_SECS, TimerMode::Repeating),
        ui_clock_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        just_built_id: None,
        last_notice_key: ctx.status_notice_key.clone(),
        notice_secs_left: NOTICE_TOAST_DURATION_SECS,
    });
    spawn_city_tree(
        &mut commands,
        &ctx,
        &fjord_art,
        &vfx,
        &sound_bank,
        &scroll_mem,
        is_portrait,
        false,
        false,
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

#[allow(clippy::too_many_arguments)]
fn spawn_city_tree(
    commands: &mut Commands,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    vfx: &VfxTextures,
    sound_bank: &SoundBank,
    scroll_mem: &UiScrollMemory,
    is_portrait: bool,
    menu_open: bool,
    inspector_open: bool,
    just_built_id: Option<&str>,
) {
    let i18n = &ctx.data.i18n;
    let res = ctx.city.resources;
    let max_energy = ctx.city.max_energy(&ctx.data.balance);
    let now = now_unix_secs();

    commands
        .spawn((
            CityRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(COLOR_BG_DEEP),
        ))
        .with_children(|root| {
            // 1. Top Resource & Navigation Bar
            root.spawn((
                sliced_panel_image(vfx),
                Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(11.0)),
                    row_gap: Val::Px(6.0),
                    column_gap: Val::Px(10.0),
                    ..default()
                },
                panel_shadow(),
            ))
            .with_children(|bar| {
                bar.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(1.0),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        Text::new(i18n.t("city.header.title")),
                        display_font(fjord_art, FONT_HEADING),
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
                            &[("rank", rank_label.to_string()), ("mastery", mastery_label)],
                        )
                    } else {
                        let perk_label = i18n.t(rank.perk_key());
                        i18n.t_fmt(
                            "city.header.subtitle_full",
                            &[
                                ("rank", rank_label.to_string()),
                                ("mastery", mastery_label),
                                ("perk", perk_label.to_string()),
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

                bar.spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|pills| {
                    spawn_resource_pill_with_icon(
                        pills,
                        vfx.icon_wood.clone(),
                        &resource_pill_text(i18n, "resources.label.wood", res.wood),
                        Color::srgb(0.44, 0.80, 0.52),
                    );
                    spawn_resource_pill_with_icon(
                        pills,
                        vfx.icon_stone.clone(),
                        &resource_pill_text(i18n, "resources.label.stone", res.stone),
                        Color::srgb(0.76, 0.74, 0.88),
                    );
                    spawn_resource_pill_with_icon(
                        pills,
                        vfx.icon_gold.clone(),
                        &resource_pill_text(i18n, "resources.label.gold", res.gold),
                        COLOR_ACCENT_GOLD,
                    );
                    spawn_resource_pill_with_icon(
                        pills,
                        vfx.icon_mead.clone(),
                        &resource_pill_text(i18n, "resources.label.mead", res.mead),
                        Color::srgb(0.95, 0.66, 0.30),
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
                            BorderRadius::all(Val::Px(6.0)),
                        ))
                        .with_children(|pill| {
                            pill.spawn((
                                ImageNode::new(vfx.icon_energy.clone()),
                                Node {
                                    width: Val::Px(16.0),
                                    height: Val::Px(16.0),
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
                                TextColor(Color::srgb(0.45, 0.84, 0.96)),
                            ));
                        });
                    if ctx.city.stats.win_streak >= 2 {
                        spawn_resource_pill(
                            pills,
                            &i18n.t_fmt(
                                "city.header.win_streak",
                                &[("streak", ctx.city.stats.win_streak.to_string())],
                            ),
                            COLOR_ACCENT_GOLD,
                        );
                    }
                    let blessing = ctx.today_blessing();
                    let blessing_lbl = if is_portrait {
                        i18n.t(blessing.name_key()).to_string()
                    } else {
                        i18n.t_fmt(
                            "city.header.blessing_pill",
                            &[
                                ("name", i18n.t(blessing.name_key()).to_string()),
                                ("desc", i18n.t(blessing.desc_key()).to_string()),
                            ],
                        )
                    };
                    spawn_resource_pill(pills, &blessing_lbl, COLOR_ACCENT_TEAL);
                    if ctx.city.energy < max_energy {
                        let can_drink = res.mead >= ctx.data.balance.mead_refill_cost;
                        let lbl = i18n.t_fmt(
                            "city.button.drink_mead",
                            &[
                                ("energy", ctx.data.balance.mead_refill_energy.to_string()),
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

                bar.spawn(Node {
                    column_gap: Val::Px(10.0),
                    row_gap: Val::Px(6.0),
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|groups| {
                    // Group A: Play Modes & Harvests
                    groups
                        .spawn((
                            Node {
                                column_gap: Val::Px(6.0),
                                row_gap: Val::Px(4.0),
                                flex_wrap: FlexWrap::Wrap,
                                padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.08, 0.11, 0.17, 0.72)),
                            BorderColor(Color::srgba(0.93, 0.74, 0.29, 0.28)),
                            BorderRadius::all(Val::Px(8.0)),
                        ))
                        .with_children(|btns| {
                            let ready_harvests = count_ready_harvests(ctx, now);
                            if ready_harvests >= 2 {
                                spawn_small_button_colored(
                                    btns,
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
                            let daily_ready = ctx.city.is_daily_available(today_day_index());
                            let daily_label = if daily_ready {
                                if ctx.city.stats.daily_streak > 0 {
                                    i18n.t_fmt(
                                        "city.button.label_with_count",
                                        &[
                                            (
                                                "label",
                                                i18n.t("city.panel.daily.button").to_string(),
                                            ),
                                            (
                                                "count",
                                                i18n.t_fmt(
                                                    "city.panel.daily.streak",
                                                    &[(
                                                        "days",
                                                        ctx.city.stats.daily_streak.to_string(),
                                                    )],
                                                ),
                                            ),
                                        ],
                                    )
                                } else {
                                    i18n.t("city.panel.daily.button").to_string()
                                }
                            } else {
                                i18n.t("city.panel.daily.done").to_string()
                            };
                            spawn_small_button_colored(
                                btns,
                                &daily_label,
                                CityAction::PlayDailyTrial,
                                if daily_ready {
                                    COLOR_ACCENT_GOLD
                                } else {
                                    COLOR_BUTTON_DISABLED
                                },
                                if daily_ready {
                                    COLOR_TEXT_DARK
                                } else {
                                    COLOR_TEXT_PRIMARY
                                },
                                daily_ready,
                            );
                            spawn_small_button(
                                btns,
                                i18n.t("city.panel.raids.button"),
                                CityAction::ToggleModal(ActiveModal::Raids),
                                COLOR_ACCENT_CRIMSON,
                                true,
                            );
                            let exp_lbl = if let Some(run) = &ctx.active_expedition {
                                i18n.t_fmt(
                                    "city.panel.expedition.resume",
                                    &[("wave", run.wave.to_string())],
                                )
                            } else {
                                i18n.t("city.panel.expedition.button").to_string()
                            };
                            spawn_small_button(
                                btns,
                                &exp_lbl,
                                CityAction::StartExpedition,
                                if ctx.active_expedition.is_some() {
                                    COLOR_ACCENT_GREEN
                                } else {
                                    COLOR_ACCENT_TEAL
                                },
                                true,
                            );
                        });

                    // Group B: Clan Progression, Gear & Collapsible Menu (with numeric claimable badges)
                    groups
                        .spawn((
                            Node {
                                column_gap: Val::Px(6.0),
                                row_gap: Val::Px(4.0),
                                flex_wrap: FlexWrap::Wrap,
                                padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.08, 0.11, 0.17, 0.72)),
                            BorderColor(COLOR_PANEL_BORDER),
                            BorderRadius::all(Val::Px(8.0)),
                        ))
                        .with_children(|btns| {
                            let claimable_feats = ctx.city.claimable_achievement_count(
                                &ctx.data.achievements,
                                &ctx.data.buildings,
                                ctx.data.bosses.len(),
                            );
                            let feats_lbl = if claimable_feats > 0 {
                                i18n.t_fmt(
                                    "city.button.label_with_count",
                                    &[
                                        ("label", i18n.t("city.panel.feats.button").to_string()),
                                        ("count", claimable_feats.to_string()),
                                    ],
                                )
                            } else {
                                i18n.t("city.panel.feats.button").to_string()
                            };
                            spawn_small_button(
                                btns,
                                &feats_lbl,
                                CityAction::ToggleModal(ActiveModal::Achievements),
                                if claimable_feats > 0 {
                                    COLOR_ACCENT_GREEN
                                } else {
                                    COLOR_PANEL_CARD
                                },
                                true,
                            );
                            let claimable_cols =
                                ctx.city.claimable_collection_count(&ctx.data.collections);
                            let cols_lbl = if claimable_cols > 0 {
                                i18n.t_fmt(
                                    "city.button.label_with_count",
                                    &[
                                        (
                                            "label",
                                            i18n.t("city.panel.collections.button").to_string(),
                                        ),
                                        ("count", claimable_cols.to_string()),
                                    ],
                                )
                            } else {
                                i18n.t("city.panel.collections.button").to_string()
                            };
                            spawn_small_button(
                                btns,
                                &cols_lbl,
                                CityAction::ToggleModal(ActiveModal::Collections),
                                if claimable_cols > 0 {
                                    COLOR_ACCENT_GREEN
                                } else {
                                    COLOR_PANEL_CARD
                                },
                                true,
                            );
                            spawn_small_button(
                                btns,
                                i18n.t("city.panel.amulets.button"),
                                CityAction::ToggleModal(ActiveModal::Amulets),
                                if ctx.city.equipped_amulet.is_some() {
                                    COLOR_ACCENT_TEAL
                                } else {
                                    COLOR_PANEL_CARD
                                },
                                true,
                            );
                            spawn_small_button(
                                btns,
                                i18n.t("city.panel.shop.button"),
                                CityAction::ToggleModal(ActiveModal::Shop),
                                COLOR_PANEL_CARD,
                                true,
                            );
                            let tribute_ready =
                                ctx.city.is_login_tribute_available(today_day_index());
                            spawn_small_button_colored(
                                btns,
                                i18n.t("city.button.menu"),
                                CityAction::ToggleMenu,
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
            });

            // Optional collapsible secondary Menu strip (7-Day Tribute, Patrons, Chronicles, Settings, Guide)
            if menu_open {
                let tribute_ready = ctx.city.is_login_tribute_available(today_day_index());
                root.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::FlexEnd,
                        align_items: AlignItems::Center,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(8.0),
                        row_gap: Val::Px(6.0),
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.12, 0.19, 0.96)),
                    BorderColor(COLOR_ACCENT_TEAL),
                    BorderRadius::all(Val::Px(8.0)),
                ))
                .with_children(|menu| {
                    spawn_small_button_colored(
                        menu,
                        i18n.t("city.panel.tribute.button"),
                        CityAction::ToggleModal(ActiveModal::LoginCalendar),
                        if tribute_ready {
                            COLOR_ACCENT_GOLD
                        } else {
                            COLOR_PANEL_CARD
                        },
                        if tribute_ready {
                            COLOR_TEXT_DARK
                        } else {
                            COLOR_TEXT_PRIMARY
                        },
                        true,
                    );
                    spawn_small_button(
                        menu,
                        i18n.t("city.panel.patrons.button"),
                        CityAction::ToggleModal(ActiveModal::Patrons),
                        COLOR_PANEL_CARD,
                        true,
                    );
                    spawn_small_button(
                        menu,
                        i18n.t("city.panel.stats.button"),
                        CityAction::ToggleModal(ActiveModal::Stats),
                        COLOR_PANEL_CARD,
                        true,
                    );
                    spawn_small_button(
                        menu,
                        i18n.t("city.panel.settings.button"),
                        CityAction::ToggleModal(ActiveModal::Settings),
                        COLOR_PANEL_CARD,
                        true,
                    );
                    spawn_small_button(
                        menu,
                        i18n.t("city.panel.help.button"),
                        CityAction::ToggleModal(ActiveModal::Help),
                        COLOR_PANEL_CARD,
                        true,
                    );
                });
            }

            // 2. Active Saga Quest Ribbon with Speaker Portrait
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    column_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_CARD),
                BorderColor(COLOR_ACCENT_GOLD),
                BorderRadius::all(Val::Px(8.0)),
            ))
            .with_children(|qbar| {
                if let Some(quest) = ctx.city.active_quest(&ctx.data.quests) {
                    let ready = ctx.city.is_quest_ready(quest);
                    qbar.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.0),
                        flex_shrink: 1.0,
                        ..default()
                    })
                    .with_children(|left| {
                        if let Some(portrait) = fjord_art.portrait(&quest.speaker) {
                            left.spawn((
                                ImageNode::new(portrait),
                                Node {
                                    width: Val::Px(46.0),
                                    height: Val::Px(46.0),
                                    flex_shrink: 0.0,
                                    border: UiRect::all(Val::Px(1.5)),
                                    ..default()
                                },
                                BorderColor(COLOR_ACCENT_GOLD),
                                BorderRadius::all(Val::Px(8.0)),
                            ));
                        }
                        left.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(1.0),
                            ..default()
                        })
                        .with_children(|info| {
                            info.spawn((
                                Text::new(i18n.t_fmt(
                                    "city.panel.quests.banner",
                                    &[
                                        ("title", i18n.t("city.panel.quests.title").to_string()),
                                        ("quest", i18n.t(&quest.title_key).to_string()),
                                    ],
                                )),
                                TextFont {
                                    font_size: FONT_BODY,
                                    ..default()
                                },
                                TextColor(COLOR_ACCENT_GOLD),
                            ));
                            info.spawn((
                                Text::new(i18n.t(&quest.desc_key)),
                                TextFont {
                                    font_size: FONT_SMALL,
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
                            font_size: FONT_BODY,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                }
            });

            // Guided 3-step Tutorial Banner for first-time players
            if ctx.tutorial_step < 3 {
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
                root.spawn((
                    Node {
                        width: Val::Percent(100.0),
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

            // Optional Celebration Banner
            if let Some(msg) = &ctx.celebration_message {
                root.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
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

            // 3. Fullscreen Panoramic Fjord Map with Slide-Over Building Inspector Drawer
            root.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                ..default()
            })
            .with_children(|main| {
                // Interactive Painted Fjord Map (100% panoramic; clicking empty terrain closes inspector)
                main.spawn((
                    Button,
                    NoButtonScale,
                    CityAction::CloseInspector,
                    ImageNode::new(fjord_art.fjord_map.clone()),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(2.0)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BorderColor(COLOR_PANEL_BORDER),
                    BorderRadius::all(Val::Px(12.0)),
                ))
                .with_children(|map| {
                    // Animated Aurora Borealis ribbons glowing brighter with Clan Glory.
                    let aurora_intensity = (0.14 + (ctx.city.glory as f32 / 1200.0)).min(0.38);
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

                    let quest_target_bldg =
                        ctx.city
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
                            now,
                        );
                    }
                });

                // Slide-Over Building Inspector Drawer (opens when a building is selected)
                if inspector_open {
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
                            top: Val::Px(8.0),
                            bottom: Val::Px(8.0),
                            right: Val::Px(8.0),
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
                    main.spawn((
                        FocusPolicy::Block,
                        Interaction::default(),
                        sliced_panel_image(vfx),
                        drawer_node,
                        modal_shadow(),
                        ZIndex(40),
                    ))
                    .with_children(|insp| {
                        if let Some(selected) = ctx.data.building_by_id(&ctx.selected_building_id) {
                            inspector::spawn_building_inspector(
                                insp,
                                selected,
                                ctx,
                                fjord_art,
                                scroll_mem.inspector_offset_y,
                            );
                        }
                    });
                }
            });

            // 4. Exclusive Modal Overlay
            let m_scroll = scroll_mem.modal_offset_y;
            match ctx.active_modal {
                ActiveModal::None => {}
                ActiveModal::Amulets => {
                    modals::spawn_amulets_modal(root, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Collections => {
                    modals::spawn_collections_modal(root, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Shop => {
                    modals::spawn_tool_shop_modal(root, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Stats => {
                    modals::spawn_stats_modal(root, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Settings => {
                    modals::spawn_settings_modal(root, ctx, fjord_art, vfx, sound_bank, m_scroll)
                }
                ActiveModal::Help => modals::spawn_help_modal(root, ctx, fjord_art, vfx, m_scroll),
                ActiveModal::Raids => {
                    modals::spawn_raids_modal(root, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Patrons => {
                    modals::spawn_patrons_modal(root, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::Achievements => {
                    modals::spawn_achievements_modal(root, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::ExpeditionBoons => {
                    modals::spawn_expedition_boons_modal(root, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::LoginCalendar => {
                    modals::spawn_login_calendar_modal(root, ctx, fjord_art, vfx, m_scroll)
                }
                ActiveModal::ChapterDialogue => {
                    modals::spawn_chapter_dialogue_modal(root, ctx, fjord_art, vfx, m_scroll)
                }
            }

            // 5. Hub-level notice toast (modals and the inspector render notices themselves).
            if ctx.active_modal == ActiveModal::None && !inspector_open {
                if let Some(notice_key) = &ctx.status_notice_key {
                    spawn_notice_toast(root, i18n.t(notice_key), None);
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

    // Landscape progression: Tier 0 shows a low mossy ruin cairn; Tier 1..=3 raises and grows the real Viking building!
    let tier_scale = match tier {
        0 => 0.62,
        1 => 0.86,
        2 => 1.00,
        _ => 1.15,
    };
    let pin_w = if is_portrait { 108.0 } else { 146.0 };
    let base_w = if is_portrait { 72.0 } else { 116.0 };
    let base_h = if is_portrait { 64.0 } else { 102.0 };
    let sprite_w = base_w * tier_scale;
    let sprite_h = base_h * tier_scale;

    let nameplate_shadow = if selected || harvest_ready || is_quest_target {
        gold_glow_shadow()
    } else {
        button_shadow()
    };
    let ground_glow_color = if selected || harvest_ready || is_quest_target || is_just_built {
        Color::srgba(0.96, 0.78, 0.28, 0.56)
    } else if tier >= 2 {
        Color::srgba(0.95, 0.70, 0.26, 0.26)
    } else {
        Color::srgba(0.02, 0.04, 0.08, 0.72)
    };

    let max_left_pct = if is_portrait { 64.0 } else { 76.0 };
    let max_top_pct = if is_portrait { 54.0 } else { 68.0 };
    // Portrait pins use explicit coordinates when the building defines them, so that pins
    // never collapse into the same clamped column.
    let pin_left_pct = match (is_portrait, b.portrait_left_pct) {
        (true, Some(left)) => left,
        _ => (b.map_x_pct - 9.0).clamp(1.0, max_left_pct),
    };
    let pin_top_pct = match (is_portrait, b.portrait_top_pct) {
        (true, Some(top)) => top,
        _ => (b.map_y_pct - 14.0).clamp(2.0, max_top_pct),
    };

    map.spawn((
        Button,
        CityAction::SelectBuilding(b.id.clone()),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(pin_left_pct),
            top: Val::Percent(pin_top_pct),
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
        pin.spawn(Node {
            width: Val::Px(sprite_w),
            height: Val::Px(sprite_h),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::FlexEnd,
            ..default()
        })
        .with_children(|stage| {
            // Soft elliptical ground contact shadow or golden selection halo at the building base.
            stage.spawn((
                ImageNode::new(vfx.soft_orb.clone()).with_color(ground_glow_color),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(-4.0),
                    width: Val::Px(sprite_w * 0.96),
                    height: Val::Px(sprite_h * 0.36),
                    ..default()
                },
                PickingBehavior::IGNORE,
            ));

            if is_just_built {
                stage.spawn((
                    BuildingBuildRing {
                        elapsed: 0.0,
                        duration: 0.95,
                    },
                    ImageNode::new(vfx.shockwave_ring.clone())
                        .with_color(Color::srgba(1.0, 0.86, 0.34, 0.96)),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(sprite_w * 1.45),
                        height: Val::Px(sprite_w * 1.45),
                        ..default()
                    },
                    Transform::from_scale(Vec3::splat(0.35)),
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
                        max_width: Val::Percent(100.0),
                        max_height: Val::Percent(100.0),
                        ..default()
                    },
                ));
                if is_just_built {
                    img_cmd.insert((
                        BuildingBuildAnim {
                            elapsed: 0.0,
                            duration: 0.85,
                        },
                        Transform::from_scale(Vec3::splat(0.25)),
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
                                        base_x_pct: sx,
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
            plate.spawn((
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
                        COLOR_ACCENT_GREEN
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

fn spawn_resource_pill_with_icon(
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
    let text_color = readable_text_color(bg_color, text_color);
    let mut cmd = parent.spawn((
        Button,
        action,
        Node {
            min_height: Val::Px(MIN_TOUCH_TARGET_PX),
            padding: UiRect::axes(Val::Px(11.0), Val::Px(7.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(bg_color),
        BorderColor(COLOR_PANEL_BORDER),
        BorderRadius::all(Val::Px(6.0)),
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
    let text_color = readable_text_color(bg_color, text_color);
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
        BorderColor(COLOR_PANEL_BORDER),
        BorderRadius::all(Val::Px(8.0)),
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
            ctx.active_modal = ActiveModal::None;
            ctx.confirm_import_backup = false;
            ctx.confirm_end_expedition = false;
            ctx.status_notice_key = None;
            scroll_mem.modal_offset_y = 0.0;
            rebuild = true;
        } else if layout_mode.menu_open {
            layout_mode.menu_open = false;
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
                    layout_mode.inspector_open = true;
                    scroll_mem.inspector_offset_y = 0.0;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    rebuild = true;
                }
            }
        }
        if keys.just_pressed(KeyCode::KeyC) {
            let buildings = ctx.data.buildings.clone();
            let balance = ctx.data.balance.clone();
            let now = now_unix_secs();
            let (count, _) = ctx.city.collect_all_harvests(&buildings, now, &balance);
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
            if let Some(b) = ctx.data.building_by_id(&ctx.selected_building_id).cloned() {
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
                    COLOR_ACCENT_TEAL,
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
        let now_portrait = ev.width < ev.height;
        if now_portrait != layout_mode.is_portrait {
            layout_mode.is_portrait = now_portrait;
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
                layout_mode.inspector_open = true;
                scroll_mem.inspector_offset_y = 0.0;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::CloseInspector => {
                if has_pin_press {
                    continue;
                }
                if layout_mode.inspector_open {
                    layout_mode.inspector_open = false;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                    rebuild = true;
                }
            }
            CityAction::SelectDifficulty(diff) => {
                ctx.selected_difficulty = *diff;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
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
                let claim = {
                    let state = &mut *ctx;
                    state
                        .city
                        .try_claim_login_tribute_with(today_day_index(), &state.data.balance)
                };
                if let Some((day, res, _tool)) = claim {
                    let summary = modals::format_reward_summary(&ctx.data.i18n, res, 0);
                    let title = ctx
                        .data
                        .i18n
                        .t_fmt("city.panel.tribute.day", &[("day", day.to_string())]);
                    ctx.celebration_message =
                        Some(celebration_claim_line(&ctx.data.i18n, title, summary));
                    ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                    ctx.status_notice_key = None;
                    ctx.persist();
                    play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                    rebuild = true;
                }
            }
            CityAction::CollectHarvest(building_id) => {
                if let Some(def) = ctx.data.building_by_id(building_id).cloned() {
                    let balance = ctx.data.balance.clone();
                    let now = now_unix_secs();
                    if ctx.city.try_collect_harvest(&def, now, &balance).is_some() {
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
                let buildings = ctx.data.buildings.clone();
                let balance = ctx.data.balance.clone();
                let now = now_unix_secs();
                let (count, _) = ctx.city.collect_all_harvests(&buildings, now, &balance);
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
            CityAction::PickExpeditionBoon(idx) => {
                if ctx.continue_expedition_with_boon(*idx).is_ok() {
                    next_state.set(AppState::Match3);
                    return;
                }
                ctx.active_modal = ActiveModal::None;
                scroll_mem.modal_offset_y = 0.0;
                rebuild = true;
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
                    let balance = ctx.data.balance.clone();
                    match ctx.city.try_upgrade_building(&def, &balance) {
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
                            if new_tier == 1 && !ctx.city.seen_chapters.contains(&def.id) {
                                if let Some(dlg) = chapter_dialogue_for_building(&def.id) {
                                    ctx.city.seen_chapters.insert(def.id.clone());
                                    ctx.pending_dialogue = Some(dlg);
                                    ctx.active_modal = ActiveModal::ChapterDialogue;
                                    scroll_mem.modal_offset_y = 0.0;
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
                let balance = ctx.data.balance.clone();
                match ctx.city.try_drink_mead(&balance) {
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
                    let balance = ctx.data.balance.clone();
                    if ctx.city.try_claim_quest(&quest, &balance) {
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
            CityAction::ToggleMenu => {
                layout_mode.menu_open = !layout_mode.menu_open;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::ToggleModal(modal) => {
                layout_mode.menu_open = false;
                ctx.toggle_modal(*modal);
                scroll_mem.modal_offset_y = 0.0;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::CloseModal => {
                if ctx.active_modal != ActiveModal::None {
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
                    let balance = ctx.data.balance.clone();
                    if ctx.city.try_claim_collection(&col, &balance) {
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
                let collections = ctx.data.collections.clone();
                let balance = ctx.data.balance.clone();
                let mut claimed_any = false;
                let mut total_res = wyrdstone_core::ResourceBundle::default();
                for col in &collections {
                    if ctx.city.try_claim_collection(col, &balance) {
                        claimed_any = true;
                        total_res.add(&col.reward_resources);
                    }
                }
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
                if let Some(amulet) = ctx.data.amulet_by_id(amulet_id).cloned() {
                    let balance = ctx.data.balance.clone();
                    match ctx.city.try_craft_amulet(&amulet, &balance) {
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
            }
            CityAction::UpgradeAmulet(amulet_id) => {
                if let Some(amulet) = ctx.data.amulet_by_id(amulet_id).cloned() {
                    let balance = ctx.data.balance.clone();
                    match ctx.city.try_upgrade_amulet(&amulet, &balance) {
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
                ctx.colorblind_mode = !ctx.colorblind_mode;
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::ToggleFastAnim => {
                ctx.fast_animations = !ctx.fast_animations;
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::ToggleReducedMotion => {
                ctx.reduced_motion = !ctx.reduced_motion;
                ctx.persist();
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::CycleBgmVolume => {
                cycle_bgm_volume(&mut ctx, &mut sound_bank, &mut bgm_sinks);
                rebuild = true;
            }
            CityAction::CycleSfxVolume => {
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
                ctx.data.i18n.toggle_language();
                ctx.persist();
                rebuild = true;
            }
            CityAction::ToggleAudio => {
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
            layout_mode.menu_open,
            layout_mode.inspector_open,
            just_built.as_deref(),
        );
    }
}

fn chapter_dialogue_for_building(building_id: &str) -> Option<ChapterDialogue> {
    let speaker = match building_id {
        wyrdstone_core::HUB_BUILDING_ID | "watchtower" => "jarl",
        "forge" => "smith",
        "longship_dock" => "navigator",
        "odin_shrine" => "seer",
        "meadery" => "innkeeper",
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
    mut snow_q: Query<
        (&mut FjordSnowflake, &mut Node),
        (
            Without<FjordAurora>,
            Without<FjordEmber>,
            Without<FjordWaterShimmer>,
            Without<FjordSmokePuff>,
            Without<FjordBeaconGlow>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut aurora_q: Query<
        (&FjordAurora, &mut Node, &mut ImageNode),
        (
            Without<FjordSnowflake>,
            Without<FjordEmber>,
            Without<FjordWaterShimmer>,
            Without<FjordSmokePuff>,
            Without<FjordBeaconGlow>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut ember_q: Query<
        (&mut FjordEmber, &mut Node, &mut ImageNode),
        (
            Without<FjordSnowflake>,
            Without<FjordAurora>,
            Without<FjordWaterShimmer>,
            Without<FjordSmokePuff>,
            Without<FjordBeaconGlow>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut water_q: Query<
        (&FjordWaterShimmer, &mut Node, &mut ImageNode),
        (
            Without<FjordSnowflake>,
            Without<FjordAurora>,
            Without<FjordEmber>,
            Without<FjordSmokePuff>,
            Without<FjordBeaconGlow>,
            Without<BuildingBuildRing>,
        ),
    >,
    mut smoke_q: Query<
        (&mut FjordSmokePuff, &mut Node, &mut ImageNode),
        (
            Without<FjordSnowflake>,
            Without<FjordAurora>,
            Without<FjordEmber>,
            Without<FjordWaterShimmer>,
            Without<FjordBeaconGlow>,
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
        Without<BuildingBuildRing>,
    >,
    mut build_ring_q: Query<
        (
            Entity,
            &mut BuildingBuildRing,
            &mut Transform,
            &mut ImageNode,
        ),
        (
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
        for (mut flake, mut node) in &mut snow_q {
            flake.y_pct += flake.speed_y * dt;
            if flake.y_pct > 98.0 {
                flake.y_pct = 1.0;
            }
            let sway = (t * 1.1 + flake.phase).sin() * 1.6;
            node.top = Val::Percent(flake.y_pct);
            node.left = Val::Percent((flake.x_pct + sway).clamp(1.0, 98.0));
        }

        for (aurora, mut node, mut img) in &mut aurora_q {
            let drift = (t * 0.35 + aurora.phase).sin() * 5.0;
            let pulse = 0.72 + 0.28 * (t * 0.8 + aurora.phase).sin();
            node.left = Val::Percent((aurora.base_left_pct + drift).clamp(0.0, 60.0));
            img.color = img
                .color
                .with_alpha((aurora.base_alpha * pulse).clamp(0.05, 0.45));
        }

        for (water, mut node, mut img) in &mut water_q {
            let ripple = (t * 0.65 + water.phase).sin() * 2.2;
            let glint = 0.65 + 0.35 * (t * 1.4 + water.phase * 1.3).cos();
            node.left = Val::Percent((water.base_left_pct + ripple).clamp(15.0, 70.0));
            img.color = img
                .color
                .with_alpha((water.base_alpha * glint).clamp(0.05, 0.36));
        }

        for (mut ember, mut node, mut img) in &mut ember_q {
            ember.y_pct -= 22.0 * dt;
            if ember.y_pct < 6.0 {
                ember.y_pct = 86.0;
            }
            let sway = (t * 2.4 + ember.phase).sin() * 4.0;
            let alpha = ((ember.y_pct - 6.0) / 80.0).clamp(0.1, 0.9);
            node.top = Val::Percent(ember.y_pct);
            node.left = Val::Percent((ember.base_x_pct + sway).clamp(4.0, 94.0));
            img.color = img.color.with_alpha(alpha);
        }

        for (mut smoke, mut node, mut img) in &mut smoke_q {
            smoke.y_pct -= 12.0 * dt;
            if smoke.y_pct < -24.0 {
                smoke.y_pct = 18.0;
            }
            let life = ((18.0 - smoke.y_pct) / 42.0).clamp(0.0, 1.0);
            let sway = (t * 1.4 + smoke.phase).sin() * (4.0 + life * 8.0);
            let size = 12.0 + life * 16.0;
            node.top = Val::Percent(smoke.y_pct);
            node.left = Val::Percent(smoke.base_x_pct + sway);
            node.width = Val::Px(size);
            node.height = Val::Px(size);
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

#[cfg(test)]
mod tests {
    use wyrdstone_core::{GameDataBundle, Language};

    /// Portrait pin footprint and the smallest map size the layout is designed for.
    const PIN_WIDTH_PX: f32 = 108.0;
    const PIN_HEIGHT_PX: f32 = 100.0;
    const MIN_PORTRAIT_MAP_WIDTH_PX: f32 = 336.0;
    const MIN_PORTRAIT_MAP_HEIGHT_PX: f32 = 560.0;

    #[test]
    fn portrait_pins_never_overlap() {
        let data = GameDataBundle::load_bundled(Language::En).expect("bundled data must parse");
        let pins: Vec<(&str, f32, f32)> = data
            .buildings
            .iter()
            .map(|b| {
                (
                    b.id.as_str(),
                    b.portrait_left_pct
                        .expect("every building needs a portrait_left_pct"),
                    b.portrait_top_pct
                        .expect("every building needs a portrait_top_pct"),
                )
            })
            .collect();
        for (i, a) in pins.iter().enumerate() {
            assert!(
                a.1 >= 0.0
                    && a.1 / 100.0 * MIN_PORTRAIT_MAP_WIDTH_PX + PIN_WIDTH_PX
                        <= MIN_PORTRAIT_MAP_WIDTH_PX,
                "pin {} leaves the map horizontally",
                a.0
            );
            assert!(
                a.2 >= 0.0
                    && a.2 / 100.0 * MIN_PORTRAIT_MAP_HEIGHT_PX + PIN_HEIGHT_PX
                        <= MIN_PORTRAIT_MAP_HEIGHT_PX,
                "pin {} leaves the map vertically",
                a.0
            );
            for b in &pins[i + 1..] {
                let dx = (a.1 - b.1).abs() / 100.0 * MIN_PORTRAIT_MAP_WIDTH_PX;
                let dy = (a.2 - b.2).abs() / 100.0 * MIN_PORTRAIT_MAP_HEIGHT_PX;
                assert!(
                    dx >= PIN_WIDTH_PX || dy >= PIN_HEIGHT_PX,
                    "portrait pins {} and {} overlap",
                    a.0,
                    b.0
                );
            }
        }
    }
}
