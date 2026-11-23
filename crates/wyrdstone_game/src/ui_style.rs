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

//! Shared Nordic UI styling helpers: elevation shadows, display typography, and state fade transitions.

use crate::fjord_art::FjordTextures;
use crate::state::GameContext;
use crate::vfx_art::VfxTextures;
use bevy::prelude::*;
use bevy::ui::widget::NodeImageMode;

/// Marker placed on buttons that must not receive hover/press scale animation
/// (such as fullscreen modal backdrops, disabled buttons, or nested child buttons).
#[derive(Component)]
pub struct NoButtonScale;

const FADE_DURATION_SECS: f32 = 0.24;

/// Returns a 9-slice sculpted Nordic obsidian/bronze panel `ImageNode`.
pub fn sliced_panel_image(vfx: &VfxTextures) -> ImageNode {
    ImageNode::new(vfx.panel_frame_9slice.clone()).with_mode(NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect::square(18.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        ..default()
    }))
}

/// Returns a 9-slice sculpted forged gold primary button `ImageNode`.
pub fn sliced_gold_button_image(vfx: &VfxTextures) -> ImageNode {
    ImageNode::new(vfx.button_gold_9slice.clone()).with_mode(NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect::square(16.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        ..default()
    }))
}

/// Elevation shadow for top-level HUD bars and main drawers.
pub fn panel_shadow() -> BoxShadow {
    BoxShadow {
        color: Color::srgba(0.01, 0.02, 0.05, 0.62),
        x_offset: Val::Px(0.0),
        y_offset: Val::Px(6.0),
        spread_radius: Val::Px(1.0),
        blur_radius: Val::Px(14.0),
    }
}

/// Crisp drop shadow for interactive buttons and building pins.
pub fn button_shadow() -> BoxShadow {
    BoxShadow {
        color: Color::srgba(0.01, 0.02, 0.05, 0.56),
        x_offset: Val::Px(0.0),
        y_offset: Val::Px(4.0),
        spread_radius: Val::Px(0.0),
        blur_radius: Val::Px(8.0),
    }
}

/// Deep dramatic shadow for centered modals and victory/defeat cards.
pub fn modal_shadow() -> BoxShadow {
    BoxShadow {
        color: Color::srgba(0.0, 0.01, 0.03, 0.78),
        x_offset: Val::Px(0.0),
        y_offset: Val::Px(12.0),
        spread_radius: Val::Px(2.0),
        blur_radius: Val::Px(26.0),
    }
}

/// Warm golden glow shadow for ready harvests, active quests, and primary CTAs.
pub fn gold_glow_shadow() -> BoxShadow {
    BoxShadow {
        color: Color::srgba(0.95, 0.76, 0.28, 0.42),
        x_offset: Val::Px(0.0),
        y_offset: Val::Px(0.0),
        spread_radius: Val::Px(2.0),
        blur_radius: Val::Px(16.0),
    }
}

/// Builds a `TextFont` using the bundled carved Norse serif display font.
pub fn display_font(fjord_art: &FjordTextures, font_size: f32) -> TextFont {
    TextFont {
        font: fjord_art.display_font.clone(),
        font_size,
        ..default()
    }
}

/// Full-screen fade-in overlay spawned briefly on screen transitions so state switches feel cinematic.
#[derive(Component)]
pub struct ScreenFadeOverlay {
    pub elapsed: f32,
    pub duration: f32,
}

/// Spawns a quick fade-from-dark overlay on top of the UI hierarchy.
pub fn spawn_screen_fade(mut commands: Commands) {
    commands.spawn((
        ScreenFadeOverlay {
            elapsed: 0.0,
            duration: FADE_DURATION_SECS,
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.03, 0.05, 0.09, 0.92)),
        ZIndex(999),
        PickingBehavior::IGNORE,
    ));
}

/// Ticks and despawns active screen fade overlays.
pub fn tick_screen_fades(
    mut commands: Commands,
    time: Res<Time>,
    mut fades: Query<(Entity, &mut ScreenFadeOverlay, &mut BackgroundColor)>,
) {
    let dt = time.delta_secs();
    for (entity, mut fade, mut bg) in &mut fades {
        fade.elapsed += dt;
        if fade.elapsed >= fade.duration {
            commands.entity(entity).despawn();
            continue;
        }
        let t = (fade.elapsed / fade.duration).clamp(0.0, 1.0);
        let alpha = (1.0 - t) * (1.0 - t) * 0.92;
        bg.0 = Color::srgba(0.03, 0.05, 0.09, alpha);
    }
}

/// Smooth tactile hover/press scale feedback applied globally across eligible UI buttons.
#[allow(clippy::type_complexity)]
pub fn animate_button_feedback(
    time: Res<Time>,
    ctx: Res<GameContext>,
    mut buttons: Query<(&Interaction, &mut Transform), (With<Button>, Without<NoButtonScale>)>,
) {
    if ctx.reduced_motion {
        for (_, mut tf) in &mut buttons {
            if (tf.scale.x - 1.0).abs() > 0.001 {
                tf.scale = Vec3::ONE;
            }
        }
        return;
    }
    let blend = (time.delta_secs() * 18.0).clamp(0.0, 1.0);
    for (interaction, mut tf) in &mut buttons {
        let target = match *interaction {
            Interaction::Pressed => 0.96,
            Interaction::Hovered => 1.035,
            Interaction::None => 1.0,
        };
        let next = tf.scale.x + (target - tf.scale.x) * blend;
        tf.scale = Vec3::splat(next);
    }
}
