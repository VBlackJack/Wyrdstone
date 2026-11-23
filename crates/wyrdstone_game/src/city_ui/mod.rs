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
use crate::scroll::UiScrollMemory;
use crate::state::{now_unix_secs, today_day_index, ActiveModal, AppState, GameContext};
use crate::theme::*;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use wyrdstone_core::{ActiveTool, BuildingDef, PatronGod, QuestCondition};

const CELEBRATION_DURATION_SECS: f32 = 3.2;
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

/// Marker on the top bar's Endurance pill text so periodic energy regen updates in-place.
#[derive(Component)]
pub struct CityEnergyText;

/// Tracks City layout mode, displayed endurance, audio state, and periodic harvest readiness checks.
#[derive(Resource)]
pub struct CityLayoutMode {
    pub is_portrait: bool,
    pub last_displayed_energy: u32,
    pub last_ready_harvests: usize,
    pub last_muted: bool,
    pub harvest_poll_timer: Timer,
}

#[derive(Component)]
pub enum CityAction {
    SelectBuilding(String),
    SelectDifficulty(u8),
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
    UpgradeBuilding(String),
    DrinkMead,
    ClaimQuest(String),
    ToggleModal(ActiveModal),
    ClaimCollection(String),
    CraftTool(ActiveTool),
    CraftOrEquipAmulet(String),
    UpgradeAmulet(String),
    AdvanceTutorial,
    ToggleColorblind,
    CycleBgmVolume,
    CycleSfxVolume,
    ExportBackup,
    ImportBackup,
    ToggleLanguage,
    ToggleAudio,
    BackToTitle,
}

/// Spawns the Fjord City hub UI.
pub fn setup_city_ui(
    mut commands: Commands,
    ctx: Res<GameContext>,
    fjord_art: Res<FjordTextures>,
    sound_bank: Res<SoundBank>,
    scroll_mem: Res<UiScrollMemory>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let is_portrait = windows
        .get_single()
        .map(|w| w.width() < w.height())
        .unwrap_or(false);
    let ready_harvests = count_ready_harvests(&ctx, now_unix_secs());
    commands.insert_resource(CityLayoutMode {
        is_portrait,
        last_displayed_energy: ctx.city.energy,
        last_ready_harvests: ready_harvests,
        last_muted: sound_bank.muted,
        harvest_poll_timer: Timer::from_seconds(HARVEST_POLL_SECS, TimerMode::Repeating),
    });
    spawn_city_tree(
        &mut commands,
        &ctx,
        &fjord_art,
        &sound_bank,
        &scroll_mem,
        is_portrait,
    );
}

/// Cleans up the Fjord City UI on state exit.
pub fn cleanup_city_ui(mut commands: Commands, roots: Query<Entity, With<CityRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<CityLayoutMode>();
}

fn count_ready_harvests(ctx: &GameContext, now: u64) -> usize {
    ctx.data
        .buildings
        .iter()
        .filter(|b| ctx.city.is_harvest_ready(&b.id, now, &ctx.data.balance))
        .count()
}

