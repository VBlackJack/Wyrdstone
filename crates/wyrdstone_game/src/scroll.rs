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

//! Mouse-wheel and touch-drag scrolling for Bevy 0.15 `Overflow::scroll_y()` UI panels.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

const WHEEL_LINE_SCROLL_PX: f32 = 34.0;
const TOUCH_SCROLL_DEADZONE_PX: f32 = 6.0;

/// Identifies which scrollable panel owns a `ScrollPosition` so offsets survive UI rebuilds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollPanelKind {
    Modal,
    Inspector,
}

/// Marker component placed on every `Overflow::scroll_y()` UI container.
#[derive(Component)]
pub struct ScrollablePanel {
    pub kind: ScrollPanelKind,
}

/// Optional footer hint node that automatically displays only when its panel has more content below.
#[derive(Component)]
pub struct ScrollMoreHint {
    pub kind: ScrollPanelKind,
}

/// Visual vertical scrollbar track container (auto-hides when panel fits without scrolling).
#[derive(Component)]
pub struct ScrollBarTrack {
    pub kind: ScrollPanelKind,
}

/// Visual vertical scrollbar thumb positioned proportionally inside its `ScrollBarTrack`.
#[derive(Component)]
pub struct ScrollBarThumb {
    pub kind: ScrollPanelKind,
}

/// Spawns a slim Nordic vertical scrollbar track + proportional thumb next to a `ScrollablePanel`.
pub fn spawn_scrollbar_track(parent: &mut ChildBuilder, kind: ScrollPanelKind) {
    parent
        .spawn((
            ScrollBarTrack { kind },
            Node {
                display: Display::None,
                width: Val::Px(6.0),
                height: Val::Percent(100.0),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.09, 0.14, 0.72)),
            BorderRadius::all(Val::Px(3.0)),
            PickingBehavior::IGNORE,
        ))
        .with_children(|track| {
            track.spawn((
                ScrollBarThumb { kind },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Percent(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(30.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.93, 0.74, 0.29, 0.78)),
                BorderRadius::all(Val::Px(3.0)),
                PickingBehavior::IGNORE,
            ));
        });
}

/// Preserves scroll offsets across full-tree UI rebuilds and tracks active touch-scroll gestures.
#[derive(Resource, Default)]
pub struct UiScrollMemory {
    pub modal_offset_y: f32,
    pub inspector_offset_y: f32,
    touch_id: Option<u64>,
    touch_last_y: f32,
    touch_scrolling: bool,
    suppress_clicks_frames: u8,
}

impl UiScrollMemory {
    pub fn set_offset(&mut self, kind: ScrollPanelKind, val: f32) {
        match kind {
            ScrollPanelKind::Modal => self.modal_offset_y = val,
            ScrollPanelKind::Inspector => self.inspector_offset_y = val,
        }
    }

    /// Returns true while a vertical touch-scroll gesture is active or just ended,
    /// so UI button handlers ignore accidental taps caused by releasing a scroll drag.
    pub fn is_touch_scrolling(&self) -> bool {
        self.touch_scrolling || self.suppress_clicks_frames > 0
    }
}

type ScrollPanelQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ScrollablePanel,
        &'static ComputedNode,
        &'static GlobalTransform,
        &'static RelativeCursorPosition,
        &'static mut ScrollPosition,
        Option<&'static Children>,
    ),
>;

