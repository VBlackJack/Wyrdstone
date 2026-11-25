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

//! Responsive layout breakpoints, mobile safe-area insets, and platform haptic feedback events.

use bevy::prelude::*;

/// Responsive layout mode shared between PC desktop and Android mobile viewports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HudLayoutMode {
    /// Narrow or vertical screen (e.g. Android portrait phone).
    Portrait,
    /// Medium landscape screen (tablet or small desktop window).
    Compact,
    /// Wide desktop screen (`>= 1060px` wide) using side-rail Match-3 panels.
    #[default]
    DesktopWide,
}

impl HudLayoutMode {
    pub const DESKTOP_WIDE_MIN_WIDTH: f32 = 1060.0;
    pub const PORTRAIT_MAX_WIDTH: f32 = 760.0;

    /// Classifies a logical viewport size into the best-fit ergonomics mode.
    pub fn from_viewport(win_w: f32, win_h: f32) -> Self {
        if win_w < Self::PORTRAIT_MAX_WIDTH || win_h > win_w {
            Self::Portrait
        } else if win_w >= Self::DESKTOP_WIDE_MIN_WIDTH {
            Self::DesktopWide
        } else {
            Self::Compact
        }
    }

    pub fn is_compact(self) -> bool {
        self == Self::Portrait
    }

    pub fn is_desktop_wide(self) -> bool {
        self == Self::DesktopWide
    }
}

/// Safe-area padding in logical pixels to clear notches, status bars, and gesture bars.
#[derive(Resource, Debug, Clone, Copy)]
pub struct SafeAreaInsets {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl Default for SafeAreaInsets {
    fn default() -> Self {
        #[cfg(target_os = "android")]
        {
            Self {
                top: 28.0,
                bottom: 20.0,
                left: 0.0,
                right: 0.0,
            }
        }
        #[cfg(not(target_os = "android"))]
        {
            Self {
                top: 0.0,
                bottom: 0.0,
                left: 0.0,
                right: 0.0,
            }
        }
    }
}

/// Semantic tactile feedback cues emitted by gameplay and UI interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HapticKind {
    LightTap,
    MatchClear,
    ComboSurge,
    BoosterBlast,
    Victory,
    Error,
}

/// Event emitted when a tactile pulse should fire on mobile devices.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HapticEvent(pub HapticKind);

/// Consumes semantic haptic events and dispatches platform vibration when supported.
pub fn handle_haptic_events(mut events: EventReader<HapticEvent>) {
    #[cfg(target_os = "android")]
    {
        // Android VibrationEffect bridge hook: kept non-blocking until JNI vibrator wiring is enabled.
        for _ev in events.read() {}
    }
    #[cfg(not(target_os = "android"))]
    {
        events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_layout_mode_classifies_desktop_compact_and_portrait_viewports() {
        assert_eq!(
            HudLayoutMode::from_viewport(360.0, 780.0),
            HudLayoutMode::Portrait
        );
        assert_eq!(
            HudLayoutMode::from_viewport(700.0, 900.0),
            HudLayoutMode::Portrait
        );
        assert_eq!(
            HudLayoutMode::from_viewport(800.0, 600.0),
            HudLayoutMode::Compact
        );
        assert_eq!(
            HudLayoutMode::from_viewport(1280.0, 720.0),
            HudLayoutMode::DesktopWide
        );
        assert_eq!(
            HudLayoutMode::from_viewport(1920.0, 1080.0),
            HudLayoutMode::DesktopWide
        );
        assert_eq!(
            HudLayoutMode::from_viewport(3440.0, 1440.0),
            HudLayoutMode::DesktopWide
        );
    }
}