fn spawn_city_tree(
    commands: &mut Commands,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    sound_bank: &SoundBank,
    scroll_mem: &UiScrollMemory,
    is_portrait: bool,
) {
    let i18n = &ctx.data.i18n;
    let res = ctx.city.resources;
    let max_energy = ctx.city.max_energy(&ctx.data.balance);
    let now = now_unix_secs();
    let sound_label = if sound_bank.muted {
        i18n.t("app.audio.off")
    } else {
        i18n.t("app.audio.on")
    };

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
                Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(9.0)),
                    row_gap: Val::Px(6.0),
                    column_gap: Val::Px(10.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BackgroundColor(COLOR_PANEL_DARK),
                BorderColor(COLOR_PANEL_BORDER),
                BorderRadius::all(Val::Px(10.0)),
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
                        TextFont {
                            font_size: FONT_HEADING,
                            ..default()
                        },
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                    let rank = ctx.city.clan_rank_with_balance(&ctx.data.balance);
                    let rank_label = i18n.t(rank.i18n_key());
                    let perk_label = i18n.t(rank.perk_key());
                    col.spawn((
                        Text::new(format!(
                            "{}  -  {}  ({})",
                            rank_label,
                            i18n.t_fmt(
                                "city.header.mastery",
                                &[("glory", ctx.city.glory.to_string())],
                            ),
                            perk_label
                        )),
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
                    spawn_resource_pill(
                        pills,
                        &format!("{}: {}", i18n.t("resources.label.wood"), res.wood),
                        Color::srgb(0.44, 0.80, 0.52),
                    );
                    spawn_resource_pill(
                        pills,
                        &format!("{}: {}", i18n.t("resources.label.stone"), res.stone),
                        Color::srgb(0.76, 0.74, 0.88),
                    );
                    spawn_resource_pill(
                        pills,
                        &format!("{}: {}", i18n.t("resources.label.gold"), res.gold),
                        COLOR_ACCENT_GOLD,
                    );
                    spawn_resource_pill(
                        pills,
                        &format!("{}: {}", i18n.t("resources.label.mead"), res.mead),
                        Color::srgb(0.95, 0.66, 0.30),
                    );
                    pills
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
                                CityEnergyText,
                                Text::new(format!(
                                    "{}: {}/{}",
                                    i18n.t("resources.label.energy"),
                                    ctx.city.energy,
                                    max_energy
                                )),
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
                        );
                    }
                });

                bar.spawn(Node {
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(4.0),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                })
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
                        );
                    }
                    let daily_ready = ctx.city.is_daily_available(today_day_index());
                    let daily_label = if daily_ready {
                        if ctx.city.stats.daily_streak > 0 {
                            format!(
                                "{} ({})",
                                i18n.t("city.panel.daily.button"),
                                i18n.t_fmt(
                                    "city.panel.daily.streak",
                                    &[("days", ctx.city.stats.daily_streak.to_string())]
                                )
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
                    );
                    spawn_small_button(
                        btns,
                        i18n.t("city.panel.raids.button"),
                        CityAction::ToggleModal(ActiveModal::Raids),
                        COLOR_ACCENT_CRIMSON,
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
                    );
                    spawn_small_button(
                        btns,
                        i18n.t("city.panel.patrons.button"),
                        CityAction::ToggleModal(ActiveModal::Patrons),
                        COLOR_PANEL_CARD,
                    );
                    spawn_small_button(
                        btns,
                        i18n.t("city.panel.feats.button"),
                        CityAction::ToggleModal(ActiveModal::Achievements),
                        if ctx.city.has_any_claimable_achievement(
                            &ctx.data.achievements,
                            &ctx.data.buildings,
                            ctx.data.bosses.len(),
                        ) {
                            COLOR_ACCENT_GREEN
                        } else {
                            COLOR_PANEL_CARD
                        },
                    );
                    spawn_small_button(
                        btns,
                        i18n.t("city.panel.collections.button"),
                        CityAction::ToggleModal(ActiveModal::Collections),
                        if ctx.city.has_any_claimable_collection(&ctx.data.collections) {
                            COLOR_ACCENT_GREEN
                        } else {
                            COLOR_PANEL_CARD
                        },
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
                    );
                    spawn_small_button(
                        btns,
                        i18n.t("city.panel.shop.button"),
                        CityAction::ToggleModal(ActiveModal::Shop),
                        COLOR_PANEL_CARD,
                    );
                    spawn_small_button(
                        btns,
                        i18n.t("city.panel.stats.button"),
                        CityAction::ToggleModal(ActiveModal::Stats),
                        COLOR_PANEL_CARD,
                    );
                    spawn_small_button(
                        btns,
                        i18n.t("city.panel.help.button"),
                        CityAction::ToggleModal(ActiveModal::Help),
                        COLOR_PANEL_CARD,
                    );
                    spawn_small_button(
                        btns,
                        sound_label,
                        CityAction::ToggleAudio,
                        COLOR_PANEL_CARD,
                    );
                    spawn_small_button(
                        btns,
                        i18n.t("app.title.button.language"),
                        CityAction::ToggleLanguage,
                        COLOR_PANEL_CARD,
                    );
                    spawn_small_button(
                        btns,
                        i18n.t("city.button.title"),
                        CityAction::BackToTitle,
                        COLOR_PANEL_CARD,
                    );
                });
            });

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
                                Text::new(format!(
                                    "{} - {}",
                                    i18n.t("city.panel.quests.title"),
                                    i18n.t(&quest.title_key)
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
                        TextColor(COLOR_ACCENT_GOLD),
                    ));
                });
            }

            // 3. Main Split View: Responsive Row (Landscape) / Column (Portrait)
            let (map_w, map_h, insp_w, insp_h) = if is_portrait {
                (
                    Val::Percent(100.0),
                    Val::Percent(50.0),
                    Val::Percent(100.0),
                    Val::Percent(50.0),
                )
            } else {
                (
                    Val::Percent(66.0),
                    Val::Percent(100.0),
                    Val::Percent(34.0),
                    Val::Percent(100.0),
                )
            };

            root.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: if is_portrait {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                column_gap: Val::Px(12.0),
                row_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|main| {
                // Interactive Painted Fjord Map
                main.spawn((
                    ImageNode::new(fjord_art.fjord_map.clone()),
                    Node {
                        width: map_w,
                        height: map_h,
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
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(left_p),
                                top: Val::Percent(top_p),
                                width: Val::Percent(w_p),
                                height: Val::Percent(h_p),
                                ..default()
                            },
                            BackgroundColor(col),
                            BorderRadius::all(Val::Px(36.0)),
                        ));
                    }

                    for i in 0..SNOWFLAKE_COUNT {
                        let x_pct = ((i * 37 + 11) % 96) as f32 + 2.0;
                        let y_pct = ((i * 53 + 7) % 94) as f32 + 2.0;
                        let size = if i % 3 == 0 { 4.5 } else { 3.0 };
                        map.spawn((
                            FjordSnowflake {
                                x_pct,
                                y_pct,
                                speed_y: 4.5 + (i % 5) as f32 * 1.4,
                                phase: i as f32 * 0.9,
                            },
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(x_pct),
                                top: Val::Percent(y_pct),
                                width: Val::Px(size),
                                height: Val::Px(size),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.92, 0.97, 1.0, 0.55)),
                            BorderRadius::all(Val::Px(size)),
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
                        spawn_map_building_pin(
                            map,
                            b,
                            ctx,
                            fjord_art,
                            is_quest_target,
                            is_portrait,
                            now,
                        );
                    }
                });

                // Building Inspector Drawer
                main.spawn((
                    Node {
                        width: insp_w,
                        height: insp_h,
                        min_height: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::all(Val::Px(12.0)),
                        row_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(2.0)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(COLOR_PANEL_DARK),
                    BorderColor(COLOR_PANEL_BORDER),
                    BorderRadius::all(Val::Px(12.0)),
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
                    } else {
                        insp.spawn((
                            Text::new(i18n.t("city.panel.building.select_hint")),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                        ));
                    }
                });
            });

            // 4. Exclusive Modal Overlay
            let m_scroll = scroll_mem.modal_offset_y;
            match ctx.active_modal {
                ActiveModal::None => {}
                ActiveModal::Amulets => modals::spawn_amulets_modal(root, ctx, fjord_art, m_scroll),
                ActiveModal::Collections => modals::spawn_collections_modal(root, ctx, m_scroll),
                ActiveModal::Shop => modals::spawn_tool_shop_modal(root, ctx, fjord_art, m_scroll),
                ActiveModal::Stats => modals::spawn_stats_modal(root, ctx, sound_bank, m_scroll),
                ActiveModal::Help => modals::spawn_help_modal(root, ctx, m_scroll),
                ActiveModal::Raids => modals::spawn_raids_modal(root, ctx, fjord_art, m_scroll),
                ActiveModal::Patrons => modals::spawn_patrons_modal(root, ctx, fjord_art, m_scroll),
                ActiveModal::Achievements => modals::spawn_achievements_modal(root, ctx, m_scroll),
                ActiveModal::ExpeditionBoons => {
                    modals::spawn_expedition_boons_modal(root, ctx, m_scroll)
                }
            }
        });
}

