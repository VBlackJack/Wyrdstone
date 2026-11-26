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

/// Responsive layout mode shared between PC desktop and mobile viewports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HudLayoutMode {
    /// Narrow or vertical screen (e.g. smartphone portrait).
    Portrait,
    /// Wide, short landscape phone screen (e.g. `667x375` or `844x390`).
    PhoneLandscape,
    /// Medium landscape screen (tablet or small desktop window).
    Compact,
    /// Wide desktop screen (`>= 1060px` wide) using full side-rail Match-3 panels.
    #[default]
    DesktopWide,
}

impl HudLayoutMode {
    pub const DESKTOP_WIDE_MIN_WIDTH: f32 = 1060.0;
    pub const PORTRAIT_MAX_WIDTH: f32 = 760.0;
    pub const PHONE_LANDSCAPE_MAX_HEIGHT: f32 = 560.0;
    pub const PHONE_LANDSCAPE_MIN_ASPECT: f32 = 1.25;

    /// Classifies a logical viewport size into the best-fit ergonomics mode.
    pub fn from_viewport(win_w: f32, win_h: f32) -> Self {
        if win_h > win_w {
            Self::Portrait
        } else if win_h < Self::PHONE_LANDSCAPE_MAX_HEIGHT
            && win_w < Self::DESKTOP_WIDE_MIN_WIDTH
            && win_w >= win_h * Self::PHONE_LANDSCAPE_MIN_ASPECT
        {
            Self::PhoneLandscape
        } else if win_w < Self::PORTRAIT_MAX_WIDTH {
            Self::Portrait
        } else if win_w >= Self::DESKTOP_WIDE_MIN_WIDTH {
            Self::DesktopWide
        } else {
            Self::Compact
        }
    }

    pub fn is_portrait(self) -> bool {
        self == Self::Portrait
    }

    pub fn is_phone_landscape(self) -> bool {
        self == Self::PhoneLandscape
    }

    pub fn is_mobile(self) -> bool {
        matches!(self, Self::Portrait | Self::PhoneLandscape)
    }

    pub fn uses_side_rails(self) -> bool {
        matches!(self, Self::DesktopWide | Self::PhoneLandscape)
    }

    pub fn is_compact(self) -> bool {
        self != Self::DesktopWide
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

/// True when the current platform supports toggling fullscreen mode from in-game UI.
#[cfg(not(target_os = "android"))]
pub fn is_fullscreen_supported() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        true
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(func) =
                js_sys::Reflect::get(&win, &JsValue::from_str("wyrdstoneFullscreenSupported"))
            {
                if let Ok(callable) = func.dyn_into::<js_sys::Function>() {
                    if let Ok(res) = callable.call0(&win) {
                        if let Some(supported) = res.as_bool() {
                            return supported;
                        }
                    }
                }
            }
            if let Some(doc) = win.document() {
                return doc.fullscreen_enabled();
            }
        }
        false
    }
}

/// True when the browser document is currently in fullscreen mode on WASM32.
#[cfg(target_arch = "wasm32")]
pub fn is_web_fullscreen() -> bool {
    if let Some(win) = web_sys::window() {
        if let Ok(func) = js_sys::Reflect::get(&win, &JsValue::from_str("wyrdstoneIsFullscreen")) {
            if let Ok(callable) = func.dyn_into::<js_sys::Function>() {
                if let Ok(res) = callable.call0(&win) {
                    if let Some(active) = res.as_bool() {
                        return active;
                    }
                }
            }
        }
        if let Some(doc) = win.document() {
            return doc.fullscreen_element().is_some();
        }
    }
    false
}

/// Requested screen orientation lock when entering browser fullscreen on mobile.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullscreenOrientation {
    Auto,
    Portrait,
    Landscape,
}

#[cfg(target_arch = "wasm32")]
impl FullscreenOrientation {
    pub fn as_js_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Portrait => "portrait",
            Self::Landscape => "landscape",
        }
    }
}

/// True when browser screen orientation locking is available on a touch/coarse-pointer device.
#[cfg(target_arch = "wasm32")]
pub fn is_orientation_lock_supported() -> bool {
    if let Some(win) = web_sys::window() {
        if let Ok(func) =
            js_sys::Reflect::get(&win, &JsValue::from_str("wyrdstoneCanLockOrientation"))
        {
            if let Ok(callable) = func.dyn_into::<js_sys::Function>() {
                if let Ok(res) = callable.call0(&win) {
                    return res.as_bool().unwrap_or(false);
                }
            }
        }
    }
    false
}

/// Enters browser fullscreen and requests the given orientation lock on WASM32.
#[cfg(target_arch = "wasm32")]
pub fn enter_web_fullscreen(orientation: FullscreenOrientation) {
    if let Some(win) = web_sys::window() {
        if let Ok(func) = js_sys::Reflect::get(&win, &JsValue::from_str("wyrdstoneEnterFullscreen"))
        {
            if let Ok(callable) = func.dyn_into::<js_sys::Function>() {
                let _ = callable.call1(&win, &JsValue::from_str(orientation.as_js_str()));
                return;
            }
        }
    }
    toggle_web_fullscreen();
}

/// Exits browser fullscreen and unlocks screen orientation on WASM32.
#[cfg(target_arch = "wasm32")]
pub fn exit_web_fullscreen() {
    if let Some(win) = web_sys::window() {
        if let Ok(func) = js_sys::Reflect::get(&win, &JsValue::from_str("wyrdstoneExitFullscreen"))
        {
            if let Ok(callable) = func.dyn_into::<js_sys::Function>() {
                let _ = callable.call0(&win);
                return;
            }
        }
    }
    toggle_web_fullscreen();
}

/// Toggles browser fullscreen mode via the `web/index.html` Fullscreen API bridge on WASM32.
#[cfg(target_arch = "wasm32")]
pub fn toggle_web_fullscreen() {
    if is_web_fullscreen() {
        exit_web_fullscreen();
    } else {
        enter_web_fullscreen(FullscreenOrientation::Auto);
    }
}

#[cfg(target_arch = "wasm32")]
use web_sys::wasm_bindgen::{JsCast, JsValue};

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
            HudLayoutMode::from_viewport(500.0, 500.0),
            HudLayoutMode::Portrait
        );
        for &(w, h) in &[(667.0, 375.0), (844.0, 390.0), (915.0, 412.0)] {
            let m = HudLayoutMode::from_viewport(w, h);
            assert_eq!(m, HudLayoutMode::PhoneLandscape);
            assert!(m.is_phone_landscape());
            assert!(m.is_mobile());
            assert!(m.uses_side_rails());
            assert!(m.is_compact());
            assert!(!m.is_desktop_wide());
        }
        assert_eq!(
            HudLayoutMode::from_viewport(800.0, 600.0),
            HudLayoutMode::Compact
        );
        assert!(HudLayoutMode::Compact.is_compact());
        assert!(!HudLayoutMode::Compact.is_mobile());
        assert!(!HudLayoutMode::DesktopWide.is_compact());
        assert!(HudLayoutMode::Portrait.is_compact());
        assert!(HudLayoutMode::Portrait.is_mobile());
        assert_eq!(
            HudLayoutMode::from_viewport(1280.0, 540.0),
            HudLayoutMode::DesktopWide
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
