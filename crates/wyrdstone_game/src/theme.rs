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

//! Centralized Norse color palette and UI layout constants.

use bevy::prelude::*;
use wyrdstone_core::RuneType;

/// Window defaults for desktop targets.
pub const WINDOW_DEFAULT_WIDTH: f32 = 1360.0;
pub const WINDOW_DEFAULT_HEIGHT: f32 = 860.0;

/// Procedural colorblind badge texture resolution in pixels.
pub const RUNE_TEX_SIZE: usize = 64;
/// High-resolution procedural board overlay texture size in pixels.
pub const OVERLAY_TEX_SIZE: usize = 128;
/// High-resolution procedural VFX sprite texture size in pixels.
pub const VFX_TEX_SIZE: usize = 128;
/// Large procedural VFX texture size for rings, mandalas, and frames.
pub const VFX_LARGE_TEX_SIZE: usize = 256;

/// Board layout ratios and spacing constants.
pub const BOARD_TOP_RESERVED_PX: f32 = 116.0;
pub const BOARD_BOTTOM_RESERVED_PX: f32 = 104.0;
pub const BOARD_SIDE_MARGIN_PX: f32 = 32.0;
pub const TILE_GAP_RATIO: f32 = 0.08;
pub const MAX_TILE_SIZE_PX: f32 = 96.0;
pub const MIN_TILE_SIZE_PX: f32 = 34.0;

/// Z-layer ordering for 2D board sprites.
pub const Z_BOARD_BG: f32 = 0.0;
pub const Z_CELL_BG: f32 = 1.0;
pub const Z_RUNE_SPRITE: f32 = 3.0;
pub const Z_BOOSTER_BADGE: f32 = 4.0;
pub const Z_ICE_TOP_FROST: f32 = 4.5;
pub const Z_CHAIN_OVERLAY: f32 = 5.0;
pub const Z_SELECTION_RING: f32 = 6.0;

/// Typography scale constants.
pub const FONT_TITLE_MD: f32 = 28.0;
pub const FONT_HEADING: f32 = 21.0;
pub const FONT_BODY: f32 = 16.0;
pub const FONT_SMALL: f32 = 13.5;

/// Nordic fjord UI color palette.
pub const COLOR_BG_DEEP: Color = Color::srgb(0.06, 0.09, 0.14);
pub const COLOR_PANEL_DARK: Color = Color::srgb(0.11, 0.15, 0.22);
pub const COLOR_PANEL_CARD: Color = Color::srgb(0.15, 0.20, 0.28);
pub const COLOR_PANEL_BORDER: Color = Color::srgb(0.32, 0.42, 0.54);
pub const COLOR_ACCENT_GOLD: Color = Color::srgb(0.93, 0.74, 0.29);
pub const COLOR_ACCENT_GOLD_HOVER: Color = Color::srgb(0.98, 0.83, 0.44);
pub const COLOR_ACCENT_TEAL: Color = Color::srgb(0.22, 0.62, 0.68);
pub const COLOR_ACCENT_GREEN: Color = Color::srgb(0.24, 0.64, 0.42);
pub const COLOR_ACCENT_CRIMSON: Color = Color::srgb(0.76, 0.24, 0.26);
pub const COLOR_TEXT_PRIMARY: Color = Color::srgb(0.94, 0.95, 0.97);
pub const COLOR_TEXT_MUTED: Color = Color::srgb(0.68, 0.75, 0.84);
pub const COLOR_TEXT_DARK: Color = Color::srgb(0.08, 0.10, 0.14);
pub const COLOR_BUTTON_DISABLED: Color = Color::srgb(0.24, 0.27, 0.32);
/// Text drawn on green or teal accent backgrounds (same value as `COLOR_TEXT_DARK`).
/// WCAG contrast: 5.56:1 on `COLOR_ACCENT_GREEN`, 5.57:1 on `COLOR_ACCENT_TEAL`.
pub const COLOR_TEXT_ON_ACCENT: Color = COLOR_TEXT_DARK;
/// Lighter crimson for text on `COLOR_PANEL_CARD` and `COLOR_PANEL_DARK`.
/// WCAG contrast: 5.00:1 on `COLOR_PANEL_CARD`, 5.94:1 on `COLOR_PANEL_DARK`
/// (the plain `COLOR_ACCENT_CRIMSON` only reaches 2.45:1 and 2.91:1).
pub const COLOR_ACCENT_CRIMSON_TEXT: Color = Color::srgb(0.96, 0.50, 0.50);

/// Minimum logical size in px for tappable controls.
pub const MIN_TOUCH_TARGET_PX: f32 = 44.0;

// Compile-time references keep the shared tokens alive until the UI call sites adopt them.
const _: Color = COLOR_TEXT_ON_ACCENT;
const _: Color = COLOR_ACCENT_CRIMSON_TEXT;
const _: () = assert!(MIN_TOUCH_TARGET_PX > 0.0);

/// Returns RGB tuple `[r, g, b]` in 0..=255 for procedural rune stone rendering.
pub const fn rune_base_rgb(rune: RuneType) -> [u8; 3] {
    match rune {
        RuneType::Fire => [204, 62, 46],    // Muspel crimson
        RuneType::Ice => [236, 234, 228],   // Niflheim pearl-silver (distinct from cyan ice blocks)
        RuneType::Wood => [62, 168, 92],    // Yggdrasil emerald
        RuneType::Stone => [138, 128, 168], // Midgard slate violet
        RuneType::Gold => [232, 178, 46],   // Asgard solar gold
        RuneType::Water => [52, 108, 212],  // Fjord deep blue
    }
}