/// Routes mouse-wheel and vertical touch-drag input to hovered `ScrollablePanel` nodes.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn handle_ui_scroll(
    mut wheel_events: EventReader<MouseWheel>,
    touches: Res<Touches>,
    mut memory: ResMut<UiScrollMemory>,
    mut panels: ScrollPanelQuery,
    child_nodes: Query<(&ComputedNode, &GlobalTransform), Without<ScrollablePanel>>,
    mut hints: Query<
        (&ScrollMoreHint, &mut Node),
        (
            Without<ScrollablePanel>,
            Without<ScrollBarTrack>,
            Without<ScrollBarThumb>,
        ),
    >,
    mut tracks: Query<
        (&ScrollBarTrack, &mut Node),
        (
            Without<ScrollablePanel>,
            Without<ScrollMoreHint>,
            Without<ScrollBarThumb>,
        ),
    >,
    mut thumbs: Query<
        (&ScrollBarThumb, &mut Node),
        (
            Without<ScrollablePanel>,
            Without<ScrollMoreHint>,
            Without<ScrollBarTrack>,
        ),
    >,
) {
    if memory.suppress_clicks_frames > 0 && !memory.touch_scrolling {
        memory.suppress_clicks_frames -= 1;
    }

    let mut wheel_delta_y = 0.0f32;
    for ev in wheel_events.read() {
        wheel_delta_y += match ev.unit {
            MouseScrollUnit::Line => ev.y * WHEEL_LINE_SCROLL_PX,
            MouseScrollUnit::Pixel => ev.y,
        };
    }

    let has_modal_panel = panels
        .iter()
        .any(|(panel, ..)| panel.kind == ScrollPanelKind::Modal);

    if wheel_delta_y.abs() > f32::EPSILON {
        for (panel, computed, gtf, rcp, mut scroll, children) in &mut panels {
            if has_modal_panel && panel.kind == ScrollPanelKind::Inspector {
                continue;
            }
            if !rcp.mouse_over() {
                continue;
            }
            let max_scroll = max_scroll_y(computed, gtf, scroll.offset_y, children, &child_nodes);
            scroll.offset_y = (scroll.offset_y - wheel_delta_y).clamp(0.0, max_scroll);
            memory.set_offset(panel.kind, scroll.offset_y);
        }
    }

    // Touch vertical drag scrolling for Android and touchscreens.
    if let Some(touch) = touches.iter_just_pressed().next() {
        memory.touch_id = Some(touch.id());
        memory.touch_last_y = touch.position().y;
        memory.touch_scrolling = false;
    } else if let Some(tid) = memory.touch_id {
        if let Some(touch) = touches.get_pressed(tid) {
            let pos = touch.position();
            let dy = pos.y - memory.touch_last_y;
            if !memory.touch_scrolling && dy.abs() >= TOUCH_SCROLL_DEADZONE_PX {
                memory.touch_scrolling = true;
            }
            if memory.touch_scrolling && dy.abs() > f32::EPSILON {
                memory.touch_last_y = pos.y;
                for (panel, computed, gtf, _, mut scroll, children) in &mut panels {
                    if has_modal_panel && panel.kind == ScrollPanelKind::Inspector {
                        continue;
                    }
                    if point_inside_node(pos, computed, gtf) {
                        let max_scroll =
                            max_scroll_y(computed, gtf, scroll.offset_y, children, &child_nodes);
                        scroll.offset_y = (scroll.offset_y - dy).clamp(0.0, max_scroll);
                        memory.set_offset(panel.kind, scroll.offset_y);
                    }
                }
            }
        } else {
            if memory.touch_scrolling {
                memory.suppress_clicks_frames = 2;
            }
            memory.touch_id = None;
            memory.touch_scrolling = false;
        }
    }

    // Update optional scroll-more hints and visual scrollbar tracks/thumbs.
    if !hints.is_empty() || !tracks.is_empty() {
        let mut modal_metrics: Option<(bool, f32, f32, f32)> = None;
        let mut inspector_metrics: Option<(bool, f32, f32, f32)> = None;
        for (panel, computed, gtf, _, scroll, children) in &panels {
            let max_scroll = max_scroll_y(computed, gtf, scroll.offset_y, children, &child_nodes);
            let viewport_h = (computed.size().y * computed.inverse_scale_factor()).max(1.0);
            let scrollable = max_scroll > 8.0;
            let has_more = scrollable && scroll.offset_y + 8.0 < max_scroll;
            let thumb_pct = if scrollable {
                ((viewport_h / (viewport_h + max_scroll)) * 100.0).clamp(14.0, 96.0)
            } else {
                100.0
            };
            let top_pct = if scrollable {
                ((scroll.offset_y / max_scroll).clamp(0.0, 1.0)) * (100.0 - thumb_pct)
            } else {
                0.0
            };
            let metrics = (
                has_more,
                if scrollable { 1.0 } else { 0.0 },
                thumb_pct,
                top_pct,
            );
            match panel.kind {
                ScrollPanelKind::Modal => modal_metrics = Some(metrics),
                ScrollPanelKind::Inspector => inspector_metrics = Some(metrics),
            }
        }
        for (hint, mut node) in &mut hints {
            let show = match hint.kind {
                ScrollPanelKind::Modal => modal_metrics.map(|m| m.0).unwrap_or(false),
                ScrollPanelKind::Inspector => inspector_metrics.map(|m| m.0).unwrap_or(false),
            };
            node.display = if show { Display::Flex } else { Display::None };
        }
        for (track, mut node) in &mut tracks {
            let show = match track.kind {
                ScrollPanelKind::Modal => modal_metrics.map(|m| m.1 > 0.5).unwrap_or(false),
                ScrollPanelKind::Inspector => inspector_metrics.map(|m| m.1 > 0.5).unwrap_or(false),
            };
            node.display = if show { Display::Flex } else { Display::None };
        }
        for (thumb, mut node) in &mut thumbs {
            let metrics = match thumb.kind {
                ScrollPanelKind::Modal => modal_metrics,
                ScrollPanelKind::Inspector => inspector_metrics,
            };
            if let Some((_, _, thumb_pct, top_pct)) = metrics {
                node.height = Val::Percent(thumb_pct);
                node.top = Val::Percent(top_pct);
            }
        }
    }
}

fn max_scroll_y(
    computed: &ComputedNode,
    gtf: &GlobalTransform,
    current_offset_y: f32,
    children: Option<&Children>,
    child_nodes: &Query<(&ComputedNode, &GlobalTransform), Without<ScrollablePanel>>,
) -> f32 {
    let inv = computed.inverse_scale_factor();
    let container_bottom = (gtf.translation().y + computed.size().y * 0.5) * inv;
    let mut max_child_bottom = container_bottom;
    if let Some(kids) = children {
        for &child in kids {
            if let Ok((c_node, c_gtf)) = child_nodes.get(child) {
                let bottom =
                    (c_gtf.translation().y + c_node.size().y * 0.5) * inv + current_offset_y;
                if bottom > max_child_bottom {
                    max_child_bottom = bottom;
                }
            }
        }
    }
    ((max_child_bottom - container_bottom) + 20.0).max(0.0)
}

fn point_inside_node(pos: Vec2, computed: &ComputedNode, gtf: &GlobalTransform) -> bool {
    let inv = computed.inverse_scale_factor();
    let half = computed.size() * inv * 0.5;
    let center = gtf.translation().truncate() * inv;
    let delta = (pos - center).abs();
    delta.x <= half.x && delta.y <= half.y
}