fn spawn_map_building_pin(
    map: &mut ChildBuilder,
    b: &BuildingDef,
    ctx: &GameContext,
    fjord_art: &FjordTextures,
    is_quest_target: bool,
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

    let pin_w = if is_portrait { 108.0 } else { 134.0 };
    let img_h = if is_portrait { 48.0 } else { 64.0 };

    let mut pin_cmd = map.spawn((
        Button,
        CityAction::SelectBuilding(b.id.clone()),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent((b.map_x_pct - 9.0).clamp(1.5, 76.0)),
            top: Val::Percent((b.map_y_pct - 12.0).clamp(3.0, 68.0)),
            width: Val::Px(pin_w),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(4.0)),
            row_gap: Val::Px(3.0),
            border: UiRect::all(Val::Px(if selected || is_quest_target || harvest_ready {
                3.0
            } else {
                2.0
            })),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.09, 0.14, 0.88)),
        BorderColor(border_col),
        BorderRadius::all(Val::Px(10.0)),
    ));
    if is_quest_target || harvest_ready {
        pin_cmd.insert(QuestBeaconPin);
    }
    pin_cmd.with_children(|pin| {
        if let Some(img) = fjord_art.building_image(&b.id) {
            pin.spawn((
                ImageNode {
                    image: img,
                    color: img_tint,
                    ..default()
                },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(img_h),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderRadius::all(Val::Px(6.0)),
            ))
            .with_children(|img_box| {
                if tier > 0 {
                    for k in 0..3usize {
                        let base_x = 22.0 + k as f32 * 26.0;
                        let start_y = 78.0 - (k as f32 * 18.0);
                        img_box.spawn((
                            FjordEmber {
                                base_x_pct: base_x,
                                y_pct: start_y,
                                phase: k as f32 * 2.1 + b.map_x_pct * 0.1,
                            },
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(base_x),
                                top: Val::Percent(start_y),
                                width: Val::Px(4.0),
                                height: Val::Px(4.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(1.0, 0.78, 0.28, 0.85)),
                            BorderRadius::all(Val::Px(4.0)),
                        ));
                    }
                }
            });
        }

        pin.spawn((
            Text::new(i18n.t(&b.name_key)),
            TextFont {
                font_size: if is_portrait { 11.5 } else { FONT_SMALL },
                ..default()
            },
            TextColor(if selected {
                COLOR_ACCENT_GOLD
            } else {
                COLOR_TEXT_PRIMARY
            }),
        ));

        if harvest_ready {
            pin.spawn((
                Button,
                CityAction::CollectHarvest(b.id.clone()),
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(3.0)),
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
                        font_size: 11.5,
                        ..default()
                    },
                    TextColor(COLOR_TEXT_DARK),
                ));
            });
        } else {
            let badge = if !unlocked {
                "---".to_string()
            } else if can_upgrade {
                format!(
                    "{} ({}/{})",
                    i18n.t("city.panel.building.status.ready"),
                    tier,
                    b.max_tier
                )
            } else if tier == 0 {
                i18n.t("city.panel.building.status.ruined").to_string()
            } else {
                format!("T{} / {}", tier, b.max_tier)
            };
            pin.spawn((
                Text::new(badge),
                TextFont {
                    font_size: 11.5,
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

pub(crate) fn spawn_small_button(
    parent: &mut ChildBuilder,
    label: &str,
    action: CityAction,
    bg_color: Color,
) {
    spawn_small_button_colored(parent, label, action, bg_color, COLOR_TEXT_PRIMARY);
}

pub(crate) fn spawn_small_button_colored(
    parent: &mut ChildBuilder,
    label: &str,
    action: CityAction,
    bg_color: Color,
    text_color: Color,
) {
    parent
        .spawn((
            Button,
            action,
            Node {
                padding: UiRect::axes(Val::Px(11.0), Val::Px(7.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            BorderColor(COLOR_PANEL_BORDER),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|btn| {
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
) {
    parent
        .spawn((
            Button,
            action,
            Node {
                flex_grow: 1.0,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(11.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.5)),
                ..default()
            },
            BackgroundColor(bg_color),
            BorderColor(COLOR_PANEL_BORDER),
            BorderRadius::all(Val::Px(8.0)),
        ))
        .with_children(|btn| {
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

/// Processes button presses, passive harvest checks, and Escape modal-close shortcut.
#[allow(clippy::too_many_arguments)]
pub fn handle_city_buttons(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut ctx: ResMut<GameContext>,
    fjord_art: Res<FjordTextures>,
    mut sound_bank: ResMut<SoundBank>,
    mut bgm_sinks: Query<&mut AudioSink, With<BgmTrack>>,
    mut layout_mode: ResMut<CityLayoutMode>,
    mut scroll_mem: ResMut<UiScrollMemory>,
    mut resize_events: EventReader<WindowResized>,
    mut next_state: ResMut<NextState<AppState>>,
    roots: Query<Entity, With<CityRoot>>,
    mut energy_texts: Query<&mut Text, With<CityEnergyText>>,
    buttons: Query<(&Interaction, &CityAction), Changed<Interaction>>,
) {
    let mut rebuild = false;

    if keys.just_pressed(KeyCode::Escape) && ctx.active_modal != ActiveModal::None {
        if ctx.active_modal == ActiveModal::ExpeditionBoons {
            ctx.active_expedition = None;
            ctx.persist();
        }
        ctx.active_modal = ActiveModal::None;
        scroll_mem.modal_offset_y = 0.0;
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
            let formatted = format!(
                "{}: {}/{}",
                ctx.data.i18n.t("resources.label.energy"),
                ctx.city.energy,
                max_energy
            );
            for mut text in &mut energy_texts {
                text.0 = formatted.clone();
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

    let has_harvest_press = buttons
        .iter()
        .any(|(i, a)| *i == Interaction::Pressed && matches!(a, CityAction::CollectHarvest(_)));

    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            CityAction::SelectBuilding(id) => {
                if has_harvest_press {
                    continue;
                }
                ctx.selected_building_id = id.clone();
                ctx.selected_difficulty = 0;
                ctx.status_notice_key = None;
                scroll_mem.inspector_offset_y = 0.0;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
            }
            CityAction::SelectDifficulty(diff) => {
                ctx.selected_difficulty = *diff;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
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
                if ctx.city.is_daily_available(today_day_index()) && ctx.start_daily_trial().is_ok()
                {
                    next_state.set(AppState::Match3);
                    return;
                }
                play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
            }
            CityAction::UpgradeBuilding(building_id) => {
                if let Some(def) = ctx.data.building_by_id(building_id).cloned() {
                    let balance = ctx.data.balance.clone();
                    match ctx.city.try_upgrade_building(&def, &balance) {
                        Ok(new_tier) => {
                            ctx.status_notice_key = None;
                            let bname = ctx.data.i18n.t(&def.name_key).to_string();
                            ctx.celebration_message = Some(ctx.data.i18n.t_fmt(
                                "city.celebration.upgraded",
                                &[("building", bname), ("tier", new_tier.to_string())],
                            ));
                            ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                            layout_mode.last_ready_harvests =
                                count_ready_harvests(&ctx, now_unix_secs());
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
                        ctx.status_notice_key = None;
                        ctx.persist();
                        play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                        rebuild = true;
                    }
                }
            }
            CityAction::ToggleModal(modal) => {
                ctx.toggle_modal(*modal);
                scroll_mem.modal_offset_y = 0.0;
                play_sfx(&mut commands, &sound_bank, SfxKind::Swap);
                rebuild = true;
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
                        ctx.persist();
                        play_sfx(&mut commands, &sound_bank, SfxKind::Victory);
                        rebuild = true;
                    }
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
                Ok(()) => {
                    ctx.status_notice_key = None;
                    ctx.celebration_message =
                        Some(ctx.data.i18n.t("city.panel.backup.exported").to_string());
                    ctx.celebration_timer_secs = CELEBRATION_DURATION_SECS;
                    play_sfx(&mut commands, &sound_bank, SfxKind::Upgrade);
                    rebuild = true;
                }
                Err(err_key) => {
                    ctx.status_notice_key = Some(err_key.to_string());
                    play_sfx(&mut commands, &sound_bank, SfxKind::Invalid);
                    rebuild = true;
                }
            },
            CityAction::ImportBackup => match ctx.import_backup() {
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
                    layout_mode.last_ready_harvests = count_ready_harvests(&ctx, now_unix_secs());
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
            },
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
        spawn_city_tree(
            &mut commands,
            &ctx,
            &fjord_art,
            &sound_bank,
            &scroll_mem,
            layout_mode.is_portrait,
        );
    }
}

/// Animates gentle falling snow, aurora borealis ribbons, rising building embers, and pulsing beacons.
#[allow(clippy::type_complexity)]
pub fn animate_city_ambience(
    time: Res<Time>,
    mut snow_q: Query<
        (&mut FjordSnowflake, &mut Node),
        (Without<FjordAurora>, Without<FjordEmber>),
    >,
    mut aurora_q: Query<
        (&FjordAurora, &mut Node, &mut BackgroundColor),
        (Without<FjordSnowflake>, Without<FjordEmber>),
    >,
    mut ember_q: Query<
        (&mut FjordEmber, &mut Node, &mut BackgroundColor),
        (Without<FjordSnowflake>, Without<FjordAurora>),
    >,
    mut beacon_q: Query<&mut BorderColor, With<QuestBeaconPin>>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();

    for (mut flake, mut node) in &mut snow_q {
        flake.y_pct += flake.speed_y * dt;
        if flake.y_pct > 98.0 {
            flake.y_pct = 1.0;
        }
        let sway = (t * 1.1 + flake.phase).sin() * 1.6;
        node.top = Val::Percent(flake.y_pct);
        node.left = Val::Percent((flake.x_pct + sway).clamp(1.0, 98.0));
    }

    for (aurora, mut node, mut bg) in &mut aurora_q {
        let drift = (t * 0.35 + aurora.phase).sin() * 5.0;
        let pulse = 0.72 + 0.28 * (t * 0.8 + aurora.phase).sin();
        node.left = Val::Percent((aurora.base_left_pct + drift).clamp(0.0, 60.0));
        bg.0 =
            bg.0.with_alpha((aurora.base_alpha * pulse).clamp(0.05, 0.45));
    }

    for (mut ember, mut node, mut bg) in &mut ember_q {
        ember.y_pct -= 22.0 * dt;
        if ember.y_pct < 6.0 {
            ember.y_pct = 86.0;
        }
        let sway = (t * 2.4 + ember.phase).sin() * 4.0;
        let alpha = ((ember.y_pct - 6.0) / 80.0).clamp(0.1, 0.9);
        node.top = Val::Percent(ember.y_pct);
        node.left = Val::Percent((ember.base_x_pct + sway).clamp(4.0, 94.0));
        bg.0 = bg.0.with_alpha(alpha);
    }

    let glow = 0.5 + 0.5 * (t * 4.5).sin();
    let beacon_col = Color::srgb(0.95 + 0.05 * glow, 0.72 + 0.25 * glow, 0.24 + 0.45 * glow);
    for mut border in &mut beacon_q {
        border.0 = beacon_col;
    }
}
