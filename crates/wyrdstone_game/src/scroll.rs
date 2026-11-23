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

/// Preserves scroll offsets across full-tree UI rebuilds and tracks active touch-scroll gestures.
#[derive(Resource, Default)]
pub struct UiScrollMemory {
    pub modal_offset_y: f32,
    pub inspector_offset_y: f32,
    touch_id: Option<u64>,
    touch_last_y: f32,
    touch_scrolling: bool,
}

impl UiScrollMemory {
    pub fn set_offset(&mut self, kind: ScrollPanelKind, val: f32) {
        match kind {
            ScrollPanelKind::Modal => self.modal_offset_y = val,
            ScrollPanelKind::Inspector => self.inspector_offset_y = val,
        }
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
pub fn handle_ui_scroll(
    mut wheel_events: EventReader<MouseWheel>,
    touches: Res<Touches>,
    mut memory: ResMut<UiScrollMemory>,
    mut panels: ScrollPanelQuery,
    child_nodes: Query<(&ComputedNode, &GlobalTransform), Without<ScrollablePanel>>,
) {
    let mut wheel_delta_y = 0.0f32;
    for ev in wheel_events.read() {
        wheel_delta_y += match ev.unit {
            MouseScrollUnit::Line => ev.y * WHEEL_LINE_SCROLL_PX,
            MouseScrollUnit::Pixel => ev.y,
        };
    }

    if wheel_delta_y.abs() > f32::EPSILON {
        for (panel, computed, gtf, rcp, mut scroll, children) in &mut panels {
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
                    if point_inside_node(pos, computed, gtf) {
                        let max_scroll =
                            max_scroll_y(computed, gtf, scroll.offset_y, children, &child_nodes);
                        scroll.offset_y = (scroll.offset_y - dy).clamp(0.0, max_scroll);
                        memory.set_offset(panel.kind, scroll.offset_y);
                    }
                }
            }
        } else {
            memory.touch_id = None;
            memory.touch_scrolling = false;
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
